//! #317 baseline: REAL registry fences before any proposed worker-lifetime API.
//! No synthetic native process, no altered runtime admission or revoke policy.
//! These tests cannot establish atomicity with Windows ResumeThread.
use super::*;
use optic_bridge_core::{Capability, PrincipalId, ProjectId};
use std::{
    collections::BTreeSet,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

fn grant(label: &str) -> SessionGrant {
    SessionGrant {
        handle: SessionHandle::generate().expect("unique test session"),
        principal: PrincipalId::new(label).expect("test principal"),
        project: ProjectId::new("contract").expect("test project"),
        capabilities: BTreeSet::from([Capability::GitRead]),
        expires_at: MonotonicTime::from_millis(10_000),
        policy_epoch: 1,
    }
}

fn await_revoked(registry: &SessionRegistry, handle: &SessionHandle, now: MonotonicTime) {
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        match registry.get_active(handle, now) {
            Err(SessionRegistryError::Revoked) => return,
            Ok(_) => {}
            other => panic!("unexpected session state during revoke: {other:?}"),
        }
        assert!(Instant::now() < until, "close admission deadline");
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn actual_registry_revocation_drains_all_permits_without_blocking_other_sessions() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant("contract-a");
    let b = grant("contract-b");
    registry.register(a.clone()).expect("register A");
    registry.register(b.clone()).expect("register B");
    let now = MonotonicTime::from_millis(1);
    let admission_one = registry
        .begin_admission(&a.handle, now)
        .expect("A admission 1");
    let admission_two = registry
        .begin_admission(&a.handle, now)
        .expect("A admission 2");
    #[cfg(windows)]
    let (stop_a, _) = registry.watch_active(&a.handle, now).expect("watch A");
    let revoking_a = Arc::clone(&registry);
    let a_handle = a.handle.clone();
    let (a_done_tx, a_done_rx) = mpsc::channel();
    let a_thread = thread::spawn(move || {
        let _ = a_done_tx.send(revoking_a.revoke(&a_handle));
    });

    await_revoked(&registry, &a.handle, now);
    #[cfg(windows)]
    assert!(
        stop_a.load(Ordering::Acquire),
        "real session stop published before permits drained"
    );
    assert!(matches!(
        registry.begin_admission(&a.handle, now),
        Err(SessionRegistryError::Revoked)
    ));
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(SessionRegistryError::AdmissionsInFlight)
    ));
    assert!(matches!(
        a_done_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));

    // A's unbounded admission drain must NOT hold the global registry lock.
    assert!(registry.get_active(&b.handle, now).is_ok());
    let admission_b = registry
        .begin_admission(&b.handle, now)
        .expect("B remains admissible");
    drop(admission_b);
    let revoking_b = Arc::clone(&registry);
    let b_handle = b.handle.clone();
    let (b_done_tx, b_done_rx) = mpsc::channel();
    let b_thread = thread::spawn(move || {
        let _ = b_done_tx.send(revoking_b.revoke(&b_handle));
    });
    assert!(
        b_done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("B revoke completes independently")
            .expect("B revoke state")
    );
    b_thread.join().expect("B thread");

    drop(admission_one);
    assert!(matches!(
        a_done_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(SessionRegistryError::AdmissionsInFlight)
    ));

    drop(admission_two);
    assert!(
        a_done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("A revoke completes after final permit")
            .expect("A revoke state")
    );
    a_thread.join().expect("A thread");
    assert!(
        !registry
            .revoke(&a.handle)
            .expect("idempotent second revoke")
    );
    assert!(
        registry
            .remove_quiescent_inactive(&a.handle, now)
            .expect("A capacity reclaim after drain")
    );
    assert!(matches!(
        registry.get_active(&a.handle, now),
        Err(SessionRegistryError::UnknownSession)
    ));
    assert!(matches!(
        registry.get_active(&b.handle, now),
        Err(SessionRegistryError::Revoked)
    ));
}

#[test]
fn real_registry_renewal_refuses_closed_session_while_admissions_still_drain() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant("contract-renewal-a");
    let b = grant("contract-renewal-b");
    registry.register(a.clone()).expect("register A");
    registry.register(b.clone()).expect("register B");
    let now = MonotonicTime::from_millis(1);
    let final_expiry = MonotonicTime::from_millis(10_001);
    let admission = registry
        .begin_admission(&a.handle, now)
        .expect("A admission");

    let revoking_a = Arc::clone(&registry);
    let a_handle = a.handle.clone();
    let (closed_tx, closed_rx) = mpsc::channel();
    let closing = thread::spawn(move || {
        let _ = closed_tx.send(revoking_a.revoke(&a_handle));
    });
    await_revoked(&registry, &a.handle, now);
    assert!(matches!(
        registry.validate_renewal(&a.handle, now, final_expiry),
        Err(SessionRenewalError::Revoked)
    ));
    assert!(matches!(
        registry.renew_active(&a.handle, now, final_expiry),
        Err(SessionRenewalError::Revoked)
    ));
    assert!(matches!(
        closed_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    // A's publication cannot poison an unrelated live session.
    assert_eq!(
        registry
            .renew_active(&b.handle, now, final_expiry)
            .expect("B renewal independent")
            .expires_at,
        final_expiry
    );
    drop(admission);
    assert!(
        closed_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("A release bounded")
            .expect("A revoke completion")
    );
    closing.join().expect("revoke thread");
}
