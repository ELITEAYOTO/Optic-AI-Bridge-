//! Safe-only research composition. Win32 and unsafe code live exclusively in
//! optic-bridge-windows, never in this #![forbid(unsafe_code)] runtime crate.
use super::{grant, reserve};
use crate::{ProcessStatus, SessionRegistry};
use optic_bridge_core::MonotonicTime;
use optic_bridge_windows::{ResearchKernelJob, ResearchSuspendedChild};
use std::{
    fs,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

#[test]
fn owned_registered_kernel_tree_stops_before_admission_drains() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (root, manager, reservation, action) = reserve(&a.handle);
    let worker_root = root.clone();

    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (observed_tx, observed_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        // Prepare a REAL kernel Job and reserve all accounting before effect.
        let job = ResearchKernelJob::prepare().expect("precreate real kill-on-close Job");
        let mut reservation = reservation;
        let receipt = owned
            .research_commit_registered_owner(&reservation, now)
            .expect("short generation-bound owner commitment");
        let stop = owned.stop_watch();
        assert!(!receipt.is_stop_required_at(now));
        assert!(!stop.is_stop_required_at(now));
        // No native effect may start if a stop was ALREADY visible.
        let child = ResearchSuspendedChild::create(&job, &worker_root)
            .expect("create fixed suspended child associated atomically with Job");
        // Once creation has succeeded the one-shot commitment was consumed.
        // The actual service must independently latch stop across this gap.
        drop(receipt);
        reservation.mark_native_created();
        assert_eq!(job.active().unwrap(), 1);
        ready_tx.send(()).unwrap();

        // This read-only signal must work WITHOUT ProcessManager::cancel_session.
        let limit = Instant::now() + Duration::from_secs(3);
        while !stop.is_stop_required_at(now) {
            assert!(Instant::now() < limit, "owner stop not published");
            thread::yield_now();
        }
        observed_tx
            .send(job.active().expect("still recorded kernel child"))
            .unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        // Keep reservation AND counted admission until POSITIVE native proof.
        job.terminate().expect("stop exact owned Job");
        child.await_exit().expect("root termination confirmed");
        job.await_empty().expect("whole native tree empty");
        reservation.finish(ProcessStatus::Stopped, None);
        drop(reservation);
        drop(child);
        drop(job);
        drop(owned);
    });
    ready_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(
        manager
            .find_recorded_private_action(&a.handle, &action)
            .unwrap()
            .is_some()
    );
    let registry_close = Arc::clone(&registry);
    let a_close = a.handle.clone();
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        done_tx.send(registry_close.revoke(&a_close)).unwrap();
    });
    assert_eq!(
        observed_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
        1,
        "worker observed stop while kernel child was STILL contained and counted"
    );
    assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(crate::SessionRegistryError::AdmissionsInFlight)
    ));
    assert!(registry.get_active(&b.handle, now).is_ok());
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    assert!(
        done_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("admission drains after positive Job-tree termination")
            .expect("registry revoke result")
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}
