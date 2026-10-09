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
mod git_blob_batch;
mod git_change_set;
mod git_integrate;
mod git_integration_authority;
mod git_merge_commit;
mod git_merge_plan;
mod git_read;
mod git_tree_manifest;
mod git_tree_write;
mod git_worktree;
mod hardened_command;
mod journaled_mutation;
mod mutation;
mod mutation_authority;
mod process;
mod process_authority;
mod read_authority;
mod recovery;
mod same_repo_coordinator;
mod same_repo_merge;
mod session_lifecycle;
mod session_registry;
mod session_worktree;
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
    MutationError, MutationObservation, PreparedFileRead, ReadTargetObservation,
};
pub use git_blob_batch::{GitBlobBatchError, SessionBlob, SessionBlobBatch};
pub use git_change_set::{
    SessionChange, SessionChangeKind, SessionChangeSet, SessionConflict, SessionConflictError,
    SessionConflictKind, SessionConflictReport,
};
pub use git_integrate::{
    GitIntegrationError, GitIntegrationMode, GitIntegrationRecoveryReport, GitIntegrationResult,
    GitIntegrationService,
};
pub use git_integration_authority::{
    GitIntegrationAuthorityError, GitIntegrationAuthoritySet, GitIntegrationAuthoritySpec,
    git_integration_resource_budget,
};
pub use git_merge_commit::{SessionMergeCommit, SessionMergeCommitError};
pub use git_merge_plan::{SessionMergePlan, SessionMergePlanError};
pub use git_read::{
    GitDiffSnapshot, GitLogCursor, GitLogEntry, GitLogPage, GitReadError, GitReadService,
    GitStatusSnapshot,
};
pub use git_tree_manifest::{
    GitTreeEntry, GitTreeEntryKind, GitTreeManifest, GitTreeManifestError,
};
pub use git_tree_write::{SessionMergeTree, SessionMergeTreeError};
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
pub use same_repo_coordinator::{
    CoordinatedSession, SameRepositoryCoordinatorError, SameRepositorySessionCoordinator,
};
pub use same_repo_merge::{
    SameRepositoryMergePublishError, SameRepositoryMergePublisher, SessionMergePublishOutcome,
};
pub use session_lifecycle::{
    QuiescentSessionSeal, SessionGrantSpec, SessionLifecycleError, SessionLifecycleManager,
    SessionReapReport, SessionRenewalReport, SessionRevokeReport,
};
pub use session_registry::{SessionAdmissionPermit, SessionRegistry, SessionRegistryError};
pub use session_worktree::{
    SessionWorktree, SessionWorktreeError, SessionWorktreeManager, SessionWorktreeRecoveryReport,
};
pub use task_lease_registry::{TaskLeaseRegistry, TaskLeaseRegistryError};
pub use tool_profile_registry::{ToolProfileRegistry, ToolProfileRegistryError};
pub use transactional_file::{BytePatch, TransactionalFileError, TransactionalFileService};
pub use transport::{RequestPermit, TransportError, TransportGuard, TransportLimits};
