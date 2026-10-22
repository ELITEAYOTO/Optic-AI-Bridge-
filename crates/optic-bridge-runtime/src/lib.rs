#![forbid(unsafe_code)]

//! Runtime services for Optic AI Bridge.
//!
//! This crate owns application runtime state and hard limits. MCP remains an
//! adapter outside this boundary, so SDK defaults cannot silently weaken
//! session, transport, filesystem, Git, process, mutation-precondition,
//! recovery, or output constraints.

mod approval_broker;
mod artifact_publication;
#[path = "mutation/authorized.rs"]
mod authorized_file_mutation;
mod authorized_git_integration;
mod blocking_io;
mod clock;
mod directory_create;
mod filesystem;
#[path = "git/blob_batch.rs"]
mod git_blob_batch;
#[path = "git/change_set.rs"]
mod git_change_set;
#[path = "git/integrate.rs"]
mod git_integrate;
#[path = "git/integration_authority.rs"]
mod git_integration_authority;
#[path = "git/merge_commit.rs"]
mod git_merge_commit;
#[path = "git/merge_plan.rs"]
mod git_merge_plan;
#[cfg(windows)]
#[path = "git/owner_registry.rs"]
mod git_owner_registry;
#[path = "git/read.rs"]
mod git_read;
#[path = "git/tree_manifest.rs"]
mod git_tree_manifest;
#[path = "git/tree_write.rs"]
mod git_tree_write;
#[path = "git/worktree.rs"]
mod git_worktree;
mod hardened_command;
#[path = "mutation/journaled.rs"]
mod journaled_mutation;
#[path = "mutation/atomic.rs"]
mod mutation;
#[path = "mutation/authority.rs"]
mod mutation_authority;
#[cfg(windows)]
mod private_node_task;
mod private_project_stage;
#[cfg(windows)]
mod private_task_journal;
mod process;
mod process_authority;
mod project_inputs;
mod read_authority;
#[path = "mutation/recovery.rs"]
mod recovery;
mod reusable_approval_broker;
mod same_repo_coordinator;
mod same_repo_merge;
#[cfg(all(windows, test))]
#[path = "session/git_owner_integration.rs"]
mod session_git_owner_integration;
#[path = "session/lifecycle.rs"]
mod session_lifecycle;
#[path = "session/registry.rs"]
mod session_registry;
#[path = "session/reusable_approval.rs"]
mod session_reusable_approval;
#[path = "session/worktree.rs"]
mod session_worktree;
mod task_lease_registry;
mod tool_profile_registry;
#[path = "mutation/transactional.rs"]
mod transactional_file;
mod transport;
#[cfg(windows)]
mod windows_path;

pub use approval_broker::{ApprovalBroker, ApprovalBrokerError, ApprovalSpec};
pub use artifact_publication::{
    ArtifactPublicationError, ArtifactPublicationInterruption, ArtifactPublicationProgress,
    ArtifactPublicationReceipt, AuthorizedArtifactPublicationService,
};
pub use authorized_file_mutation::{AuthorizedFileMutationError, AuthorizedFileMutationService};
pub use authorized_git_integration::{
    AuthorizedGitIntegrationError, AuthorizedGitIntegrationService,
};
pub use blocking_io::{BlockingIoError, BlockingIoGovernor};
pub use clock::{Clock, StdClock};
pub use directory_create::DirectoryCreateError;
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
#[cfg(windows)]
pub use private_node_task::PrivateNodeTaskStart;
#[cfg(windows)]
pub use private_project_stage::PreparedPrivateProjectLease;
pub use private_project_stage::{
    PreparedPrivateProject, PrivateProjectStageError, PrivateProjectStageService,
};
#[cfg(windows)]
pub use private_task_journal::{
    PrivateTaskJournal, PrivateTaskJournalError, PrivateTaskOutcome, PrivateTaskRecoveryExecution,
    PrivateTaskRecoveryObservation, PrivateTaskRecoveryPhase, PrivateTaskRecoveryRecord,
    PrivateTaskRetirementCandidate,
};
pub use process::{
    EnvironmentGrant, EnvironmentVariableClass, ProcessError, ProcessManager, ProcessReadChunk,
    ProcessResult, ProcessStartSpec, ProcessStatus, ProcessStream,
};
pub use process_authority::{ProcessAuthority, ProcessAuthorityError, ProcessExecutableIdentity};
pub use project_inputs::{ProjectInputCaptureError, ProjectInputCaptureService};
pub use read_authority::{
    ReadAuthorityError, ReadAuthoritySet, ReadAuthoritySpec, read_authority_resource_budget,
};
pub use recovery::{
    JournalTicket, MutationRecoveryJournal, RecoveryJournalError, RecoveryOutcome, RecoveryRecord,
    RecoveryReport,
};
pub use reusable_approval_broker::{
    ReusableApprovalBroker, ReusableApprovalBrokerError, ReusableApprovalLimits,
    ReusableApprovalSpec,
};
pub use same_repo_coordinator::{
    CoordinatedSession, SameRepositoryCoordinatorError, SameRepositorySessionCoordinator,
};
pub use same_repo_merge::{
    SameRepositoryMergeOrchestrationError, SameRepositoryMergeOrchestrator,
    SameRepositoryMergePublishError, SameRepositoryMergePublisher, SessionMergeCleanupOutcome,
    SessionMergeLifecycleOutcome, SessionMergePublishOutcome,
};
pub use session_lifecycle::{
    QuiescentSessionSeal, SessionGrantSpec, SessionLifecycleError, SessionLifecycleManager,
    SessionReapReport, SessionRenewalReport, SessionRevokeReport,
};
pub use session_registry::{SessionAdmissionPermit, SessionRegistry, SessionRegistryError};
pub use session_reusable_approval::{SessionReusableApprovalError, SessionReusableApprovalService};
pub use session_worktree::{
    SessionWorktree, SessionWorktreeError, SessionWorktreeManager, SessionWorktreeRecoveryReport,
};
pub use task_lease_registry::{TaskLeaseRegistry, TaskLeaseRegistryError};
pub use tool_profile_registry::{ToolProfileRegistry, ToolProfileRegistryError};
pub use transactional_file::{BytePatch, TransactionalFileError, TransactionalFileService};
pub use transport::{RequestPermit, TransportError, TransportGuard, TransportLimits};
