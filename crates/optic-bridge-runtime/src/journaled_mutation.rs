use std::path::Path;

#[cfg(windows)]
use std::fs::{self, File};

use optic_bridge_core::{
    ActionId, ContentVersion, ContentVersionReadError, ExpectedState, HardLimits, WorkspacePath,
};
use thiserror::Error;

#[cfg(windows)]
use crate::mutation::staging_path_for;
use crate::{
    AtomicMutationError, AtomicMutationService, DeleteCommit, MutationCommit,
    MutationRecoveryJournal, PreparedDelete, PreparedMutation, RecoveryJournalError,
    RecoveryReport,
};
#[cfg(windows)]
use crate::{CommitVerification, DeleteVerification, RecoveryOutcome};

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JournalCommitBoundary {
    PreparedDurable,
    CommittingDurable,
    AtomicCommitReturned,
    TerminalDurable,
}

#[derive(Debug)]
pub struct JournaledMutationService {
    atomic: AtomicMutationService,
    journal: MutationRecoveryJournal,
    #[cfg(windows)]
    max_mutation_bytes: u64,
}

impl JournaledMutationService {
    pub fn from_hard_limits(
        workspace_root: impl AsRef<Path>,
        state_root: impl AsRef<Path>,
        limits: HardLimits,
    ) -> Result<Self, JournaledMutationError> {
        let atomic = AtomicMutationService::from_hard_limits(workspace_root, limits)?;
        let journal =
            MutationRecoveryJournal::open(state_root, atomic.filesystem().root(), limits)?;
        Ok(Self {
            atomic,
            journal,
            #[cfg(windows)]
            max_mutation_bytes: limits.max_fs_mutation_bytes,
        })
    }

    pub fn prepare_write(
        &self,
        path: &WorkspacePath,
        expected: ExpectedState,
    ) -> Result<PreparedMutation, JournaledMutationError> {
        Ok(self.atomic.prepare_write(path, expected)?)
    }

    pub fn prepare_delete(
        &self,
        path: &WorkspacePath,
        expected: ContentVersion,
    ) -> Result<PreparedDelete, JournaledMutationError> {
        Ok(self.atomic.prepare_delete(path, expected)?)
    }

    pub fn commit_write(
        &self,
        plan: &PreparedMutation,
        content: &[u8],
    ) -> Result<JournaledMutationCommit, JournaledMutationError> {
        #[cfg(not(windows))]
        {
            let _ = (plan, content);
            Err(JournaledMutationError::UnsupportedPlatform)
        }

        #[cfg(windows)]
        {
            let mut ignore_boundary = |_| {};
            self.commit_write_with_observer(plan, content, &mut ignore_boundary)
        }
    }

    pub fn commit_delete(
        &self,
        plan: &PreparedDelete,
    ) -> Result<JournaledDeleteCommit, JournaledMutationError> {
        #[cfg(not(windows))]
        {
            let _ = plan;
            Err(JournaledMutationError::UnsupportedPlatform)
        }

        #[cfg(windows)]
        {
            let mut ignore_boundary = |_| {};
            self.commit_delete_with_observer(plan, &mut ignore_boundary)
        }
    }

    pub fn recover(&self) -> Result<RecoveryReport, JournaledMutationError> {
        let report = self.journal.inspect_pending(self.atomic.filesystem())?;
        #[cfg(windows)]
        self.cleanup_recovered_staging(&report)?;
        for record in &report.records {
            self.journal.retire_action(&record.action_id)?;
        }
        Ok(report)
    }

    #[must_use]
    pub fn atomic(&self) -> &AtomicMutationService {
        &self.atomic
    }

    #[must_use]
    pub fn journal(&self) -> &MutationRecoveryJournal {
        &self.journal
    }

