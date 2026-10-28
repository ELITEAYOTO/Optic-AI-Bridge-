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
    #[cfg(windows)]
    lease: TaskLease,
    sessions: Arc<SessionRegistry>,
    leases: TaskLeaseRegistry,
    envelope: ActionEnvelope,
}

impl Fixture {
    fn new() -> Self {
        Self::build(None)
    }

    #[cfg(windows)]
    fn new_on_clock(clock: &crate::StdClock) -> Self {
        Self::build(Some(clock))
    }

    fn build(clock: Option<&crate::StdClock>) -> Self {
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
        if let Some(clock) = clock {
            sessions
                .register_on_clock(grant, 10_000, clock)
                .expect("clock-issued session registry");
        } else {
            sessions.register(grant).expect("session registry");
        }
        let leases = TaskLeaseRegistry::new();
        let lease = if let Some(clock) = clock {
            leases
                .register_on_clock(lease, 10_000, clock)
                .expect("clock-issued pinned scoped lease")
        } else {
            leases.register(lease.clone()).expect("pinned scoped lease");
            lease
        };
        #[cfg(not(windows))]
        drop(lease);
        Self {
            root,
            repository,
            executable,
            #[cfg(windows)]
            lease,
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

    #[cfg(windows)]
    fn owned_on_clock(
        &self,
        guard: &TransportGuard,
        clock: &crate::StdClock,
    ) -> NativeGitReadOwnedPermits {
        NativeGitReadOwnedPermits::new(
            self.sessions
                .begin_owned_admission_on_clock(&self.envelope.session, clock)
                .expect("original clocked admission"),
            guard
                .begin_execution_on_clock(clock)
                .expect("original clocked transport"),
        )
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

#[cfg(windows)]
#[test]
fn precreated_native_git_status_custody_holds_job_pipes_and_original_permits() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
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
        .expect("counted original worker admission");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("private process capacity");
    let id = registered.job_id();
    let custody = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("open actual unstarted Windows Job and pipes");
    assert_eq!(
        custody
            .native_active_processes()
            .expect("real kernel Job observation"),
        0,
        "precreation must not launch a root process"
    );
    assert_eq!(transport.active_requests(), 1);
    assert_eq!(
        manager
            .active_session_job_count(&fixture.envelope.session)
            .expect("real process registry capacity"),
        1
    );
    fixture
        .sessions
        .publish_revocation_before_drain(&fixture.envelope.session)
        .expect("revoke original session generation");
    assert!(custody.is_stop_required_at(MonotonicTime::from_millis(11)));
    assert_eq!(
        fixture
            .sessions
            .remove_quiescent_inactive(&fixture.envelope.session, MonotonicTime::from_millis(110))
            .expect_err("counted permit still owned despite revocation"),
        SessionRegistryError::AdmissionsInFlight
    );
    drop(custody); // No spawn interface exists: synthetic no-child teardown only.
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &id)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed
    );
    assert!(
        fixture
            .sessions
            .remove_quiescent_inactive(&fixture.envelope.session, MonotonicTime::from_millis(110))
            .expect("precreated but unstarted reservation fully released")
    );
}

#[cfg(windows)]
#[test]
fn revoked_native_status_cannot_precreate_kernel_custody() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
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
        .expect("original valid preparation");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("counted registration");
    let job = registered.job_id();
    fixture
        .sessions
        .publish_revocation_before_drain(&fixture.envelope.session)
        .expect("revoke after reservation");
    assert!(matches!(
        registered.precreate_native_custody(MonotonicTime::from_millis(11)),
        Err(crate::NativeGitStatusCustodyError::StopRequired)
    ));
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed,
        "no process was created on rejected custody"
    );
}

#[cfg(windows)]
#[test]
fn unstarted_native_custody_pins_repository_rename_until_original_permits_release() {
    let fixture = Fixture::new();
    let guard = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .expect("manager for exact repository root");
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&guard),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("original authorized native status");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("counted private reservation");
    let custody = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("unstarted kernel Job/pipes and pinned targets");
    let target = fixture.root.join("repo-renamed");
    assert!(
        fs::rename(&fixture.repository, &target).is_err(),
        "a pinned canonical GitRead directory must deny rename while native custody lives"
    );
    assert_eq!(guard.active_requests(), 1);
    assert_eq!(
        custody.native_active_processes().expect("kernel Job query"),
        0,
        "this test must never spawn Git"
    );
    drop(custody);
    assert_eq!(guard.active_requests(), 0);
    fs::rename(&fixture.repository, &target)
        .expect("after last custody handle closes, repository can be renamed");
    fs::rename(&target, &fixture.repository).expect("restore fixture directory for cleanup");
}

