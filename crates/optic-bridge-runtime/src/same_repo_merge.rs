use std::sync::Arc;

use optic_bridge_core::{ActionId, GitObjectId, IdError};
use thiserror::Error;

use crate::{
    GitIntegrationError, GitIntegrationMode, GitIntegrationResult, GitIntegrationService,
    QuiescentSessionSeal, SessionLifecycleError, SessionLifecycleManager, SessionMergeCommit,
    SessionReapReport, SessionWorktreeError, SessionWorktreeManager,
};

pub struct SameRepositoryMergePublisher {
    worktrees: Arc<SessionWorktreeManager>,
    integration: Arc<GitIntegrationService>,
}

pub struct SameRepositoryMergeOrchestrator {
    lifecycle: Arc<SessionLifecycleManager>,
    publisher: SameRepositoryMergePublisher,
}

#[derive(Debug)]
pub struct SessionMergeLifecycleOutcome {
    pub publication: SessionMergePublishOutcome,
    pub left_cleanup: SessionMergeCleanupOutcome,
    pub right_cleanup: SessionMergeCleanupOutcome,
}

impl SessionMergeLifecycleOutcome {
    #[must_use]
    pub fn cleanup_complete(&self) -> bool {
        self.left_cleanup.is_reaped() && self.right_cleanup.is_reaped()
    }
}

#[derive(Debug)]
pub enum SessionMergeCleanupOutcome {
    Reaped(SessionReapReport),
    Incomplete(SessionReapReport),
    Failed(SessionLifecycleError),
}

impl SessionMergeCleanupOutcome {
    #[must_use]
    pub fn is_reaped(&self) -> bool {
        matches!(self, Self::Reaped(_))
    }
}

#[derive(Debug)]
pub enum SessionMergePublishOutcome {
    Published {
        merge: SessionMergeCommit,
        integration: GitIntegrationResult,
    },
    AlreadyPublished {
        merge: SessionMergeCommit,
    },
    NoChanges {
        head: GitObjectId,
    },
}

impl SameRepositoryMergeOrchestrator {
    pub fn new(
        lifecycle: Arc<SessionLifecycleManager>,
        worktrees: Arc<SessionWorktreeManager>,
        integration: Arc<GitIntegrationService>,
    ) -> Result<Self, SameRepositoryMergePublishError> {
        let publisher = SameRepositoryMergePublisher::new(worktrees, integration)?;
        Ok(Self {
            lifecycle,
            publisher,
        })
    }

    pub fn merge_publish_and_cleanup(
        &self,
        left: &optic_bridge_core::SessionHandle,
        right: &optic_bridge_core::SessionHandle,
        now: optic_bridge_core::MonotonicTime,
    ) -> Result<SessionMergeLifecycleOutcome, SameRepositoryMergeOrchestrationError> {
        if left == right {
            return Err(SameRepositoryMergeOrchestrationError::SameSession);
        }
        let left_seal = self
            .lifecycle
            .seal_quiescent(left)
            .map_err(SameRepositoryMergeOrchestrationError::LeftSeal)?;
        let right_seal = self
            .lifecycle
            .seal_quiescent(right)
            .map_err(SameRepositoryMergeOrchestrationError::RightSeal)?;
        let publication = self
            .publisher
            .publish(&left_seal, &right_seal)
            .map_err(SameRepositoryMergeOrchestrationError::Publish)?;
        Ok(self.cleanup_after_publish(publication, left, right, now))
    }

    fn cleanup_after_publish(
        &self,
        publication: SessionMergePublishOutcome,
        left: &optic_bridge_core::SessionHandle,
        right: &optic_bridge_core::SessionHandle,
        now: optic_bridge_core::MonotonicTime,
    ) -> SessionMergeLifecycleOutcome {
        let left_cleanup = cleanup_session(&self.lifecycle, left, now);
        let right_cleanup = cleanup_session(&self.lifecycle, right, now);
        SessionMergeLifecycleOutcome {
            publication,
            left_cleanup,
            right_cleanup,
        }
    }
}