    #[cfg(windows)]
    fn commit_write_with_observer(
        &self,
        plan: &PreparedMutation,
        content: &[u8],
        on_boundary: &mut dyn FnMut(JournalCommitBoundary),
    ) -> Result<JournaledMutationCommit, JournaledMutationError> {
        let intended = ExpectedState::Content(ContentVersion::from_bytes(content));
        let ticket = self.journal.begin(plan, intended)?;
        on_boundary(JournalCommitBoundary::PreparedDurable);

        self.journal.mark_committing(&ticket)?;
        on_boundary(JournalCommitBoundary::CommittingDurable);

        let action_id = ticket.action_id().clone();
        let commit = match self
            .atomic
            .commit_write_for_action(plan, content, ticket.action_id())
        {
            Ok(commit) => {
                on_boundary(JournalCommitBoundary::AtomicCommitReturned);
                commit
            }
            Err(source) => {
                if self.journal.mark_ambiguous(&ticket).is_ok() {
                    on_boundary(JournalCommitBoundary::TerminalDurable);
                }
                return Err(JournaledMutationError::RecoveryRequiredAfterAtomicError {
                    action_id,
                    source,
                });
            }
        };

        match commit.verification {
            CommitVerification::Verified(_) => {
                if let Err(source) = self.journal.mark_verified(&ticket) {
                    return Err(JournaledMutationError::RecoveryRequiredAfterJournalError {
                        action_id,
                        source,
                    });
                }
                on_boundary(JournalCommitBoundary::TerminalDurable);
                let journal_retired = self.journal.retire(&ticket).is_ok();
                Ok(JournaledMutationCommit {
                    action_id,
                    commit,
                    journal_retired,
                })
            }
            CommitVerification::CommittedButUnverified => {
                if self.journal.mark_ambiguous(&ticket).is_ok() {
                    on_boundary(JournalCommitBoundary::TerminalDurable);
                }
                Err(JournaledMutationError::CommittedButNeedsRecovery { action_id })
            }
        }
    }

    #[cfg(windows)]
    fn commit_delete_with_observer(
        &self,
        plan: &PreparedDelete,
        on_boundary: &mut dyn FnMut(JournalCommitBoundary),
    ) -> Result<JournaledDeleteCommit, JournaledMutationError> {
        let journal_plan = plan.journal_plan();
        let ticket = self.journal.begin(&journal_plan, ExpectedState::Absent)?;
        on_boundary(JournalCommitBoundary::PreparedDurable);

        self.journal.mark_committing(&ticket)?;
        on_boundary(JournalCommitBoundary::CommittingDurable);

        let action_id = ticket.action_id().clone();
        let commit = match self.atomic.commit_delete(plan) {
            Ok(commit) => {
                on_boundary(JournalCommitBoundary::AtomicCommitReturned);
                commit
            }
            Err(source) => {
                if self.journal.mark_ambiguous(&ticket).is_ok() {
                    on_boundary(JournalCommitBoundary::TerminalDurable);
                }
                return Err(JournaledMutationError::RecoveryRequiredAfterAtomicError {
                    action_id,
                    source,
                });
            }
        };

        match commit.verification {
            DeleteVerification::VerifiedAbsent => {
                if let Err(source) = self.journal.mark_verified(&ticket) {
                    return Err(JournaledMutationError::RecoveryRequiredAfterJournalError {
                        action_id,
                        source,
                    });
                }
                on_boundary(JournalCommitBoundary::TerminalDurable);
                let journal_retired = self.journal.retire(&ticket).is_ok();
                Ok(JournaledDeleteCommit {
                    action_id,
                    commit,
                    journal_retired,
                })
            }
            DeleteVerification::CommittedButUnverified => {
                if self.journal.mark_ambiguous(&ticket).is_ok() {
                    on_boundary(JournalCommitBoundary::TerminalDurable);
                }
                Err(JournaledMutationError::CommittedButNeedsRecovery { action_id })
            }
        }
    }