#[cfg(windows)]
#[test]
fn moved_repository_before_native_custody_is_denied_with_no_child_or_capacity_leak() {
    let fixture = Fixture::new();
    let guard = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .expect("canonical manager");
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&guard),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("authorize original canonical path");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("record exact private action");
    let job_id = registered.job_id();
    let target = fixture.root.join("repo-moved-before-pin");
    fs::rename(&fixture.repository, &target)
        .expect("without custody, repository is not yet pinned");
    let outcome = registered.precreate_native_custody(MonotonicTime::from_millis(11));
    assert!(
        outcome.is_err(),
        "fail closed instead of opening an unbound native Job when the repository moved"
    );
    assert_eq!(guard.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job_id)
            .expect("failed no-child private record")
            .status,
        crate::ProcessStatus::Failed
    );
    fs::rename(&target, &fixture.repository).expect("restore fixture for cleanup");
}

#[cfg(windows)]
#[test]
fn prelaunch_snapshot_rejects_foreign_same_id_task_lease_and_retain_original_admission() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .expect("synthetic manager");
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&transport),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("original admission and lease generation");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("private capacity");
    let custody = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("unstarted OS Job/pipes and pinned GitRead targets");

    let foreign = TaskLeaseRegistry::new();
    foreign
        .register(fixture.lease.clone())
        .expect("independently pinned lease with identical opaque ID");
    let foreign_context = NativeGitReadTrustedContext {
        sessions: &fixture.sessions,
        trusted_session: &fixture.envelope.session,
        task_leases: &foreign,
        policy: &PolicyEngine,
    };
    assert!(matches!(
        custody
            .observe_original_prelaunch_snapshot(&foreign_context, MonotonicTime::from_millis(12)),
        Err(crate::NativeGitStatusSnapshotError::LeaseGenerationMismatch)
    ));
    assert_eq!(transport.active_requests(), 1);
    assert_eq!(
        custody
            .native_active_processes()
            .expect("no child launched"),
        0
    );

    custody
        .observe_original_prelaunch_snapshot(&fixture.trusted(), MonotonicTime::from_millis(12))
        .expect("same original registered lease may be observed");
    fixture
        .leases
        .revoke(fixture.envelope.task_lease.as_ref().unwrap())
        .expect("original generation revoke");
    assert!(matches!(
        custody.observe_original_prelaunch_snapshot(
            &fixture.trusted(),
            MonotonicTime::from_millis(12)
        ),
        Err(crate::NativeGitStatusSnapshotError::StopRequired)
    ));
    assert_eq!(
        transport.active_requests(),
        1,
        "snapshot rejection never releases the original permits prematurely"
    );
    drop(custody);
    assert_eq!(transport.active_requests(), 0);
}

#[cfg(windows)]
#[test]
fn prelaunch_snapshot_rejects_foreign_session_registry_before_relooking_up_lease() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .expect("synthetic manager");
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&transport),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("original admitted native read");
    let custody = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("private process reservation")
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("unstarted native custody");
    let foreign_sessions = Arc::new(SessionRegistry::new());
    let forged_context = NativeGitReadTrustedContext {
        sessions: &foreign_sessions,
        trusted_session: &fixture.envelope.session,
        task_leases: &fixture.leases,
        policy: &PolicyEngine,
    };
    assert!(matches!(
        custody
            .observe_original_prelaunch_snapshot(&forged_context, MonotonicTime::from_millis(12)),
        Err(crate::NativeGitStatusSnapshotError::ForeignSessionRegistry)
    ));
    assert_eq!(transport.active_requests(), 1);
    drop(custody);
    assert_eq!(transport.active_requests(), 0);
}

