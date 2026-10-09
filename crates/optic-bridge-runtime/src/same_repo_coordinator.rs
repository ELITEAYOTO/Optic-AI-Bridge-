use std::sync::Arc;

use optic_bridge_core::{GitObjectId, MonotonicTime, SessionGrant, SessionHandle};
use thiserror::Error;

use crate::{
    ApprovalBroker, ProcessManager, SessionGrantSpec, SessionLifecycleError,
    SessionLifecycleManager, SessionReapReport, SessionRegistry, SessionRevokeReport,
    SessionWorktree, SessionWorktreeError, SessionWorktreeManager, TaskLeaseRegistry,
};

pub struct SameRepositorySessionCoordinator {
    lifecycle: Arc<SessionLifecycleManager>,
    worktrees: Arc<SessionWorktreeManager>,
}

#[derive(Clone, Debug)]
pub struct CoordinatedSession {
    pub grant: SessionGrant,
    pub worktree: SessionWorktree,
}

impl SameRepositorySessionCoordinator {
    #[must_use]
    pub fn new(
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        processes: Arc<ProcessManager>,
        approvals: Arc<ApprovalBroker>,
        worktrees: Arc<SessionWorktreeManager>,
    ) -> Self {
        let lifecycle = Arc::new(
            SessionLifecycleManager::new_with_approval_broker_and_worktrees(
                sessions,
                task_leases,
                processes,
                approvals,
                Some(Arc::clone(&worktrees)),
            ),
        );
        Self {
            lifecycle,
            worktrees,
        }
    }

    #[must_use]
    pub fn lifecycle(&self) -> Arc<SessionLifecycleManager> {
        Arc::clone(&self.lifecycle)
    }

    pub fn provision(
        &self,
        spec: SessionGrantSpec,
        base_head: &GitObjectId,
        now: MonotonicTime,
    ) -> Result<CoordinatedSession, SameRepositoryCoordinatorError> {
        let grant = self.lifecycle.provision(spec, now)?;
        match self.worktrees.provision(&grant.handle, base_head) {
            Ok(worktree) => Ok(CoordinatedSession { grant, worktree }),
            Err(worktree_error) => {
                let rollback = self
                    .lifecycle
                    .revoke(&grant.handle)
                    .and_then(|_| self.lifecycle.try_reap(&grant.handle, now));
                match rollback {
                    Ok(report) if report.session_removed => {
                        Err(SameRepositoryCoordinatorError::Worktree(worktree_error))
                    }
                    Ok(_) => Err(
                        SameRepositoryCoordinatorError::ProvisionRollbackIncomplete {
                            worktree: Box::new(worktree_error),
                        },
                    ),
                    Err(rollback) => Err(SameRepositoryCoordinatorError::ProvisionRollbackFailed {
                        worktree: Box::new(worktree_error),
                        rollback: Box::new(rollback),
                    }),
                }
            }
        }
    }

    pub fn worktree(
        &self,
        session: &SessionHandle,
    ) -> Result<SessionWorktree, SameRepositoryCoordinatorError> {
        Ok(self.worktrees.get(session)?)
    }

    pub fn revoke(
        &self,
        session: &SessionHandle,
    ) -> Result<SessionRevokeReport, SameRepositoryCoordinatorError> {
        Ok(self.lifecycle.revoke(session)?)
    }

