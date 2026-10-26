//! Registry-backed, policy-checked lease witness for a future *native* GitRead.
//!
//! This is intentionally NOT a process-launch permit. Real MCP session
//! admission, sensitive metadata checks, native owner/Job registration,
//! opened-executable identity and a final pre-spawn stop/deadline check are
//! separate mandatory controls. Legacy GitRead is deliberately unchanged.

use std::{path::Path, sync::Arc};

use optic_bridge_core::{ActionEnvelope, MonotonicTime, SessionGrant, SessionHandle, TaskLeaseId};
use optic_bridge_policy::{PolicyDecision, PolicyEngine, PolicyReason};
use thiserror::Error;

use crate::{
    session_registry::{OwnedSessionAdmissionPermit, SessionRegistry, SessionRegistryError},
    task_lease_registry::{TaskLeaseRegistry, TaskLeaseRegistryError, TaskLeaseRevocationWatch},
    transport::RequestPermit,
};

/// A read-only witness to one concrete registry entry, never a spawn permit.
///
/// The witness continues observing the original entry even if the lease is
/// revoked, physically removed, then recreated with the same opaque ID.
#[derive(Debug)]
#[must_use = "a lease witness must be retained by its native owner"]
pub struct NativeGitReadLeaseWitness {
    lease_id: TaskLeaseId,
    lease_expires_at: MonotonicTime,
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

    /// A once-admitted lease never gains extra time from later renewal.
    /// This is a stop request only, not proof that native cleanup completed.
    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.is_revoked() || now >= self.lease_expires_at
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
                lease_expires_at: lease.expires_at,
                revocation,
            })
        }
        PolicyDecision::Deny(reason) | PolicyDecision::RequireApproval(reason) => {
            Err(NativeGitReadLeaseError::Policy(reason))
        }
    }
}

/// Non-cloneable ownership of a real, counted session admission and an
/// independently registered, policy-checked GitRead lease witness.
///
/// This is *not* a native launch permit. Native Job ownership and pipes must
/// be registered before a child can exist, and the owner must keep this value
/// through provable native quiescence (including TerminationUncertain).
#[must_use = "native owners must retain admission through terminal OS proof"]
pub struct NativeGitReadAdmittedPreflight {
    session_admission: OwnedSessionAdmissionPermit,
    lease_witness: NativeGitReadLeaseWitness,
    request_permit: RequestPermit,
}

impl NativeGitReadAdmittedPreflight {
    #[must_use]
    pub fn lease_id(&self) -> &TaskLeaseId {
        self.lease_witness.lease_id()
    }