#[cfg(windows)]
#[test]
fn prelaunch_snapshot_never_rebinds_after_original_lease_id_reuse() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
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
        .expect("original admission and task lease");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("reserved private process capacity");
    let job = registered.job_id();
    let custody = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("real empty Job, pipes and pinned service targets");
    custody
        .observe_original_prelaunch_snapshot(&fixture.trusted(), MonotonicTime::from_millis(12))
        .expect("original still-authorized registration");

    // The original TaskLease can disappear while this service-owned admission
    // and its inert kernel Job are still retained. Re-registering the same
    // opaque lease ID must NOT re-arm the original owner's stop witness.
    assert!(
        fixture
            .leases
            .revoke(&fixture.lease.id)
            .expect("revoke original")
    );
    assert!(
        fixture
            .leases
            .remove_revoked(&fixture.lease.id)
            .expect("physically remove revoked lease")
    );
    fixture
        .leases
        .register(fixture.lease.clone())
        .expect("new generation with the exact old ID and scopes");
    assert!(
        fixture
            .leases
            .get_active(
                &fixture.lease.id,
                &fixture.envelope.session,
                MonotonicTime::from_millis(12),
            )
            .is_ok(),
        "the replacement record is independently valid"
    );
    assert!(custody.is_stop_required_at(MonotonicTime::from_millis(12)));
    assert!(matches!(
        custody.observe_original_prelaunch_snapshot(
            &fixture.trusted(),
            MonotonicTime::from_millis(12)
        ),
        Err(crate::NativeGitStatusSnapshotError::StopRequired)
    ));
    assert_eq!(transport.active_requests(), 1);
    assert_eq!(
        manager
            .active_session_job_count(&fixture.envelope.session)
            .expect("original capacity remains reserved"),
        1
    );
    assert_eq!(
        custody
            .native_active_processes()
            .expect("kernel is still idle"),
        0,
        "no process exists: Job-empty before launch is not completion proof"
    );
    drop(custody); // Only no-child cleanup, never an uncertain live-child release.
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed
    );
}

#[cfg(windows)]
#[test]
fn session_only_commit_precedes_direct_close_and_preserves_registered_owner() {
    use std::{sync::mpsc, thread, time::Instant};

    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
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
        .expect("admitted original native status");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("registered private process capacity");
    let job = registered.job_id();
    let unstarted = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("empty native Job/pipes and pinned original targets");
    let committed = unstarted
        .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
        .expect("commit session only, without a native launch");
    assert_eq!(committed.job_id(), job);
    assert!(!committed.is_stop_required_at(MonotonicTime::from_millis(12)));
    assert_eq!(
        committed.native_active_processes().expect("kernel Job"),
        0,
        "session commitment never executes a native child"
    );

    let registry = Arc::clone(&fixture.sessions);
    let handle = fixture.envelope.session.clone();
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        done_tx
            .send(registry.revoke(&handle))
            .expect("revoke result");
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let observation = fixture
            .sessions
            .get_active(&fixture.envelope.session, MonotonicTime::from_millis(12));
        if matches!(observation, Err(SessionRegistryError::Revoked)) {
            break;
        }
        assert!(Instant::now() < deadline, "direct close not published");
        thread::yield_now();
    }

    let stopped = committed.is_stop_required_at(MonotonicTime::from_millis(12));
    let no_child = committed.native_active_processes().expect("real empty Job") == 0;
    let retained_transport = transport.active_requests() == 1;
    let retained_process = manager
        .active_session_job_count(&fixture.envelope.session)
        .expect("registered job count")
        == 1;
    let revoke_is_draining = matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty));

    // Release before asserting so the real synchronous revoker is never
    // left blocked by a failed test.
    drop(committed);
    let result = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("revoke drains after original custody drops")
        .expect("direct revoke");
    closer.join().expect("revoke thread");
    assert!(result);
    assert!(stopped, "the committed generation must receive stop");
    assert!(no_child, "no native child may exist in this phase");
    assert!(
        retained_transport,
        "original worker permit retained through close"
    );
    assert!(
        retained_process,
        "registered native quota retained through close"
    );
    assert!(revoke_is_draining, "close is not process quiescence");
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed,
        "no-child disposal must not counterfeit native terminal proof"
    );
}