    #[cfg(windows)]
    fn cleanup_recovered_staging(
        &self,
        report: &RecoveryReport,
    ) -> Result<(), JournaledMutationError> {
        for record in &report.records {
            let staging = staging_path_for(
                self.atomic.filesystem(),
                &record.canonical_path,
                &record.action_id,
            );
            let metadata = match fs::symlink_metadata(&staging) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(source) => {
                    return Err(JournaledMutationError::StagingCleanupFailed {
                        action_id: record.action_id.clone(),
                        source,
                    });
                }
            };

            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(JournaledMutationError::UnsafeStagingArtifact {
                    action_id: record.action_id.clone(),
                });
            }
            if record.outcome == RecoveryOutcome::PreparedNotCommitted {
                return Err(JournaledMutationError::UnexpectedPreparedStagingArtifact {
                    action_id: record.action_id.clone(),
                });
            }
            let intended_version = match record.intended {
                ExpectedState::Content(version) => version,
                ExpectedState::Absent => {
                    return Err(
                        JournaledMutationError::UnexpectedAbsentIntentStagingArtifact {
                            action_id: record.action_id.clone(),
                        },
                    );
                }
            };
            if metadata.len() > self.max_mutation_bytes {
                return Err(JournaledMutationError::StagingArtifactTooLarge {
                    action_id: record.action_id.clone(),
                    limit: self.max_mutation_bytes,
                });
            }

            let mut file = File::open(&staging).map_err(|source| {
                JournaledMutationError::StagingCleanupFailed {
                    action_id: record.action_id.clone(),
                    source,
                }
            })?;
            let observed = ContentVersion::from_reader_bounded(&mut file, self.max_mutation_bytes)
                .map_err(|source| JournaledMutationError::StagingObservationFailed {
                    action_id: record.action_id.clone(),
                    source,
                })?;
            if observed != intended_version {
                return Err(JournaledMutationError::StagingContentMismatch {
                    action_id: record.action_id.clone(),
                });
            }
            drop(file);