    /// Combines original session and TaskLease generation stop conditions.
    /// A false result is NOT authority to spawn: policy, executable pins,
    /// native registration and final pre-spawn checks remain mandatory.
    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.request_permit.is_expired_at(now)
            || self.session_admission.stop_watch().is_stop_required_at(now)
            || self.lease_witness.is_stop_required_at(now)
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum NativeGitReadAdmissionError {
    #[error("GitRead ActionEnvelope session is not the trusted application session")]
    WrongTrustedSession,
    #[error("native GitRead session admission denied: {0}")]
    Session(#[from] SessionRegistryError),
    #[error("native GitRead lease/policy preflight denied: {0}")]
    Preflight(#[from] NativeGitReadLeaseError),
    #[error("native GitRead revoked or expired during admission preflight")]
    StopRequired,
    #[error("native GitRead request transport permit has expired")]
    TransportExpired,
}

/// Runtime-owned dependencies; never populate from untrusted MCP parameters.
/// Grouped to keep the admitted preflight interface explicit and reviewable.
pub struct NativeGitReadTrustedContext<'a> {
    pub sessions: &'a Arc<SessionRegistry>,
    pub trusted_session: &'a SessionHandle,
    pub task_leases: &'a TaskLeaseRegistry,
    pub policy: &'a PolicyEngine,
}

/// Acquire a real owned session admission from the trusted application handle,
/// then combine it with the verified registry-backed GitRead lease witness.
///
/// Caller supplies the actual server-owned handle, canonical target and fixed
/// executable, never identities originating from untrusted MCP arguments.
/// Admission automatically releases on any failure. On success, the future
/// service-owned native Job guardian must retain it until actual terminal
/// evidence, not just a timeout/result code. The consumed `RequestPermit`
/// keeps transport capacity and its original deadline through that lifetime.
/// This function never spawns.
pub fn begin_admitted_native_git_read_preflight(
    trusted: &NativeGitReadTrustedContext<'_>,
    envelope: &ActionEnvelope,
    canonical_repository: &str,
    git_executable: &str,
    request_permit: RequestPermit,
    now: MonotonicTime,
) -> Result<NativeGitReadAdmittedPreflight, NativeGitReadAdmissionError> {
    if request_permit.is_expired_at(now) {
        return Err(NativeGitReadAdmissionError::TransportExpired);
    }
    if &envelope.session != trusted.trusted_session {
        return Err(NativeGitReadAdmissionError::WrongTrustedSession);
    }
    let admission = trusted
        .sessions
        .begin_owned_admission(trusted.trusted_session, now)?;
    let lease_witness = verify_registered_native_git_read_preflight(
        trusted.task_leases,
        trusted.policy,
        envelope,
        admission.grant(),
        canonical_repository,
        git_executable,
        now,
    )?;
    let result = NativeGitReadAdmittedPreflight {
        session_admission: admission,
        lease_witness,
        request_permit,
    };
    if result.is_stop_required_at(now) {
        return Err(NativeGitReadAdmissionError::StopRequired);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, env, fs, path::PathBuf, sync::Arc};

    use optic_bridge_core::{
        ActionId, Capability, Effect, LeaseScope, PrincipalId, ProcessExecutionClass, ProjectId,
        ResourceBudget, SessionHandle, TaskLease, WorkloadClass,
    };

    use super::*;
    use crate::transport::{TransportGuard, TransportLimits};

    fn transport_guard() -> TransportGuard {
        TransportGuard::new(TransportLimits {
            max_request_bytes: 1024,
            max_response_bytes: 1024,
            max_request_duration_ms: 20,
            max_concurrent_requests: 1,
        })
        .expect("valid bounded transport")
    }

    struct Fixture {
        root: PathBuf,
        repository: String,
        executable: String,
        registry: TaskLeaseRegistry,
        sessions: Arc<SessionRegistry>,
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
            let sessions = Arc::new(SessionRegistry::new());
            sessions
                .register(grant.clone())
                .expect("registered session");
            assert!(now < grant.expires_at);
            Self {
                root,
                repository,
                executable,
                registry,
                sessions,
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

        fn admit(
            &self,
            action: &ActionEnvelope,
            trusted_session: &SessionHandle,
            now: MonotonicTime,
        ) -> Result<NativeGitReadAdmittedPreflight, NativeGitReadAdmissionError> {
            let guard = transport_guard();
            let request_permit = guard.begin_execution(now).expect("bounded test request");
            let trusted = NativeGitReadTrustedContext {
                sessions: &self.sessions,
                trusted_session,
                task_leases: &self.registry,
                policy: &PolicyEngine,
            };
            begin_admitted_native_git_read_preflight(
                &trusted,
                action,
                &self.repository,
                &self.executable,
                request_permit,
                now,
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
    fn admitted_native_gitread_owns_transport_capacity_and_shorter_deadline() {
        let fixture = Fixture::new();
        let guard = transport_guard();
        let now = MonotonicTime::from_millis(10);
        let permit = guard
            .begin_execution(now)
            .expect("reserve transport capacity");
        let trusted = NativeGitReadTrustedContext {
            sessions: &fixture.sessions,
            trusted_session: &fixture.grant.handle,
            task_leases: &fixture.registry,
            policy: &PolicyEngine,
        };
        let admitted = begin_admitted_native_git_read_preflight(
            &trusted,
            &fixture.envelope,
            &fixture.repository,
            &fixture.executable,
            permit,
            now,
        )
        .expect("session, task lease and transport are all admitted");

        assert_eq!(
            guard.active_requests(),
            1,
            "native owner keeps the original slot"
        );
        assert!(!admitted.is_stop_required_at(MonotonicTime::from_millis(29)));
        assert!(
            admitted.is_stop_required_at(MonotonicTime::from_millis(30)),
            "transport timeout must stop owner before long session/lease expiry"
        );
        drop(admitted);
        assert_eq!(guard.active_requests(), 0, "owner release returns the slot");
        assert!(
            fixture
                .sessions
                .revoke(&fixture.grant.handle)
                .expect("owner drained")
        );
    }

    #[test]
    fn admitted_native_gitread_rejects_expired_transport_and_releases_slot() {
        let fixture = Fixture::new();
        let guard = transport_guard();
        let start = MonotonicTime::from_millis(10);
        let permit = guard.begin_execution(start).expect("reserve transport");
        let trusted = NativeGitReadTrustedContext {
            sessions: &fixture.sessions,
            trusted_session: &fixture.grant.handle,
            task_leases: &fixture.registry,
            policy: &PolicyEngine,
        };
        assert!(matches!(
            begin_admitted_native_git_read_preflight(
                &trusted,
                &fixture.envelope,
                &fixture.repository,
                &fixture.executable,
                permit,
                MonotonicTime::from_millis(30),
            ),
            Err(NativeGitReadAdmissionError::TransportExpired)
        ));
        assert_eq!(
            guard.active_requests(),
            0,
            "failed admission releases transport"
        );
        assert!(
            fixture
                .sessions
                .revoke(&fixture.grant.handle)
                .expect("no leaked session admission")
        );
    }

    #[test]
    fn native_preflight_accepts_transferred_worker_admission_without_double_counting() {
        let fixture = Fixture::new();
        let guard = transport_guard();
        let now = MonotonicTime::from_millis(10);
        let request = guard.begin_execution(now).expect("worker transport");
        let admission = fixture
            .sessions
            .begin_owned_admission(&fixture.grant.handle, now)
            .expect("worker admission");
        let trusted = NativeGitReadTrustedContext {
            sessions: &fixture.sessions,
            trusted_session: &fixture.grant.handle,
            task_leases: &fixture.registry,
            policy: &PolicyEngine,
        };
        let admitted = finish_owned_native_git_read_preflight(
            &trusted,
            NativeGitReadOwnedPermits::new(admission, request),
            &fixture.envelope,
            &fixture.repository,
            &fixture.executable,
            now,
        )
        .expect("transfer the existing worker permits without readmission");
        assert_eq!(guard.active_requests(), 1);
        assert_eq!(admitted.lease_id(), &fixture.lease.id);
        assert!(
            fixture
                .sessions
                .publish_revocation_before_drain(&fixture.grant.handle)
                .expect("publish stop")
        );
        assert!(admitted.is_stop_required_at(now));
        assert_eq!(
            fixture
                .sessions
                .remove_quiescent_inactive(&fixture.grant.handle, now)
                .expect_err("admitted guardian must retain owner capacity"),
            SessionRegistryError::AdmissionsInFlight
        );
        drop(admitted);
        assert_eq!(guard.active_requests(), 0);
        assert!(
            fixture
                .sessions
                .remove_quiescent_inactive(&fixture.grant.handle, now)
                .expect("owner dropped")
        );
    }

    #[test]
    fn native_preflight_rejects_foreign_registry_even_when_session_handle_matches() {
        let fixture = Fixture::new();
        let foreign = Arc::new(SessionRegistry::new());
        foreign
            .register(fixture.grant.clone())
            .expect("foreign registration");
        let guard = transport_guard();
        let now = MonotonicTime::from_millis(10);
        let request = guard.begin_execution(now).expect("transport");
        let admission = foreign
            .begin_owned_admission(&fixture.grant.handle, now)
            .expect("foreign owned admission");
        let trusted = NativeGitReadTrustedContext {
            sessions: &fixture.sessions,
            trusted_session: &fixture.grant.handle,
            task_leases: &fixture.registry,
            policy: &PolicyEngine,
        };
        assert!(matches!(
            finish_owned_native_git_read_preflight(
                &trusted,
                NativeGitReadOwnedPermits::new(admission, request),
                &fixture.envelope,
                &fixture.repository,
                &fixture.executable,
                now,
            ),
            Err(NativeGitReadAdmissionError::ForeignSessionRegistry)
        ));
        assert_eq!(
            guard.active_requests(),
            0,
            "failed transfer returns transport"
        );
        assert!(
            foreign
                .revoke(&fixture.grant.handle)
                .expect("foreign drained")
        );
        assert!(
            fixture
                .sessions
                .revoke(&fixture.grant.handle)
                .expect("trusted admission never entered")
        );
    }

    #[test]
    fn admitted_native_gitread_binds_real_session_and_lease_without_spawning() {
        let fixture = Fixture::new();
        let now = MonotonicTime::from_millis(10);
        let admitted = fixture
            .admit(&fixture.envelope, &fixture.grant.handle, now)
            .expect("real admitted session and registered lease");
        assert_eq!(admitted.lease_id(), &fixture.lease.id);
        assert!(!admitted.is_stop_required_at(now));
        assert!(admitted.is_stop_required_at(fixture.lease.expires_at));
        drop(admitted);
        assert!(
            fixture
                .sessions
                .revoke(&fixture.grant.handle)
                .expect("admission released")
        );
    }

    #[test]
    fn admitted_native_gitread_rejects_forged_session_and_releases_failed_admission() {
        let fixture = Fixture::new();
        let now = MonotonicTime::from_millis(10);
        let mut forged = fixture.envelope.clone();
        forged.session = SessionHandle::generate().expect("foreign handle");
        assert!(matches!(
            fixture.admit(&forged, &fixture.grant.handle, now),
            Err(NativeGitReadAdmissionError::WrongTrustedSession)
        ));

        let mut missing_lease = fixture.envelope.clone();
        missing_lease.task_lease = None;
        assert!(matches!(
            fixture.admit(&missing_lease, &fixture.grant.handle, now),
            Err(NativeGitReadAdmissionError::Preflight(
                NativeGitReadLeaseError::LeaseRequired
            ))
        ));

        let unknown_session = SessionHandle::generate().expect("unknown session");
        let mut unknown_action = fixture.envelope.clone();
        unknown_action.session = unknown_session.clone();
        assert!(matches!(
            fixture.admit(&unknown_action, &unknown_session, now),
            Err(NativeGitReadAdmissionError::Session(
                SessionRegistryError::UnknownSession
            ))
        ));
        // No failed attempt may leave admission capacity counted.
        assert!(
            fixture
                .sessions
                .revoke(&fixture.grant.handle)
                .expect("no hanging permits")
        );
        assert!(matches!(
            fixture.admit(&fixture.envelope, &fixture.grant.handle, now),
            Err(NativeGitReadAdmissionError::Session(
                SessionRegistryError::Revoked
            ))
        ));
    }

    #[test]
    fn admitted_native_gitread_retains_session_capacity_until_explicit_drop() {
        let fixture = Fixture::new();
        let now = MonotonicTime::from_millis(10);
        let admitted = fixture
            .admit(&fixture.envelope, &fixture.grant.handle, now)
            .expect("owned session admission");
        assert!(
            fixture
                .sessions
                .publish_revocation_before_drain(&fixture.grant.handle)
                .expect("publish session revoke")
        );
        assert!(admitted.is_stop_required_at(now));
        assert_eq!(
            fixture
                .sessions
                .remove_quiescent_inactive(&fixture.grant.handle, now)
                .expect_err("registered owner must block early removal"),
            SessionRegistryError::AdmissionsInFlight
        );
        drop(admitted);
        assert!(
            !fixture
                .sessions
                .revoke(&fixture.grant.handle)
                .expect("already revoked")
        );
        assert!(
            fixture
                .sessions
                .remove_quiescent_inactive(&fixture.grant.handle, now)
                .expect("terminal owner releases capacity")
        );
    }

    #[test]
    fn admitted_native_gitread_old_lease_witness_stays_stopped_after_id_reuse() {
        let fixture = Fixture::new();
        let now = MonotonicTime::from_millis(10);
        let admitted = fixture
            .admit(&fixture.envelope, &fixture.grant.handle, now)
            .expect("original admitted owner");
        assert!(
            fixture
                .registry
                .revoke(&fixture.lease.id)
                .expect("lease revoke")
        );
        assert!(admitted.is_stop_required_at(now));
        assert!(
            fixture
                .registry
                .remove_revoked(&fixture.lease.id)
                .expect("remove old record")
        );
        fixture
            .registry
            .register(fixture.lease.clone())
            .expect("replace same ID");
        assert!(
            admitted.is_stop_required_at(now),
            "old owner must not rebind"
        );
        drop(admitted);
        assert!(
            fixture
                .sessions
                .revoke(&fixture.grant.handle)
                .expect("finish")
        );
    }

    #[test]
    fn admitted_native_gitread_rejects_expired_session_or_lease() {
        let fixture = Fixture::new();
        let expiration = fixture.grant.expires_at;
        assert!(matches!(
            fixture.admit(&fixture.envelope, &fixture.grant.handle, expiration),
            Err(NativeGitReadAdmissionError::Session(
                SessionRegistryError::Expired
            ))
        ));
        // A failed admission on expiry must not block synchronous revocation.
        assert!(
            fixture
                .sessions
                .revoke(&fixture.grant.handle)
                .expect("revoke")
        );
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
