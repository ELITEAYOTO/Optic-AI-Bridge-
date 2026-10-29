//! One owned synchronous pipe reader. Cancellation requests are never joins.
use std::{
    fs::File,
    io::{self, Read},
    os::windows::io::{AsHandle, AsRawHandle},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, ReadBuf},
    sync::mpsc,
};
use windows::Win32::{
    Foundation::{ERROR_BROKEN_PIPE, ERROR_NOT_FOUND, ERROR_OPERATION_ABORTED, HANDLE},
    Storage::FileSystem::{FILE_TYPE_PIPE, GetFileType},
    System::IO::CancelSynchronousIo,
};

const CHUNK: usize = 8 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(5);
type Frame = io::Result<Vec<u8>>;
static STDIN_CLAIMED: AtomicBool = AtomicBool::new(false);

pub struct PipeStdinReader {
    receive: mpsc::Receiver<Frame>,
    chunk: Vec<u8>,
    offset: usize,
    ended: bool,
}

/// Retain this owner through original session/admission cleanup, then join it.
/// Drop also cancels and joins; an uncertain read is never detached or timed out.
pub struct PipeStdinOwner {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

pub fn start_pipe_stdin() -> io::Result<(PipeStdinReader, PipeStdinOwner)> {
    if STDIN_CLAIMED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "process stdin reader already claimed",
        ));
    }
    let input = io::stdin().as_handle().try_clone_to_owned()?;
    start_reader(File::from(input))
}

fn start_reader<R: Read + AsHandle + Send + 'static>(
    input: R,
) -> io::Result<(PipeStdinReader, PipeStdinOwner)> {
    // SAFETY: input owns a live handle; this query borrows it without transfer.
    if unsafe { GetFileType(HANDLE(input.as_handle().as_raw_handle())) } != FILE_TYPE_PIPE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Windows MCP stdin requires a synchronous pipe",
        ));
    }
    let (send, receive) = mpsc::channel(1);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker = thread::Builder::new()
        .name("optic-stdin-reader".into())
        .stack_size(256 * 1024)
        .spawn(move || read_loop(input, send, worker_stop))?;
    Ok((
        PipeStdinReader {
            receive,
            chunk: Vec::new(),
            offset: 0,
            ended: false,
        },
        PipeStdinOwner {
            stop,
            worker: Some(worker),
        },
    ))
}

fn send_frame(send: &mpsc::Sender<Frame>, mut frame: Frame, stop: &AtomicBool) -> bool {
    loop {
        if stop.load(Ordering::Acquire) {
            return false;
        }
        match send.try_send(frame) {
            Ok(()) => return true,
            Err(mpsc::error::TrySendError::Closed(_)) => return false,
            Err(mpsc::error::TrySendError::Full(value)) => {
                frame = value;
                thread::sleep(POLL_INTERVAL);
            }
        }
    }
}

fn read_loop<R: Read>(
    mut input: R,
    send: mpsc::Sender<Frame>,
    stop: Arc<AtomicBool>,
) -> io::Result<()> {
    while !stop.load(Ordering::Acquire) {
        // At most three 8KiB payloads: reader's current chunk, queued frame,
        // and this producer's read/pending frame. No unbounded spool or queue.
        let mut bytes = vec![0; CHUNK];
        match input.read(&mut bytes) {
            Ok(0) => return Ok(()),
            Ok(count) => {
                bytes.truncate(count);
                if !send_frame(&send, Ok(bytes), &stop) {
                    return Ok(());
                }
            }
            Err(error) if error.raw_os_error() == Some(ERROR_BROKEN_PIPE.0 as i32) => return Ok(()),
            Err(error)
                if stop.load(Ordering::Acquire)
                    && error.raw_os_error() == Some(ERROR_OPERATION_ABORTED.0 as i32) =>
            {
                return Ok(());
            }
            Err(error) => {
                let copy = error
                    .raw_os_error()
                    .map(io::Error::from_raw_os_error)
                    .unwrap_or_else(|| io::Error::new(error.kind(), error.to_string()));
                send_frame(&send, Err(copy), &stop);
                return Err(error);
            }
        }
    }
    Ok(())
}