            fs::remove_file(&staging).map_err(|source| {
                JournaledMutationError::StagingCleanupFailed {
                    action_id: record.action_id.clone(),
                    source,
                }
            })?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournaledMutationCommit {
    pub action_id: ActionId,
    pub commit: MutationCommit,
    pub journal_retired: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournaledDeleteCommit {
    pub action_id: ActionId,
    pub commit: DeleteCommit,
    pub journal_retired: bool,
}

#[derive(Debug, Error)]
pub enum JournaledMutationError {
    #[error("atomic mutation setup or pre-commit operation failed: {0}")]
    Atomic(#[from] AtomicMutationError),
    #[error("mutation recovery journal operation failed: {0}")]
    Journal(#[from] RecoveryJournalError),
    #[error("durable mutation commit is not implemented on this platform yet")]
    UnsupportedPlatform,
    #[error("operation {action_id:?} entered committing state and now requires recovery: {source}")]
    RecoveryRequiredAfterAtomicError {
        action_id: ActionId,
        #[source]
        source: AtomicMutationError,
    },
    #[error(
        "operation {action_id:?} may have committed but could not be verified; recovery is required"
    )]
    CommittedButNeedsRecovery { action_id: ActionId },
    #[error(
        "operation {action_id:?} committed but its terminal journal state could not be persisted: {source}"
    )]
    RecoveryRequiredAfterJournalError {
        action_id: ActionId,
        #[source]
        source: RecoveryJournalError,
    },
    #[error("operation {action_id:?} recovery found a non-regular staging artifact")]
    UnsafeStagingArtifact { action_id: ActionId },
    #[error(
        "operation {action_id:?} has a staging artifact even though only prepared state was durable"
    )]
    UnexpectedPreparedStagingArtifact { action_id: ActionId },
    #[error(
        "operation {action_id:?} has a write staging artifact for an intended absent final state"
    )]
    UnexpectedAbsentIntentStagingArtifact { action_id: ActionId },
    #[error("operation {action_id:?} staging artifact exceeds hard byte ceiling {limit}")]
    StagingArtifactTooLarge { action_id: ActionId, limit: u64 },
    #[error("operation {action_id:?} staging content observation failed: {source}")]
    StagingObservationFailed {
        action_id: ActionId,
        #[source]
        source: ContentVersionReadError,
    },
    #[error("operation {action_id:?} staging content no longer matches its journaled intent")]
    StagingContentMismatch { action_id: ActionId },
    #[error("operation {action_id:?} staging cleanup failed: {source}")]
    StagingCleanupFailed {
        action_id: ActionId,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    #[cfg(windows)]
    use std::process::Command;

    use super::*;

    #[cfg(windows)]
    const CRASH_BOUNDARY_ENV: &str = "OPTIC_TEST_CRASH_BOUNDARY";
    #[cfg(windows)]
    const CRASH_BASE_ENV: &str = "OPTIC_TEST_CRASH_BASE";
    #[cfg(windows)]
    const DELETE_CRASH_BOUNDARY_ENV: &str = "OPTIC_TEST_DELETE_CRASH_BOUNDARY";
    #[cfg(windows)]
    const DELETE_CRASH_BASE_ENV: &str = "OPTIC_TEST_DELETE_CRASH_BASE";
    #[cfg(windows)]
    const CRASH_EXIT_CODE: i32 = 86;

    fn fixture(label: &str) -> (PathBuf, PathBuf) {
        let token = ActionId::generate().expect("test entropy").to_token();
        let base = env::temp_dir().join(format!("optic-journaled-mutation-{label}-{token}"));
        let workspace = base.join("workspace");
        let state = base.join("state");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&state).expect("state");
        (workspace, state)
    }

    #[cfg(windows)]
    impl JournalCommitBoundary {
        fn test_name(self) -> &'static str {
            match self {
                Self::PreparedDurable => "prepared",
                Self::CommittingDurable => "committing",
                Self::AtomicCommitReturned => "after_atomic_commit",
                Self::TerminalDurable => "terminal",
            }
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_commit_fails_before_creating_a_journal() {
        let (workspace, state) = fixture("unsupported");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = service
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");

        assert!(matches!(
            service.commit_write(&plan, b"new"),
            Err(JournaledMutationError::UnsupportedPlatform)
        ));
        assert!(service.recover().expect("recover").is_empty());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_delete_fails_before_creating_a_journal() {
        let (workspace, state) = fixture("unsupported-delete");
        fs::write(workspace.join("target.txt"), b"old").expect("fixture");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_delete(&path, ContentVersion::from_bytes(b"old"))
            .expect("plan");

        assert!(matches!(
            service.commit_delete(&plan),
            Err(JournaledMutationError::UnsupportedPlatform)
        ));
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("target"),
            b"old"
        );
        assert!(service.recover().expect("recover").is_empty());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn verified_windows_commit_retires_its_journal() {
        let (workspace, state) = fixture("verified");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(
                &path,
                ExpectedState::Content(ContentVersion::from_bytes(b"old")),
            )
            .expect("plan");

        let result = service.commit_write(&plan, b"new").expect("commit");
        assert!(matches!(
            result.commit.verification,
            CommitVerification::Verified(_)
        ));
        assert!(result.journal_retired);
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("target"),
            b"new"
        );
        assert!(service.recover().expect("recover").is_empty());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn verified_windows_delete_retires_its_journal() {
        let (workspace, state) = fixture("verified-delete");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_delete(&path, ContentVersion::from_bytes(b"old"))
            .expect("plan");

        let result = service.commit_delete(&plan).expect("delete");
        assert_eq!(
            result.commit.verification,
            DeleteVerification::VerifiedAbsent
        );
        assert!(result.journal_retired);
        assert!(!workspace.join("target.txt").exists());
        assert!(service.recover().expect("recover").is_empty());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn post_committing_atomic_failure_is_left_for_recovery_not_retry() {
        let (workspace, state) = fixture("recovery-required");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(
                &path,
                ExpectedState::Content(ContentVersion::from_bytes(b"old")),
            )
            .expect("plan");

        fs::write(workspace.join("target.txt"), b"changed").expect("external change");
        assert!(matches!(
            service.commit_write(&plan, b"new"),
            Err(JournaledMutationError::RecoveryRequiredAfterAtomicError { .. })
        ));

        assert!(matches!(
            service.recover(),
            Err(JournaledMutationError::Journal(
                RecoveryJournalError::UnresolvedRecoveryConflict { .. }
            ))
        ));
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn stale_after_committing_can_reconcile_as_not_committed() {
        let (workspace, state) = fixture("recover-old");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(
                &path,
                ExpectedState::Content(ContentVersion::from_bytes(b"old")),
            )
            .expect("plan");

        let ticket = service
            .journal()
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        service
            .journal()
            .mark_committing(&ticket)
            .expect("committing");

        let report = service.recover().expect("recover");
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::ObservedNotCommitted
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn staging_mismatch_fails_closed_and_retains_journal_evidence() {
        let (workspace, state) = fixture("staging-mismatch");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let ticket = service
            .journal()
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        service
            .journal()
            .mark_committing(&ticket)
            .expect("committing");

        let staging = staging_path_for(service.atomic().filesystem(), &path, ticket.action_id());
        fs::write(&staging, b"tampered").expect("staging fixture");
        assert!(matches!(
            service.recover(),
            Err(JournaledMutationError::StagingContentMismatch { .. })
        ));
        assert!(staging.exists());
        assert_eq!(
            fs::read_dir(service.journal().root())
                .expect("journal root")
                .count(),
            1
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn recovery_removes_regular_operation_owned_staging_after_committing() {
        let (workspace, state) = fixture("staging-cleanup");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let ticket = service
            .journal()
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        service
            .journal()
            .mark_committing(&ticket)
            .expect("committing");

        let staging = staging_path_for(service.atomic().filesystem(), &path, ticket.action_id());
        fs::write(&staging, b"new").expect("staging fixture");
        let report = service.recover().expect("recover");
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::ObservedNotCommitted
        );
        assert!(!staging.exists());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn forced_crash_child_entrypoint() {
        let Some(boundary_name) = env::var_os(CRASH_BOUNDARY_ENV) else {
            return;
        };
        let base = PathBuf::from(env::var_os(CRASH_BASE_ENV).expect("crash base"));
        let workspace = base.join("workspace");
        let state = base.join("state");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let boundary_name = boundary_name.to_string_lossy().into_owned();
        let mut observer = |boundary: JournalCommitBoundary| {
            if boundary.test_name() == boundary_name {
                std::process::exit(CRASH_EXIT_CODE);
            }
        };

        let result = service.commit_write_with_observer(&plan, b"new", &mut observer);
        panic!("child reached end without forced crash: {result:?}");
    }

    #[cfg(windows)]
    #[test]
    fn forced_process_crashes_reconcile_all_documented_boundaries() {
        let cases = [
            (
                JournalCommitBoundary::PreparedDurable,
                RecoveryOutcome::PreparedNotCommitted,
                false,
            ),
            (
                JournalCommitBoundary::CommittingDurable,
                RecoveryOutcome::ObservedNotCommitted,
                false,
            ),
            (
                JournalCommitBoundary::AtomicCommitReturned,
                RecoveryOutcome::ObservedCommitted,
                true,
            ),
            (
                JournalCommitBoundary::TerminalDurable,
                RecoveryOutcome::VerifiedTerminal,
                true,
            ),
        ];

        for (boundary, expected_outcome, expected_target) in cases {
            let (workspace, state) = fixture(boundary.test_name());
            let base = workspace.parent().expect("base").to_path_buf();
            let status = Command::new(env::current_exe().expect("current test executable"))
                .arg("forced_crash_child_entrypoint")
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env(CRASH_BOUNDARY_ENV, boundary.test_name())
                .env(CRASH_BASE_ENV, &base)
                .status()
                .expect("spawn crash child");
            assert_eq!(
                status.code(),
                Some(CRASH_EXIT_CODE),
                "unexpected child status for {boundary:?}: {status:?}"
            );

            let service = JournaledMutationService::from_hard_limits(
                &workspace,
                &state,
                HardLimits::default(),
            )
            .expect("reopen service");
            let report = service.recover().expect("recover after crash");
            assert_eq!(report.records.len(), 1, "boundary {boundary:?}");
            assert_eq!(
                report.records[0].outcome, expected_outcome,
                "boundary {boundary:?}"
            );
            assert_eq!(
                workspace.join("target.txt").exists(),
                expected_target,
                "boundary {boundary:?}"
            );
            assert!(service.recover().expect("second recovery").is_empty());
            fs::remove_dir_all(&base).expect("cleanup crash fixture");
        }
    }

    #[cfg(windows)]
    #[test]
    fn forced_delete_crash_child_entrypoint() {
        let Some(boundary_name) = env::var_os(DELETE_CRASH_BOUNDARY_ENV) else {
            return;
        };
        let base = PathBuf::from(env::var_os(DELETE_CRASH_BASE_ENV).expect("delete crash base"));
        let workspace = base.join("workspace");
        let state = base.join("state");
        let service =
            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_delete(&path, ContentVersion::from_bytes(b"old"))
            .expect("delete plan");
        let boundary_name = boundary_name.to_string_lossy().into_owned();
        let mut observer = |boundary: JournalCommitBoundary| {
            if boundary.test_name() == boundary_name {
                std::process::exit(CRASH_EXIT_CODE);
            }
        };

        let result = service.commit_delete_with_observer(&plan, &mut observer);
        panic!("delete child reached end without forced crash: {result:?}");
    }

    #[cfg(windows)]
    #[test]
    fn forced_delete_process_crashes_reconcile_all_documented_boundaries() {
        let cases = [
            (
                JournalCommitBoundary::PreparedDurable,
                RecoveryOutcome::PreparedNotCommitted,
                true,
            ),
            (
                JournalCommitBoundary::CommittingDurable,
                RecoveryOutcome::ObservedNotCommitted,
                true,
            ),
            (
                JournalCommitBoundary::AtomicCommitReturned,
                RecoveryOutcome::ObservedCommitted,
                false,
            ),
            (
                JournalCommitBoundary::TerminalDurable,
                RecoveryOutcome::VerifiedTerminal,
                false,
            ),
        ];

        for (boundary, expected_outcome, expected_target) in cases {
            let (workspace, state) = fixture(&format!("delete-{}", boundary.test_name()));
            fs::write(workspace.join("target.txt"), b"old").expect("delete crash fixture");
            let base = workspace.parent().expect("base").to_path_buf();
            let status = Command::new(env::current_exe().expect("current test executable"))
                .arg("forced_delete_crash_child_entrypoint")
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env(DELETE_CRASH_BOUNDARY_ENV, boundary.test_name())
                .env(DELETE_CRASH_BASE_ENV, &base)
                .status()
                .expect("spawn delete crash child");
            assert_eq!(
                status.code(),
                Some(CRASH_EXIT_CODE),
                "unexpected delete child status for {boundary:?}: {status:?}"
            );

            let service = JournaledMutationService::from_hard_limits(
                &workspace,
                &state,
                HardLimits::default(),
            )
            .expect("reopen delete service");
            let report = service.recover().expect("recover after delete crash");
            assert_eq!(report.records.len(), 1, "delete boundary {boundary:?}");
            assert_eq!(
                report.records[0].outcome, expected_outcome,
                "delete boundary {boundary:?}"
            );
            assert_eq!(
                workspace.join("target.txt").exists(),
                expected_target,
                "delete boundary {boundary:?}"
            );
            assert!(
                service
                    .recover()
                    .expect("second delete recovery")
                    .is_empty()
            );
            fs::remove_dir_all(&base).expect("cleanup delete crash fixture");
        }
    }
}
