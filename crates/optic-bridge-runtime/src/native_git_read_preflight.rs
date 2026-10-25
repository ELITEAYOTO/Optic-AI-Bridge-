//! Registry-backed, policy-checked lease witness for a future *native* GitRead.
//!
//! This is intentionally NOT a process-launch permit. Real MCP session
//! admission, sensitive metadata checks, native owner/Job registration,
//! opened-executable identity and a final pre-spawn stop/deadline check are
//! separate mandatory controls. Legacy GitRead is deliberately unchanged.

use std::path::Path;

use optic_bridge_core::{ActionEnvelope, MonotonicTime, SessionGrant, TaskLeaseId};
use optic_bridge_policy::{PolicyDecision, PolicyEngine, PolicyReason};
use thiserror::Error;

use crate::task_lease_registry::{
    TaskLeaseRegistry, TaskLeaseRegistryError, TaskLeaseRevocationWatch,
};

/// A read-only witness to one concrete registry entry, never a spawn permit.
///
/// The witness continues observing the original entry even if the lease is
/// revoked, physically removed, then recreated with the same opaque ID.
#[derive(Debug)]
#[must_use = "a lease witness must be retained by its native owner"]
pub struct NativeGitReadLeaseWitness {
    lease_id: TaskLeaseId,
    revocation: TaskLeaseRevocationWatch,
}

impl NativeGitReadLeaseWitness {
    #[must_use]
    pub fn lease_id(&self) -> &TaskLeaseId {
        &self.lease_id
    }

