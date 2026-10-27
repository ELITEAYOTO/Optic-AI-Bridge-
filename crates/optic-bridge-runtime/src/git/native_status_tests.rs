//! Non-executing service/worker boundary tests. The pinned fixture is not an
//! executable, so successful preparation cannot depend on launching Git.
use super::*;
use std::{collections::BTreeSet, sync::Arc};

use optic_bridge_core::{
    ActionEnvelope, ActionId, Capability, Effect, LeaseScope, MonotonicTime, PrincipalId,
    ProcessExecutionClass, ProjectId, ResourceBudget, SessionGrant, SessionHandle, TaskLease,
    TaskLeaseId, WorkloadClass,
};
use optic_bridge_policy::{PolicyEngine, PolicyReason};

use crate::{
    NativeGitReadAdmissionError, NativeGitReadLeaseError, NativeGitReadOwnedPermits,
    NativeGitReadTrustedContext, NativeGitStatusPreparationError, SessionRegistry,
    SessionRegistryError, TaskLeaseRegistry, TransportGuard, TransportLimits,
};

struct Fixture {
    root: PathBuf,
    repository: PathBuf,
    executable: PathBuf,
    sessions: Arc<SessionRegistry>,
    leases: TaskLeaseRegistry,
    envelope: ActionEnvelope,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "optic-native-status-{}",
            ActionId::generate().expect("entropy").to_token()
        ));
        fs::create_dir_all(root.join("repo")).expect("fixture repository");
        fs::write(root.join("git.bin"), b"inert pinned bytes; never executed")
            .expect("fixture identity");
        let repository = fs::canonicalize(root.join("repo")).expect("canonical repository");
        let executable = fs::canonicalize(root.join("git.bin")).expect("canonical executable");
        let session = SessionHandle::generate().expect("session");
        let grant = SessionGrant {
            handle: session.clone(),
            principal: PrincipalId::new("test-principal").expect("principal"),
            project: ProjectId::new("test-project").expect("project"),
            capabilities: BTreeSet::from([Capability::GitRead]),
            expires_at: MonotonicTime::from_millis(10_000),
            policy_epoch: 7,
        };
        let ceiling = ResourceBudget {
            timeout_ms: 100,
            output_bytes: 1024,
            memory_bytes: 4096,
            process_count: 1,
        };
        let lease = TaskLease {
            id: TaskLeaseId::generate().expect("lease"),
            session: session.clone(),
            capabilities: BTreeSet::from([Capability::GitRead]),
            scopes: BTreeSet::from([
                LeaseScope::Repository,
                LeaseScope::GitReadRepository {
                    canonical_root: repository.to_str().expect("UTF-8 fixture").to_owned(),
                },
                LeaseScope::ProcessExecutable {
                    executable: executable.to_str().expect("UTF-8 fixture").to_owned(),
                    class: ProcessExecutionClass::FixedTool,
                },
            ]),
            resource_ceiling: ceiling,
            workload_class: WorkloadClass::Standard,
            expires_at: grant.expires_at,
            policy_epoch: grant.policy_epoch,
        };
        let envelope = ActionEnvelope {
            action_id: ActionId::generate().expect("action"),
            session,
            task_lease: Some(lease.id.clone()),
            effect: Effect::GitRead,
            resources: ResourceBudget {
                output_bytes: 512,
                ..ceiling
            },
            policy_epoch: grant.policy_epoch,
        };
        let sessions = Arc::new(SessionRegistry::new());
        sessions.register(grant).expect("session registry");
        let leases = TaskLeaseRegistry::new();
        leases.register(lease).expect("pinned scoped lease");
        Self {
            root,
            repository,
            executable,
            sessions,
            leases,
            envelope,
        }
    }

    fn service(&self) -> GitReadService {
        // Deliberately bypass executable repository probing in this unit-only
        // fixture. These canonical synthetic bytes cannot run as a program.
        GitReadService {
            repository_root: self.repository.clone(),
            git_executable: self.executable.clone(),
            max_read_bytes: 1024,
            max_log_entries: 32,
            runner: HardenedCommandRunner::new(Duration::from_millis(100)).expect("runner"),
        }
    }

    fn trusted(&self) -> NativeGitReadTrustedContext<'_> {
        NativeGitReadTrustedContext {
            sessions: &self.sessions,
            trusted_session: &self.envelope.session,
            task_leases: &self.leases,
            policy: &PolicyEngine,
        }
    }

    fn owned(&self, guard: &TransportGuard) -> NativeGitReadOwnedPermits {
        let now = MonotonicTime::from_millis(10);
        NativeGitReadOwnedPermits::new(
            self.sessions
                .begin_owned_admission(&self.envelope.session, now)
                .expect("original worker admission"),
            guard
                .begin_execution(now)
                .expect("original worker transport"),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Windows executable pins must be released before fixture deletion.
        drop(std::mem::take(&mut self.leases));
        fs::remove_dir_all(&self.root).expect("cleanup synthetic fixture");
    }
}