#[cfg(windows)]
#[test]
fn session_only_commit_after_direct_or_lifecycle_close_is_denied() {
    use std::{sync::mpsc, thread, time::Instant};

    for lifecycle_close in [false, true] {
        let fixture = Fixture::new();
        let transport = guard(1_000);
        let manager = crate::ProcessManager::new(
            &fixture.repository,
            optic_bridge_core::HardLimits::default(),
            Vec::<String>::new(),
        )
        .expect("synthetic manager");
        let prepared = fixture
            .service()
            .prepare_native_status(
                &fixture.trusted(),
                fixture.owned(&transport),
                &fixture.envelope,
                MonotonicTime::from_millis(10),
            )
            .expect("original active admission");
        let registered = manager
            .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
            .expect("native capacity");
        let job = registered.job_id();
        let custody = registered
            .precreate_native_custody(MonotonicTime::from_millis(11))
            .expect("unstarted native Job/stdio");
        assert_eq!(custody.native_active_processes().unwrap(), 0);

        let registry = Arc::clone(&fixture.sessions);
        let handle = fixture.envelope.session.clone();
        let mut direct_closer = None;
        if lifecycle_close {
            assert!(
                registry
                    .publish_revocation_before_drain(&handle)
                    .expect("lifecycle close publishes original stop")
            );
        } else {
            let (done_tx, done_rx) = mpsc::sync_channel(1);
            let closer = thread::spawn(move || {
                done_tx
                    .send(registry.revoke(&handle))
                    .expect("direct revoke result");
            });
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let observation = fixture
                    .sessions
                    .get_active(&fixture.envelope.session, MonotonicTime::from_millis(12));
                if matches!(observation, Err(SessionRegistryError::Revoked)) {
                    break;
                }
                assert!(Instant::now() < deadline, "direct close not published");
                thread::yield_now();
            }
            direct_closer = Some((closer, done_rx));
        }

        // Consumes and safely drops the already registered no-child owner
        // on rejection. A failed session-only commit creates no new process.
        let denied =
            custody.commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12));
        assert!(matches!(
            denied,
            Err(crate::NativeGitStatusSnapshotError::StopRequired)
                | Err(crate::NativeGitStatusSnapshotError::Session(
                    SessionRegistryError::Revoked
                ))
        ));
        assert_eq!(transport.active_requests(), 0);
        assert_eq!(
            manager
                .result(&fixture.envelope.session, &job)
                .unwrap()
                .status,
            crate::ProcessStatus::Failed
        );
        if let Some((closer, done_rx)) = direct_closer {
            assert!(
                done_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("direct revoke completes after denied owner drops")
                    .expect("direct revoke")
            );
            closer.join().expect("revoke thread");
        } else {
            assert!(!fixture.sessions.revoke(&fixture.envelope.session).unwrap());
        }
    }
}

#[cfg(windows)]
#[test]
fn lease_only_commit_preserves_original_stop_after_tasklease_id_reuse() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .expect("synthetic manager");
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&transport),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("original admission and lease");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("real registered private capacity");
    let job = registered.job_id();
    let ownership = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("real empty Job, pinned executable and repository");
    let session_only = ownership
        .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
        .expect("session-only authority receipt");
    let both = session_only
        .commit_original_lease(&fixture.trusted(), MonotonicTime::from_millis(12))
        .expect("separate original TaskLease receipt");
    assert_eq!(both.job_id(), job);
    assert!(!both.is_stop_required_at(MonotonicTime::from_millis(12)));
    assert_eq!(both.native_active_processes().expect("empty kernel Job"), 0);
    assert_eq!(transport.active_requests(), 1);
    assert_eq!(
        manager
            .active_session_job_count(&fixture.envelope.session)
            .expect("registered quota"),
        1
    );

    // Lease revocation is independent from the active session. Its original
    // stop must stay visible even after the same ID is physically reused.
    assert!(fixture.leases.revoke(&fixture.lease.id).unwrap());
    assert!(both.is_stop_required_at(MonotonicTime::from_millis(12)));
    assert!(fixture.leases.remove_revoked(&fixture.lease.id).unwrap());
    fixture
        .leases
        .register(fixture.lease.clone())
        .expect("new registration reuses the same opaque ID");
    assert!(
        fixture
            .sessions
            .get_active(&fixture.envelope.session, MonotonicTime::from_millis(12))
            .is_ok()
    );
    let stays_stopped = both.is_stop_required_at(MonotonicTime::from_millis(12));
    let still_empty = both.native_active_processes().expect("kernel Job") == 0;
    let still_owned = transport.active_requests() == 1;
    drop(both);
    assert!(stays_stopped, "replacement lease must not revive old owner");
    assert!(still_empty, "no process may be created by lease receipts");
    assert!(still_owned, "original admission survives TaskLease removal");
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed,
        "no child ever existed, so no native completion is claimed"
    );
}

