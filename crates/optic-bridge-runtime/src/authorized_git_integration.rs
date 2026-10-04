use std::{path::Path, sync::Arc};

use optic_bridge_core::{ActionEnvelope, Effect, HardLimits};
use optic_bridge_policy::{PolicyDecision, PolicyEngine, PolicyReason};
use thiserror::Error;

use crate::{
    Clock, GitIntegrationError, GitIntegrationResult, GitIntegrationService, SessionRegistry,
    SessionRegistryError, TaskLeaseRegistry, TaskLeaseRegistryError,
};

/// Transport-agnostic authorization boundary for Git integration.
///
/// Adapters must normalize the request into an `ActionEnvelope` first. This
/// boundary then resolves the application-owned session and exact task lease,
/// evaluates deterministic policy, verifies the effect shape, and only then
/// reaches the Git integration runtime.
pub struct AuthorizedGitIntegrationService {
    integration: GitIntegrationService,
    sessions: Arc<SessionRegistry>,
    task_leases: Arc<TaskLeaseRegistry>,
    clock: Arc<dyn Clock>,
    policy: PolicyEngine,
}

impl AuthorizedGitIntegrationService {
    #[allow(clippy::too_many_arguments)]
    pub fn from_hard_limits(
        repository_root: impl AsRef<Path>,
        git_executable: impl AsRef<Path>,
        integration_root: impl AsRef<Path>,
        target_ref: impl Into<String>,
        limits: HardLimits,
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, AuthorizedGitIntegrationError> {
        let integration = GitIntegrationService::from_hard_limits(
            repository_root,
            git_executable,
            integration_root,
            target_ref,
            limits,
        )?;
        Ok(Self::from_runtime(
            integration,
            sessions,
            task_leases,
            clock,
        ))
    }

    #[must_use]
    pub fn from_runtime(
        integration: GitIntegrationService,
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            integration,
            sessions,
            task_leases,
            clock,
            policy: PolicyEngine,
        }
    }

    #[must_use]
    pub fn repository_root(&self) -> &Path {
        self.integration.repository_root()
    }

    #[must_use]
    pub fn integration_root(&self) -> &Path {
        self.integration.integration_root()
    }

    #[must_use]
    pub fn target_ref(&self) -> &str {
        self.integration.target_ref()
    }

    pub fn integrate_fast_forward(
        &self,
        envelope: &ActionEnvelope,
    ) -> Result<GitIntegrationResult, AuthorizedGitIntegrationError> {
        let Effect::GitIntegrate {
            source_head,
            expected_target_head,
        } = &envelope.effect
        else {
            return Err(AuthorizedGitIntegrationError::EffectMismatch);
        };

        self.authorize(envelope)?;
        Ok(self.integration.integrate_fast_forward(
            &envelope.action_id,
            source_head,
            expected_target_head,
        )?)
    }

