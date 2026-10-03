#![forbid(unsafe_code)]

//! Runtime services for Optic AI Bridge.
//!
//! This crate owns application runtime state and hard limits. MCP remains an
//! adapter outside this boundary, so SDK defaults cannot silently weaken
//! session, transport, filesystem, process, mutation-precondition, or output constraints.

mod clock;
mod filesystem;
mod process;
mod session_registry;
mod task_lease_registry;
mod transport;

pub use clock::{Clock, StdClock};
pub use filesystem::{
    BoundedFileSystem, DirectoryEntry, EntryKind, FileSystemError, FsListPage, FsReadChunk,
    MutationObservation,
};
pub use process::{
    ProcessError, ProcessManager, ProcessReadChunk, ProcessResult, ProcessStartSpec, ProcessStatus,
    ProcessStream,
};
pub use session_registry::{SessionRegistry, SessionRegistryError};
pub use task_lease_registry::{TaskLeaseRegistry, TaskLeaseRegistryError};
pub use transport::{RequestPermit, TransportError, TransportGuard, TransportLimits};
