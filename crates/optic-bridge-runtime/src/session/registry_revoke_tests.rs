//! Compatibility tests: real admission permits, no native process or research gate.
use super::*;
use optic_bridge_core::{Capability, PrincipalId, ProjectId};
use std::{
    collections::BTreeSet,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

fn grant() -> SessionGrant {
    SessionGrant {
        handle: SessionHandle::generate().unwrap(),
        principal: PrincipalId::new("test").unwrap(),
        project: ProjectId::new("synthetic").unwrap(),
        capabilities: BTreeSet::from([Capability::FileRead]),
        expires_at: MonotonicTime::from_millis(100),
        policy_epoch: 1,
    }
}
fn observe_closed(registry: &SessionRegistry, handle: &SessionHandle) {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        if matches!(
            registry.get_active(handle, MonotonicTime::from_millis(1)),
            Err(SessionRegistryError::Revoked)
        ) {
            return;
        }
        assert!(Instant::now() < until, "close was not published");
        thread::yield_now();
    }
}

#[test]
fn simultaneous_admissions_two_revokers_and_other_session_notification() {
    let registry = SessionRegistry::new();
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let first = registry.begin_admission(&a.handle, now).unwrap();
    let second = registry.begin_admission(&a.handle, now).unwrap();
    let last = registry.begin_admission(&a.handle, now).unwrap();
    let b_permit = registry.begin_admission(&b.handle, now).unwrap();
    let signal = Arc::clone(
        &registry
            .sessions
            .lock()
            .unwrap()
            .get(&a.handle)
            .unwrap()
            .revocation_signal,
    );
    let (tx, rx) = mpsc::sync_channel(2);
    thread::scope(|scope| {
        let registry_ref = &registry;
        let handle = &a.handle;
        let tx2 = tx.clone();
        let one = scope.spawn(move || tx.send(registry_ref.revoke(handle)).unwrap());
        let two = scope.spawn(move || tx2.send(registry_ref.revoke(handle)).unwrap());
        observe_closed(&registry, &a.handle);
        assert!(signal.load(Ordering::Acquire));
        assert!(matches!(
            registry.begin_admission(&a.handle, now),
            Err(SessionRegistryError::Revoked)
        ));
        assert!(matches!(
            registry.remove_quiescent_inactive(&a.handle, now),
            Err(SessionRegistryError::AdmissionsInFlight)
        ));
        drop(first);
        drop(second);
        // Cross-session last-permit drop and an explicit notification cannot
        // satisfy A's predicate. These notifications are not completion receipts.
        drop(b_permit);
        registry.admissions_drained.notify_all();
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(registry.get_active(&b.handle, now).is_ok());
        assert!(registry.revoke(&b.handle).unwrap());
        assert_eq!(
            registry
                .renew_active(&a.handle, now, MonotonicTime::from_millis(200))
                .unwrap_err(),
            SessionRenewalError::Revoked
        );
        drop(last);
        let results = [
            rx.recv_timeout(Duration::from_secs(2)).unwrap().unwrap(),
            rx.recv_timeout(Duration::from_secs(2)).unwrap().unwrap(),
        ];
        assert_eq!(results.iter().filter(|v| **v).count(), 1);
        one.join().unwrap();
        two.join().unwrap();
    });
    assert!(!registry.revoke(&a.handle).unwrap());
    assert!(registry.remove_quiescent_inactive(&a.handle, now).unwrap());
    assert!(!registry.revoke(&a.handle).unwrap());
}

