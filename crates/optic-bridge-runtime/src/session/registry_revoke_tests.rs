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
fn research_admitted_stop_latch_survives_close_and_late_capture() {
    let registry = SessionRegistry::new();
    let g = grant();
    registry.register(g.clone()).unwrap();
    let permit = registry
        .begin_admission(&g.handle, MonotonicTime::from_millis(1))
        .unwrap();
    let before = registry.research_admitted_stop_signal(&permit).unwrap();
    assert!(!before.load(Ordering::Acquire));
    thread::scope(|scope| {
        let worker = scope.spawn(|| registry.revoke(&g.handle));
        observe_closed(&registry, &g.handle);
        let late = registry.research_admitted_stop_signal(&permit).unwrap();
        assert!(Arc::ptr_eq(&before, &late) && late.load(Ordering::Acquire));
        assert!(matches!(
            registry.remove_quiescent_inactive(&g.handle, MonotonicTime::from_millis(1)),
            Err(SessionRegistryError::AdmissionsInFlight)
        ));
        drop(permit);
        assert!(worker.join().unwrap().unwrap());
    });
}

#[test]
fn research_admitted_stop_rejects_foreign_registry_even_same_handle() {
    let one = SessionRegistry::new();
    let two = SessionRegistry::new();
    let g = grant();
    one.register(g.clone()).unwrap();
    two.register(g.clone()).unwrap();
    let permit = one
        .begin_admission(&g.handle, MonotonicTime::from_millis(1))
        .unwrap();
    assert_eq!(
        two.research_admitted_stop_signal(&permit).unwrap_err(),
        SessionRegistryError::StateUnavailable
    );
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