    /// Must be checked again at the native launch boundary. False alone is
    /// never sufficient to authorize process creation.
    #[must_use]
    pub fn is_revoked(&self) -> bool {
        self.revocation.is_revoked()
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum NativeGitReadLeaseError {
    #[error("native GitRead requires a registered task lease")]
    LeaseRequired,
    #[error("native GitRead requires absolute canonical repository and executable paths")]
    NonAbsoluteTarget,
    #[error("task lease registry denied native GitRead: {0}")]
    Registry(#[from] TaskLeaseRegistryError),
    #[error("native GitRead policy denied: {0:?}")]
    Policy(PolicyReason),
}

/// Atomically fetch an active registered lease and its original revocation
/// witness, then check its exact typed GitRead policy against the MCP envelope.
///
/// The registry also verifies its captured executable authority, but this
/// function cannot establish atomicity with CreateProcessW: the native owner
/// must revalidate identity/revocation/deadlines immediately before spawn.
/// It does not take session admission, grant sensitive data access, create
/// a Job, reserve quotas or start a process.
pub fn verify_registered_native_git_read_preflight(
    registry: &TaskLeaseRegistry,
    policy: &PolicyEngine,
    envelope: &ActionEnvelope,
    session: &SessionGrant,
    canonical_repository: &str,
    git_executable: &str,
    now: MonotonicTime,
) -> Result<NativeGitReadLeaseWitness, NativeGitReadLeaseError> {
    let id = envelope
        .task_lease
        .as_ref()
        .ok_or(NativeGitReadLeaseError::LeaseRequired)?;

    // The eventual caller supplies these from trusted canonicalized runtime
    // state, not from model-provided strings or user-controlled path aliases.
    if !Path::new(canonical_repository).is_absolute() || !Path::new(git_executable).is_absolute() {
        return Err(NativeGitReadLeaseError::NonAbsoluteTarget);
    }

    let (lease, revocation) =
        registry.get_active_with_revocation_watch(id, &session.handle, now)?;
    match policy.evaluate_native_git_read_preflight(
        envelope,
        session,
        Some(&lease),
        canonical_repository,
        git_executable,
        now,
    ) {
        PolicyDecision::Allow => {
            // Revocation may race the policy check. Even after this check,
            // only an admitted, registered native owner may attempt spawn.
            if revocation.is_revoked() {
                return Err(NativeGitReadLeaseError::Registry(
                    TaskLeaseRegistryError::Revoked,
                ));
            }
            Ok(NativeGitReadLeaseWitness {
                lease_id: lease.id,
                revocation,
            })
        }
        PolicyDecision::Deny(reason) | PolicyDecision::RequireApproval(reason) => {
            Err(NativeGitReadLeaseError::Policy(reason))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, env, fs, path::PathBuf};

    use optic_bridge_core::{
        ActionId, Capability, Effect, LeaseScope, PrincipalId, ProcessExecutionClass, ProjectId,
        ResourceBudget, SessionHandle, TaskLease, WorkloadClass,
    };

    use super::*;

    struct Fixture {
        root: PathBuf,
        repository: String,
        executable: String,
        registry: TaskLeaseRegistry,
        grant: SessionGrant,
        lease: TaskLease,
        envelope: ActionEnvelope,
    }

    impl Fixture {
        fn new() -> Self {
            let root = env::temp_dir().join(format!(
                "optic-native-gitread-witness-{}",
                ActionId::generate().expect("entropy").to_token()
            ));
            fs::create_dir_all(&root).expect("fixture directory");
            let repository = root.join("repo");
            fs::create_dir(&repository).expect("repository directory");
            let executable = root.join("fixed-tool.bin");
            fs::write(&executable, b"synthetic fixed tool; never executed").expect("tool bytes");
            let repository = fs::canonicalize(repository)
                .expect("canonical repository")
                .to_string_lossy()
                .into_owned();
            let executable = fs::canonicalize(executable)
                .expect("canonical tool")
                .to_string_lossy()
                .into_owned();

            let now = MonotonicTime::from_millis(10);
            let session = SessionHandle::generate().expect("session");
            let grant = SessionGrant {
                handle: session.clone(),
                principal: PrincipalId::new("test-principal").expect("principal"),
                project: ProjectId::new("test-project").expect("project"),
                capabilities: BTreeSet::from([Capability::GitRead]),
                expires_at: MonotonicTime::from_millis(10_000),
                policy_epoch: 7,
            };
            let budget = ResourceBudget {
                timeout_ms: 100,
                output_bytes: 1024,
                memory_bytes: 4096,
                process_count: 1,
            };
            let lease = TaskLease {
                id: optic_bridge_core::TaskLeaseId::generate().expect("lease"),
                session: session.clone(),
                capabilities: BTreeSet::from([Capability::GitRead]),
                scopes: BTreeSet::from([
                    LeaseScope::Repository,
                    LeaseScope::GitReadRepository {
                        canonical_root: repository.clone(),
                    },
                    LeaseScope::ProcessExecutable {
                        executable: executable.clone(),
                        class: ProcessExecutionClass::FixedTool,
                    },
                ]),
                resource_ceiling: budget,
                workload_class: WorkloadClass::Standard,
                expires_at: MonotonicTime::from_millis(10_000),
                policy_epoch: 7,
            };
            let envelope = ActionEnvelope {
                action_id: ActionId::generate().expect("action"),
                session,
                task_lease: Some(lease.id.clone()),
                effect: Effect::GitRead,
                resources: budget,
                policy_epoch: grant.policy_epoch,
            };
            let registry = TaskLeaseRegistry::new();
            registry
                .register(lease.clone())
                .expect("registered real record");
            assert!(now < grant.expires_at);
            Self {
                root,
                repository,
                executable,
                registry,
                grant,
                lease,
                envelope,
            }
        }

        fn check(
            &self,
            action: &ActionEnvelope,
            repo: &str,
        ) -> Result<NativeGitReadLeaseWitness, NativeGitReadLeaseError> {
            verify_registered_native_git_read_preflight(
                &self.registry,
                &PolicyEngine,
                action,
                &self.grant,
                repo,
                &self.executable,
                MonotonicTime::from_millis(10),
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            // Windows executable pins deny deletion while the registry owns them.
            drop(std::mem::take(&mut self.registry));
            fs::remove_dir_all(&self.root).expect("cleanup fixture");
        }
    }

    #[test]
    fn native_witness_requires_a_real_registered_lease_not_legacy_authority() {
        let fixture = Fixture::new();
        let mut legacy = fixture.envelope.clone();
        legacy.task_lease = None;
        assert_eq!(
            PolicyEngine.evaluate(
                &legacy,
                &fixture.grant,
                None,
                MonotonicTime::from_millis(10),
            ),
            PolicyDecision::Allow,
            "legacy GitRead authorization must remain unchanged"
        );
        assert!(matches!(
            fixture.check(&legacy, &fixture.repository),
            Err(NativeGitReadLeaseError::LeaseRequired)
        ));

        let mut unregistered = fixture.envelope.clone();
        unregistered.task_lease =
            Some(optic_bridge_core::TaskLeaseId::generate().expect("unknown lease"));
        assert!(matches!(
            fixture.check(&unregistered, &fixture.repository),
            Err(NativeGitReadLeaseError::Registry(
                TaskLeaseRegistryError::UnknownLease
            ))
        ));
        let witness = fixture
            .check(&fixture.envelope, &fixture.repository)
            .expect("registered and correctly scoped GitRead");
        assert_eq!(witness.lease_id(), &fixture.lease.id);
        assert!(!witness.is_revoked());
    }

    #[test]
    fn native_witness_rejects_wrong_repo_session_epoch_and_budget() {
        let fixture = Fixture::new();
        let other_repo = fixture.root.join("another-repo");
        fs::create_dir(&other_repo).expect("other repo");
        let other_repo = fs::canonicalize(other_repo)
            .expect("canonical")
            .to_string_lossy()
            .into_owned();
        assert!(matches!(
            fixture.check(&fixture.envelope, &other_repo),
            Err(NativeGitReadLeaseError::Policy(
                PolicyReason::ScopeNotAuthorized
            ))
        ));
        let mut wrong_session = fixture.envelope.clone();
        wrong_session.session = SessionHandle::generate().expect("other session");
        assert!(matches!(
            fixture.check(&wrong_session, &fixture.repository),
            Err(NativeGitReadLeaseError::Policy(PolicyReason::WrongSession))
        ));
        let mut stale_epoch = fixture.envelope.clone();
        stale_epoch.policy_epoch += 1;
        assert!(matches!(
            fixture.check(&stale_epoch, &fixture.repository),
            Err(NativeGitReadLeaseError::Policy(
                PolicyReason::StalePolicyEpoch
            ))
        ));
        let mut excess = fixture.envelope.clone();
        excess.resources.timeout_ms = 20_000;
        assert!(matches!(
            fixture.check(&excess, &fixture.repository),
            Err(NativeGitReadLeaseError::Policy(
                PolicyReason::ResourceBudgetExceeded
            ))
        ));
        assert!(matches!(
            fixture.check(&fixture.envelope, "relative/repo"),
            Err(NativeGitReadLeaseError::NonAbsoluteTarget)
        ));
    }

    #[test]
    fn witness_remains_bound_after_revocation_removal_and_same_id_reuse() {
        let fixture = Fixture::new();
        let original = fixture
            .check(&fixture.envelope, &fixture.repository)
            .expect("original witness");
        assert!(fixture.registry.revoke(&fixture.lease.id).expect("revoke"));
        assert!(
            original.is_revoked(),
            "published revoke reaches original watch"
        );
        assert!(matches!(
            fixture.check(&fixture.envelope, &fixture.repository),
            Err(NativeGitReadLeaseError::Registry(
                TaskLeaseRegistryError::Revoked
            ))
        ));
        assert!(
            fixture
                .registry
                .remove_revoked(&fixture.lease.id)
                .expect("remove old")
        );
        let replacement = fixture.lease.clone();
        fixture
            .registry
            .register(replacement)
            .expect("same-ID new record");
        let current = fixture
            .check(&fixture.envelope, &fixture.repository)
            .expect("new registered generation is independently checked");
        assert!(
            original.is_revoked(),
            "old witness cannot observe replacement"
        );
        assert!(!current.is_revoked(), "new witness belongs to new record");
    }
}