#[cfg(windows)]
#[test]
fn lease_only_commit_rejects_revoke_after_session_commit_without_spawn() {
    for revoke_session in [false, true] {
        let fixture = Fixture::new();
        let transport = guard(1_000);
        let manager = crate::ProcessManager::new(
            &fixture.repository,
            optic_bridge_core::HardLimits::default(),
            Vec::<String>::new(),
        )
        .expect("synthetic manager");
        let prepared = fixture
            .service()
            .prepare_native_status(
                &fixture.trusted(),
                fixture.owned(&transport),
                &fixture.envelope,
                MonotonicTime::from_millis(10),
            )
            .expect("original valid session and TaskLease");
        let registered = manager
            .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
            .expect("registered private capacity");
        let job = registered.job_id();
        let session_only = registered
            .precreate_native_custody(MonotonicTime::from_millis(11))
            .expect("kernel native custody")
            .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
            .expect("original session commitment");
        assert_eq!(session_only.native_active_processes().unwrap(), 0);
        if revoke_session {
            assert_eq!(
                fixture
                    .leases
                    .revoke_session(&fixture.envelope.session)
                    .unwrap(),
                1
            );
        } else {
            assert!(fixture.leases.revoke(&fixture.lease.id).unwrap());
        }
        assert!(session_only.is_stop_required_at(MonotonicTime::from_millis(12)));
        assert!(matches!(
            session_only.commit_original_lease(&fixture.trusted(), MonotonicTime::from_millis(12)),
            Err(crate::NativeGitStatusSnapshotError::StopRequired)
                | Err(crate::NativeGitStatusSnapshotError::Lease(_))
                | Err(crate::NativeGitStatusSnapshotError::LeaseRecord(_))
        ));
        assert_eq!(transport.active_requests(), 0);
        assert_eq!(
            manager
                .result(&fixture.envelope.session, &job)
                .unwrap()
                .status,
            crate::ProcessStatus::Failed,
            "rejected no-child owner cleans up original capacity"
        );
    }
}

#[cfg(windows)]
#[test]
fn session_close_between_original_session_and_lease_receipts_denies_native_custody() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .expect("synthetic manager");
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&transport),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("real original session admission and lease");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("counted private Job");
    let job = registered.job_id();
    let session_only = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("real empty Job and pinned targets")
        .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
        .expect("original session receipt");

    assert_eq!(session_only.native_active_processes().unwrap(), 0);
    assert_eq!(transport.active_requests(), 1);
    // Lifecycle close must publish session stop immediately, even though
    // original admission and private Job ownership remain counted.
    assert!(
        fixture
            .sessions
            .publish_revocation_before_drain(&fixture.envelope.session)
            .expect("lifecycle publishes original stop")
    );
    assert!(session_only.is_stop_required_at(MonotonicTime::from_millis(12)));
    // The independent TaskLease registry is still active. Its availability
    // cannot override the earlier original session close.
    assert!(
        fixture
            .leases
            .get_active(
                &fixture.lease.id,
                &fixture.envelope.session,
                MonotonicTime::from_millis(12),
            )
            .is_ok()
    );
    let stopped =
        session_only.commit_original_lease(&fixture.trusted(), MonotonicTime::from_millis(12));
    assert!(matches!(
        stopped,
        Err(crate::NativeGitStatusSnapshotError::StopRequired)
            | Err(crate::NativeGitStatusSnapshotError::Session(
                SessionRegistryError::Revoked
            ))
    ));
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed,
        "aborted no-child owner cannot report process termination"
    );
    assert!(!fixture.sessions.revoke(&fixture.envelope.session).unwrap());
}

#[cfg(windows)]
#[test]
fn dual_receipt_session_close_after_lease_commit_drains_and_stops() {
    use std::{sync::mpsc, thread, time::Instant};

    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .expect("synthetic manager");
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&transport),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .expect("original session and lease");
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .expect("original private Job capacity");
    let job = registered.job_id();
    let dual = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .expect("real empty kernel Job")
        .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
        .expect("original session commitment")
        .commit_original_lease(&fixture.trusted(), MonotonicTime::from_millis(12))
        .expect("independent original lease commitment");
    assert_eq!(dual.job_id(), job);
    assert!(!dual.is_stop_required_at(MonotonicTime::from_millis(12)));
    assert_eq!(dual.native_active_processes().unwrap(), 0);
    assert_eq!(transport.active_requests(), 1);

    let registry = Arc::clone(&fixture.sessions);
    let handle = fixture.envelope.session.clone();
    let (tx, rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        tx.send(registry.revoke(&handle))
            .expect("deliver direct revoke outcome");
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let observation = fixture
            .sessions
            .get_active(&fixture.envelope.session, MonotonicTime::from_millis(12));
        if matches!(observation, Err(SessionRegistryError::Revoked)) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "direct session close never published"
        );
        thread::yield_now();
    }

    let stopped = dual.is_stop_required_at(MonotonicTime::from_millis(12));
    let job_empty = dual.native_active_processes().expect("real empty Job") == 0;
    let permit_counted = transport.active_requests() == 1;
    let job_counted = manager
        .active_session_job_count(&fixture.envelope.session)
        .expect("original registered job count")
        == 1;
    let revoke_draining = matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty));
    // The second receipt is NOT a global permission: the independent TaskLease
    // can still be active while the original session has been closed.
    let lease_still_active = fixture
        .leases
        .get_active(
            &fixture.lease.id,
            &fixture.envelope.session,
            MonotonicTime::from_millis(12),
        )
        .is_ok();

    // Always release the owner BEFORE assertions: the synchronous revocation
    // is required to drain its original permit and must not hang on a panic.
    drop(dual);
    let revoked = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("session close completes after owner drop")
        .expect("direct session revoke");
    closer.join().expect("join direct revoker");

    assert!(revoked);
    assert!(
        stopped,
        "first-generation stop remains visible after both commits"
    );
    assert!(job_empty, "receipts never create a child");
    assert!(
        permit_counted,
        "original transport admission retained to drop"
    );
    assert!(job_counted, "registered Job capacity retained to drop");
    assert!(revoke_draining, "close is not native process quiescence");
    assert!(
        lease_still_active,
        "lease authority cannot revive closed session"
    );
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed,
        "no-child cleanup is not proof of termination of a real GitRead"
    );
}

