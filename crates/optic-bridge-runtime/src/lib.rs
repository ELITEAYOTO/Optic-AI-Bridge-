#![forbid(unsafe_code)]

//! Runtime services for Optic AI Bridge.
//!
//! This crate owns application runtime state and hard limits. MCP remains an
//! adapter outside this boundary, so SDK defaults cannot silently weaken
//! session, transport, filesystem, Git, process, mutation-precondition,
//! recovery, or output constraints.

mod approval_broker;
mod authorized_file_mutation;
mod authorized_git_integration;
mod blocking_io;
mod clock;
mod filesystem;
mod git_integrate;
mod git_integration_authority;
mod git_read;
mod hardened_command;
mod journaled_mutation;
mod mutation;
mod mutation_authority;
mod process;
mod process_authority;
mod read_authority;
mod recovery;
mod session_lifecycle;
mod session_registry;
mod task_lease_registry;
mod tool_profile_registry;
mod transactional_file;
mod transport;
#[cfg(windows)]
mod windows_path;

pub use approval_broker::{ApprovalBroker, ApprovalBrokerError, ApprovalSpec};
pub use authorized_file_mutation::{AuthorizedFileMutationError, AuthorizedFileMutationService};
pub use authorized_git_integration::{
    AuthorizedGitIntegrationError, AuthorizedGitIntegrationService,
};
pub use blocking_io::{BlockingIoError, BlockingIoGovernor};
pub use clock::{Clock, StdClock};
pub use filesystem::{
    BoundedFileSystem, DirectoryEntry, EntryKind, FileSystemError, FsListPage, FsReadChunk,
    MutationError, MutationObservation,
};
pub use git_integrate::{
    GitIntegrationError, GitIntegrationMode, GitIntegrationRecoveryReport, GitIntegrationResult,
    GitIntegrationService,
};
pub use git_integration_authority::{
    GitIntegrationAuthorityError, GitIntegrationAuthoritySet, GitIntegrationAuthoritySpec,
    git_integration_resource_budget,
};
pub use git_read::{
    GitDiffSnapshot, GitLogCursor, GitLogEntry, GitLogPage, GitReadError, GitReadService,
    GitStatusSnapshot,
};
pub use hardened_command::{
    HardenedCommandError, HardenedCommandOutput, HardenedCommandRunner, HardenedCommandSpec,
};
pub use journaled_mutation::{
    JournaledDeleteCommit, JournaledMutationCommit, JournaledMutationError,
    JournaledMutationService,
};
pub use mutation::{
    AtomicMutationError, AtomicMutationService, CommitVerification, DeleteCommit,
    DeleteVerification, MutationCommit, PreparedDelete, PreparedMutation,
};
pub use mutation_authority::{
    MutationAuthorityError, MutationAuthoritySet, MutationAuthoritySpec, mutation_resource_budget,
};
pub use process::{
    EnvironmentGrant, EnvironmentVariableClass, ProcessError, ProcessManager, ProcessReadChunk,
    ProcessResult, ProcessStartSpec, ProcessStatus, ProcessStream,
};
pub use process_authority::{ProcessAuthority, ProcessAuthorityError, ProcessExecutableIdentity};
pub use read_authority::{
    ReadAuthorityError, ReadAuthoritySet, ReadAuthoritySpec, read_authority_resource_budget,
};
pub use recovery::{
    JournalTicket, MutationRecoveryJournal, RecoveryJournalError, RecoveryOutcome, RecoveryRecord,
    RecoveryReport,
};
pub use session_lifecycle::{
    SessionGrantSpec, SessionLifecycleError, SessionLifecycleManager, SessionReapReport,
    SessionRevokeReport,
};
pub use session_registry::{SessionAdmissionPermit, SessionRegistry, SessionRegistryError};
pub use task_lease_registry::{TaskLeaseRegistry, TaskLeaseRegistryError};
pub use tool_profile_registry::{ToolProfileRegistry, ToolProfileRegistryError};
pub use transactional_file::{BytePatch, TransactionalFileError, TransactionalFileService};
pub use transport::{RequestPermit, TransportError, TransportGuard, TransportLimits};