#[test]
fn renewal_before_close_and_absent_session_keep_existing_results() {
    let registry = SessionRegistry::new();
    let g = grant();
    let now = MonotonicTime::from_millis(1);
    assert!(!registry.revoke(&g.handle).unwrap());
    registry.register(g.clone()).unwrap();
    registry
        .renew_active(&g.handle, now, MonotonicTime::from_millis(200))
        .unwrap();
    assert_eq!(
        registry
            .get_active(&g.handle, MonotonicTime::from_millis(150))
            .unwrap()
            .expires_at,
        MonotonicTime::from_millis(200)
    );
    assert!(registry.revoke(&g.handle).unwrap());
    assert!(!registry.revoke(&g.handle).unwrap());
    assert_eq!(
        registry
            .renew_active(&g.handle, now, MonotonicTime::from_millis(300))
            .unwrap_err(),
        SessionRenewalError::Revoked
    );
}

#[test]
fn poisoned_entry_mutex_returns_state_unavailable() {
    let registry = SessionRegistry::new();
    let g = grant();
    registry.register(g.clone()).unwrap();
    let failure = std::panic::catch_unwind(|| {
        let _guard = registry.sessions.lock().unwrap();
        panic!("fixed poison injection");
    });
    assert!(failure.is_err());
    assert_eq!(
        registry.revoke(&g.handle),
        Err(SessionRegistryError::StateUnavailable)
    );
}

#[test]
fn poisoned_condvar_reacquisition_preserves_published_close_and_error() {
    let registry = SessionRegistry::new();
    let g = grant();
    registry.register(g.clone()).unwrap();
    let permit = registry
        .begin_admission(&g.handle, MonotonicTime::from_millis(1))
        .unwrap();
    let signal = Arc::clone(
        &registry
            .sessions
            .lock()
            .unwrap()
            .get(&g.handle)
            .unwrap()
            .revocation_signal,
    );
    let (tx, rx) = mpsc::sync_channel(1);
    thread::scope(|scope| {
        let registry_ref = &registry;
        let handle = &g.handle;
        let worker = scope.spawn(move || tx.send(registry_ref.revoke(handle)).unwrap());
        observe_closed(&registry, &g.handle);
        assert!(signal.load(Ordering::Acquire));
        let failure = std::panic::catch_unwind(|| {
            // Acquiring this mutex after close proves the revoker released it
            // in Condvar::wait, since it still owns a genuine live admission.
            let _guard = registry.sessions.lock().unwrap();
            registry.admissions_drained.notify_all();
            panic!("fixed poison during drainage");
        });
        assert!(failure.is_err());
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Err(SessionRegistryError::StateUnavailable)
        );
        worker.join().unwrap();
    });
    drop(permit); // Existing fail-closed poison handling; no native resources.
}

// Frozen predicate/wait loop from main da34eae, used only as a compatibility
// oracle for controlled guarded states. This is not a new generation contract.
fn legacy_drain<'a>(
    registry: &SessionRegistry,
    handle: &SessionHandle,
    mut guard: std::sync::MutexGuard<'a, HashMap<SessionHandle, SessionRecord>>,
) -> Result<std::sync::MutexGuard<'a, HashMap<SessionHandle, SessionRecord>>, SessionRegistryError>
{
    loop {
        let count = guard
            .get(handle)
            .ok_or(SessionRegistryError::UnknownSession)?
            .active_admissions;
        if count == 0 {
            return Ok(guard);
        }
        guard = registry
            .admissions_drained
            .wait(guard)
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
    }
}

#[test]
fn missing_record_at_predicate_check_matches_legacy_unknown_session() {
    let registry = SessionRegistry::new();
    let g = grant();
    for legacy in [true, false] {
        let guard = registry.sessions.lock().unwrap();
        let error = if legacy {
            legacy_drain(&registry, &g.handle, guard)
        } else {
            registry.wait_for_admission_drain(&g.handle, guard)
        }
        .unwrap_err();
        assert_eq!(error, SessionRegistryError::UnknownSession);
    }
}

