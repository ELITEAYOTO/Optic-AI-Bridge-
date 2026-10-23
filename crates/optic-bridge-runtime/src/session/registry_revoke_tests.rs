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