fn cleanup_session(
    lifecycle: &SessionLifecycleManager,
    session: &optic_bridge_core::SessionHandle,
    now: optic_bridge_core::MonotonicTime,
) -> SessionMergeCleanupOutcome {
    match lifecycle.try_reap(session, now) {
        Ok(report) if report.session_removed => SessionMergeCleanupOutcome::Reaped(report),
        Ok(report) => SessionMergeCleanupOutcome::Incomplete(report),
        Err(error) => SessionMergeCleanupOutcome::Failed(error),
    }
}

impl SameRepositoryMergePublisher {
    pub fn new(
        worktrees: Arc<SessionWorktreeManager>,
        integration: Arc<GitIntegrationService>,
    ) -> Result<Self, SameRepositoryMergePublishError> {
        if worktrees.repository_root() != integration.repository_root() {
            return Err(SameRepositoryMergePublishError::RepositoryMismatch);
        }
        let worktree_root = worktrees.worktree_root();
        let integration_root = integration.integration_root();
        if worktree_root.starts_with(integration_root)
            || integration_root.starts_with(worktree_root)
        {
            return Err(SameRepositoryMergePublishError::RuntimeRootOverlap);
        }
        Ok(Self {
            worktrees,
            integration,
        })
    }

    pub fn publish(
        &self,
        left: &QuiescentSessionSeal,
        right: &QuiescentSessionSeal,
    ) -> Result<SessionMergePublishOutcome, SameRepositoryMergePublishError> {
        if left.session() == right.session() {
            return Err(SameRepositoryMergePublishError::SameSession);
        }

        let left_worktree = self.worktrees.get(left.session())?;
        let right_worktree = self.worktrees.get(right.session())?;
        if left_worktree.base_head != right_worktree.base_head {
            return Err(SameRepositoryMergePublishError::SessionBaseMismatch);
        }

        if left_worktree.current_head == right_worktree.current_head {
            let shared_head = left_worktree.current_head.clone();
            if shared_head != left_worktree.base_head {
                return Err(
                    SameRepositoryMergePublishError::SharedChangedHeadUnsupported {
                        head: shared_head,
                    },
                );
            }
            let observed_target = self.integration.target_head()?;
            if observed_target != shared_head {
                return Err(SameRepositoryMergePublishError::TargetNotAtSessionBase {
                    expected: shared_head,
                    observed: observed_target,
                });
            }
            return Ok(SessionMergePublishOutcome::NoChanges { head: shared_head });
        }

        let merge = self
            .worktrees
            .write_merge_commit(left.session(), right.session())?;
        if merge.base_head != left_worktree.base_head {
            return Err(SameRepositoryMergePublishError::MergeContractMismatch);
        }

        let observed_target = self.integration.target_head()?;
        if observed_target == merge.commit {
            return Ok(SessionMergePublishOutcome::AlreadyPublished { merge });
        }
        if observed_target != merge.base_head {
            return Err(SameRepositoryMergePublishError::TargetNotAtSessionBase {
                expected: merge.base_head.clone(),
                observed: observed_target,
            });
        }

        let action_id =
            ActionId::generate().map_err(SameRepositoryMergePublishError::ActionIdGeneration)?;
        let integration = match self.integration.integrate_fast_forward(
            &action_id,
            &merge.commit,
            &merge.base_head,
        ) {
            Ok(result) => result,
            Err(GitIntegrationError::StaleTarget { observed, .. }) if observed == merge.commit => {
                return Ok(SessionMergePublishOutcome::AlreadyPublished { merge });
            }
            Err(error) => return Err(error.into()),
        };

        if integration.mode != GitIntegrationMode::FastForward
            || integration.previous_target_head != merge.base_head
            || integration.new_target_head != merge.commit
        {
            return Err(SameRepositoryMergePublishError::IntegrationVerificationMismatch);
        }

        Ok(SessionMergePublishOutcome::Published { merge, integration })
    }
}

#[derive(Debug, Error)]
pub enum SameRepositoryMergeOrchestrationError {
    #[error("a session cannot be merged with itself")]
    SameSession,
    #[error("left session could not be sealed quiescently: {0}")]
    LeftSeal(SessionLifecycleError),
    #[error("right session could not be sealed quiescently after left-session revoke: {0}")]
    RightSeal(SessionLifecycleError),
    #[error("sealed sessions could not be published: {0}")]
    Publish(SameRepositoryMergePublishError),
}

