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

#[test]
fn prearmed_native_creation_drop_retains_quota_with_live_suspended_child() {
    // Companion to the unmarked negative control: arm conservatively BEFORE
    // CreateProcessW. This proves accounting only, not guardian ownership.
    let a = grant();
    let (root, manager, mut reservation, action) = reserve(&a.handle);
    let job_id = reservation.id();
    let job = ResearchKernelJob::prepare().expect("precreate real kernel Job");
    reservation.mark_native_created();
    let child = ResearchSuspendedChild::create(&job, &root)
        .expect("fixed suspended synthetic child associated during CreateProcessW");
    assert_eq!(job.active().unwrap(), 1);
    drop(reservation);
    assert_eq!(job.active().unwrap(), 1);
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::TerminationUncertain,
        "prearmed Drop may not publish Failed while the Job contains a child"
    );
    assert_eq!(
        manager.active_session_job_count(&a.handle).unwrap(),
        1,
        "capacity must remain reserved under uncertain native ownership"
    );
    assert_eq!(
        manager.remove_terminal_session_records(&a.handle).unwrap(),
        0,
        "an uncertain record cannot be reaped as completed"
    );
    assert_eq!(
        manager
            .find_recorded_private_action(&a.handle, &action)
            .unwrap(),
        Some(job_id)
    );
    // The test still owns the independent real Job. A production guardian
    // needs explicit responsibility and a proof-driven reconciliation path.
    job.terminate().expect("terminate only this synthetic Job");
    child
        .await_exit()
        .expect("wait for its exact suspended child");
    job.await_empty()
        .expect("whole Job empty before releasing native handle");
    drop(child);
    drop(job);
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prearmed_creation_without_child_retains_uncertain_quota_until_reconciliation() {
    // Safety/liveness tradeoff: prearming *before* a failed native call cannot
    // distinguish an empty Job from a live child on its own. The result stays
    // uncertain even when this fixture never calls CreateProcessW.
    let a = grant();
    let (root, manager, mut reservation, _) = reserve(&a.handle);
    let job_id = reservation.id();
    let job = ResearchKernelJob::prepare().expect("precreate real empty Job");
    assert_eq!(job.active().unwrap(), 0);
    reservation.mark_native_created();
    drop(reservation);
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::TerminationUncertain
    );
    assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
    assert_eq!(
        manager.remove_terminal_session_records(&a.handle).unwrap(),
        0
    );
    assert_eq!(
        job.active().unwrap(),
        0,
        "empty-Job proof exists but the abandoned record cannot consume it"
    );
    drop(job);
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn close_request_returns_before_native_quiescence_while_owner_and_quota_remain_retained() {
    // This proves a test-only CLOSE operation can return before the real
    // counted admission drains. It cannot be used as a terminal Job receipt.
    const FAILED_OBSERVATIONS: usize = 3;
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let watcher = owned.stop_watch();
    let (root, manager, reservation, action) = reserve(&a.handle);
    let job_id = reservation.id();
    let worker_root = root.clone();

    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (fault_tx, fault_rx) = mpsc::sync_channel(1);
    let (continue_tx, continue_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let job = ResearchKernelJob::prepare().expect("precreate exact native Job");
        let mut reservation = reservation;
        let receipt = owned
            .research_commit_registered_owner(&reservation, now)
            .expect("registered owner committed before close");
        let stop = owned.stop_watch();
        let child = ResearchSuspendedChild::create(&job, &worker_root)
            .expect("fixed synthetic child assigned at native creation");
        drop(receipt);
        reservation.mark_native_created();
        assert_eq!(job.active().unwrap(), 1);
        ready_tx.send(()).unwrap();

        let until = Instant::now() + Duration::from_secs(3);
        while !stop.is_stop_required_at(now) {
            assert!(Instant::now() < until, "close signal did not arrive");
            thread::yield_now();
        }
        job.terminate()
            .expect("request termination of exact owned Job");
        child
            .await_exit()
            .expect("root exit still does not prove Job empty");
        for _ in 0..FAILED_OBSERVATIONS {
            assert!(
                job.observe_active_with_injected_failure(true).is_err(),
                "injected query failure must never be mistaken for Job-empty"
            );
            fault_tx.send(()).unwrap();
            continue_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("bounded independent supervisor observation");
        }

        assert_eq!(
            job.observe_active_with_injected_failure(false)
                .expect("fixture-only positive native Job observation"),
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
        .expect("worker owns an actual suspended Job member before close");
    assert!(
        registry
            .research_request_close_without_drain(&a.handle)
            .unwrap(),
        "initial close must change the session generation"
    );
    assert!(
        watcher.is_stop_required_at(now),
        "original-generation worker must observe stop before drain"
    );
    assert!(
        !registry
            .research_request_close_without_drain(&a.handle)
            .unwrap(),
        "repeated stop publication is idempotent"
    );
    assert!(matches!(
        registry.begin_owned_admission(&a.handle, now),
        Err(crate::SessionRegistryError::Revoked)
    ));

    let closer_registry = Arc::clone(&registry);
    let closer_handle = a.handle.clone();
    let (drained_tx, drained_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        drained_tx
            .send(closer_registry.revoke(&closer_handle))
            .unwrap();
    });

    for _ in 0..FAILED_OBSERVATIONS {
        fault_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("worker observed close despite retained admission");
        assert!(
            matches!(drained_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "ordinary synchronous revoke is still waiting on the real owner"
        );
        assert_eq!(
            manager.result(&a.handle, &job_id).unwrap().status,
            ProcessStatus::Running,
            "native uncertainty cannot publish a terminal result"
        );
        assert_eq!(
            manager.active_session_job_count(&a.handle).unwrap(),
            1,
            "native uncertainty cannot return a quota"
        );
        assert_eq!(
            manager
                .find_recorded_private_action(&a.handle, &action)
                .unwrap(),
            Some(job_id.clone())
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
        .expect("owner cleaned up after genuine empty-Job proof");
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::Stopped
    );
    assert!(
        !drained_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("normal revoke drains after the owner releases its admission")
            .expect("synchronous revoke result"),
        "the prior close already published the generation's revoked bit"
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn in_process_guardian_outlives_submitter_and_retains_uncertain_native_owner() {
    // Research-only service ownership boundary. The submitter thread finishes
    // before close; the service keeps the guardian JoinHandle, counted session
    // admission, JobStore reservation and actual kill-on-close Windows Job.
    // This does NOT survive service-process death or prove bounded Drop.
    const FAILED_OBSERVATIONS: usize = 5;
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

    let (submitted_tx, submitted_rx) = mpsc::sync_channel(1);
    let (client_result_tx, client_result_rx) = mpsc::sync_channel(1);
    let (fault_tx, fault_rx) = mpsc::sync_channel(1);
    let (retry_tx, retry_rx) = mpsc::sync_channel(1);
    let guardian = thread::spawn(move || {
        let job = ResearchKernelJob::prepare().expect("guardian owns precreated native Job");
        let mut reservation = reservation;
        let receipt = owned
            .research_commit_registered_owner(&reservation, now)
            .expect("single authorized owner commitment");
        let stop = owned.stop_watch();
        // The commitment receipt borrows the registered reservation; release
        // that read-only borrow before mutating the conservative creation state.
        // No stop/close authority is inferred from dropping this receipt.
        drop(receipt);
        // Conservative accounting is armed before CreateProcessW. This alone
        // does not solve the empty-Job recovery problem from the prior tests.
        reservation.mark_native_created();
        let child = ResearchSuspendedChild::create(&job, &worker_root)
            .expect("synthetic suspended child associated at native creation");
        assert_eq!(job.active().unwrap(), 1);
        submitted_tx
            .send(reservation.id())
            .expect("transmit JobId, never ownership, to short-lived submitter");

        let until = Instant::now() + Duration::from_secs(3);
        while !stop.is_stop_required_at(now) {
            assert!(Instant::now() < until, "service guardian did not see close");
            thread::yield_now();
        }
        job.terminate()
            .expect("stop only this guardian's native Job");
        child.await_exit().expect("synthetic root exited");
        for _ in 0..FAILED_OBSERVATIONS {
            assert!(
                job.observe_active_with_injected_failure(true).is_err(),
                "unavailable native accounting is not zero-membership evidence"
            );
            fault_tx
                .send(())
                .expect("supervisor still retains guardian handle");
            retry_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("bounded research observation gate");
        }
        assert_eq!(
            job.observe_active_with_injected_failure(false)
                .expect("real query for controlled fixture cleanup only"),
            0
        );
        reservation.finish(ProcessStatus::Stopped, None);
        drop(reservation);
        drop(child);
        drop(job);
        drop(owned);
    });

    // A caller receives an identifier, owns none of the process handles, and
    // fully exits before service shutdown. It cannot dispose of the guardian.
    let submitter = thread::spawn(move || {
        let id = submitted_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("service guardian admitted the synthetic native owner");
        client_result_tx.send(id).unwrap();
    });
    assert_eq!(
        client_result_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("short-lived submitter response"),
        job_id
    );
    submitter
        .join()
        .expect("submitter is completely gone before close");
    assert!(!guardian.is_finished());
    assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);

    // Close is only a stop request; it cannot wait for the native accounting
    // gate, nor may it be reported as whole-process quiescence.
    assert!(
        registry
            .research_request_close_without_drain(&a.handle)
            .unwrap()
    );
    assert!(matches!(
        registry.begin_owned_admission(&a.handle, now),
        Err(crate::SessionRegistryError::Revoked)
    ));
    let closer_registry = Arc::clone(&registry);
    let closer_handle = a.handle.clone();
    let (drained_tx, drained_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        drained_tx
            .send(closer_registry.revoke(&closer_handle))
            .unwrap();
    });

    for _ in 0..FAILED_OBSERVATIONS {
        fault_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("guardian encountered native observation uncertainty");
        assert!(
            !guardian.is_finished(),
            "service must still own its guardian"
        );
        assert!(matches!(
            drained_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert_eq!(
            manager.result(&a.handle, &job_id).unwrap().status,
            ProcessStatus::Running,
            "no false terminal result while the service retains native ownership"
        );
        assert_eq!(
            manager.active_session_job_count(&a.handle).unwrap(),
            1,
            "no capacity release while observation remains unavailable"
        );
        assert_eq!(
            manager
                .find_recorded_private_action(&a.handle, &action)
                .unwrap(),
            Some(job_id.clone())
        );
        assert!(matches!(
            registry.remove_quiescent_inactive(&a.handle, now),
            Err(crate::SessionRegistryError::AdmissionsInFlight)
        ));
        assert!(registry.get_active(&b.handle, now).is_ok());
        retry_tx.send(()).unwrap();
    }

    // The service explicitly joins the still-owned guardian only after the
    // fixture's REAL empty-Job query; no detached task or timeout-based Drop.
    guardian
        .join()
        .expect("guardian completed with native proof");
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::Stopped
    );
    assert!(
        !drained_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("synchronous revoke drains after proven native quiescence")
            .expect("revoke result"),
        "early close already published the stop intent"
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn transferred_native_job_remains_owned_after_creator_exits() {
    // Research-only successful ownership transfer, not a crash-atomic handoff.
    // A separate short-lived launcher creates the actual native child; the
    // guardian owns the counted admission and JobStore reservation first,
    // then receives BOTH Win32 Job and child handles before launcher exit.
    const FAILED_OBSERVATIONS: usize = 3;
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (root, manager, reservation, action) = reserve(&a.handle);
    let job_id = reservation.id();
    let launcher_root = root.clone();
    let job = ResearchKernelJob::prepare().expect("precreate real native Job");

    let (launch_tx, launch_rx) = mpsc::sync_channel(1);
    let (handoff_tx, handoff_rx) = mpsc::sync_channel(1);
    let (accepted_tx, accepted_rx) = mpsc::sync_channel(1);
    let (fault_tx, fault_rx) = mpsc::sync_channel(1);
    let (retry_tx, retry_rx) = mpsc::sync_channel(1);

    let guardian = thread::spawn(move || {
        let mut reservation = reservation;
        let receipt = owned
            .research_commit_registered_owner(&reservation, now)
            .expect("real admission commits original registered owner");
        let stop = owned.stop_watch();
        drop(receipt);
        reservation.mark_native_created();
        launch_tx
            .send(())
            .expect("prearmed accounting precedes native launch");
        let (job, child): (ResearchKernelJob, ResearchSuspendedChild) = handoff_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("bounded transfer of both real native handles");
        assert_eq!(job.active().unwrap(), 1);
        accepted_tx
            .send(())
            .expect("service guardian acknowledges resource acceptance");

        let until = Instant::now() + Duration::from_secs(3);
        while !stop.is_stop_required_at(now) {
            assert!(Instant::now() < until, "original stop signal missing");
            thread::yield_now();
        }
        job.terminate()
            .expect("terminate the exact transferred Job");
        child.await_exit().expect("transferred root process exited");
        for _ in 0..FAILED_OBSERVATIONS {
            assert!(
                job.observe_active_with_injected_failure(true).is_err(),
                "native observation failure is not empty-Job proof"
            );
            fault_tx.send(()).expect("guardian remains supervised");
            retry_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("bounded synthetic observation gate");
        }
        assert_eq!(
            job.observe_active_with_injected_failure(false)
                .expect("real positive observation for controlled cleanup"),
            0
        );
        reservation.finish(ProcessStatus::Stopped, None);
        drop(reservation);
        drop(child);
        drop(job);
        drop(owned);
    });
    let launcher = thread::spawn(move || {
        launch_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("service prearms before CreateProcessW");
        let child = ResearchSuspendedChild::create(&job, &launcher_root)
            .expect("only fixed synthetic libtest is created suspended");
        assert_eq!(job.active().unwrap(), 1);
        handoff_tx
            .send((job, child))
            .expect("transfer native handles to service guardian");
        // No native handle or quota is owned by the launcher after send.
    });

    accepted_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("guardian accepted both kernel handles");
    launcher
        .join()
        .expect("native creating worker has fully exited");
    assert!(
        !guardian.is_finished(),
        "guardian survives the process-creating worker"
    );
    assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
    assert!(
        registry
            .research_request_close_without_drain(&a.handle)
            .unwrap()
    );
    assert!(matches!(
        registry.begin_owned_admission(&a.handle, now),
        Err(crate::SessionRegistryError::Revoked)
    ));
    let draining_registry = Arc::clone(&registry);
    let draining_handle = a.handle.clone();
    let (drained_tx, drained_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        drained_tx
            .send(draining_registry.revoke(&draining_handle))
            .unwrap();
    });
    for _ in 0..FAILED_OBSERVATIONS {
        fault_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("native guardian still retains uncertainty");
        assert!(!guardian.is_finished());
        assert!(matches!(
            drained_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert_eq!(
            manager.result(&a.handle, &job_id).unwrap().status,
            ProcessStatus::Running,
            "no terminal publication from termination request or failed query"
        );
        assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
        assert_eq!(
            manager
                .find_recorded_private_action(&a.handle, &action)
                .unwrap(),
            Some(job_id.clone())
        );
        assert!(matches!(
            registry.remove_quiescent_inactive(&a.handle, now),
            Err(crate::SessionRegistryError::AdmissionsInFlight)
        ));
        assert!(registry.get_active(&b.handle, now).is_ok());
        retry_tx.send(()).unwrap();
    }
    guardian
        .join()
        .expect("service guardian joined after real Job-empty observation");
    assert_eq!(
        manager.result(&a.handle, &job_id).unwrap().status,
        ProcessStatus::Stopped
    );
    assert!(
        !drained_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("drain follows native proof")
            .expect("revoke result"),
        "early close already published the stop request"
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}