    fn authorize(&self, envelope: &ActionEnvelope) -> Result<(), AuthorizedGitIntegrationError> {
        let now = self.clock.now();
        let session = self.sessions.get_active(&envelope.session, now)?;
        let lease_id = envelope
            .task_lease
            .as_ref()
            .ok_or(AuthorizedGitIntegrationError::MissingTaskLease)?;
        let lease = self
            .task_leases
            .get_active(lease_id, &envelope.session, now)?;

        match self.policy.evaluate(envelope, &session, Some(&lease), now) {
            PolicyDecision::Allow => Ok(()),
            PolicyDecision::RequireApproval(reason) | PolicyDecision::Deny(reason) => {
                Err(AuthorizedGitIntegrationError::PolicyDenied(reason))
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthorizedGitIntegrationError {
    #[error("application session is not active: {0}")]
    Session(#[from] SessionRegistryError),
    #[error("task lease is not active for this session: {0}")]
    TaskLease(#[from] TaskLeaseRegistryError),
    #[error("Git integration requires an exact application-owned task lease")]
    MissingTaskLease,
    #[error("Git integration policy denied the normalized action: {0:?}")]
    PolicyDenied(PolicyReason),
    #[error("normalized effect does not match GitIntegrate")]
    EffectMismatch,
    #[error("Git integration runtime failed: {0}")]
    Integration(#[from] GitIntegrationError),
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        env,
        ffi::OsStr,
        fs,
        path::{Path, PathBuf},
        process::Command,
    };

    use optic_bridge_core::{
        ActionId, Capability, GitObjectId, LeaseScope, MonotonicTime, PrincipalId, ProjectId,
        ResourceBudget, SessionGrant, SessionHandle, TaskLease, TaskLeaseId,
    };

    use super::*;

    const TARGET_REF: &str = "refs/optic/integration/authorized";

    #[derive(Debug)]
    struct FixedClock(MonotonicTime);

    impl Clock for FixedClock {
        fn now(&self) -> MonotonicTime {
            self.0
        }
    }

    struct RepoFixture {
        base: PathBuf,
        repo: PathBuf,
        integration_root: PathBuf,
        git: PathBuf,
        initial: GitObjectId,
        source: GitObjectId,
    }

    impl RepoFixture {
        fn new(label: &str) -> Option<Self> {
            let git = find_git_executable()?;
            let token = ActionId::generate().ok()?.to_token();
            let base = env::temp_dir().join(format!("optic-auth-git-{label}-{}", &token[..12]));
            let repo = base.join("repo");
            let integration_root = base.join("integration");
            fs::create_dir_all(&repo).ok()?;
            fs::create_dir_all(&integration_root).ok()?;
            run_git(
                &git,
                Some(&repo),
                [OsStr::new("init"), OsStr::new("--quiet")],
            );
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("config"),
                    OsStr::new("user.email"),
                    OsStr::new("optic@example.invalid"),
                ],
            );
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("config"),
                    OsStr::new("user.name"),
                    OsStr::new("Optic Test"),
                ],
            );
            fs::write(repo.join("tracked.txt"), b"alpha\n").ok()?;
            commit_all(&git, &repo, "initial");
            let initial = rev_parse(&git, &repo, "HEAD");
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("update-ref"),
                    OsStr::new(TARGET_REF),
                    OsStr::new(initial.as_str()),
                ],
            );
            fs::write(repo.join("tracked.txt"), b"bravo\n").ok()?;
            commit_all(&git, &repo, "source");
            let source = rev_parse(&git, &repo, "HEAD");
            Some(Self {
                base,
                repo,
                integration_root,
                git,
                initial,
                source,
            })
        }

        fn target_head(&self) -> GitObjectId {
            rev_parse(&self.git, &self.repo, TARGET_REF)
        }
    }

    impl Drop for RepoFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn now() -> MonotonicTime {
        MonotonicTime::from_millis(10_000)
    }

    fn budget() -> ResourceBudget {
        ResourceBudget {
            timeout_ms: 10_000,
            output_bytes: 64 * 1024,
            memory_bytes: 64 * 1024 * 1024,
            process_count: 1,
        }
    }

