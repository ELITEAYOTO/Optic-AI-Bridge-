//! #317 RESEARCH ONLY: authority-owned per-session launch/close gate.
//!
//! These tests run against the TEST-INSTRUMENTED real SessionRegistry, not
//! production semantics. No OS process is spawned or native resume proven.
//! In particular, this module is NOT evidence for a production implementation.
use super::*;
use optic_bridge_core::{Capability, PrincipalId, ProjectId};
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

fn grant(principal: &str) -> SessionGrant {
    SessionGrant {
        handle: SessionHandle::generate().expect("synthetic unique handle"),
        principal: PrincipalId::new(principal).expect("synthetic principal"),
        project: ProjectId::new("research-gate").expect("synthetic project"),
        capabilities: BTreeSet::from([Capability::GitRead]),
        expires_at: MonotonicTime::from_millis(10_000),
        policy_epoch: 1,
    }
}

fn wait_closed(registry: &SessionRegistry, handle: &SessionHandle, now: MonotonicTime) {
    let limit = Instant::now() + Duration::from_secs(2);
    while !matches!(
        registry.get_active(handle, now),
        Err(SessionRegistryError::Revoked)
    ) {
        assert!(Instant::now() < limit, "session close publication timeout");
        thread::yield_now();
    }
}

#[test]
fn close_wins_and_no_effect_runs_after_real_record_is_revoked() {
    let registry = SessionRegistry::new();
    let a = grant("close-wins");
    registry.register(a.clone()).expect("register");
    let now = MonotonicTime::from_millis(1);
    let count = AtomicUsize::new(0);
    assert_eq!(
        registry.research_close_admission(&a.handle).expect("prompt close"),
        (true, 0)
    );
    assert!(matches!(
        registry.research_commit_launch(&a.handle, now, || {
            count.fetch_add(1, Ordering::SeqCst)
        }),
        Err(SessionRegistryError::Revoked)
    ));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(
        registry.research_close_admission(&a.handle).expect("idempotent close"),
        (false, 0)
    );
    assert!(!registry.revoke(&a.handle).expect("idempotent direct revoke"));
}

#[test]
fn direct_public_revoke_uses_authority_gate_not_private_launcher_wrapper() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant("direct-revoke-a");
    let b = grant("direct-revoke-b");
    registry.register(a.clone()).expect("A registered");
    registry.register(b.clone()).expect("B registered");
    let now = MonotonicTime::from_millis(1);
    // Hold one real permit to verify that the gate is released BEFORE
    // revoke waits for the synchronous admissions-drained condvar.
    let permit = registry.begin_admission(&a.handle, now).expect("admission");

    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (continue_tx, continue_rx) = mpsc::sync_channel(1);
    let (closed_tx, closed_rx) = mpsc::sync_channel(1);
    let commit_registry = Arc::clone(&registry);
    let a_commit = a.handle.clone();
    let launch = thread::spawn(move || {
        commit_registry.research_commit_launch(&a_commit, now, || {
            entered_tx.send(()).expect("entered commit");
            continue_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("release commit");
        })
    });
    entered_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("launcher entered shared authority gate");
    let closer_registry = Arc::clone(&registry);
    let a_close = a.handle.clone();
    let (attempt_tx, attempt_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        attempt_tx.send(()).expect("closer attempts direct revoke");
        closed_tx
            .send(closer_registry.revoke(&a_close))
            .expect("send revoke result");
    });
    attempt_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("direct revoke scheduled");

    // No global registry mutex is held across the launch closure:
    // another session commits while A's launch is waiting.
    let unrelated_commits = AtomicUsize::new(0);
    registry
        .research_commit_launch(&b.handle, now, || {
            unrelated_commits.fetch_add(1, Ordering::SeqCst);
        })
        .expect("B may commit independently");
    assert_eq!(unrelated_commits.load(Ordering::SeqCst), 1);
    assert!(registry.get_active(&a.handle, now).is_ok());
    assert!(matches!(closed_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));

    continue_tx.send(()).expect("release finite launch closure");
    launch.join().expect("launch thread").expect("A launch committed");
    wait_closed(&registry, &a.handle, now);
    // The closer must have dropped the gate after publishing close, while
    // still waiting for the genuine admission permit to drain.
    assert!(matches!(closed_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert!(matches!(
        registry.research_commit_launch(&a.handle, now, || panic!("forbidden effect")),
        Err(SessionRegistryError::Revoked)
    ));
    drop(permit);
    assert!(
        closed_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("direct revoke drained")
            .expect("direct revoke succeeds")
    );
    closer.join().expect("closer thread");
    assert!(registry.get_active(&b.handle, now).is_ok());
}

#[test]
fn rejected_commit_does_not_execute_on_expiry_or_unknown_handle() {
    let registry = SessionRegistry::new();
    let a = grant("expiry");
    registry.register(a.clone()).expect("register");
    let executed = AtomicUsize::new(0);
    assert!(matches!(
        registry.research_commit_launch(
            &a.handle,
            MonotonicTime::from_millis(10_000),
            || executed.fetch_add(1, Ordering::SeqCst)
        ),
        Err(SessionRegistryError::Expired)
    ));
    let missing = SessionHandle::generate().expect("another handle");
    assert!(matches!(
        registry.research_commit_launch(
            &missing,
            MonotonicTime::from_millis(1),
            || executed.fetch_add(1, Ordering::SeqCst)
        ),
        Err(SessionRegistryError::UnknownSession)
    ));
    assert_eq!(executed.load(Ordering::SeqCst), 0);
}
