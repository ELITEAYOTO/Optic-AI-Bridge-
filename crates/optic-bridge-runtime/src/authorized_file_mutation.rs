use std::{path::Path, sync::Arc};

use optic_bridge_core::{ActionEnvelope, Effect, ExpectedState, HardLimits};
use optic_bridge_policy::{PolicyDecision, PolicyEngine, PolicyReason};
use thiserror::Error;

use crate::{
    BytePatch, Clock, JournaledDeleteCommit, JournaledMutationCommit, SessionRegistry,
    SessionRegistryError, TaskLeaseRegistry, TaskLeaseRegistryError, TransactionalFileError,
    TransactionalFileService,
};

/// Transport-agnostic authorization boundary for durable file mutation.
///
/// Raw adapter arguments must already be normalized into an `ActionEnvelope`.
/// This service resolves the application-owned session and task lease, applies
/// deterministic policy, checks that the normalized effect matches the called
/// mutation primitive, and only then reaches the transactional filesystem.
pub struct AuthorizedFileMutationService {
    transactions: TransactionalFileService,
    sessions: Arc<SessionRegistry>,
    task_leases: Arc<TaskLeaseRegistry>,
    clock: Arc<dyn Clock>,
    policy: PolicyEngine,
}

impl AuthorizedFileMutationService {
    pub fn from_hard_limits(
        workspace_root: impl AsRef<Path>,
        state_root: impl AsRef<Path>,
        limits: HardLimits,
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, AuthorizedFileMutationError> {
        let transactions =
            TransactionalFileService::from_hard_limits(workspace_root, state_root, limits)?;
        Ok(Self {
            transactions,
            sessions,
            task_leases,
            clock,
            policy: PolicyEngine,
        })
    }

    pub fn write(
        &self,
        envelope: &ActionEnvelope,
        content: &[u8],
    ) -> Result<JournaledMutationCommit, AuthorizedFileMutationError> {
        let Effect::FileWrite { path, expected } = &envelope.effect else {
            return Err(AuthorizedFileMutationError::EffectMismatch);
        };
        self.authorize(envelope)?;
        Ok(self
            .transactions
            .write_for_action(&envelope.action_id, path, *expected, content)?)
    }

    pub fn apply_patch(
        &self,
        envelope: &ActionEnvelope,
        patch: &BytePatch,
    ) -> Result<JournaledMutationCommit, AuthorizedFileMutationError> {
        let Effect::FileWrite { path, expected } = &envelope.effect else {
            return Err(AuthorizedFileMutationError::EffectMismatch);
        };
        let ExpectedState::Content(expected) = expected else {
            return Err(AuthorizedFileMutationError::PatchRequiresContentState);
        };
        self.authorize(envelope)?;
        Ok(self
            .transactions
            .apply_patch_for_action(&envelope.action_id, path, *expected, patch)?)
    }

    pub fn delete(
        &self,
        envelope: &ActionEnvelope,
    ) -> Result<JournaledDeleteCommit, AuthorizedFileMutationError> {
        let Effect::FileDelete { path, expected } = &envelope.effect else {
            return Err(AuthorizedFileMutationError::EffectMismatch);
        };
        self.authorize(envelope)?;
        Ok(self
            .transactions
            .delete_for_action(&envelope.action_id, path, *expected)?)
    }

    fn authorize(&self, envelope: &ActionEnvelope) -> Result<(), AuthorizedFileMutationError> {
        let now = self.clock.now();
        let session = self.sessions.get_active(&envelope.session, now)?;
        let lease_id = envelope
            .task_lease
            .as_ref()
            .ok_or(AuthorizedFileMutationError::MissingTaskLease)?;
        let lease = self
            .task_leases
            .get_active(lease_id, &envelope.session, now)?;

        match self.policy.evaluate(envelope, &session, Some(&lease), now) {
            PolicyDecision::Allow => Ok(()),
            PolicyDecision::RequireApproval(reason) | PolicyDecision::Deny(reason) => {
                Err(AuthorizedFileMutationError::PolicyDenied(reason))
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthorizedFileMutationError {
    #[error("application session is not active: {0}")]
    Session(#[from] SessionRegistryError),
    #[error("task lease is not active for this session: {0}")]
    TaskLease(#[from] TaskLeaseRegistryError),
    #[error("file mutation requires an exact application-owned task lease")]
    MissingTaskLease,
    #[error("file mutation policy denied the normalized action: {0:?}")]
    PolicyDenied(PolicyReason),
    #[error("normalized effect does not match the requested mutation primitive")]
    EffectMismatch,
    #[error("patch requires FileWrite with an exact existing ContentVersion")]
    PatchRequiresContentState,
    #[error("transactional file mutation failed: {0}")]
    Transaction(#[from] TransactionalFileError),
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, env, fs, path::PathBuf};

    use optic_bridge_core::{
        ActionId, Capability, ContentVersion, LeaseScope, MonotonicTime, PrincipalId, ProjectId,
        ResourceBudget, SessionGrant, SessionHandle, TaskLease, TaskLeaseId, WorkspacePath,
    };

    use super::*;

    #[derive(Debug)]
    struct FixedClock(MonotonicTime);

    impl Clock for FixedClock {
        fn now(&self) -> MonotonicTime {
            self.0
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

    fn fixture(label: &str) -> (PathBuf, PathBuf, PathBuf) {
        let token = ActionId::generate().expect("test entropy").to_token();
        let base = env::temp_dir().join(format!("optic-authorized-mutation-{label}-{token}"));
        let workspace = base.join("workspace");
        let state = base.join("state");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&state).expect("state");
        (base, workspace, state)
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

    fn path(value: &str) -> WorkspacePath {
        WorkspacePath::parse(value).expect("workspace path")
    }

    fn envelope(
        session: &SessionGrant,
        lease: Option<&TaskLease>,
        effect: Effect,
    ) -> ActionEnvelope {
        ActionEnvelope {
            action_id: ActionId::generate().expect("action entropy"),
            session: session.handle.clone(),
            task_lease: lease.map(|value| value.id.clone()),
            effect,
            resources: budget(),
            policy_epoch: session.policy_epoch,
        }
    }

    fn service(
        workspace: &Path,
        state: &Path,
        limits: HardLimits,
        sessions: Arc<SessionRegistry>,
        leases: Arc<TaskLeaseRegistry>,
    ) -> AuthorizedFileMutationService {
        AuthorizedFileMutationService::from_hard_limits(
            workspace,
            state,
            limits,
            sessions,
            leases,
            Arc::new(FixedClock(now())),
        )
        .expect("service")
    }

    #[test]
    fn write_requires_an_exact_task_lease_before_touching_filesystem() {
        let (base, workspace, state) = fixture("missing-lease");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &grant,
            None,
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Absent,
            },
        );

        assert!(matches!(
            service.write(&action, b"new"),
            Err(AuthorizedFileMutationError::MissingTaskLease)
        ));
        assert!(!workspace.join("target.txt").exists());
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn session_capability_is_required_even_when_lease_has_it() {
        let (base, workspace, state) = fixture("missing-session-capability");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileRead]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Absent,
            },
        );

        assert!(matches!(
            service.write(&action, b"new"),
            Err(AuthorizedFileMutationError::PolicyDenied(
                PolicyReason::MissingCapability
            ))
        ));
        assert!(!workspace.join("target.txt").exists());
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn cross_session_lease_is_rejected_before_policy_execution() {
        let (base, workspace, state) = fixture("cross-session");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let first = session(&[Capability::FileWrite]);
        let second = session(&[Capability::FileWrite]);
        sessions.register(first.clone()).expect("first session");
        sessions.register(second.clone()).expect("second session");
        let task = lease(
            &second,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &first,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Absent,
            },
        );

        assert!(matches!(
            service.write(&action, b"new"),
            Err(AuthorizedFileMutationError::TaskLease(
                TaskLeaseRegistryError::WrongSession
            ))
        ));
        assert!(!workspace.join("target.txt").exists());
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn stale_policy_epoch_is_denied_before_mutation() {
        let (base, workspace, state) = fixture("stale-epoch");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let mut action = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Absent,
            },
        );
        action.policy_epoch = action.policy_epoch.saturating_add(1);

        assert!(matches!(
            service.write(&action, b"new"),
            Err(AuthorizedFileMutationError::PolicyDenied(
                PolicyReason::StalePolicyEpoch
            ))
        ));
        assert!(!workspace.join("target.txt").exists());
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn workspace_scope_escape_is_denied_before_mutation() {
        let (base, workspace, state) = fixture("scope");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspacePrefix(path("src"))],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("docs/README.md"),
                expected: ExpectedState::Absent,
            },
        );

        assert!(matches!(
            service.write(&action, b"new"),
            Err(AuthorizedFileMutationError::PolicyDenied(
                PolicyReason::ScopeNotAuthorized
            ))
        ));
        assert!(!workspace.join("docs/README.md").exists());
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn delete_requires_file_delete_not_file_write_authority() {
        let (base, workspace, state) = fixture("delete-capability");
        fs::write(workspace.join("target.txt"), b"old").expect("fixture");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &grant,
            Some(&task),
            Effect::FileDelete {
                path: path("target.txt"),
                expected: ContentVersion::from_bytes(b"old"),
            },
        );

        assert!(matches!(
            service.delete(&action),
            Err(AuthorizedFileMutationError::PolicyDenied(
                PolicyReason::MissingCapability
            ))
        ));
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("target"),
            b"old"
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn patch_is_authorized_as_file_write() {
        let (base, workspace, state) = fixture("patch-authority");
        fs::write(workspace.join("target.txt"), b"old").expect("fixture");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Content(ContentVersion::from_bytes(b"old")),
            },
        );

        assert!(service.authorize(&action).is_ok());
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn primitive_rejects_mismatched_normalized_effect_before_mutation() {
        let (base, workspace, state) = fixture("effect-mismatch");
        fs::write(workspace.join("target.txt"), b"old").expect("fixture");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Content(ContentVersion::from_bytes(b"old")),
            },
        );

        assert!(matches!(
            service.delete(&action),
            Err(AuthorizedFileMutationError::EffectMismatch)
        ));
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("target"),
            b"old"
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn stale_expected_state_is_rejected_after_authorization_without_modification() {
        let (base, workspace, state) = fixture("stale-state");
        fs::write(workspace.join("target.txt"), b"current").expect("fixture");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Content(ContentVersion::from_bytes(b"stale")),
            },
        );