#[derive(Debug, Error)]
pub enum SameRepositoryMergePublishError {
    #[error("session worktree manager and Git integration service target different repositories")]
    RepositoryMismatch,
    #[error("session worktree root overlaps the Git integration runtime root")]
    RuntimeRootOverlap,
    #[error("a session cannot be merged with itself")]
    SameSession,
    #[error("sealed sessions do not share the same exact base HEAD")]
    SessionBaseMismatch,
    #[error(
        "sealed sessions share changed HEAD {head:?}; direct shared-head publication is not yet lineage-validated"
    )]
    SharedChangedHeadUnsupported { head: GitObjectId },
    #[error("deterministic merge output no longer matches the sealed session base")]
    MergeContractMismatch,
    #[error(
        "Git integration target moved away from sealed session base: expected {expected:?}, observed {observed:?}"
    )]
    TargetNotAtSessionBase {
        expected: GitObjectId,
        observed: GitObjectId,
    },
    #[error("Git integration result did not match the deterministic merge commit")]
    IntegrationVerificationMismatch,
    #[error("failed to generate application-owned integration action id: {0}")]
    ActionIdGeneration(IdError),
    #[error(transparent)]
    Worktree(#[from] SessionWorktreeError),
    #[error(transparent)]
    Integration(#[from] GitIntegrationError),
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

    use optic_bridge_core::{Capability, HardLimits, MonotonicTime, PrincipalId, ProjectId};

    use super::*;
    use crate::{
        ApprovalBroker, ProcessManager, SameRepositorySessionCoordinator, SessionRegistry,
        TaskLeaseRegistry,
    };

    const TARGET_REF: &str = "refs/optic/integration/session-merge";

    struct RepoFixture {
        base: PathBuf,
        repo: PathBuf,
        worktrees: PathBuf,
        integration: PathBuf,
        git: PathBuf,
        head: GitObjectId,
    }

    impl RepoFixture {
        fn new(git: PathBuf) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let base = env::temp_dir().join(format!("optic-session-merge-publish-{nonce}"));
            let repo = base.join("repo");
            let worktrees = base.join("worktrees");
            let integration = base.join("integration");
            fs::create_dir_all(&repo).expect("repo");
            fs::create_dir_all(&worktrees).expect("worktrees");
            fs::create_dir_all(&integration).expect("integration");
            run_git(&git, &repo, &["init", "--quiet"]);
            run_git(
                &git,
                &repo,
                &["config", "user.email", "optic@example.invalid"],
            );
            run_git(&git, &repo, &["config", "user.name", "Optic Test"]);
            fs::write(repo.join("tracked.txt"), b"base\n").expect("fixture file");
            run_git(&git, &repo, &["add", "."]);
            run_git(&git, &repo, &["commit", "--quiet", "-m", "base"]);
            let head = parse_oid(&git_output(&git, &repo, &["rev-parse", "HEAD"]));
            run_git(&git, &repo, &["update-ref", TARGET_REF, head.as_str()]);
            Self {
                base,
                repo,
                worktrees,
                integration,
                git,
                head,
            }
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

    fn run_git(git: &Path, repo: &Path, args: &[&str]) {
        let status = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("fixture git");
        assert!(status.success());
    }

    fn git_output(git: &Path, repo: &Path, args: &[&str]) -> Vec<u8> {
        let output = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("fixture git output");
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        output.stdout
    }

    fn parse_oid(bytes: &[u8]) -> GitObjectId {
        GitObjectId::parse(
            std::str::from_utf8(bytes)
                .expect("oid utf8")
                .trim()
                .to_owned(),
        )
        .expect("oid")
    }

    fn spec(label: &str) -> crate::SessionGrantSpec {
        crate::SessionGrantSpec {
            principal: PrincipalId::new(format!("principal-{label}")).expect("principal"),
            project: ProjectId::new("project").expect("project"),
            capabilities: BTreeSet::from([Capability::FileRead]),
            expires_at: MonotonicTime::from_millis(10_000),
            policy_epoch: 1,
        }
    }

    #[test]
    fn unchanged_shared_base_is_explicit_noop_and_cleanup_succeeds() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git);
        let limits = HardLimits {
            max_sessions: 2,
            ..HardLimits::default()
        };
        let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
        let leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
        let processes = Arc::new(
            ProcessManager::new(&fixture.repo, limits, Vec::new()).expect("process manager"),
        );
        let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits).expect("approvals"));
        let worktrees = Arc::new(
            SessionWorktreeManager::from_hard_limits(
                &fixture.repo,
                &fixture.git,
                &fixture.worktrees,
                limits,
            )
            .expect("worktree manager"),
        );
        let coordinator = SameRepositorySessionCoordinator::new(
            sessions,
            leases,
            processes,
            approvals,
            Arc::clone(&worktrees),
        );
        let integration = Arc::new(
            GitIntegrationService::from_hard_limits(
                &fixture.repo,
                &fixture.git,
                &fixture.integration,
                TARGET_REF,
                limits,
            )
            .expect("integration"),
        );
        let orchestrator = SameRepositoryMergeOrchestrator::new(
            coordinator.lifecycle(),
            Arc::clone(&worktrees),
            Arc::clone(&integration),
        )
        .expect("orchestrator");
        let now = MonotonicTime::from_millis(10);
        let left = coordinator
            .provision(spec("noop-left"), &fixture.head, now)
            .expect("left session");
        let right = coordinator
            .provision(spec("noop-right"), &fixture.head, now)
            .expect("right session");

        let outcome = orchestrator
            .merge_publish_and_cleanup(&left.grant.handle, &right.grant.handle, now)
            .expect("no-op merge lifecycle");
        match &outcome.publication {
            SessionMergePublishOutcome::NoChanges { head } => {
                assert_eq!(head, &fixture.head);
            }
            _ => panic!("unchanged shared base must not create a merge commit"),
        }
        assert!(outcome.cleanup_complete());
        assert_eq!(integration.target_head().expect("target"), fixture.head);
        assert!(!left.worktree.path.exists());
        assert!(!right.worktree.path.exists());
    }

    #[test]
    fn shared_changed_head_fails_closed_before_duplicate_parent_commit() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git);
        let limits = HardLimits {
            max_sessions: 2,
            ..HardLimits::default()
        };
        let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
        let leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
        let processes = Arc::new(
            ProcessManager::new(&fixture.repo, limits, Vec::new()).expect("process manager"),
        );
        let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits).expect("approvals"));
        let worktrees = Arc::new(
            SessionWorktreeManager::from_hard_limits(
                &fixture.repo,
                &fixture.git,
                &fixture.worktrees,
                limits,
            )
            .expect("worktree manager"),
        );
        let coordinator = SameRepositorySessionCoordinator::new(
            sessions,
            leases,
            processes,
            approvals,
            Arc::clone(&worktrees),
        );
        let integration = Arc::new(
            GitIntegrationService::from_hard_limits(
                &fixture.repo,
                &fixture.git,
                &fixture.integration,
                TARGET_REF,
                limits,
            )
            .expect("integration"),
        );
        let publisher =
            SameRepositoryMergePublisher::new(Arc::clone(&worktrees), Arc::clone(&integration))
                .expect("publisher");
        let now = MonotonicTime::from_millis(10);
        let left = coordinator
            .provision(spec("shared-left"), &fixture.head, now)
            .expect("left session");
        let right = coordinator
            .provision(spec("shared-right"), &fixture.head, now)
            .expect("right session");

        let tree = parse_oid(&git_output(
            &fixture.git,
            &fixture.repo,
            &["rev-parse", "HEAD^{tree}"],
        ));
        let shared_head = parse_oid(&git_output(
            &fixture.git,
            &fixture.repo,
            &[
                "commit-tree",
                tree.as_str(),
                "-p",
                fixture.head.as_str(),
                "-m",
                "shared changed head",
            ],
        ));
        run_git(
            &fixture.git,
            &left.worktree.path,
            &["update-ref", "HEAD", shared_head.as_str()],
        );
        run_git(
            &fixture.git,
            &right.worktree.path,
            &["update-ref", "HEAD", shared_head.as_str()],
        );

        let lifecycle = coordinator.lifecycle();
        let left_seal = lifecycle
            .seal_quiescent(&left.grant.handle)
            .expect("left seal");
        let right_seal = lifecycle
            .seal_quiescent(&right.grant.handle)
            .expect("right seal");
        assert!(matches!(
            publisher.publish(&left_seal, &right_seal),
            Err(SameRepositoryMergePublishError::SharedChangedHeadUnsupported { head })
                if head == shared_head
        ));
        assert_eq!(integration.target_head().expect("target"), fixture.head);
    }

    #[test]
    fn published_merge_reports_cleanup_failure_without_hiding_publication() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git);
        let limits = HardLimits {
            max_sessions: 2,
            ..HardLimits::default()
        };
        let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
        let leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
        let processes = Arc::new(
            ProcessManager::new(&fixture.repo, limits, Vec::new()).expect("process manager"),
        );
        let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits).expect("approvals"));
        let worktrees = Arc::new(
            SessionWorktreeManager::from_hard_limits(
                &fixture.repo,
                &fixture.git,
                &fixture.worktrees,
                limits,
            )
            .expect("worktree manager"),
        );
        let coordinator = SameRepositorySessionCoordinator::new(
            sessions,
            leases,
            processes,
            approvals,
            Arc::clone(&worktrees),
        );
        let integration = Arc::new(
            GitIntegrationService::from_hard_limits(
                &fixture.repo,
                &fixture.git,
                &fixture.integration,
                TARGET_REF,
                limits,
            )
            .expect("integration"),
        );
        let publisher =
            SameRepositoryMergePublisher::new(Arc::clone(&worktrees), Arc::clone(&integration))
                .expect("publisher");
        let now = MonotonicTime::from_millis(10);
        let left = coordinator
            .provision(spec("cleanup-left"), &fixture.head, now)
            .expect("left session");
        let right = coordinator
            .provision(spec("cleanup-right"), &fixture.head, now)
            .expect("right session");

        let tree = parse_oid(&git_output(
            &fixture.git,
            &fixture.repo,
            &["rev-parse", "HEAD^{tree}"],
        ));
        let left_head = parse_oid(&git_output(
            &fixture.git,
            &fixture.repo,
            &[
                "commit-tree",
                tree.as_str(),
                "-p",
                fixture.head.as_str(),
                "-m",
                "cleanup left",
            ],
        ));
        let right_head = parse_oid(&git_output(
            &fixture.git,
            &fixture.repo,
            &[
                "commit-tree",
                tree.as_str(),
                "-p",
                fixture.head.as_str(),
                "-m",
                "cleanup right",
            ],
        ));
        assert_ne!(left_head, right_head);
        run_git(
            &fixture.git,
            &left.worktree.path,
            &["update-ref", "HEAD", left_head.as_str()],
        );
        run_git(
            &fixture.git,
            &right.worktree.path,
            &["update-ref", "HEAD", right_head.as_str()],
        );

        let lifecycle = coordinator.lifecycle();
        let left_seal = lifecycle
            .seal_quiescent(&left.grant.handle)
            .expect("left seal");
        let right_seal = lifecycle
            .seal_quiescent(&right.grant.handle)
            .expect("right seal");
        let publication = publisher
            .publish(&left_seal, &right_seal)
            .expect("publication");
        let published_commit = match &publication {
            SessionMergePublishOutcome::Published { merge, .. }
            | SessionMergePublishOutcome::AlreadyPublished { merge } => merge.commit.clone(),
            SessionMergePublishOutcome::NoChanges { head } => head.clone(),
        };
        assert_eq!(integration.target_head().expect("target"), published_commit);

        fs::remove_dir_all(&left.worktree.path).expect("sabotage left cleanup path");
        let orchestrator = SameRepositoryMergeOrchestrator {
            lifecycle: Arc::clone(&lifecycle),
            publisher,
        };
        let outcome = orchestrator.cleanup_after_publish(
            publication,
            &left.grant.handle,
            &right.grant.handle,
            now,
        );
        assert!(!outcome.cleanup_complete());
        assert!(matches!(
            outcome.left_cleanup,
            SessionMergeCleanupOutcome::Failed(_)
        ));
        assert!(matches!(
            outcome.right_cleanup,
            SessionMergeCleanupOutcome::Reaped(_)
        ));
        assert_eq!(
            integration.target_head().expect("target after cleanup"),
            published_commit
        );
    }

    #[test]
    fn sealed_session_merge_is_published_once_and_retry_is_idempotent() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git);
        let limits = HardLimits {
            max_sessions: 2,
            ..HardLimits::default()
        };
        let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
        let leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
        let processes = Arc::new(
            ProcessManager::new(&fixture.repo, limits, Vec::new()).expect("process manager"),
        );
        let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits).expect("approvals"));
        let worktrees = Arc::new(
            SessionWorktreeManager::from_hard_limits(
                &fixture.repo,
                &fixture.git,
                &fixture.worktrees,
                limits,
            )
            .expect("worktree manager"),
        );
        let coordinator = SameRepositorySessionCoordinator::new(
            sessions,
            leases,
            processes,
            approvals,
            Arc::clone(&worktrees),
        );
        let integration = Arc::new(
            GitIntegrationService::from_hard_limits(
                &fixture.repo,
                &fixture.git,
                &fixture.integration,
                TARGET_REF,
                limits,
            )
            .expect("integration"),
        );
        let publisher =
            SameRepositoryMergePublisher::new(Arc::clone(&worktrees), Arc::clone(&integration))
                .expect("publisher");
        let now = MonotonicTime::from_millis(10);
        let left = coordinator
            .provision(spec("left"), &fixture.head, now)
            .expect("left session");
        let right = coordinator
            .provision(spec("right"), &fixture.head, now)
            .expect("right session");

        let tree = parse_oid(&git_output(
            &fixture.git,
            &fixture.repo,
            &["rev-parse", "HEAD^{tree}"],
        ));
        let left_head = parse_oid(&git_output(
            &fixture.git,
            &fixture.repo,
            &[
                "commit-tree",
                tree.as_str(),
                "-p",
                fixture.head.as_str(),
                "-m",
                "left session",
            ],
        ));
        let right_head = parse_oid(&git_output(
            &fixture.git,
            &fixture.repo,
            &[
                "commit-tree",
                tree.as_str(),
                "-p",
                fixture.head.as_str(),
                "-m",
                "right session",
            ],
        ));
        assert_ne!(left_head, right_head);
        run_git(
            &fixture.git,
            &left.worktree.path,
            &["update-ref", "HEAD", left_head.as_str()],
        );
        run_git(
            &fixture.git,
            &right.worktree.path,
            &["update-ref", "HEAD", right_head.as_str()],
        );

        let lifecycle = coordinator.lifecycle();
        let left_seal = lifecycle
            .seal_quiescent(&left.grant.handle)
            .expect("left seal");
        let right_seal = lifecycle
            .seal_quiescent(&right.grant.handle)
            .expect("right seal");
        let first = publisher
            .publish(&left_seal, &right_seal)
            .expect("publish merge");
        let published_commit = match first {
            SessionMergePublishOutcome::Published {
                merge,
                integration: result,
            } => {
                assert_eq!(result.previous_target_head, fixture.head);
                assert_eq!(result.new_target_head, merge.commit);
                merge.commit
            }
            SessionMergePublishOutcome::AlreadyPublished { .. } => {
                panic!("first publish cannot already be complete")
            }
            SessionMergePublishOutcome::NoChanges { .. } => {
                panic!("changed workers cannot publish as no-op")
            }
        };
        assert_eq!(integration.target_head().expect("target"), published_commit);

        let second = publisher
            .publish(&left_seal, &right_seal)
            .expect("idempotent retry");
        match second {
            SessionMergePublishOutcome::AlreadyPublished { merge } => {
                assert_eq!(merge.commit, published_commit);
            }
            SessionMergePublishOutcome::Published { .. } => {
                panic!("retry must not mutate the ref twice")
            }
            SessionMergePublishOutcome::NoChanges { .. } => {
                panic!("retry of changed merge cannot become no-op")
            }
        }

        assert!(
            coordinator
                .try_reap(&left.grant.handle, now)
                .expect("reap left")
                .session_removed
        );
        assert!(
            coordinator
                .try_reap(&right.grant.handle, now)
                .expect("reap right")
                .session_removed
        );
    }
}
