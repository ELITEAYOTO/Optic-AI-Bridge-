//! Process-owned OS listeners; no MCP request can supply or reset this source.

use std::io;

pub(super) async fn requested() -> io::Result<()> {
    #[cfg(windows)]
    let (mut interrupt, mut terminate) = (
        tokio::signal::windows::ctrl_c()?,
        tokio::signal::windows::ctrl_break()?,
    );
    #[cfg(unix)]
    let (mut interrupt, mut terminate) = (
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?,
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?,
    );
    #[cfg(any(windows, unix))]
    {
        let event = tokio::select! {
            event = interrupt.recv() => event,
            event = terminate.recv() => event,
        };
        event.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "host shutdown signal stream closed",
            )
        })
    }
    #[cfg(not(any(windows, unix)))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "host shutdown signals are unsupported",
        ))
    }
}