fn guard(duration_ms: u64) -> TransportGuard {
    TransportGuard::new(TransportLimits {
        max_request_bytes: 1024,
        max_response_bytes: 1024,
        max_request_duration_ms: duration_ms,
        max_concurrent_requests: 1,
    })
    .expect("bounded transport")
}

#[test]
fn prepared_native_status_binds_service_command_budget_and_original_worker_capacity() {
    let fixture = Fixture::new();
    let guard = guard(1_000);
    let service = fixture.service();
    let mut action = fixture.envelope.clone();
    let prepared = service
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&guard),
            &action,
            MonotonicTime::from_millis(10),
        )
        .expect("prepare without executing the inert fixture");
    let expected = service.status_command_spec(512).expect("fixed status");
    assert_eq!(prepared.command.executable, expected.executable);
    assert_eq!(prepared.command.cwd, expected.cwd);
    assert_eq!(prepared.command.args, expected.args);
    assert_eq!(prepared.command.env, expected.env);
    assert_eq!(prepared.command.output_limit, 512);
    action.resources.output_bytes = 1024;
    action.policy_epoch += 1;
    assert_eq!(prepared.action_id(), &fixture.envelope.action_id);
    assert_eq!(prepared.resources(), fixture.envelope.resources);
    assert_ne!(prepared.resources(), action.resources);
    assert_eq!(
        prepared.envelope.policy_epoch,
        fixture.envelope.policy_epoch
    );
    assert_ne!(prepared.envelope.policy_epoch, action.policy_epoch);
    assert_eq!(
        prepared.lease_id(),
        fixture.envelope.task_lease.as_ref().unwrap()
    );
    assert_eq!(prepared.deadline(), MonotonicTime::from_millis(110));
    assert!(!prepared.is_stop_required_at(MonotonicTime::from_millis(109)));
    assert!(prepared.is_stop_required_at(MonotonicTime::from_millis(110)));
    assert_eq!(guard.active_requests(), 1);
    fixture
        .sessions
        .publish_revocation_before_drain(&fixture.envelope.session)
        .expect("publish revoke");
    assert!(prepared.is_stop_required_at(MonotonicTime::from_millis(10)));
    assert_eq!(
        fixture
            .sessions
            .remove_quiescent_inactive(&fixture.envelope.session, MonotonicTime::from_millis(110))
            .expect_err("prepared owner retains counted admission"),
        SessionRegistryError::AdmissionsInFlight
    );
    drop(prepared);
    assert_eq!(guard.active_requests(), 0);
    assert!(
        fixture
            .sessions
            .remove_quiescent_inactive(&fixture.envelope.session, MonotonicTime::from_millis(110))
            .expect("no native process exists")
    );
}

#[test]
fn prepared_native_status_rejects_missing_lease_and_wrong_service_identity() {
    for case in ["lease", "repository", "executable", "session", "epoch"] {
        let fixture = Fixture::new();
        let guard = guard(1_000);
        let mut service = fixture.service();
        let mut envelope = fixture.envelope.clone();
        match case {
            "lease" => envelope.task_lease = None,
            "repository" => {
                fs::create_dir(fixture.root.join("other")).expect("other repository");
                service.repository_root =
                    fs::canonicalize(fixture.root.join("other")).expect("canonical other");
            }
            "executable" => {
                fs::write(fixture.root.join("other.bin"), b"other inert bytes")
                    .expect("other tool");
                service.git_executable =
                    fs::canonicalize(fixture.root.join("other.bin")).expect("canonical other");
            }
            "session" => envelope.session = SessionHandle::generate().expect("foreign session"),
            "epoch" => envelope.policy_epoch += 1,
            _ => unreachable!(),
        }
        let result = service.prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&guard),
            &envelope,
            MonotonicTime::from_millis(10),
        );
        let error = result.err().expect("identity mismatch denied");
        let expected = match case {
            "lease" => {
                NativeGitReadAdmissionError::Preflight(NativeGitReadLeaseError::LeaseRequired)
            }
            "session" => NativeGitReadAdmissionError::WrongTrustedSession,
            "epoch" => NativeGitReadAdmissionError::Preflight(NativeGitReadLeaseError::Policy(
                PolicyReason::StalePolicyEpoch,
            )),
            _ => NativeGitReadAdmissionError::Preflight(NativeGitReadLeaseError::Policy(
                PolicyReason::ScopeNotAuthorized,
            )),
        };
        assert!(matches!(
            error,
            NativeGitStatusPreparationError::Admission(actual) if actual == expected
        ));
        assert_eq!(guard.active_requests(), 0);
        fixture
            .sessions
            .revoke(&fixture.envelope.session)
            .expect("failed preparation releases the original admission");
    }
}