impl AsyncRead for PipeStdinReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            if this.offset < this.chunk.len() {
                let count = output.remaining().min(this.chunk.len() - this.offset);
                output.put_slice(&this.chunk[this.offset..this.offset + count]);
                this.offset += count;
                return Poll::Ready(Ok(()));
            }
            this.chunk = Vec::new();
            this.offset = 0;
            if this.ended {
                return Poll::Ready(Ok(()));
            }
            match this.receive.poll_recv(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    this.ended = true;
                    return Poll::Ready(Ok(()));
                }
                Poll::Ready(Some(Err(error))) => {
                    this.ended = true;
                    return Poll::Ready(Err(error));
                }
                Poll::Ready(Some(Ok(chunk))) => this.chunk = chunk,
            }
        }
    }
}

impl PipeStdinOwner {
    pub fn stop_and_join(mut self) -> io::Result<()> {
        self.finish()
    }

    fn finish(&mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Release);
        let mut cancellation_error = None;
        if let Some(worker) = self.worker.as_ref() {
            while !worker.is_finished() {
                // SAFETY: the live, uniquely retained JoinHandle owns this exact
                // dedicated reader thread. It performs no unrelated IO. Never
                // cancel a pooled thread or release its handle before join.
                if let Err(error) =
                    unsafe { CancelSynchronousIo(HANDLE(worker.as_handle().as_raw_handle())) }
                    && error.code() != windows::core::HRESULT::from_win32(ERROR_NOT_FOUND.0)
                    && cancellation_error.is_none()
                {
                    cancellation_error = Some(io::Error::other(error));
                }
                // Retry closes the stop-check/enter-ReadFile race, including
                // ERROR_NOT_FOUND before that read is actually pending. A cancel
                // request alone is not completion; retain owner until real join.
                thread::sleep(POLL_INTERVAL);
            }
        }
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| io::Error::other("stdin reader panicked"))??;
        }
        cancellation_error.map_or(Ok(()), Err)
    }
}

impl Drop for PipeStdinOwner {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn pipe_stdin_preserves_raw_bytes_eof_and_real_join() {
        let (reader, mut writer) = io::pipe().unwrap();
        let (mut reader, owner) = start_reader(reader).unwrap();
        let expected = b"a\0\xff\xc3\xa9\xe9\x9b\xaa\n";
        writer.write_all(expected).unwrap();
        drop(writer);
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_secs(2), reader.read_to_end(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bytes, expected);
        owner.stop_and_join().unwrap();
    }

    #[test]
    fn pipe_stdin_joins_without_writer_eof_including_early_stop() {
        for pause in [false, true] {
            let (reader, mut writer) = io::pipe().unwrap();
            let (_reader, owner) = start_reader(reader).unwrap();
            if pause {
                thread::sleep(Duration::from_millis(20));
            }
            owner.stop_and_join().unwrap();
            assert!(
                writer.write_all(b"x").is_err(),
                "joined reader must have closed its pipe handle"
            );
        }
    }

    #[test]
    fn pipe_stdin_drop_joins_under_backpressure() {
        let (reader, mut writer) = io::pipe().unwrap();
        let (_reader, owner) = start_reader(reader).unwrap();
        let writing = thread::spawn(move || writer.write_all(&vec![b'x'; CHUNK * 8]));
        thread::sleep(Duration::from_millis(20));
        drop(owner); // Must cancel ReadFile or leave try_send backpressure, then join.
        let _ = writing.join().unwrap();
    }

    #[test]
    fn pipe_stdin_rejects_non_pipe_before_spawning() {
        assert!(start_reader(File::open("NUL").unwrap()).is_err());
    }
}
