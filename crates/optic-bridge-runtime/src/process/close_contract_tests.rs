//! Research-only real registry/lifecycle/accounting; no OS process is launched.
use super::*;
use crate::{
    SessionGrantSpec, SessionLifecycleManager, SessionRegistry, SessionRegistryError,
    TaskLeaseRegistry,
};
use optic_bridge_core::{PrincipalId, ProjectId};
use std::sync::mpsc;

fn wait(mut predicate: impl FnMut() -> bool) {
    let until = std::time::Instant::now() + Duration::from_secs(2);
    while !predicate() {
        assert!(
            std::time::Instant::now() < until,
            "research synchronization deadline"
        );
        std::thread::yield_now();
    }
}

fn exercise(early_close: bool) {
    let root = std::env::temp_dir().join(format!(
        "optic317-close-{}",
        JobId::generate().unwrap().to_token()
    ));
    fs::create_dir(&root).unwrap();
    let registry = Arc::new(SessionRegistry::new());
    let processes =
        Arc::new(ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap());
    let lifecycle = SessionLifecycleManager::new(
        Arc::clone(&registry),
        Arc::new(TaskLeaseRegistry::new()),
        Arc::clone(&processes),
    );
    let now = optic_bridge_core::MonotonicTime::from_millis(1);
    let provision = || {
        lifecycle
            .provision(
                SessionGrantSpec {
                    principal: PrincipalId::new("synthetic").unwrap(),
                    project: ProjectId::new("synthetic").unwrap(),
                    capabilities: BTreeSet::new(),
                    expires_at: optic_bridge_core::MonotonicTime::from_millis(10_000),
                    policy_epoch: 1,
                },
                now,
            )
            .unwrap()
    };
    let a = provision();
    let b = provision();
    let id = JobId::generate().unwrap();
    let record =
        super::tests::synthetic_record(a.handle.clone(), 0, ProcessStatus::Running, 1024, 1);
    processes
        .jobs
        .lock()
        .unwrap()
        .jobs
        .insert(id.clone(), Arc::clone(&record));
    let (admitted_tx, admitted_rx) = mpsc::sync_channel(1);
    let (stopped_tx, stopped_rx) = mpsc::sync_channel(1);
    let (proof_tx, proof_rx) = mpsc::sync_channel(1);
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    std::thread::scope(|scope| {
        let worker_registry = Arc::clone(&registry);
        let worker_record = Arc::clone(&record);
        let worker_session = a.handle.clone();
        let worker = scope.spawn(move || {
            // Genuine admission belongs to the operation worker, not the test caller.
            let _admission = worker_registry
                .begin_admission(&worker_session, now)
                .unwrap();
            admitted_tx.send(()).unwrap();
            wait(|| worker_record.stop_requested.load(Ordering::Acquire));
            worker_record.state.lock().unwrap().status = ProcessStatus::TerminationUncertain;
            stopped_tx.send(()).unwrap();
            // Bounded observation suppression, never an unbounded test hang.
            proof_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            worker_record.state.lock().unwrap().status = ProcessStatus::Stopped;
        });
        admitted_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        if early_close {
            assert_eq!(
                registry.research_close_admission(&a.handle).unwrap(),
                (true, 1)
            );
            assert_eq!(
                registry.research_close_admission(&a.handle).unwrap(),
                (false, 1)
            );
            // Stage B is independent of synchronous admission drainage.
            assert_eq!(processes.cancel_session(&a.handle).unwrap(), 1);
        }
        let revoker = scope.spawn(|| done_tx.send(lifecycle.revoke(&a.handle)).unwrap());
        wait(|| {
            matches!(
                registry.get_active(&a.handle, now),
                Err(SessionRegistryError::Revoked)
            )
        });
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(matches!(
            registry.begin_admission(&a.handle, now),
            Err(SessionRegistryError::Revoked)
        ));
        // B's authority works while A is closed and its operation still admitted.
        drop(registry.begin_admission(&b.handle, now).unwrap());
        assert!(lifecycle.revoke(&b.handle).is_ok());
        if !early_close {
            // Negative control: actual lifecycle has closed authority but has
            // NOT reached ProcessManager cancellation while the worker drains.
            assert!(!record.stop_requested.load(Ordering::Acquire));
            assert_eq!(processes.active_session_job_count(&a.handle).unwrap(), 1);
            // Finite recovery through actual out-of-band cancellation.
            assert_eq!(processes.cancel_session(&a.handle).unwrap(), 1);
        }
        stopped_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(processes.active_session_job_count(&a.handle).unwrap(), 1);
        assert_eq!(
            processes
                .remove_terminal_session_records(&a.handle)
                .unwrap(),
            0
        );
        assert_eq!(
            registry
                .remove_quiescent_inactive(&a.handle, now)
                .unwrap_err(),
            SessionRegistryError::AdmissionsInFlight
        );
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        proof_tx.send(()).unwrap();
        worker.join().unwrap();
        let report = done_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        revoker.join().unwrap();
        // A split close changes report meaning: document, do not silently ship.
        assert_eq!(report.session_changed, !early_close);
        assert_eq!(report.cancellation_requests, 0);
    });
    assert_eq!(processes.active_session_job_count(&a.handle).unwrap(), 0);
    assert!(lifecycle.seal_quiescent(&a.handle).is_ok());
    assert!(lifecycle.try_reap(&a.handle, now).unwrap().session_removed);
    fs::remove_dir(&root).unwrap();
}

#[test]
fn lifecycle_drain_postpones_cancellation_until_out_of_band_stop() {
    exercise(false);
}

#[test]
fn research_close_then_stop_preserves_pending_ownership_and_report_difference() {
    exercise(true);
}