    fn session(capabilities: &[Capability]) -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().expect("session entropy"),
            principal: PrincipalId::new("test-principal").expect("principal"),
            project: ProjectId::new("test-project").expect("project"),
            capabilities: capabilities.iter().copied().collect::<BTreeSet<_>>(),
            expires_at: now().saturating_add_millis(60_000),
            policy_epoch: 7,
        }
    }

    fn lease(
        session: &SessionGrant,
        capabilities: &[Capability],
        scopes: &[LeaseScope],
    ) -> TaskLease {
        TaskLease {
            id: TaskLeaseId::generate().expect("lease entropy"),
            session: session.handle.clone(),
            capabilities: capabilities.iter().copied().collect::<BTreeSet<_>>(),
            scopes: scopes.iter().cloned().collect::<BTreeSet<_>>(),
            resource_ceiling: budget(),
            expires_at: now().saturating_add_millis(30_000),
            policy_epoch: session.policy_epoch,
        }
    }

    fn envelope(
        session: &SessionGrant,
        lease: Option<&TaskLease>,
        source: &GitObjectId,
        expected: &GitObjectId,
    ) -> ActionEnvelope {
        ActionEnvelope {
            action_id: ActionId::generate().expect("action entropy"),
            session: session.handle.clone(),
            task_lease: lease.map(|value| value.id.clone()),
            effect: Effect::GitIntegrate {
                source_head: source.clone(),
                expected_target_head: expected.clone(),
            },
            resources: budget(),
            policy_epoch: session.policy_epoch,
        }
    }

    fn service(
        fixture: &RepoFixture,
        sessions: Arc<SessionRegistry>,
        leases: Arc<TaskLeaseRegistry>,
    ) -> AuthorizedGitIntegrationService {
        AuthorizedGitIntegrationService::from_hard_limits(
            &fixture.repo,
            &fixture.git,
            &fixture.integration_root,
            TARGET_REF,
            HardLimits::default(),
            sessions,
            leases,
            Arc::new(FixedClock(now())),
        )
        .expect("service")
    }

    #[test]
    fn missing_task_lease_fails_before_target_update() {
        let Some(fixture) = RepoFixture::new("missing-lease") else {
            return;
        };
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::GitIntegrate]);
        sessions.register(grant.clone()).expect("session");
        let service = service(&fixture, sessions, leases);
        let action = envelope(&grant, None, &fixture.source, &fixture.initial);

        assert!(matches!(
            service.integrate_fast_forward(&action),
            Err(AuthorizedGitIntegrationError::MissingTaskLease)
        ));
        assert_eq!(fixture.target_head(), fixture.initial);
    }

    #[test]
    fn repository_scope_is_required_before_target_update() {
        let Some(fixture) = RepoFixture::new("scope") else {
            return;
        };
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::GitIntegrate]);
        sessions.register(grant.clone()).expect("session");
        let task = lease(
            &grant,
            &[Capability::GitIntegrate],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("lease");
        let service = service(&fixture, sessions, leases);
        let action = envelope(&grant, Some(&task), &fixture.source, &fixture.initial);

        assert!(matches!(
            service.integrate_fast_forward(&action),
            Err(AuthorizedGitIntegrationError::PolicyDenied(
                PolicyReason::ScopeNotAuthorized
            ))
        ));
        assert_eq!(fixture.target_head(), fixture.initial);
    }

    #[test]
    fn session_capability_is_required_even_with_repository_lease() {
        let Some(fixture) = RepoFixture::new("session-capability") else {
            return;
        };
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::GitRead]);
        sessions.register(grant.clone()).expect("session");
        let task = lease(
            &grant,
            &[Capability::GitIntegrate],
            &[LeaseScope::Repository],
        );
        leases.register(task.clone()).expect("lease");
        let service = service(&fixture, sessions, leases);
        let action = envelope(&grant, Some(&task), &fixture.source, &fixture.initial);

        assert!(matches!(
            service.integrate_fast_forward(&action),
            Err(AuthorizedGitIntegrationError::PolicyDenied(
                PolicyReason::MissingCapability
            ))
        ));
        assert_eq!(fixture.target_head(), fixture.initial);
    }

    #[test]
    fn cross_session_lease_is_rejected_before_target_update() {
        let Some(fixture) = RepoFixture::new("cross-session") else {
            return;
        };
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let first = session(&[Capability::GitIntegrate]);
        let second = session(&[Capability::GitIntegrate]);
        sessions.register(first.clone()).expect("first session");
        sessions.register(second.clone()).expect("second session");
        let task = lease(
            &second,
            &[Capability::GitIntegrate],
            &[LeaseScope::Repository],
        );
        leases.register(task.clone()).expect("lease");
        let service = service(&fixture, sessions, leases);
        let action = envelope(&first, Some(&task), &fixture.source, &fixture.initial);

        assert!(matches!(
            service.integrate_fast_forward(&action),
            Err(AuthorizedGitIntegrationError::TaskLease(
                TaskLeaseRegistryError::WrongSession
            ))
        ));
        assert_eq!(fixture.target_head(), fixture.initial);
    }

    #[test]
    fn matching_session_and_repository_lease_reach_runtime() {
        let Some(fixture) = RepoFixture::new("success") else {
            return;
        };
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::GitIntegrate]);
        sessions.register(grant.clone()).expect("session");
        let task = lease(
            &grant,
            &[Capability::GitIntegrate],
            &[LeaseScope::Repository],
        );
        leases.register(task.clone()).expect("lease");
        let service = service(&fixture, sessions, leases);
        let action = envelope(&grant, Some(&task), &fixture.source, &fixture.initial);

        let result = service
            .integrate_fast_forward(&action)
            .expect("authorized integration");
        assert_eq!(result.previous_target_head, fixture.initial);
        assert_eq!(result.new_target_head, fixture.source);
        assert_eq!(fixture.target_head(), fixture.source);
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

    fn run_git<const N: usize>(git: &Path, repo: Option<&Path>, args: [&OsStr; N]) {
        let mut command = Command::new(git);
        if let Some(repo) = repo {
            command.arg("-C").arg(repo);
        }
        let status = command.args(args).status().expect("run fixture git");
        assert!(status.success(), "fixture git command failed");
    }

    fn commit_all(git: &Path, repo: &Path, message: &str) {
        run_git(git, Some(repo), [OsStr::new("add"), OsStr::new(".")]);
        run_git(
            git,
            Some(repo),
            [
                OsStr::new("commit"),
                OsStr::new("--quiet"),
                OsStr::new("-m"),
                OsStr::new(message),
            ],
        );
    }

    fn rev_parse(git: &Path, repo: &Path, value: &str) -> GitObjectId {
        let output = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(["rev-parse", "--verify", value])
            .output()
            .expect("rev-parse");
        assert!(output.status.success(), "rev-parse failed");
        let value = std::str::from_utf8(&output.stdout)
            .expect("utf8 oid")
            .trim();
        GitObjectId::parse(value.to_owned()).expect("oid")
    }
}
