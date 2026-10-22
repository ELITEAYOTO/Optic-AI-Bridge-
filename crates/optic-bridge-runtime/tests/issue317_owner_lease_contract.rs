//! Research-only ownership contract for issue #317.
//!
//! This is an executable MODEL, not a Git process-owner implementation. It checks
//! the state/authority gates that a future native owner must satisfy while using
//! real SessionAdmissionPermit and RequestPermit values. It has no OS children,
//! native handles, pipe readers, CPU accounting, or production lifecycle wiring.

use std::{
    collections::BTreeSet,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

use optic_bridge_core::{
    Capability, MonotonicTime, PrincipalId, ProjectId, SessionGrant, SessionHandle,
};
use optic_bridge_runtime::{
    RequestPermit, SessionAdmissionPermit, SessionRegistry, SessionRegistryError, TransportError,
    TransportGuard, TransportLimits,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Authority {
    Session(SessionHandle),
    HostStartup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    RunningOwned,
    StopRequested,
    RetainedUncertain,
    NativeTreeProvenTerminal,
    PipeIoCancelledAndJoined,
}

struct Owner<'a> {
    id: u64,
    authority: Authority,
    phase: Phase,
    _admission: Option<SessionAdmissionPermit<'a>>,
    _transport: Option<RequestPermit>,
}

// The capacity reservation is the presence of an entry; neither uncertainty
// nor a dropped caller is permitted to evict that entry.
struct ContractRegistry<'a> {
    entries: Vec<Owner<'a>>,
    next_id: u64,
    max_slots: usize,
}

impl<'a> ContractRegistry<'a> {
    fn new(max_slots: usize) -> Self {
        assert!(max_slots > 0);
        Self {
            entries: Vec::new(),
            next_id: 0,
            max_slots,
        }
    }

    fn reserve(
        &mut self,
        authority: Authority,
        admission: Option<SessionAdmissionPermit<'a>>,
        transport: Option<RequestPermit>,
    ) -> Result<u64, &'static str> {
        if self.entries.len() >= self.max_slots {
            return Err("capacity held");
        }
        if matches!(authority, Authority::Session(_)) != admission.is_some() {
            return Err("session admission mismatch");
        }
        if matches!(authority, Authority::HostStartup) && transport.is_some() {
            return Err("startup must not impersonate a transport request");
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("id exhausted")?;
        self.entries.push(Owner {
            id,
            authority,
            phase: Phase::RunningOwned,
            _admission: admission,
            _transport: transport,
        });
        Ok(id)
    }

    fn owner_mut(&mut self, id: u64, actor: &Authority) -> Result<&mut Owner<'a>, &'static str> {
        let owner = self
            .entries
            .iter_mut()
            .find(|owner| owner.id == id)
            .ok_or("unknown owner")?;
        if &owner.authority != actor {
            return Err("foreign authority");
        }
        Ok(owner)
    }

    // This path intentionally does NOT request TransportGuard capacity or wait
    // for SessionRegistry::revoke to drain its admissions.
    fn request_stop(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if owner.phase != Phase::RunningOwned {
            return Err("stop already requested");
        }
        owner.phase = Phase::StopRequested;
        Ok(())
    }

    fn retain_uncertain(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if owner.phase != Phase::StopRequested {
            return Err("unexpected cleanup phase");
        }
        owner.phase = Phase::RetainedUncertain;
        Ok(())
    }

    fn prove_native_terminal(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if !matches!(owner.phase, Phase::StopRequested | Phase::RetainedUncertain) {
            return Err("native termination not requested");
        }
        owner.phase = Phase::NativeTreeProvenTerminal;
        Ok(())
    }

    fn prove_io_joined(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if owner.phase != Phase::NativeTreeProvenTerminal {
            return Err("native tree not proven terminal");
        }
        owner.phase = Phase::PipeIoCancelledAndJoined;
        Ok(())
    }