#[test]
fn replacement_generation_at_predicate_check_keeps_legacy_handle_lookup() {
    for legacy in [true, false] {
        let registry = SessionRegistry::new();
        let old = grant();
        registry.register(old.clone()).unwrap();
        let now = MonotonicTime::from_millis(1);
        let old_signal = {
            let mut guard = registry.sessions.lock().unwrap();
            let record = guard.get_mut(&old.handle).unwrap();
            assert!(SessionRegistry::publish_revocation(record));
            Arc::clone(&record.revocation_signal)
        };
        assert!(
            registry
                .remove_quiescent_inactive(&old.handle, now)
                .unwrap()
        );
        let mut replacement = grant();
        replacement.handle = old.handle.clone();
        registry.register(replacement).unwrap();
        let permit = registry.begin_admission(&old.handle, now).unwrap();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        thread::scope(|scope| {
            let registry_ref = &registry;
            let handle = &old.handle;
            let old_signal_ref = &old_signal;
            let worker = scope.spawn(move || {
                let guard = registry_ref.sessions.lock().unwrap();
                assert!(!Arc::ptr_eq(
                    &guard.get(handle).unwrap().revocation_signal,
                    old_signal_ref
                ));
                ready_tx.send(()).unwrap();
                // Model the state seen on mutex reacquisition after a permitted
                // removal/re-registration. Do NOT claim this forces that whole
                // public-revoke race; the entry schedule is controlled here.
                let guard = if legacy {
                    legacy_drain(registry_ref, handle, guard)
                } else {
                    registry_ref.wait_for_admission_drain(handle, guard)
                }
                .unwrap();
                done_tx.send(!guard.get(handle).unwrap().revoked).unwrap();
            });
            ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(registry.get_active(&old.handle, now).is_ok());
            assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
            drop(permit);
            assert!(done_rx.recv_timeout(Duration::from_secs(2)).unwrap());
            worker.join().unwrap();
        });
        assert!(old_signal.load(Ordering::Acquire));
        assert!(registry.get_active(&old.handle, now).is_ok());
    }
}