#[test]
fn prepared_native_status_rejects_output_outside_service_ceiling() {
    for output_bytes in [0, 1025] {
        let fixture = Fixture::new();
        let guard = guard(1_000);
        let mut envelope = fixture.envelope.clone();
        envelope.resources.output_bytes = output_bytes;
        let result = fixture.service().prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&guard),
            &envelope,
            MonotonicTime::from_millis(10),
        );
        assert!(matches!(
            result,
            Err(NativeGitStatusPreparationError::Command(
                GitReadError::ZeroRequestLimit | GitReadError::ReadByteLimitExceeded { .. }
            ))
        ));
        assert_eq!(guard.active_requests(), 0);
        fixture
            .sessions
            .revoke(&fixture.envelope.session)
            .expect("no leaked admission on command rejection");
    }
}

#[test]
fn prepared_native_status_preserves_shorter_original_transport_horizon() {
    let fixture = Fixture::new();
    let guard = guard(20);
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&guard),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("prepared within original horizons");
    assert_eq!(prepared.deadline(), MonotonicTime::from_millis(30));
    assert!(!prepared.is_stop_required_at(MonotonicTime::from_millis(29)));
    assert!(prepared.is_stop_required_at(MonotonicTime::from_millis(30)));
    assert_eq!(guard.active_requests(), 1);
    drop(prepared);
    assert_eq!(guard.active_requests(), 0);
}

#[cfg(windows)]
#[test]
fn registered_native_status_retains_original_permits_and_process_quota_without_spawn() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .expect("synthetic repository process manager");
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&transport),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("admitted, but no native process");
    let reserved = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("register counted reservation without creating a child");
    let job = reserved.job_id();
    assert_eq!(
        manager
            .find_recorded_private_action(&fixture.envelope.session, &fixture.envelope.action_id)
            .unwrap(),
        Some(job.clone()),
        "process registration must be visible before any future native launch"
    );
    assert_eq!(
        manager
            .active_session_job_count(&fixture.envelope.session)
            .unwrap(),
        1
    );
    assert_eq!(transport.active_requests(), 1);
    fixture
        .sessions
        .publish_revocation_before_drain(&fixture.envelope.session)
        .expect("stop generation before admission drain");
    assert!(reserved.is_stop_required_at(MonotonicTime::from_millis(11)));
    assert_eq!(
        fixture
            .sessions
            .remove_quiescent_inactive(&fixture.envelope.session, MonotonicTime::from_millis(110))
            .expect_err("native reservation retains original admission"),
        SessionRegistryError::AdmissionsInFlight
    );
    assert_eq!(
        manager.cancel_session(&fixture.envelope.session).unwrap(),
        1
    );
    drop(reserved); // Safe synthetic no-child cleanup, NOT terminal Job/EOF proof.
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed
    );
    assert_eq!(
        manager
            .active_session_job_count(&fixture.envelope.session)
            .unwrap(),
        0
    );
    assert!(
        fixture
            .sessions
            .remove_quiescent_inactive(&fixture.envelope.session, MonotonicTime::from_millis(110))
            .unwrap()
    );
}

#[cfg(windows)]
#[test]
fn native_status_registration_rejects_foreign_process_root_and_expired_action() {
    for mode in ["foreign-root", "expired"] {
        let fixture = Fixture::new();
        let transport = guard(1_000);
        let process_root = if mode == "foreign-root" {
            &fixture.root
        } else {
            &fixture.repository
        };
        let manager = crate::ProcessManager::new(
            process_root,
            optic_bridge_core::HardLimits::default(),
            Vec::<String>::new(),
        )
        .expect("synthetic process manager");
        let prepared = fixture
            .service()
            .prepare_native_status(
                &fixture.trusted(),
                fixture.owned(&transport),
                &fixture.envelope,
                MonotonicTime::from_millis(10),
            )
            .expect("valid prepared status");
        let now = if mode == "expired" { 110 } else { 11 };
        let error = manager
            .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(now))
            .err()
            .expect("deny before recording native capacity");
        assert!(
            matches!(
                (mode, error),
                (
                    "foreign-root",
                    crate::NativeGitStatusRegistrationError::WrongProcessRoot
                ) | (
                    "expired",
                    crate::NativeGitStatusRegistrationError::StopRequired
                )
            ),
            "no foreign-root or expired status may reserve a process slot"
        );
        assert_eq!(transport.active_requests(), 0);
        assert_eq!(
            manager
                .find_recorded_private_action(
                    &fixture.envelope.session,
                    &fixture.envelope.action_id
                )
                .unwrap(),
            None
        );
    }
}
