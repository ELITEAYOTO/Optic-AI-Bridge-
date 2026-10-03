#![forbid(unsafe_code)]

//! Runtime services for the Phase 1 Optic AI Bridge vertical slice.
//!
//! This crate owns application runtime state and hard limits. MCP remains an
//! adapter outside this boundary, so SDK defaults cannot silently weaken
//! session, transport, filesystem, or output constraints.

mod clock;
mod filesystem;
mod session_registry;
mod transport;

pub use clock::{Clock, StdClock};
pub use filesystem::{
    BoundedFileSystem, DirectoryEntry, EntryKind, FileSystemError, FsListPage, FsReadChunk,
};
pub use session_registry::{SessionRegistry, SessionRegistryError};
pub use transport::{RequestPermit, TransportError, TransportGuard, TransportLimits};
