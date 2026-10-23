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

/// Test-only exploration, not a new runtime API. Mutate the SAME real
/// registry record / stop signal as the current synchronous revoke, but
/// return before waiting for admissions to drain. Production code does not
/// call this helper or change its public semantics.
fn research_close_admission_without_drain(
    registry: &SessionRegistry,
    handle: &SessionHandle,
) -> Result<(bool, u32), SessionRegistryError> {
    let mut sessions = registry
        .sessions
        .lock()
        .map_err(|_| SessionRegistryError::StateUnavailable)?;
    let Some(record) = sessions.get_mut(handle) else {
        return Ok((false, 0));
    };
    let changed = !record.revoked;
    record.revoked = true;
    record.revocation_signal.store(true, Ordering::Release);
    Ok((changed, record.active_admissions))
}

/// This snapshot is ONLY admission drainage, not native Job/EOF quiescence.
fn research_pending_admissions(
    registry: &SessionRegistry,
    handle: &SessionHandle,
) -> Result<u32, SessionRegistryError> {
    let sessions = registry
        .sessions
        .lock()
        .map_err(|_| SessionRegistryError::StateUnavailable)?;
    let record = sessions
        .get(handle)
        .ok_or(SessionRegistryError::UnknownSession)?;
    Ok(record.active_admissions)
}

#[test]
fn prototype_two_stage_close_is_immediate_but_never_reports_early_quiescence() {
    let registry = SessionRegistry::new();
    let a = grant("two-stage-a");
    let b = grant("two-stage-b");
    registry.register(a.clone()).expect("A active");
    registry.register(b.clone()).expect("B active");
    let now = MonotonicTime::from_millis(1);
    let permit_one = registry
        .begin_admission(&a.handle, now)
        .expect("first A admission");
    let permit_two = registry
        .begin_admission(&a.handle, now)
        .expect("second A admission");
    #[cfg(windows)]
    let (signal, _) = registry
        .watch_active(&a.handle, now)
        .expect("A stop signal");

    // Closure is a short registry-mutex operation, not a wait for an
    // uncertain native owner to finish. '2' is pending, NOT terminal.
    assert_eq!(
        research_close_admission_without_drain(&registry, &a.handle)
            .expect("test-only nonblocking close"),
        (true, 2)
    );
    #[cfg(windows)]
    assert!(signal.load(Ordering::Acquire));
    assert!(matches!(
        registry.get_active(&a.handle, now),
        Err(SessionRegistryError::Revoked)
    ));
    assert!(matches!(
        registry.begin_admission(&a.handle, now),
        Err(SessionRegistryError::Revoked)
    ));
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(SessionRegistryError::AdmissionsInFlight)
    ));
    assert_eq!(
        research_pending_admissions(&registry, &a.handle).unwrap(),
        2
    );

    // A second close is idempotent, and unrelated sessions keep moving.
    assert_eq!(
        research_close_admission_without_drain(&registry, &a.handle).unwrap(),
        (false, 2)
    );
    assert!(registry.begin_admission(&b.handle, now).is_ok());
    drop(permit_one);
    assert_eq!(
        research_pending_admissions(&registry, &a.handle).unwrap(),
        1
    );
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(SessionRegistryError::AdmissionsInFlight)
    ));
    drop(permit_two);
    assert_eq!(
        research_pending_admissions(&registry, &a.handle).unwrap(),
        0
    );
    assert!(
        !registry
            .revoke(&a.handle)
            .expect("existing revoke idempotence")
    );
    assert!(
        registry
            .remove_quiescent_inactive(&a.handle, now)
            .expect("reclaim only after drainage")
    );
    assert!(registry.get_active(&b.handle, now).is_ok());
}

#[test]
fn external_wrapper_gate_cannot_intercept_direct_registry_revoke() {
    // A Mutex owned only by the caller is not a security boundary: a
    // direct public revoke does not acquire it. This is a deliberate
    // negative design proof with the ACTUAL production registry.
    let registry = Arc::new(SessionRegistry::new());
    let a = grant("wrapper-bypass");
    registry.register(a.clone()).expect("register");
    let now = MonotonicTime::from_millis(1);
    let permit = registry.begin_admission(&a.handle, now).expect("admit");
    let external_launch_gate = std::sync::Mutex::new(());
    let held_by_launcher = external_launch_gate.lock().expect("test gate");

    let closing = Arc::clone(&registry);
    let handle = a.handle.clone();
    let (tx, rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        tx.send(closing.revoke(&handle)).expect("report revoke");
    });

    // Despite the test gate being held by a launch, direct revoke
    // changes the real session state before the permit is dropped.
    await_revoked(&registry, &a.handle, now);
    assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert!(matches!(
        registry.begin_admission(&a.handle, now),
        Err(SessionRegistryError::Revoked)
    ));
    drop(held_by_launcher);
    drop(permit);
    assert!(
        rx.recv_timeout(Duration::from_secs(2))
            .expect("bounded test revoke completion")
            .expect("real revoke succeeded")
    );
    worker.join().expect("revoke thread");
}
