use std::path::Path;

use optic_bridge_core::{ActionId, ContentVersion, ExpectedState, HardLimits, WorkspacePath};
use thiserror::Error;

use crate::{
    AtomicMutationError, AtomicMutationService, CommitVerification, MutationCommit,
    MutationRecoveryJournal, PreparedMutation, RecoveryJournalError, RecoveryReport,
};

#[derive(Debug)]
pub struct JournaledMutationService {
    atomic: AtomicMutationService,
    journal: MutationRecoveryJournal,
}

impl JournaledMutationService {
    pub fn from_hard_limits(
        workspace_root: impl AsRef<Path>,
        state_root: impl AsRef<Path>,
        limits: HardLimits,
    ) -> Result<Self, JournaledMutationError> {
        let atomic = AtomicMutationService::from_hard_limits(workspace_root, limits)?;
        let journal = MutationRecoveryJournal::open(state_root, atomic.filesystem().root(), limits)?;
        Ok(Self { atomic, journal })
    }

    pub fn prepare_write(
        &self,
        path: &WorkspacePath,
        expected: ExpectedState,
    ) -> Result<PreparedMutation, JournaledMutationError> {
        Ok(self.atomic.prepare_write(path, expected)?)
    }

    pub fn commit_write(
        &self,
        plan: &PreparedMutation,
        content: &[u8],
    ) -> Result<JournaledMutationCommit, JournaledMutationError> {
        #[cfg(not(windows))]
        {
            let _ = (plan, content);
            return Err(JournaledMutationError::UnsupportedPlatform);
        }

        #[cfg(windows)]
        {
            let intended = ContentVersion::from_bytes(content);
            let ticket = self.journal.begin(plan, intended)?;
            self.journal.mark_committing(&ticket)?;

            let action_id = ticket.action_id().clone();
            let commit = match self
                .atomic
                .commit_write_for_action(plan, content, ticket.action_id())
            {
                Ok(commit) => commit,
                Err(source) => {
                    let _ = self.journal.mark_ambiguous(&ticket);
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
                    let journal_retired = self.journal.retire(&ticket).is_ok();
                    Ok(JournaledMutationCommit {
                        action_id,
                        commit,
                        journal_retired,
                    })
                }
                CommitVerification::CommittedButUnverified => {
                    let _ = self.journal.mark_ambiguous(&ticket);
                    Err(JournaledMutationError::CommittedButNeedsRecovery { action_id })
                }
            }
        }
    }

    pub fn recover(&self) -> Result<RecoveryReport, JournaledMutationError> {
        Ok(self.journal.reconcile(self.atomic.filesystem())?)
    }

    #[must_use]
    pub fn atomic(&self) -> &AtomicMutationService {
        &self.atomic
    }

    #[must_use]
    pub fn journal(&self) -> &MutationRecoveryJournal {
        &self.journal
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournaledMutationCommit {
    pub action_id: ActionId,
    pub commit: MutationCommit,
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
    #[error("operation {action_id:?} may have committed but could not be verified; recovery is required")]
    CommittedButNeedsRecovery { action_id: ActionId },
    #[error("operation {action_id:?} committed but its terminal journal state could not be persisted: {source}")]
    RecoveryRequiredAfterJournalError {
        action_id: ActionId,
        #[source]
        source: RecoveryJournalError,
    },
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use super::*;
    use crate::RecoveryOutcome;

    fn fixture(label: &str) -> (PathBuf, PathBuf) {
        let token = ActionId::generate().expect("test entropy").to_token();
        let base = env::temp_dir().join(format!("optic-journaled-mutation-{label}-{token}"));
        let workspace = base.join("workspace");
        let state = base.join("state");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&state).expect("state");
        (workspace, state)
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_commit_fails_before_creating_a_journal() {
        let (workspace, state) = fixture("unsupported");
        let service = JournaledMutationService::from_hard_limits(
            &workspace,
            &state,
            HardLimits::default(),
        )
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

    #[cfg(windows)]
    #[test]
    fn verified_windows_commit_retires_its_journal() {
        let (workspace, state) = fixture("verified");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let service = JournaledMutationService::from_hard_limits(
            &workspace,
            &state,
            HardLimits::default(),
        )
        .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(
                &path,
                ExpectedState::Content(ContentVersion::from_bytes(b"old")),
            )
            .expect("plan");

        let result = service.commit_write(&plan, b"new").expect("commit");
        assert!(matches!(result.commit.verification, CommitVerification::Verified(_)));
        assert!(result.journal_retired);
        assert_eq!(fs::read(workspace.join("target.txt")).expect("target"), b"new");
        assert!(service.recover().expect("recover").is_empty());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn post_committing_atomic_failure_is_left_for_recovery_not_retry() {
        let (workspace, state) = fixture("recovery-required");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let service = JournaledMutationService::from_hard_limits(
            &workspace,
            &state,
            HardLimits::default(),
        )
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
        let service = JournaledMutationService::from_hard_limits(
            &workspace,
            &state,
            HardLimits::default(),
        )
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
            .begin(&plan, ContentVersion::from_bytes(b"new"))
            .expect("begin");
        service.journal().mark_committing(&ticket).expect("committing");

        let report = service.recover().expect("recover");
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.records[0].outcome, RecoveryOutcome::ObservedNotCommitted);
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }
}