#[test]
fn admission_revocation_watch_is_read_only_and_remains_bound_to_original_record() {
    let registry = SessionRegistry::new();
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let original_permit = registry.begin_admission(&a.handle, now).unwrap();
    let watcher = original_permit.revocation_watch();
    let old_watcher = watcher.clone();
    let b_permit = registry.begin_admission(&b.handle, now).unwrap();
    let unrelated_watcher = b_permit.revocation_watch();
    assert!(!watcher.is_revoked());
    assert!(!unrelated_watcher.is_revoked());

    let (closed_tx, closed_rx) = mpsc::sync_channel(1);
    thread::scope(|scope| {
        let closer = scope.spawn(|| {
            closed_tx.send(registry.revoke(&a.handle)).unwrap();
        });
        observe_closed(&registry, &a.handle);
        assert!(watcher.is_revoked(), "real direct revoke publishes stop");
        assert!(old_watcher.is_revoked());
        assert!(
            !unrelated_watcher.is_revoked(),
            "closing A never publishes B's stop"
        );
        assert!(matches!(
            closed_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        drop(original_permit);
        assert!(
            closed_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap()
        );
        closer.join().unwrap();
    });

    assert!(registry.remove_quiescent_inactive(&a.handle, now).unwrap());
    // Reusing a handle is a valid historical compatibility path once all
    // genuine permits have drained. The old watch must NEVER follow it.
    registry.register(a.clone()).unwrap();
    let new_permit = registry.begin_admission(&a.handle, now).unwrap();
    let replacement_watcher = new_permit.revocation_watch();
    assert!(old_watcher.is_revoked());
    assert!(
        !replacement_watcher.is_revoked(),
        "the new session record has a distinct revocation identity"
    );
    drop(new_permit);
    assert!(registry.revoke(&a.handle).unwrap());
    assert!(replacement_watcher.is_revoked());
    assert!(old_watcher.is_revoked());
    drop(b_permit);
}

#[test]
fn admission_revocation_watch_does_not_replace_monotonic_expiry_check() {
    let registry = SessionRegistry::new();
    let a = grant(); // expires at 100 ms.
    registry.register(a.clone()).unwrap();
    let admitted = registry
        .begin_admission(&a.handle, MonotonicTime::from_millis(1))
        .unwrap();
    let watch = admitted.revocation_watch();
    assert!(!watch.is_revoked());
    assert!(matches!(
        registry.get_active(&a.handle, MonotonicTime::from_millis(100)),
        Err(SessionRegistryError::Expired)
    ));
    assert!(
        !watch.is_revoked(),
        "expiry alone does not flip the explicit-revocation latch"
    );
    let (tx, rx) = mpsc::sync_channel(1);
    thread::scope(|scope| {
        let closer = scope.spawn(|| tx.send(registry.revoke(&a.handle)).unwrap());
        observe_closed(&registry, &a.handle);
        assert!(watch.is_revoked());
        drop(admitted);
        assert!(rx.recv_timeout(Duration::from_secs(2)).unwrap().unwrap());
        closer.join().unwrap();
    });
}

#[test]
fn admitted_stop_watch_keeps_deadline_and_original_generation_after_renewal_and_reuse() {
    let registry = SessionRegistry::new();
    let a = grant(); // Captured expiry = 100.
    registry.register(a.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let first_permit = registry.begin_admission(&a.handle, now).unwrap();
    let original = first_permit.stop_watch();
    let historical = original.clone();
    assert!(!original.is_stop_required_at(MonotonicTime::from_millis(99)));
    assert!(original.is_stop_required_at(MonotonicTime::from_millis(100)));

    registry
        .renew_active(&a.handle, now, MonotonicTime::from_millis(200))
        .unwrap();
    let renewed_permit = registry
        .begin_admission(&a.handle, MonotonicTime::from_millis(101))
        .unwrap();
    let renewed = renewed_permit.stop_watch();
    assert!(
        original.is_stop_required_at(MonotonicTime::from_millis(101)),
        "renewal cannot extend an already-admitted operation"
    );
    assert!(!renewed.is_stop_required_at(MonotonicTime::from_millis(101)));
    assert!(renewed.is_stop_required_at(MonotonicTime::from_millis(200)));
    drop(first_permit);
    drop(renewed_permit);
    assert!(registry.revoke(&a.handle).unwrap());
    assert!(original.is_stop_required_at(now));
    assert!(renewed.is_stop_required_at(now));

    assert!(registry.remove_quiescent_inactive(&a.handle, now).unwrap());
    registry.register(a.clone()).unwrap(); // deliberately reused opaque handle
    let replacement_permit = registry.begin_admission(&a.handle, now).unwrap();
    let replacement = replacement_permit.stop_watch();
    assert!(!replacement.is_stop_required_at(now));
    assert!(historical.is_stop_required_at(now));
    assert!(renewed.is_stop_required_at(now));
    drop(replacement_permit);
    assert!(registry.revoke(&a.handle).unwrap());
    assert!(replacement.is_stop_required_at(now));
}

#[test]
fn real_worker_held_admission_observes_stop_before_registry_revoke_drains() {
    let registry = SessionRegistry::new();
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (drained_tx, drained_rx) = mpsc::sync_channel(1);
    let (closed_tx, closed_rx) = mpsc::sync_channel(1);

    thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let permit = registry.begin_admission(&a.handle, now).unwrap();
            let watch = permit.stop_watch();
            ready_tx.send(()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            while !watch.is_stop_required_at(now) {
                assert!(
                    Instant::now() < deadline,
                    "original-generation stop signal did not reach admitted worker"
                );
                thread::yield_now();
            }
            // This worker retains its genuine admission until it sees stop.
            drop(permit);
            drained_tx.send(()).unwrap();
        });
        ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let closer = scope.spawn(|| closed_tx.send(registry.revoke(&a.handle)).unwrap());
        drained_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("worker releases admission on the actual revoke signal");
        assert!(
            closed_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("synchronous registry revoke is unblocked")
                .expect("revoke result")
        );
        assert!(registry.get_active(&b.handle, now).is_ok());
        worker.join().unwrap();
        closer.join().unwrap();
    });
}