        assert!(matches!(
            service.write(&action, b"new"),
            Err(AuthorizedFileMutationError::Transaction(_))
        ));
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("target"),
            b"current"
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn oversized_write_is_rejected_after_authorization() {
        let (base, workspace, state) = fixture("oversized");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let limits = HardLimits {
            max_fs_mutation_bytes: 4,
            ..HardLimits::default()
        };
        let service = service(&workspace, &state, limits, sessions, leases);
        let action = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Absent,
            },
        );

        assert!(matches!(
            service.write(&action, b"12345"),
            Err(AuthorizedFileMutationError::Transaction(
                TransactionalFileError::NewContentTooLarge { limit: 4 }
            ))
        ));
        assert!(!workspace.join("target.txt").exists());
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_target_remains_rejected_after_authorization() {
        use std::os::unix::fs::symlink;

        let (base, workspace, state) = fixture("symlink");
        let outside = base.join("outside.txt");
        fs::write(&outside, b"outside").expect("outside fixture");
        symlink(&outside, workspace.join("target.txt")).expect("symlink fixture");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);
        let action = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Content(ContentVersion::from_bytes(b"outside")),
            },
        );

        assert!(matches!(
            service.write(&action, b"new"),
            Err(AuthorizedFileMutationError::Transaction(_))
        ));
        assert_eq!(fs::read(&outside).expect("outside"), b"outside");
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn authorized_write_and_delete_reach_only_the_transactional_runtime() {
        let (base, workspace, state) = fixture("windows-end-to-end");
        let sessions = Arc::new(SessionRegistry::new());
        let leases = Arc::new(TaskLeaseRegistry::new());
        let grant = session(&[Capability::FileWrite, Capability::FileDelete]);
        sessions.register(grant.clone()).expect("register session");
        let task = lease(
            &grant,
            &[Capability::FileWrite, Capability::FileDelete],
            &[LeaseScope::WorkspaceAll],
        );
        leases.register(task.clone()).expect("register lease");
        let service = service(&workspace, &state, HardLimits::default(), sessions, leases);

        let write = envelope(
            &grant,
            Some(&task),
            Effect::FileWrite {
                path: path("target.txt"),
                expected: ExpectedState::Absent,
            },
        );
        let write_result = service.write(&write, b"alpha").expect("authorized write");
        assert_eq!(write_result.action_id, write.action_id);
        assert!(write_result.journal_retired);
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("target"),
            b"alpha"
        );

        let delete = envelope(
            &grant,
            Some(&task),
            Effect::FileDelete {
                path: path("target.txt"),
                expected: ContentVersion::from_bytes(b"alpha"),
            },
        );
        let delete_result = service.delete(&delete).expect("authorized delete");
        assert_eq!(delete_result.action_id, delete.action_id);
        assert!(delete_result.journal_retired);
        assert!(!workspace.join("target.txt").exists());

        fs::remove_dir_all(base).expect("cleanup");
    }
}
