//! Single-reader bounded native polling; no temporary stdout/stderr spool.
use std::{
    io::{self, PipeReader, PipeWriter, Read},
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle},
};
use windows::Win32::{
    Foundation::{ERROR_BROKEN_PIPE, HANDLE},
    System::Pipes::PeekNamedPipe,
};

pub struct CapturedOutputPipe {
    reader: PipeReader,
    writer: Option<PipeWriter>,
}

#[derive(Debug)]
pub struct CapturedOutputChunk {
    pub bytes: Vec<u8>,
    pub eof: bool,
}

impl CapturedOutputPipe {
    pub fn new() -> io::Result<Self> {
        let (reader, writer) = io::pipe()?;
        Ok(Self {
            reader,
            writer: Some(writer),
        })
    }

    pub fn writer_handle(&self) -> io::Result<BorrowedHandle<'_>> {
        self.writer
            .as_ref()
            .map(AsHandle::as_handle)
            .ok_or_else(|| io::Error::other("parent output writer already closed"))
    }

    /// Call after process creation duplicated only the writer into the child.
    pub fn close_parent_writer(&mut self) {
        self.writer.take();
    }

    pub fn try_read(&mut self, max_bytes: usize) -> io::Result<CapturedOutputChunk> {
        if !(1..=64 * 1024).contains(&max_bytes) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output chunk requires 1..=65536 bytes",
            ));
        }
        let mut available = 0_u32;
        // SAFETY: private uniquely owned anonymous-pipe reader, never exposed
        // or inherited. No other reader can consume the reported available data.
        if let Err(error) = unsafe {
            PeekNamedPipe(
                HANDLE(self.reader.as_raw_handle()),
                None,
                0,
                None,
                Some(&mut available),
                None,
            )
        } {
            if error.code() == windows::core::HRESULT::from_win32(ERROR_BROKEN_PIPE.0) {
                return Ok(CapturedOutputChunk {
                    bytes: Vec::new(),
                    eof: true,
                });
            }
            return Err(io::Error::other(error));
        }
        if available == 0 {
            return Ok(CapturedOutputChunk {
                bytes: Vec::new(),
                eof: false,
            });
        }
        let mut bytes = vec![0; max_bytes.min(available as usize)];
        let read = self.reader.read(&mut bytes)?;
        bytes.truncate(read);
        Ok(CapturedOutputChunk {
            bytes,
            eof: read == 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bounded_pipe_distinguishes_idle_data_and_confirmed_eof() {
        let mut pipe = CapturedOutputPipe::new().unwrap();
        let idle = pipe.try_read(4).unwrap();
        assert!(idle.bytes.is_empty() && !idle.eof);
        pipe.writer.as_mut().unwrap().write_all(b"abcdef").unwrap();
        pipe.close_parent_writer();
        assert!(pipe.writer_handle().is_err());
        assert_eq!(pipe.try_read(4).unwrap().bytes, b"abcd");
        assert_eq!(pipe.try_read(4).unwrap().bytes, b"ef");
        assert!(pipe.try_read(4).unwrap().eof);
        assert!(pipe.try_read(0).is_err());
        assert!(pipe.try_read(65537).is_err());
    }
}