#[cfg(windows)]
#[test]
fn original_watchdog_observes_session_lease_generation_and_job_stops_without_child() {
    for source in ["session", "lease-reused-id", "job"] {
        let clock = crate::StdClock::new();
        let mut fixture = Fixture::new_on_clock(&clock);
        // Keep explicit revocation distinct from deadline expiry in this test.
        fixture.envelope.resources.timeout_ms = 5_000;
        fixture.lease.resource_ceiling.timeout_ms = 5_000;
        assert!(fixture.leases.revoke(&fixture.lease.id).unwrap());
        assert!(fixture.leases.remove_revoked(&fixture.lease.id).unwrap());
        fixture
            .leases
            .register_on_clock(fixture.lease.clone(), 10_000, &clock)
            .unwrap();
        let transport = guard(10_000);
        let manager = crate::ProcessManager::new(
            &fixture.repository,
            optic_bridge_core::HardLimits::default(),
            Vec::<String>::new(),
        )
        .unwrap();
        let prepared = fixture
            .service()
            .prepare_native_status_on_clock(
                &fixture.trusted(),
                fixture.owned_on_clock(&transport, &clock),
                &fixture.envelope,
                &clock,
            )
            .unwrap();
        let registered = manager
            .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
            .unwrap();
        let job = registered.job_id();
        let mut owner = registered
            .precreate_native_custody(MonotonicTime::from_millis(11))
            .unwrap()
            .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
            .unwrap()
            .commit_original_lease(&fixture.trusted(), MonotonicTime::from_millis(12))
            .unwrap();
        owner.arm_original_watchdog(&fixture.trusted()).unwrap();
        assert!(matches!(
            owner.arm_original_watchdog(&fixture.trusted()),
            Err(crate::NativeGitStatusSnapshotError::Watchdog(_))
        ));
        match source {
            "session" => {
                fixture
                    .sessions
                    .publish_revocation_before_drain(&fixture.envelope.session)
                    .unwrap();
            }
            "lease-reused-id" => {
                assert!(fixture.leases.revoke(&fixture.lease.id).unwrap());
                assert!(fixture.leases.remove_revoked(&fixture.lease.id).unwrap());
                fixture.leases.register(fixture.lease.clone()).unwrap();
                assert!(
                    fixture
                        .leases
                        .get_active(
                            &fixture.lease.id,
                            &fixture.envelope.session,
                            MonotonicTime::from_millis(12),
                        )
                        .is_ok()
                );
            }
            "job" => {
                assert!(manager.stop(&fixture.envelope.session, &job).unwrap());
            }
            _ => unreachable!(),
        }
        let bound = std::time::Instant::now() + Duration::from_secs(2);
        while !owner.watchdog_stop_observed() && std::time::Instant::now() < bound {
            std::thread::yield_now();
        }
        let observed = owner.watchdog_stop_observed();
        let expired = owner.watchdog_deadline_expired();
        let empty = owner.native_active_processes().unwrap() == 0;
        assert_eq!(transport.active_requests(), 1);
        assert_eq!(
            manager
                .active_session_job_count(&fixture.envelope.session)
                .unwrap(),
            1
        );
        drop(owner);
        assert!(
            observed,
            "independent worker must observe original {source}"
        );
        assert!(
            !expired,
            "revocation proof must precede the five-second deadline"
        );
        assert!(empty, "no child is created by watchdog wiring");
        assert_eq!(transport.active_requests(), 0);
        assert_eq!(
            manager
                .active_session_job_count(&fixture.envelope.session)
                .unwrap(),
            0
        );
        assert_eq!(
            manager
                .result(&fixture.envelope.session, &job)
                .unwrap()
                .status,
            crate::ProcessStatus::Failed
        );
    }
}

