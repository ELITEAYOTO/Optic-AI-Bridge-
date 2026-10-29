//! Owned synchronous stdout for pipes and redirected files; cancel is not join.
use std::{
    fs::File,
    future::Future,
    io::{self, Write},
    os::windows::io::{AsHandle, AsRawHandle},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    task::{Context, Poll},
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::{io::AsyncWrite, sync::oneshot};
use windows::Win32::{
    Foundation::{ERROR_NOT_FOUND, ERROR_OPERATION_ABORTED, HANDLE},
    Storage::FileSystem::{FILE_TYPE_DISK, FILE_TYPE_PIPE, GetFileType},
    System::IO::CancelSynchronousIo,
};

const CHUNK: usize = 8 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(5);
static STDOUT_CLAIMED: AtomicBool = AtomicBool::new(false);

struct Frame {
    bytes: Vec<u8>,
    completed: oneshot::Sender<io::Result<()>>,
}

/// Buffered AsyncWrite: accepting bytes is not a claim of physical delivery.
/// Flush awaits the actual write result before another frame can be accepted.
pub struct OwnedStdoutWriter {
    send: Option<mpsc::SyncSender<Frame>>,
    pending: Option<oneshot::Receiver<io::Result<()>>>,
    stop: Arc<AtomicBool>,
    failed: bool,
}

/// Keep through original admission drainage, then cancel AND join this thread.
/// Drop also joins. Uncertain IO never authorizes detachment or a fake success.
pub struct OwnedStdoutOwner {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

pub fn start_owned_stdout() -> io::Result<(OwnedStdoutWriter, OwnedStdoutOwner)> {
    if STDOUT_CLAIMED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "process stdout writer already claimed",
        ));
    }
    let output = io::stdout().as_handle().try_clone_to_owned()?;
    start_writer(File::from(output))
}

fn start_writer<W: Write + AsHandle + Send + 'static>(
    output: W,
) -> io::Result<(OwnedStdoutWriter, OwnedStdoutOwner)> {
    // SAFETY: the query borrows this live owned handle without transferring it.
    let kind = unsafe { GetFileType(HANDLE(output.as_handle().as_raw_handle())) };
    if kind != FILE_TYPE_PIPE && kind != FILE_TYPE_DISK {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Windows MCP stdout requires a synchronous pipe or redirected disk file",
        ));
    }
    let (send, receive) = mpsc::sync_channel(1);
    crate::stdio_mode::require_synchronous(&output)?;
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker = thread::Builder::new()
        .name("optic-stdout-writer".into())
        .stack_size(256 * 1024)
        .spawn(move || write_loop(output, receive, worker_stop))?;
    Ok((
        OwnedStdoutWriter {
            send: Some(send),
            pending: None,
            stop: Arc::clone(&stop),
            failed: false,
        },
        OwnedStdoutOwner {
            stop,
            worker: Some(worker),
        },
    ))
}

fn copy_error(error: &io::Error) -> io::Error {
    error
        .raw_os_error()
        .map(io::Error::from_raw_os_error)
        .unwrap_or_else(|| io::Error::new(error.kind(), error.to_string()))
}

fn write_frame(output: &mut impl Write, bytes: &[u8], stop: &AtomicBool) -> io::Result<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        if stop.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "stdout stopped before remaining bytes",
            ));
        }
        // Do not use write_all: its automatic Interrupted retry could enter
        // another WriteFile after stop/cancellation. Recheck before each write.
        match output.write(&bytes[offset..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "stdout made no progress",
                ));
            }
            Ok(count) => offset += count,
            Err(error)
                if error.kind() == io::ErrorKind::Interrupted && !stop.load(Ordering::Acquire) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn write_loop(
    mut output: impl Write,
    receive: mpsc::Receiver<Frame>,
    stop: Arc<AtomicBool>,
) -> io::Result<()> {
    while !stop.load(Ordering::Acquire) {
        let frame = match receive.recv_timeout(POLL_INTERVAL) {
            Ok(frame) => frame,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        };
        let result = write_frame(&mut output, &frame.bytes, &stop);
        let cancelled = result.as_ref().is_err_and(|error| {
            stop.load(Ordering::Acquire)
                && (error.kind() == io::ErrorKind::Interrupted
                    || error.raw_os_error() == Some(ERROR_OPERATION_ABORTED.0 as i32))
        });
        let _ = frame
            .completed
            .send(result.as_ref().map_err(copy_error).copied());
        if cancelled {
            return Ok(());
        } // Write returned: cancellation has completed.
        result?;
    }
    Ok(())
}

impl OwnedStdoutWriter {
    fn poll_completed(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdout writer failed",
            )));
        }
        let Some(pending) = self.pending.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        let result = match Pin::new(pending).poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(result) => result.unwrap_or_else(|_| {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "stdout writer stopped",
                ))
            }),
        };
        self.pending = None;
        if result.is_err() {
            self.failed = true;
            self.send = None;
        }
        Poll::Ready(result)
    }
}

impl AsyncWrite for OwnedStdoutWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        std::task::ready!(this.poll_completed(cx))?;
        if this.stop.load(Ordering::Acquire) {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdout stopped",
            )));
        }
        let Some(send) = &this.send else {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdout closed",
            )));
        };
        if bytes.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let count = bytes.len().min(CHUNK);
        let (completed, pending) = oneshot::channel();
        // Exactly one unacknowledged frame, one bounded channel slot. At most
        // two CHUNK allocations overlap while a completed frame is dropped.
        if send
            .try_send(Frame {
                bytes: bytes[..count].to_vec(),
                completed,
            })
            .is_err()
        {
            this.failed = true;
            this.send = None;
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdout writer unavailable",
            )));
        }
        this.pending = Some(pending);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().poll_completed(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        std::task::ready!(this.poll_completed(cx))?;
        this.send = None;
        Poll::Ready(Ok(()))
    }
}