    pub fn try_reap(
        &self,
        session: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<SessionReapReport, SameRepositoryCoordinatorError> {
        Ok(self.lifecycle.try_reap(session, now)?)
    }
}

#[derive(Debug, Error)]
pub enum SameRepositoryCoordinatorError {
    #[error(transparent)]
    Lifecycle(#[from] SessionLifecycleError),
    #[error(transparent)]
    Worktree(#[from] SessionWorktreeError),
    #[error(
        "session rollback after worktree provisioning failure did not reach physical reap: {worktree}"
    )]
    ProvisionRollbackIncomplete { worktree: Box<SessionWorktreeError> },
    #[error(
        "session rollback after worktree provisioning failure failed; worktree error: {worktree}; rollback error: {rollback}"
    )]
    ProvisionRollbackFailed {
        worktree: Box<SessionWorktreeError>,
        rollback: Box<SessionLifecycleError>,
    },
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        env, fs,
        path::{Path, PathBuf},
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use optic_bridge_core::{Capability, HardLimits, PrincipalId, ProjectId};

    use super::*;

    struct RepoFixture {
        base: PathBuf,
        repo: PathBuf,
        worktrees: PathBuf,
        git: PathBuf,
        head: GitObjectId,
    }

    impl RepoFixture {
        fn new(git: PathBuf, label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let base = env::temp_dir().join(format!("optic-coordinator-{label}-{nonce}"));
            let repo = base.join("repo");
            let worktrees = base.join("worktrees");
            fs::create_dir_all(&repo).expect("repo");
            fs::create_dir_all(&worktrees).expect("worktrees");
            run_git(&git, &repo, ["init", "--quiet"]);
            run_git(
                &git,
                &repo,
                ["config", "user.email", "optic@example.invalid"],
            );
            run_git(&git, &repo, ["config", "user.name", "Optic Test"]);
            fs::write(repo.join("tracked.txt"), b"base\n").expect("fixture file");
            run_git(&git, &repo, ["add", "."]);
            run_git(&git, &repo, ["commit", "--quiet", "-m", "base"]);
            let output = Command::new(&git)
                .arg("-C")
                .arg(&repo)
                .args(["rev-parse", "HEAD"])
                .output()
                .expect("head");
            assert!(output.status.success());
            let head = GitObjectId::parse(
                std::str::from_utf8(&output.stdout)
                    .expect("utf8")
                    .trim()
                    .to_owned(),
            )
            .expect("oid");
            Self {
                base,
                repo,
                worktrees,
                git,
                head,
            }
        }

        fn coordinator(
            &self,
            limits: HardLimits,
        ) -> (SameRepositorySessionCoordinator, Arc<SessionRegistry>) {
            let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
            let task_leases =
                Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
            let processes = Arc::new(
                ProcessManager::new(&self.repo, limits, Vec::new()).expect("process manager"),
            );
            let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits).expect("approvals"));
            let worktrees = Arc::new(
                SessionWorktreeManager::from_hard_limits(
                    &self.repo,
                    &self.git,
                    &self.worktrees,
                    limits,
                )
                .expect("worktrees"),
            );
            (
                SameRepositorySessionCoordinator::new(
                    Arc::clone(&sessions),
                    task_leases,
                    processes,
                    approvals,
                    worktrees,
                ),
                sessions,
            )
        }
    }

    impl Drop for RepoFixture {
        fn drop(&mut self) {
            let _ = Command::new(&self.git)
                .arg("-C")
                .arg(&self.repo)
                .args(["worktree", "prune", "--expire", "now"])
                .status();
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn find_git_executable() -> Option<PathBuf> {
        let path = env::var_os("PATH")?;
        for directory in env::split_paths(&path) {
            #[cfg(windows)]
            let candidate = directory.join("git.exe");
            #[cfg(not(windows))]
            let candidate = directory.join("git");
            if candidate.is_file() {
                return fs::canonicalize(candidate).ok();
            }
        }
        None
    }

    fn run_git<const N: usize>(git: &Path, repo: &Path, args: [&str; N]) {
        let status = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("fixture git");
        assert!(status.success());
    }

    fn spec(expires_at: u64) -> SessionGrantSpec {
        SessionGrantSpec {
            principal: PrincipalId::new("principal").expect("principal"),
            project: ProjectId::new("project").expect("project"),
            capabilities: BTreeSet::from([Capability::FileRead]),
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    #[test]
    fn revoke_keeps_worktree_until_reap_then_cleanup_precedes_session_removal() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "reap");
        let (coordinator, _) = fixture.coordinator(HardLimits::default());
        let now = MonotonicTime::from_millis(1);
        let session = coordinator
            .provision(spec(1_000), &fixture.head, now)
            .expect("coordinated session");
        let path = session.worktree.path.clone();
        assert!(path.exists());

        coordinator.revoke(&session.grant.handle).expect("revoke");
        assert!(
            path.exists(),
            "revoke must not delete a possibly in-use worktree"
        );

        let report = coordinator
            .try_reap(&session.grant.handle, now)
            .expect("reap");
        assert!(report.session_removed);
        assert!(
            !path.exists(),
            "worktree must be gone before capacity is reclaimed"
        );
    }

    #[test]
    fn failed_worktree_provision_rolls_back_session_capacity() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "rollback");
        let limits = HardLimits {
            max_sessions: 1,
            ..HardLimits::default()
        };
        let (coordinator, sessions) = fixture.coordinator(limits);
        let now = MonotonicTime::from_millis(1);
        let missing = GitObjectId::parse("11".repeat(20)).expect("syntactically valid oid");

        assert!(matches!(
            coordinator.provision(spec(1_000), &missing, now),
            Err(SameRepositoryCoordinatorError::Worktree(
                SessionWorktreeError::ObjectNotFound
            ))
        ));
        assert!(
            sessions
                .inactive_handles(now)
                .expect("inactive scan")
                .is_empty()
        );

        let session = coordinator
            .provision(spec(1_000), &fixture.head, now)
            .expect("capacity must be reusable after rollback");
        coordinator.revoke(&session.grant.handle).expect("revoke");
        let report = coordinator
            .try_reap(&session.grant.handle, now)
            .expect("reap");
        assert!(report.session_removed);
    }
}