#[cfg(windows)]
#[test]
fn original_watchdog_rejects_expiry_before_arm_without_resetting_original_horizon() {
    let clock = crate::StdClock::new();
    let fixture = Fixture::new_on_clock(&clock);
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .unwrap();
    let prepared = fixture
        .service()
        .prepare_native_status_on_clock(
            &fixture.trusted(),
            fixture.owned_on_clock(&transport, &clock),
            &fixture.envelope,
            &clock,
        )
        .unwrap();
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .unwrap();
    let job = registered.job_id();
    let mut owner = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .unwrap()
        .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
        .unwrap()
        .commit_original_lease(&fixture.trusted(), MonotonicTime::from_millis(12))
        .unwrap();
    fixture
        .sessions
        .renew_active(
            &fixture.envelope.session,
            MonotonicTime::from_millis(12),
            MonotonicTime::from_millis(20_000),
        )
        .unwrap();
    // Session renewal and a stale logical observation cannot restart the horizon.
    std::thread::sleep(Duration::from_millis(150));
    assert!(matches!(
        owner.arm_original_watchdog(&fixture.trusted()),
        Err(crate::NativeGitStatusSnapshotError::StopRequired)
    ));
    assert_eq!(owner.native_active_processes().unwrap(), 0);
    drop(owner);
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .active_session_job_count(&fixture.envelope.session)
            .unwrap(),
        0
    );
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed
    );
}

#[cfg(windows)]
#[test]
fn original_watchdog_rejects_logical_only_preparation_without_service_clock() {
    let fixture = Fixture::new();
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .unwrap();
    let prepared = fixture
        .service()
        .prepare_native_status(
            &fixture.trusted(),
            fixture.owned(&transport),
            &fixture.envelope,
            MonotonicTime::from_millis(10),
        )
        .unwrap();
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .unwrap();
    let job = registered.job_id();
    let mut owner = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .unwrap()
        .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
        .unwrap()
        .commit_original_lease(&fixture.trusted(), MonotonicTime::from_millis(12))
        .unwrap();
    assert!(matches!(
        owner.arm_original_watchdog(&fixture.trusted()),
        Err(crate::NativeGitStatusSnapshotError::ClockRequired)
    ));
    assert_eq!(owner.native_active_processes().unwrap(), 0);
    drop(owner);
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .active_session_job_count(&fixture.envelope.session)
            .unwrap(),
        0
    );
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed
    );
}

#[cfg(windows)]
#[test]
fn original_watchdog_expires_on_retained_service_clock_without_caller_polling() {
    let clock = crate::StdClock::new();
    let fixture = Fixture::new_on_clock(&clock);
    let transport = guard(1_000);
    let manager = crate::ProcessManager::new(
        &fixture.repository,
        optic_bridge_core::HardLimits::default(),
        Vec::<String>::new(),
    )
    .unwrap();
    let prepared = fixture
        .service()
        .prepare_native_status_on_clock(
            &fixture.trusted(),
            fixture.owned_on_clock(&transport, &clock),
            &fixture.envelope,
            &clock,
        )
        .unwrap();
    let registered = manager
        .reserve_prepared_native_status(prepared, MonotonicTime::from_millis(11))
        .unwrap();
    let job = registered.job_id();
    let mut owner = registered
        .precreate_native_custody(MonotonicTime::from_millis(11))
        .unwrap()
        .commit_original_session(&fixture.trusted(), MonotonicTime::from_millis(12))
        .unwrap()
        .commit_original_lease(&fixture.trusted(), MonotonicTime::from_millis(12))
        .unwrap();
    owner.arm_original_watchdog(&fixture.trusted()).unwrap();
    let bound = std::time::Instant::now() + Duration::from_secs(2);
    while !owner.watchdog_stop_observed() && std::time::Instant::now() < bound {
        std::thread::yield_now();
    }
    assert!(owner.watchdog_stop_observed());
    assert!(owner.watchdog_deadline_expired());
    assert!(matches!(
        owner.arm_original_watchdog(&fixture.trusted()),
        Err(crate::NativeGitStatusSnapshotError::StopRequired)
    ));
    assert_eq!(owner.native_active_processes().unwrap(), 0);
    drop(owner);
    assert_eq!(transport.active_requests(), 0);
    assert_eq!(
        manager
            .active_session_job_count(&fixture.envelope.session)
            .unwrap(),
        0
    );
    assert_eq!(
        manager
            .result(&fixture.envelope.session, &job)
            .unwrap()
            .status,
        crate::ProcessStatus::Failed
    );
}