    fn release(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if owner.phase != Phase::PipeIoCancelledAndJoined {
            return Err("cleanup incomplete");
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or("unknown owner")?;
        self.entries.remove(index);
        Ok(())
    }

    fn may_seal(&self, session: &SessionHandle) -> bool {
        !self
            .entries
            .iter()
            .any(|entry| matches!(&entry.authority, Authority::Session(owner) if owner == session))
    }

    fn slots_held(&self) -> usize {
        self.entries.len()
    }
}

fn grant() -> SessionGrant {
    SessionGrant {
        handle: SessionHandle::generate().expect("test handle"),
        principal: PrincipalId::new("research-principal").expect("principal"),
        project: ProjectId::new("research-project").expect("project"),
        capabilities: BTreeSet::from([Capability::FileRead]),
        expires_at: MonotonicTime::from_millis(30_000),
        policy_epoch: 1,
    }
}

fn one_transport_slot() -> TransportGuard {
    TransportGuard::new(TransportLimits {
        max_request_bytes: 1024,
        max_response_bytes: 1024,
        max_request_duration_ms: 1000,
        max_concurrent_requests: 1,
    })
    .expect("transport")
}

#[test]
fn retained_uncertain_owner_blocks_admission_drain_seal_and_slot_reuse() {
    let sessions = Arc::new(SessionRegistry::new());
    let session = grant();
    sessions.register(session.clone()).expect("register");
    let authority = Authority::Session(session.handle.clone());
    let now = MonotonicTime::from_millis(1);
    let guard = one_transport_slot();
    let admission = sessions
        .begin_admission(&session.handle, now)
        .expect("admission");
    let transport = guard.begin_execution(now).expect("transport");
    let mut owners = ContractRegistry::new(1);
    let id = owners
        .reserve(authority.clone(), Some(admission), Some(transport))
        .expect("reserve before launch");

    assert_eq!(guard.active_requests(), 1);
    assert!(!owners.may_seal(&session.handle));
    assert!(matches!(
        guard.begin_execution(now),
        Err(TransportError::TooManyConcurrentRequests)
    ));

    let (done_tx, done_rx) = mpsc::channel();
    let revoke_sessions = Arc::clone(&sessions);
    let revoke_handle = session.handle.clone();
    let revoke_thread = thread::spawn(move || {
        let result = revoke_sessions.revoke(&revoke_handle);
        done_tx.send(result).expect("send revoke result");
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    while !matches!(
        sessions.get_active(&session.handle, now),
        Err(SessionRegistryError::Revoked)
    ) && Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(1));
    }
    assert!(matches!(
        sessions.get_active(&session.handle, now),
        Err(SessionRegistryError::Revoked)
    ));
    assert!(done_rx.try_recv().is_err(), "admission cannot drain yet");

    // Stop must remain reachable with the sole normal request slot occupied.
    owners
        .request_stop(id, &authority)
        .expect("owned stop signal");
    owners
        .retain_uncertain(id, &authority)
        .expect("injected uncertainty");
    assert_eq!(owners.slots_held(), 1);
    assert!(!owners.may_seal(&session.handle));
    assert!(owners.release(id, &authority).is_err());
    assert!(owners.reserve(Authority::HostStartup, None, None).is_err());
    assert!(done_rx.try_recv().is_err(), "uncertainty retains admission");
    assert_eq!(guard.active_requests(), 1);

    // Cleanup evidence has two independent required steps.
    owners
        .prove_native_terminal(id, &authority)
        .expect("native proof");
    assert!(owners.release(id, &authority).is_err(), "pipes still held");
    owners
        .prove_io_joined(id, &authority)
        .expect("joined proof");
    owners
        .release(id, &authority)
        .expect("release only after both");
    assert_eq!(owners.slots_held(), 0);
    assert!(owners.may_seal(&session.handle));
    assert_eq!(guard.active_requests(), 0);
    assert!(
        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("revoke bounded")
            .expect("revoke")
    );
    revoke_thread.join().expect("revoke join");
    assert!(
        guard.begin_execution(now).is_ok(),
        "slot returned after proof"
    );
}

#[test]
fn host_startup_and_foreign_session_cannot_spoof_each_other() {
    let sessions = SessionRegistry::new();
    let first = grant();
    let second = grant();
    sessions.register(first.clone()).expect("first");
    sessions.register(second.clone()).expect("second");
    let now = MonotonicTime::from_millis(1);
    let first_actor = Authority::Session(first.handle.clone());
    let foreign_actor = Authority::Session(second.handle.clone());
    let mut owners = ContractRegistry::new(2);
    let id = owners
        .reserve(
            first_actor.clone(),
            Some(sessions.begin_admission(&first.handle, now).expect("admit")),
            None,
        )
        .expect("session reserve");
    let startup_id = owners
        .reserve(Authority::HostStartup, None, None)
        .expect("startup separate reserve");
    assert!(owners.request_stop(id, &foreign_actor).is_err());
    assert!(owners.request_stop(startup_id, &first_actor).is_err());
    assert!(!owners.may_seal(&first.handle));
    assert!(owners.may_seal(&second.handle));
    assert_eq!(owners.slots_held(), 2);

    owners.request_stop(id, &first_actor).expect("first stop");
    owners
        .prove_native_terminal(id, &first_actor)
        .expect("terminal");
    owners.prove_io_joined(id, &first_actor).expect("io joined");
    owners.release(id, &first_actor).expect("first release");
    assert!(owners.may_seal(&first.handle));

    owners
        .request_stop(startup_id, &Authority::HostStartup)
        .expect("host stop");
    owners
        .retain_uncertain(startup_id, &Authority::HostStartup)
        .expect("host uncertain");
    assert_eq!(owners.slots_held(), 1, "host capacity remains reserved");
    assert!(owners.release(startup_id, &Authority::HostStartup).is_err());
    owners
        .prove_native_terminal(startup_id, &Authority::HostStartup)
        .expect("host terminal");
    owners
        .prove_io_joined(startup_id, &Authority::HostStartup)
        .expect("host join");
    owners
        .release(startup_id, &Authority::HostStartup)
        .expect("host released");
    assert_eq!(owners.slots_held(), 0);
}
