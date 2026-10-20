//! Bounded stdout/stderr storage and owned pipe drains.
//! Kept private to the process module; admission, cursors and lifecycle remain
//! with the owner. Both streams consume the same reservation.
use std::sync::{Arc, atomic::Ordering};
use tokio::io::{AsyncRead, AsyncReadExt};

use super::{JobRecord, ProcessStream};

const DRAIN_CHUNK_BYTES: usize = 8 * 1024;

#[derive(Debug)]
pub(super) struct OutputState {
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) stdout_closed: bool,
    pub(super) stderr_closed: bool,
}

pub(super) async fn drain_output<R>(mut reader: R, stream: ProcessStream, record: Arc<JobRecord>)
where
    R: AsyncRead + Unpin,
{
    let mut chunk = [0_u8; DRAIN_CHUNK_BYTES];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) => break,
            Ok(read) => append_output(&record, stream, &chunk[..read]),
            Err(_) => {
                // A pipe read failure is not clean EOF. Preserve the diagnostic
                // uncertainty even when the child exits with code zero.
                record.output_read_failed.store(true, Ordering::Release);
                break;
            },
        }
    }
    mark_stream_closed(&record, stream);
}

pub(super) fn append_output(record: &JobRecord, stream: ProcessStream, bytes: &[u8]) {
    let Ok(mut output) = record.output.lock() else {
        record.output_overflow.store(true, Ordering::Release);
        return;
    };
    let used = output.stdout.len().saturating_add(output.stderr.len());
    let budget = usize::try_from(record.reserved_output_bytes).unwrap_or(usize::MAX);
    let remaining = budget.saturating_sub(used);
    let keep = remaining.min(bytes.len());
    let target = match stream {
        ProcessStream::Stdout => &mut output.stdout,
        ProcessStream::Stderr => &mut output.stderr,
    };
    target.extend_from_slice(&bytes[..keep]);
    if keep < bytes.len() {
        record.output_overflow.store(true, Ordering::Release);
    }
}

pub(super) fn mark_stream_closed(record: &JobRecord, stream: ProcessStream) {
    if let Ok(mut output) = record.output.lock() {
        match stream {
            ProcessStream::Stdout => output.stdout_closed = true,
            ProcessStream::Stderr => output.stderr_closed = true,
        }
    }
}
