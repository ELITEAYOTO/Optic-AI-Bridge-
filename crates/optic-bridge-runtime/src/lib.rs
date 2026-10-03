#![forbid(unsafe_code)]

//! Runtime services for Optic AI Bridge.
//!
//! This crate owns application runtime state and hard limits. MCP remains an
//! adapter outside this boundary, so SDK defaults cannot silently weaken
//! session, transport, filesystem, process, mutation-precondition, recovery,
//! or output constraints.

mod authorized_file_mutation;
mod clock;
mod filesystem;
mod journaled_mutation;
mod mutation;
mod mutation_authority;
mod process;
mod recovery;
mod session_registry;
mod task_lease_registry;
mod transactional_file;
mod transport;

pub use authorized_file_mutation::{AuthorizedFileMutationError, AuthorizedFileMutationService};
pub use clock::{Clock, StdClock};
pub use filesystem::{
    BoundedFileSystem, DirectoryEntry, EntryKind, FileSystemError, FsListPage, FsReadChunk,
    MutationError, MutationObservation,
};
pub use journaled_mutation::{
    JournaledDeleteCommit, JournaledMutationCommit, JournaledMutationError,
    JournaledMutationService,
};
pub use mutation::{
    AtomicMutationError, AtomicMutationService, CommitVerification, DeleteCommit,
    DeleteVerification, MutationCommit, PreparedDelete, PreparedMutation,
};
pub use mutation_authority::{MutationAuthorityError, MutationAuthoritySet, MutationAuthoritySpec};
pub use process::{
    ProcessError, ProcessManager, ProcessReadChunk, ProcessResult, ProcessStartSpec, ProcessStatus,
    ProcessStream,
};
pub use recovery::{
    JournalTicket, MutationRecoveryJournal, RecoveryJournalError, RecoveryOutcome, RecoveryRecord,
    RecoveryReport,
};
pub use session_registry::{SessionRegistry, SessionRegistryError};
pub use task_lease_registry::{TaskLeaseRegistry, TaskLeaseRegistryError};
pub use transactional_file::{BytePatch, TransactionalFileError, TransactionalFileService};
pub use transport::{RequestPermit, TransportError, TransportGuard, TransportLimits};
