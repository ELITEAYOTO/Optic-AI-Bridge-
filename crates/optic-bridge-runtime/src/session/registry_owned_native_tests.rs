//! Test-only registered owner + **owned counted admission** coupling.
//! No native child is ever created. This is not an authority for GitRead.
use super::*;
use crate::{ProcessManager, ProcessStartSpec};
use optic_bridge_core::{
    ActionId, Capability, HardLimits, NetworkAccess, PrincipalId, ProcessExecutionClass, ProjectId,
    ResourceBudget, SessionGrant, SessionHandle, WorkloadClass,
};
use std::{
    collections::BTreeSet,
    env, fs,
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

fn grant() -> SessionGrant {
    SessionGrant {
        handle: SessionHandle::generate().expect("fresh session"),
        principal: PrincipalId::new("test").unwrap(),
        project: ProjectId::new("synthetic").unwrap(),
        capabilities: BTreeSet::from([Capability::FileRead]),
        expires_at: MonotonicTime::from_millis(100),
        policy_epoch: 1,
    }
}

fn reserve(
    session: &SessionHandle,
) -> (
    PathBuf,
    ProcessManager,
    crate::process::NativeProcessReservation,
    ActionId,
) {
    let root = env::temp_dir().join(format!(
        "optic-317-owned-native-{}",
        SessionHandle::generate().unwrap().to_token()
    ));
    fs::create_dir_all(&root).unwrap();
    let manager = ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap();
    let executable = env::current_exe()
        .unwrap()
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let spec = ProcessStartSpec {
        session: session.clone(),
        class: ProcessExecutionClass::FixedTool,
        workload_class: WorkloadClass::Standard,
        network: NetworkAccess::Denied,
        executable,
        args: Vec::new(),
        cwd: None,
        workspace_read_files: Vec::new(),
        env_allowlist: Vec::new(),
        resources: ResourceBudget {
            timeout_ms: 1_000,
            output_bytes: 4_096,
            memory_bytes: 64 * 1024 * 1024,
            process_count: 1,
        },
    };
    let action = ActionId::generate().unwrap();
    let owner = manager.reserve_native_private(&spec, &action).unwrap();
    assert_eq!(
        manager
            .find_recorded_private_action(session, &action)
            .unwrap(),
        Some(owner.id()),
        "JobStore entry must exist before owner commitment"
    );
    (root, manager, owner, action)
}

fn observe_closed(registry: &SessionRegistry, handle: &SessionHandle) {
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        if matches!(
            registry.get_active(handle, MonotonicTime::from_millis(1)),
            Err(SessionRegistryError::Revoked)
        ) {
            return;
        }
        assert!(Instant::now() < until, "close publication exceeded bound");
        thread::yield_now();
    }
}

#[test]
fn real_owned_native_worker_retains_admission_and_jobstore_during_direct_revoke() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (root, manager, reservation, action) = reserve(&a.handle);

    let (committed_tx, committed_rx) = mpsc::sync_channel(1);
    let (stopped_tx, stopped_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        // Both actual owners live in an independent service-like worker.
        let receipt = owned
            .research_commit_registered_owner(&reservation, now)
            .expect("registered owner commit");
        assert!(!receipt.is_stop_required_at(now));
        assert!(matches!(
            owned.research_commit_registered_owner(&reservation, now),
            Err(ResearchOwnedCommitError::OwnerUnavailable)
        ));
        committed_tx.send(()).unwrap();

        let until = Instant::now() + Duration::from_secs(3);
        while !receipt.is_stop_required_at(now) {
            assert!(
                Instant::now() < until,
                "original-generation stop not visible"
            );
            thread::yield_now();
        }
        stopped_tx.send(()).unwrap();
        // Deliberately hold the two owners until supervisor authorizes release.
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        drop(receipt);
        // Safe only in this synthetic test: no native process was created.
        drop(reservation);
        drop(owned);
    });
    committed_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(
        manager
            .find_recorded_private_action(&a.handle, &action)
            .unwrap()
            .is_some(),
        "JobStore owner remains registered while the worker is active"
    );

    let reg_for_close = Arc::clone(&registry);
    let handle_for_close = a.handle.clone();
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        done_tx
            .send(reg_for_close.revoke(&handle_for_close))
            .unwrap();
    });
    stopped_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("owned native worker must observe close before revoke drain");
    assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert!(
        manager
            .find_recorded_private_action(&a.handle, &action)
            .unwrap()
            .is_some(),
        "registered owner must remain during stop-before-drain"
    );
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(SessionRegistryError::AdmissionsInFlight)
    ));
    assert!(registry.get_active(&b.handle, now).is_ok());

    release_tx.send(()).unwrap();
    worker.join().unwrap();
    assert!(
        done_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("drain completes after worker releases owned permit")
            .expect("revoke result")
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn direct_close_before_native_commit_refuses_claim_and_preserves_owner_until_explicit_release() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    registry.register(a.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (root, manager, reservation, action) = reserve(&a.handle);
    let reg_for_close = Arc::clone(&registry);
    let handle_for_close = a.handle.clone();
    let (tx, rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        tx.send(reg_for_close.revoke(&handle_for_close)).unwrap();
    });
    observe_closed(&registry, &a.handle);
    assert!(matches!(
        owned.research_commit_registered_owner(&reservation, now),
        Err(ResearchOwnedCommitError::Registry(
            SessionRegistryError::Revoked
        ))
    ));
    assert!(owned.stop_watch().is_stop_required_at(now));
    assert!(
        manager
            .find_recorded_private_action(&a.handle, &action)
            .unwrap()
            .is_some()
    );
    assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    drop(reservation);
    drop(owned);
    assert!(
        rx.recv_timeout(Duration::from_secs(3))
            .expect("revoke can drain")
            .expect("revoke result")
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn owned_native_commit_rejects_cross_session_owner_and_original_deadline_after_renewal() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (root_a, manager_a, reservation_a, _) = reserve(&a.handle);
    let (root_b, manager_b, reservation_b, _) = reserve(&b.handle);
    assert!(matches!(
        owned.research_commit_registered_owner(&reservation_b, now),
        Err(ResearchOwnedCommitError::OwnerSessionMismatch)
    ));
    registry
        .renew_active(&a.handle, now, MonotonicTime::from_millis(200))
        .unwrap();
    assert!(matches!(
        owned.research_commit_registered_owner(&reservation_a, MonotonicTime::from_millis(100)),
        Err(ResearchOwnedCommitError::Registry(
            SessionRegistryError::Expired
        ))
    ));
    let renewed = registry
        .begin_owned_admission(&a.handle, MonotonicTime::from_millis(101))
        .unwrap();
    assert!(
        renewed
            .research_commit_registered_owner(&reservation_a, MonotonicTime::from_millis(101))
            .is_ok(),
        "new genuine admission can commit unused owner"
    );
    drop(renewed);
    drop(owned);
    drop(reservation_a);
    drop(reservation_b);
    drop(manager_a);
    drop(manager_b);
    fs::remove_dir_all(root_a).unwrap();
    fs::remove_dir_all(root_b).unwrap();
}
