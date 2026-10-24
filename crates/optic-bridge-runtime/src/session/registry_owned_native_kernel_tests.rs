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

#[test]
fn uncertain_kernel_accounting_retains_owned_admission_and_jobstore_until_retry_proves_empty() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let permit = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (root, manager, reservation, action) = reserve(&a.handle);
    let job_id = reservation.id();
    let worker_root = root.clone();

    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (uncertain_tx, uncertain_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let job = ResearchKernelJob::prepare().expect("precreated native Job");
        let mut reservation = reservation;
        let receipt = permit
            .research_commit_registered_owner(&reservation, now)
            .expect("one original-generation native commitment");
        let watch = permit.stop_watch();
        let child = ResearchSuspendedChild::create(&job, &worker_root)
            .expect("suspended child contained at creation");
        drop(receipt);
        reservation.mark_native_created();
        assert_eq!(job.active().unwrap(), 1);
        ready_tx.send(()).unwrap();

        let until = Instant::now() + Duration::from_secs(3);
        while !watch.is_stop_required_at(now) {
            assert!(
                Instant::now() < until,
                "revocation not observed by service-owned native worker"
            );
            thread::yield_now();
        }
        job.terminate().expect("request stop for exact owned Job");
        child.await_exit().expect("native root has exited");
        // Root exit AND a successful TerminateJobObject call still do not
        // authorize terminal publication if whole-Job accounting is unknown.
        assert!(
            job.observe_active_with_injected_failure(true).is_err(),
            "synthetic Win32 query failure must be visible, never mapped to zero"
        );
        uncertain_tx.send(()).unwrap();
        // Simulate a persistently unobservable Job while retaining ALL owners.
        release_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("supervisor allows exactly one later observation retry");
        assert_eq!(
            job.observe_active_with_injected_failure(false)
                .expect("native accounting recovers"),
            0,
            "full Job tree must be empty after observation recovers"
        );
        // Only the positive job-empty proof permits terminal publication and
        // release of the registry admission, not the earlier termination request.
        reservation.finish(ProcessStatus::Stopped, None);
        drop(reservation);
        drop(child);
        drop(job);
        drop(permit);
    });

    ready_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("native child created and worker retained owner");
    let registry_close = Arc::clone(&registry);
    let handle = a.handle.clone();
    let (closed_tx, closed_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        closed_tx.send(registry_close.revoke(&handle)).unwrap();
    });
    uncertain_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("worker saw stop, root exited, but Job observation failed");

    assert!(
        matches!(closed_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "real revoke must remain blocked on the retained owned admission"
    );
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::Running,
        "native observation failure must not publish a terminal result"
    );
    assert_eq!(
        manager
            .find_recorded_private_action(&a.handle, &action)
            .unwrap(),
        Some(job_id.clone()),
        "original JobStore registration remains discoverable during uncertainty"
    );
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(crate::SessionRegistryError::AdmissionsInFlight)
    ));
    assert!(
        registry.get_active(&b.handle, now).is_ok(),
        "unrelated session remains available during uncertain observation"
    );

    release_tx.send(()).unwrap();
    worker.join().unwrap();
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::Stopped,
        "only recovered positive whole-Job proof publishes terminal status"
    );
    assert!(
        closed_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("drain finishes only after positive observation")
            .expect("revoke result")
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn repeated_native_accounting_failures_retain_owner_and_quota_before_recovery() {
    // Research-only negative control: four consecutive injected query errors
    // stand in for an unavailable native observer. The final positive query is
    // only test cleanup, NOT a persistent-fault shutdown implementation.
    const FAILED_OBSERVATIONS: usize = 4;
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (root, manager, reservation, action) = reserve(&a.handle);
    let job_id = reservation.id();
    let worker_root = root.clone();

    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (fault_tx, fault_rx) = mpsc::sync_channel(1);
    let (continue_tx, continue_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let job = ResearchKernelJob::prepare().expect("precreate real native Job");
        let mut reservation = reservation;
        let receipt = owned
            .research_commit_registered_owner(&reservation, now)
            .expect("single original-generation owner commitment");
        let stop = owned.stop_watch();
        let child = ResearchSuspendedChild::create(&job, &worker_root)
            .expect("synthetic suspended child belongs to the Job at creation");
        drop(receipt);
        reservation.mark_native_created();
        assert_eq!(job.active().unwrap(), 1);
        ready_tx.send(()).unwrap();

        let until = Instant::now() + Duration::from_secs(3);
        while !stop.is_stop_required_at(now) {
            assert!(Instant::now() < until, "revoke failed to publish stop");
            thread::yield_now();
        }
        job.terminate().expect("stop exact owned Job");
        child
            .await_exit()
            .expect("root exit is not full-tree proof");
        for _ in 0..FAILED_OBSERVATIONS {
            assert!(
                job.observe_active_with_injected_failure(true).is_err(),
                "OS query errors must never become an empty-Job observation"
            );
            // The supervisor checks retained capacity BEFORE allowing another
            // observation. No indefinite sleep or detached worker is needed.
            fault_tx.send(()).unwrap();
            continue_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("bounded synthetic fault gate");
        }
        assert_eq!(
            job.observe_active_with_injected_failure(false)
                .expect("test-only positive observation to permit safe cleanup"),
            0
        );
        reservation.finish(ProcessStatus::Stopped, None);
        drop(reservation);
        drop(child);
        drop(job);
        drop(owned);
    });

    ready_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("native owner prepared before revocation");
    let close_registry = Arc::clone(&registry);
    let close_handle = a.handle.clone();
    let (closed_tx, closed_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        closed_tx
            .send(close_registry.revoke(&close_handle))
            .unwrap();
    });

    for _ in 0..FAILED_OBSERVATIONS {
        fault_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("worker observed stop and an injected query error");
        assert!(
            matches!(closed_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "synchronous revoke remains pending while an admission is retained"
        );
        assert_eq!(
            manager.result(&a.handle, &job_id).unwrap().status,
            ProcessStatus::Running,
            "no terminal publication is valid during native uncertainty"
        );
        assert_eq!(
            manager.active_session_job_count(&a.handle).unwrap(),
            1,
            "uncertain process capacity is not returned to the session"
        );
        assert_eq!(
            manager
                .find_recorded_private_action(&a.handle, &action)
                .unwrap(),
            Some(job_id.clone()),
            "native owner reservation remains registered"
        );
        assert!(matches!(
            registry.remove_quiescent_inactive(&a.handle, now),
            Err(crate::SessionRegistryError::AdmissionsInFlight)
        ));
        assert!(registry.get_active(&b.handle, now).is_ok());
        continue_tx.send(()).unwrap();
    }

    worker
        .join()
        .expect("test worker reaped after positive Job proof");
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::Stopped
    );
    assert!(
        closed_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("drain eventually completes after a real empty-Job observation")
            .expect("registry revoke result")
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unmarked_native_creation_drop_reports_terminal_while_job_still_contains_child_negative_control()
{
    // Negative control, not a candidate fix: creation and mark_native_created()
    // are separate calls. Model an abort after a real suspended child was
    // created but BEFORE the reservation learned that creation succeeded.
    // No real Git command is run, and the Job remains safely owned by this test.
    let a = grant();
    let (root, manager, reservation, action) = reserve(&a.handle);
    let job_id = reservation.id();
    let job = ResearchKernelJob::prepare().expect("precreate genuine kernel Job");
    let child = ResearchSuspendedChild::create(&job, &root)
        .expect("fixed current libtest suspended in Job at creation");
    assert_eq!(job.active().unwrap(), 1);
    assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
    // Critically omit mark_native_created, as if the owner aborts in the
    // explicit post-CreateProcessW / pre-mark gap.
    drop(reservation);
    assert_eq!(
        job.active().unwrap(),
        1,
        "kernel still owns a live suspended child after reservation drop"
    );
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::Failed,
        "NEGATIVE CONTROL: Drop incorrectly infers that no native process ran"
    );
    assert_eq!(
        manager.active_session_job_count(&a.handle).unwrap(),
        0,
        "NEGATIVE CONTROL: terminal accounting returned quota before Job emptied"
    );
    assert_eq!(
        manager
            .find_recorded_private_action(&a.handle, &action)
            .unwrap(),
        Some(job_id),
        "the original ledger record is still traceable despite false terminal"
    );

    // Independently clean only our synthetic suspended child and native Job.
    job.terminate().expect("terminate exact synthetic Job");
    child.await_exit().expect("wait for its owned child");
    job.await_empty()
        .expect("positive Job-empty cleanup observation");
    drop(child);
    drop(job);
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}
