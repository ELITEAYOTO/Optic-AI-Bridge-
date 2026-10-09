use std::{
    collections::BTreeSet,
    env, fs,
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use optic_bridge_core::{
    Capability, HardLimits, MonotonicTime, NetworkAccess, PrincipalId, ProcessExecutionClass,
    ProjectId, ResourceBudget, SessionHandle, ToolApprovalRequirement, ToolProfile,
    ToolProfileName, ToolProfileSpec, WorkloadClass,
};
use optic_bridge_runtime::{
    ApprovalBroker, ProcessManager, ReusableApprovalBroker, SessionGrantSpec,
    SessionLifecycleManager, SessionRegistry, SessionRegistryError, TaskLeaseRegistry,
};

fn workspace(label: &str) -> PathBuf {
    let token = SessionHandle::generate().expect("test entropy").to_token();
    let root = env::temp_dir().join(format!("optic-reusable-lifecycle-{label}-{token}"));
    fs::create_dir_all(&root).expect("create workspace");
    root
}

fn session_spec(expires_at: u64, policy_epoch: u64) -> SessionGrantSpec {
    SessionGrantSpec {
        principal: PrincipalId::new("principal").expect("principal"),
        project: ProjectId::new("project").expect("project"),
        capabilities: BTreeSet::from([Capability::ProcessRun]),
        expires_at: MonotonicTime::from_millis(expires_at),
        policy_epoch,
    }
}

fn profile(name: &str) -> ToolProfile {
    ToolProfile::from_spec(ToolProfileSpec {
        name: ToolProfileName::parse(name).expect("profile name"),
        executable: "C:/tools/cargo.exe".to_owned(),
        class: ProcessExecutionClass::FixedTool,
        workload_class: WorkloadClass::Heavy,
        exact_args: vec!["check".to_owned()],
        cwd: None,
        workspace_read_files: BTreeSet::new(),
        env_allowlist: BTreeSet::new(),
        network: NetworkAccess::Denied,
        resource_ceiling: ResourceBudget {
            timeout_ms: 5_000,
            output_bytes: 4_096,
            memory_bytes: 64 * 1024 * 1024,
            process_count: 1,
        },
        approval: ToolApprovalRequirement::HumanRequired,
    })
    .expect("profile")
}

fn runtime(
    root: &PathBuf,
) -> (
    Arc<SessionLifecycleManager>,
    Arc<SessionRegistry>,
    Arc<ReusableApprovalBroker>,
) {
    let limits = HardLimits::default();
    let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
    let leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
    let processes = Arc::new(ProcessManager::new(root, limits, Vec::new()).expect("processes"));
    let one_shot = Arc::new(ApprovalBroker::from_hard_limits(limits).expect("one-shot"));
    let reusable = Arc::new(ReusableApprovalBroker::new());
    let lifecycle = Arc::new(SessionLifecycleManager::new_with_approval_brokers(
        Arc::clone(&sessions),
        leases,
        processes,
        one_shot,
        Arc::clone(&reusable),
    ));
    (lifecycle, sessions, reusable)
}

#[test]
fn revoke_removes_only_owned_reusable_approvals() {
    let root = workspace("owner-scope");
    let (lifecycle, sessions, reusable) = runtime(&root);
    let now = MonotonicTime::from_millis(1);
    let session_a = lifecycle
        .provision(session_spec(100, 7), now)
        .expect("session A");
    let session_b = lifecycle
        .provision(session_spec(100, 7), now)
        .expect("session B");
    let profile_a = profile("cargo-check-a");
    let profile_b = profile("cargo-check-b");
    let service = lifecycle.reusable_approval_service();
    service
        .issue(
            &session_a.handle,
            profile_a.clone(),
            MonotonicTime::from_millis(90),
            now,
        )
        .expect("approval A");
    service
        .issue(
            &session_b.handle,
            profile_b.clone(),
            MonotonicTime::from_millis(90),
            now,
        )
        .expect("approval B");

    let report = lifecycle.revoke(&session_a.handle).expect("revoke A");
    assert!(report.session_changed);
    assert_eq!(report.revoked_approvals, 0);
    assert_eq!(report.revoked_reusable_approvals, 1);
    assert!(
        reusable
            .find_active_for_profile(
                &session_a.handle,
                &profile_a,
                7,
                MonotonicTime::from_millis(2),
            )
            .expect("lookup A")
            .is_none()
    );
    assert!(
        reusable
            .find_active_for_profile(
                &session_b.handle,
                &profile_b,
                7,
                MonotonicTime::from_millis(2),
            )
            .expect("lookup B")
            .is_some()
    );
    sessions
        .get_active(&session_b.handle, MonotonicTime::from_millis(2))
        .expect("B remains active");

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn reusable_cleanup_waits_until_existing_admission_drains() {
    let root = workspace("admission-drain");
    let (lifecycle, sessions, reusable) = runtime(&root);
    let now = MonotonicTime::from_millis(1);
    let session = lifecycle
        .provision(session_spec(100, 11), now)
        .expect("session");
    let approved_profile = profile("cargo-check");
    lifecycle
        .reusable_approval_service()
        .issue(
            &session.handle,
            approved_profile.clone(),
            MonotonicTime::from_millis(90),
            now,
        )
        .expect("approval");

    let admission = sessions
        .begin_admission(&session.handle, now)
        .expect("in-flight admission");
    let revoke_lifecycle = Arc::clone(&lifecycle);
    let revoke_session = session.handle.clone();
    let (done_tx, done_rx) = mpsc::channel();
    let revoke_thread = thread::spawn(move || {
        done_tx
            .send(revoke_lifecycle.revoke(&revoke_session))
            .expect("send revoke result");
    });

    let mut revoke_started = false;
    for _ in 0..100 {
        if matches!(
            sessions.get_active(&session.handle, now),
            Err(SessionRegistryError::Revoked)
        ) {
            revoke_started = true;
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(revoke_started, "revoke must close admission first");
    assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert!(
        reusable
            .find_active_for_profile(
                &session.handle,
                &approved_profile,
                11,
                MonotonicTime::from_millis(2),
            )
            .expect("raw broker lookup while revoke is draining")
            .is_some()
    );

    drop(admission);
    let report = done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("revoke completes after admission drains")
        .expect("revoke result");
    revoke_thread.join().expect("revoke thread");
    assert_eq!(report.revoked_reusable_approvals, 1);
    assert!(
        reusable
            .find_active_for_profile(
                &session.handle,
                &approved_profile,
                11,
                MonotonicTime::from_millis(2),
            )
            .expect("raw broker lookup after cleanup")
            .is_none()
    );

    fs::remove_dir_all(root).expect("cleanup");
}