impl OwnedStdoutOwner {
    pub fn stop_and_join(mut self) -> io::Result<()> {
        self.finish()
    }

    fn finish(&mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Release);
        let mut cancellation_error = None;
        if let Some(worker) = self.worker.as_ref() {
            while !worker.is_finished() {
                // SAFETY: this retained JoinHandle owns the exact dedicated
                // output thread, which performs no unrelated synchronous IO.
                // No pooled thread, caller buffer or borrowed handle is used.
                if let Err(error) =
                    unsafe { CancelSynchronousIo(HANDLE(worker.as_handle().as_raw_handle())) }
                    && error.code() != windows::core::HRESULT::from_win32(ERROR_NOT_FOUND.0)
                    && cancellation_error.is_none()
                {
                    cancellation_error = Some(io::Error::other(error));
                }
                // Retry the stop-check/write-entry race; ERROR_NOT_FOUND can
                // precede an entering write. A request is never treated as join.
                thread::sleep(POLL_INTERVAL);
            }
        }
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| io::Error::other("stdout writer panicked"))??;
        }
        cancellation_error.map_or(Ok(()), Err)
    }
}

impl Drop for OwnedStdoutOwner {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn pipe_stdout_preserves_bytes_flush_shutdown_and_real_join() {
        let (mut reader, writer) = io::pipe().unwrap();
        let (mut writer, owner) = start_writer(writer).unwrap();
        let expected = [
            b"a\0\xff\xc3\xa9\xe9\x9b\xaa\n".as_slice(),
            &vec![b'x'; CHUNK * 3 + 9],
        ]
        .concat();
        let reading = thread::spawn(move || {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).unwrap();
            bytes
        });
        writer.write_all(&expected).await.unwrap();
        writer.flush().await.unwrap();
        writer.shutdown().await.unwrap();
        owner.stop_and_join().unwrap();
        assert_eq!(reading.join().unwrap(), expected);
        assert!(writer.write_all(b"after shutdown").await.is_err());
    }

    #[tokio::test]
    async fn pipe_stdout_cancel_and_drop_join_without_closing_or_draining_reader() {
        for drop_owner in [false, true] {
            let (mut reader, writer) = io::pipe().unwrap();
            let (mut writer, owner) = start_writer(writer).unwrap();
            let writing = tokio::spawn(async move {
                writer.write_all(&vec![b'x'; CHUNK * 16]).await?;
                writer.flush().await
            });
            tokio::time::sleep(Duration::from_millis(30)).await;
            assert!(
                !writing.is_finished(),
                "unread pipe must backpressure the writer"
            );
            tokio::task::spawn_blocking(move || {
                if drop_owner {
                    drop(owner);
                } else {
                    owner.stop_and_join().unwrap();
                }
            })
            .await
            .unwrap();
            assert!(
                writing.await.unwrap().is_err(),
                "cancelled output must not report flushed delivery"
            );
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).unwrap(); // EOF after join, not its prerequisite.
            assert!(bytes.len() < CHUNK * 16);
        }
    }

    #[tokio::test]
    async fn pipe_stdout_early_idle_stop_joins_and_rejects_later_writes() {
        let (_reader, writer) = io::pipe().unwrap();
        let (mut writer, owner) = start_writer(writer).unwrap();
        owner.stop_and_join().unwrap();
        assert!(writer.write_all(b"late").await.is_err());
    }

    #[test]
    fn stdout_rejects_character_handle_before_spawning() {
        assert!(start_writer(File::options().write(true).open("NUL").unwrap()).is_err());
    }

    #[tokio::test]
    async fn stdout_broken_reader_errors_reach_flush_and_owner_join() {
        let (reader, writer) = io::pipe().unwrap();
        drop(reader);
        let (mut writer, owner) = start_writer(writer).unwrap();
        writer.write_all(b"not delivered").await.unwrap(); // Buffer acceptance only.
        assert!(writer.flush().await.is_err());
        assert!(owner.stop_and_join().is_err());
    }

    #[tokio::test]
    async fn stdout_preserves_redirected_disk_output() {
        let name = format!(
            "optic-stdout-{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(name);
        let file = File::options()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        let (mut writer, owner) = start_writer(file).unwrap();
        writer
            .write_all(b"redirected output\0\xff\n")
            .await
            .unwrap();
        writer.flush().await.unwrap();
        writer.shutdown().await.unwrap();
        owner.stop_and_join().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"redirected output\0\xff\n");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn stdout_rejects_overlapped_handle_before_spawning() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::Storage::FileSystem::FILE_FLAG_OVERLAPPED;
        let path = std::env::temp_dir().join(format!(
            "optic-stdout-overlapped-{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = File::options()
            .create_new(true)
            .write(true)
            .custom_flags(FILE_FLAG_OVERLAPPED.0)
            .open(&path)
            .unwrap();
        // Never submit IO to this async handle through the synchronous worker.
        let rejected = start_writer(file).is_err();
        std::fs::remove_file(path).unwrap();
        assert!(rejected, "synchronous writer accepted an overlapped handle");
    }
}