#[cfg(windows)]
#[test]
fn native_clock_preparation_rejects_foreign_and_untagged_transport_without_child() {
    let clock = crate::StdClock::new();
    let fixture = Fixture::new_on_clock(&clock);
    let service = fixture.service();
    let foreign = crate::StdClock::new();
    let transport = guard(1_000);
    for tagged in [false, true] {
        let request = if tagged {
            transport.begin_execution_on_clock(&foreign).unwrap()
        } else {
            transport
                .begin_execution(crate::Clock::now(&clock))
                .unwrap()
        };
        let admission = fixture
            .sessions
            .begin_owned_admission(&fixture.envelope.session, crate::Clock::now(&clock))
            .unwrap();
        let result = service.prepare_native_status_on_clock(
            &fixture.trusted(),
            NativeGitReadOwnedPermits::new(admission, request),
            &fixture.envelope,
            &clock,
        );
        assert!(matches!(
            result,
            Err(NativeGitStatusPreparationError::TransportClockMismatch)
        ));
        assert_eq!(transport.active_requests(), 0);
    }
    let prepared = service
        .prepare_native_status_on_clock(
            &fixture.trusted(),
            fixture.owned_on_clock(&transport, &clock),
            &fixture.envelope,
            &clock.clone(),
        )
        .expect("clone retains original transport authority");
    assert_eq!(transport.active_requests(), 1);
    drop(prepared);
    assert_eq!(transport.active_requests(), 0);
    fixture.sessions.revoke(&fixture.envelope.session).unwrap();
}

#[cfg(windows)]
#[test]
fn native_clock_preparation_rejects_foreign_and_untagged_session_without_child() {
    let clock = crate::StdClock::new();
    let foreign = crate::StdClock::new();
    for tagged in [false, true] {
        let fixture = Fixture::new_on_clock(&foreign);
        let transport = guard(1_000);
        let admission = if tagged {
            fixture
                .sessions
                .begin_owned_admission_on_clock(&fixture.envelope.session, &foreign)
                .unwrap()
        } else {
            fixture
                .sessions
                .begin_owned_admission(&fixture.envelope.session, crate::Clock::now(&clock))
                .unwrap()
        };
        let request = transport.begin_execution_on_clock(&clock).unwrap();
        let result = fixture.service().prepare_native_status_on_clock(
            &fixture.trusted(),
            NativeGitReadOwnedPermits::new(admission, request),
            &fixture.envelope,
            &clock,
        );
        assert!(matches!(
            result,
            Err(NativeGitStatusPreparationError::SessionClockMismatch)
        ));
        assert_eq!(transport.active_requests(), 0);
        fixture.sessions.revoke(&fixture.envelope.session).unwrap();
    }
}

#[cfg(windows)]
#[test]
fn native_clock_preparation_rejects_foreign_and_untagged_lease_without_child() {
    let clock = crate::StdClock::new();
    let foreign = crate::StdClock::new();
    for tagged in [false, true] {
        let fixture = Fixture::new_on_clock(&clock);
        fixture.leases.revoke(&fixture.lease.id).unwrap();
        fixture.leases.remove_revoked(&fixture.lease.id).unwrap();
        if tagged {
            fixture
                .leases
                .register_on_clock(fixture.lease.clone(), 10_000, &foreign)
                .unwrap();
        } else {
            fixture.leases.register(fixture.lease.clone()).unwrap();
        }
        let transport = guard(1_000);
        let result = fixture.service().prepare_native_status_on_clock(
            &fixture.trusted(),
            fixture.owned_on_clock(&transport, &clock),
            &fixture.envelope,
            &clock,
        );
        assert!(matches!(
            result,
            Err(NativeGitStatusPreparationError::LeaseClockMismatch)
        ));
        assert_eq!(transport.active_requests(), 0);
        fixture.sessions.revoke(&fixture.envelope.session).unwrap();
    }
}