#[test]
fn owned_admission_retains_registry_and_immutable_session_deadline() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    registry.register(a.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let original = owned.stop_watch();
    assert!(!original.is_stop_required_at(MonotonicTime::from_millis(99)));
    registry
        .renew_active(&a.handle, now, MonotonicTime::from_millis(200))
        .unwrap();
    let later = registry
        .begin_owned_admission(&a.handle, MonotonicTime::from_millis(101))
        .unwrap();
    assert!(
        original.is_stop_required_at(MonotonicTime::from_millis(100)),
        "renewal must never extend the original admission deadline"
    );
    assert!(
        !later
            .stop_watch()
            .is_stop_required_at(MonotonicTime::from_millis(101))
    );
    drop(later);
    let weak = Arc::downgrade(&registry);
    drop(registry);
    assert!(
        weak.upgrade().is_some(),
        "the owned permit keeps the registry available for its own Drop"
    );
    drop(owned);
    assert!(
        weak.upgrade().is_none(),
        "owned admission releases the final Arc without leaking the registry"
    );
}

#[test]
fn owned_worker_stop_precedes_synchronous_revoke_drain_and_counts_with_borrowed_permit() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let borrowed = registry.begin_admission(&a.handle, now).unwrap();
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (observed_tx, observed_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let stop = owned.stop_watch();
        ready_tx.send(()).unwrap();
        let limit = Instant::now() + Duration::from_secs(3);
        while !stop.is_stop_required_at(now) {
            assert!(
                Instant::now() < limit,
                "owner did not see original-generation stop while admission held"
            );
            thread::yield_now();
        }
        observed_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        drop(owned);
    });
    ready_rx.recv_timeout(Duration::from_secs(3)).unwrap();

    let closing_registry = Arc::clone(&registry);
    let a_handle = a.handle.clone();
    let (closed_tx, closed_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        closed_tx.send(closing_registry.revoke(&a_handle)).unwrap();
    });
    observed_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("owner observes stop before synchronous revoke returns");
    assert!(
        matches!(closed_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "revocation must wait for counted owned and borrowed admissions"
    );
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(SessionRegistryError::AdmissionsInFlight)
    ));
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    assert!(
        matches!(closed_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "borrowed admission must independently block registry drain"
    );
    drop(borrowed);
    assert!(
        closed_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("revoke unblocks after both genuine admissions drain")
            .expect("revoke result")
    );
    closer.join().unwrap();

    assert!(registry.get_active(&b.handle, now).is_ok());
    assert!(registry.remove_quiescent_inactive(&a.handle, now).unwrap());
    registry.register(a.clone()).unwrap();
    let replacement = registry.begin_owned_admission(&a.handle, now).unwrap();
    assert!(!replacement.stop_watch().is_stop_required_at(now));
    drop(replacement);
    assert!(registry.revoke(&a.handle).unwrap());
}

#[test]
fn owned_admission_rejects_expired_revoked_and_unknown_sessions() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let missing = grant();
    registry.register(a.clone()).unwrap();
    assert!(matches!(
        registry.begin_owned_admission(&missing.handle, MonotonicTime::from_millis(1)),
        Err(SessionRegistryError::UnknownSession)
    ));
    assert!(matches!(
        registry.begin_owned_admission(&a.handle, MonotonicTime::from_millis(100)),
        Err(SessionRegistryError::Expired)
    ));
    assert!(registry.revoke(&a.handle).unwrap());
    assert!(matches!(
        registry.begin_owned_admission(&a.handle, MonotonicTime::from_millis(1)),
        Err(SessionRegistryError::Revoked)
    ));
    assert!(registry.remove_quiescent_inactive(&a.handle, MonotonicTime::from_millis(1)).unwrap());
}
