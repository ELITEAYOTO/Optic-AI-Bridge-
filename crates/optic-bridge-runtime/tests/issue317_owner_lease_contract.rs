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
    Reserved,
    NativeCreationInFlight,
    SuspendedOwned,
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
        // Before any native creation, bind the real admitted grant to its owner:
        // merely supplying SOME session permit is not enough.
        match (&authority, admission.as_ref()) {
            (Authority::Session(handle), Some(permit)) if &permit.grant().handle == handle => {}
            (Authority::HostStartup, None) => {}
            _ => return Err("admission owner mismatch"),
        }
        if matches!(&authority, Authority::HostStartup) && transport.is_some() {
            return Err("startup must not impersonate a transport request");
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("id exhausted")?;
        self.entries.push(Owner {
            id,
            authority,
            phase: Phase::Reserved,
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

    // Model a reserved slot becoming a native creation attempt before the OS
    // call. Once an attempt starts, the slot cannot be released as pre-spawn.
    fn begin_native_creation(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if owner.phase != Phase::Reserved {
            return Err("creation already started");
        }
        owner.phase = Phase::NativeCreationInFlight;
        Ok(())
    }

    fn attach_suspended_owned(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if owner.phase != Phase::NativeCreationInFlight {
            return Err("native creation not declared");
        }
        owner.phase = Phase::SuspendedOwned;
        Ok(())
    }

    fn mark_running(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if owner.phase != Phase::SuspendedOwned {
            return Err("suspended ownership not established");
        }
        owner.phase = Phase::RunningOwned;
        Ok(())
    }

    // Only safe in the model when native creation has NOT been attempted.
    fn release_before_creation(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if owner.phase != Phase::Reserved {
            return Err("native creation may have happened");
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or("unknown owner")?;
        self.entries.remove(index);
        Ok(())
    }

    // This path intentionally does NOT request TransportGuard capacity or wait
    // for SessionRegistry::revoke to drain its admissions.
    fn request_stop(&mut self, id: u64, actor: &Authority) -> Result<(), &'static str> {
        let owner = self.owner_mut(id, actor)?;
        if !matches!(
            owner.phase,
            Phase::NativeCreationInFlight | Phase::SuspendedOwned | Phase::RunningOwned
        ) {
            return Err("stop unavailable in this phase");
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

    owners
        .begin_native_creation(id, &authority)
        .expect("native creation attempt reserved");
    owners
        .attach_suspended_owned(id, &authority)
        .expect("native ownership attached");
    owners.mark_running(id, &authority).expect("running");
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
    owners
        .begin_native_creation(id, &first_actor)
        .expect("session creation started");
    owners
        .attach_suspended_owned(id, &first_actor)
        .expect("session native owner");
    owners
        .mark_running(id, &first_actor)
        .expect("session running");
    owners
        .begin_native_creation(startup_id, &Authority::HostStartup)
        .expect("host creation started");
    owners
        .attach_suspended_owned(startup_id, &Authority::HostStartup)
        .expect("host native owner");
    owners
        .mark_running(startup_id, &Authority::HostStartup)
        .expect("host running");

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

#[test]
fn admission_identity_and_interrupted_native_start_are_fail_closed() {
    let sessions = SessionRegistry::new();
    let first = grant();
    let second = grant();
    sessions.register(first.clone()).expect("first session");
    sessions.register(second.clone()).expect("second session");
    let now = MonotonicTime::from_millis(1);
    let first_actor = Authority::Session(first.handle.clone());
    let second_actor = Authority::Session(second.handle.clone());
    let mut owners = ContractRegistry::new(1);

    // A perfectly valid admission for A is not a valid reservation for B.
    let wrong_permit = sessions
        .begin_admission(&first.handle, now)
        .expect("first admission");
    assert!(
        owners
            .reserve(second_actor, Some(wrong_permit), None)
            .is_err(),
        "cannot borrow a different session's admission"
    );
    assert_eq!(owners.slots_held(), 0);

    // Before ANY native creation attempt, failed validation is allowed to
    // release the reservation and admission. This is not a native spawn result.
    let before = owners
        .reserve(
            first_actor.clone(),
            Some(
                sessions
                    .begin_admission(&first.handle, now)
                    .expect("pre-spawn admission"),
            ),
            None,
        )
        .expect("reserve before spawn");
    assert!(owners.release(before, &first_actor).is_err());
    owners
        .release_before_creation(before, &first_actor)
        .expect("nothing native was attempted");
    assert_eq!(owners.slots_held(), 0);

    let in_flight = owners
        .reserve(
            first_actor.clone(),
            Some(
                sessions
                    .begin_admission(&first.handle, now)
                    .expect("second admission"),
            ),
            None,
        )
        .expect("reserved");
    owners
        .begin_native_creation(in_flight, &first_actor)
        .expect("native creation intent");
    assert!(
        owners
            .release_before_creation(in_flight, &first_actor)
            .is_err(),
        "a possible partial native creation is NOT a pre-spawn failure"
    );
    assert!(owners.mark_running(in_flight, &first_actor).is_err());
    owners
        .attach_suspended_owned(in_flight, &first_actor)
        .expect("native owner attached");

    // Deterministic fault injection: setup failed BEFORE resume. The model
    // may signal stop but MUST retain the owner until cleanup proofs arrive.
    owners
        .request_stop(in_flight, &first_actor)
        .expect("stop suspended owner");
    owners
        .retain_uncertain(in_flight, &first_actor)
        .expect("setup failure uncertain");
    assert_eq!(owners.slots_held(), 1);
    assert!(!owners.may_seal(&first.handle));
    assert!(owners.release(in_flight, &first_actor).is_err());
    assert!(
        owners.reserve(Authority::HostStartup, None, None).is_err(),
        "uncertain native setup occupies finite capacity"
    );

    owners
        .prove_native_terminal(in_flight, &first_actor)
        .expect("injected native completion proof");
    assert!(owners.release(in_flight, &first_actor).is_err());
    owners
        .prove_io_joined(in_flight, &first_actor)
        .expect("injected I/O completion proof");
    owners
        .release(in_flight, &first_actor)
        .expect("only fully proven owner releases");
    assert!(owners.may_seal(&first.handle));
    assert_eq!(owners.slots_held(), 0);
    assert!(
        sessions
            .revoke(&first.handle)
            .expect("no stranded admission")
    );
    assert!(
        sessions
            .revoke(&second.handle)
            .expect("foreign owner untouched")
    );
}

#[test]
fn research_registry_drop_recycles_capacity_despite_uncertain_cleanup() {
    // NEGATIVE CHARACTERIZATION OF THIS TEST MODEL, NOT A PRODUCTION BUG REPRO:
    // the model owns RAII permits in a Vec. Ordinary Drop does not inspect
    // RetainedUncertain and therefore unconditionally releases admission and
    // transport capacity. No native subprocess exists in this test.
    let sessions = SessionRegistry::new();
    let session = grant();
    sessions
        .register(session.clone())
        .expect("register session");
    let authority = Authority::Session(session.handle.clone());
    let now = MonotonicTime::from_millis(1);
    let transport_guard = one_transport_slot();
    let mut owners = ContractRegistry::new(1);
    let id = owners
        .reserve(
            authority.clone(),
            Some(
                sessions
                    .begin_admission(&session.handle, now)
                    .expect("admit"),
            ),
            Some(transport_guard.begin_execution(now).expect("transport")),
        )
        .expect("reserve");
    owners
        .begin_native_creation(id, &authority)
        .expect("creation attempted");
    owners
        .request_stop(id, &authority)
        .expect("request stop during creation");
    owners
        .retain_uncertain(id, &authority)
        .expect("injected uncertain cleanup");
    assert_eq!(transport_guard.active_requests(), 1);
    assert_eq!(owners.slots_held(), 1);
    assert!(!owners.may_seal(&session.handle));
    assert!(owners.release(id, &authority).is_err());

    // Deliberately exercise the path outside the model's release methods.
    // A future production owner MUST NOT reuse capacity on this path while
    // native termination, E/S completion or mutation outcome is uncertain.
    drop(owners);
    assert_eq!(
        transport_guard.active_requests(),
        0,
        "model Drop bypasses retained-uncertain capacity"
    );
    assert!(
        sessions
            .revoke(&session.handle)
            .expect("model admission disappeared after registry drop"),
        "model Drop allows session drain without native cleanup evidence"
    );
}

#[test]
fn research_real_lifecycle_seal_after_test_owner_drop() {
    use optic_bridge_core::HardLimits;
    use optic_bridge_runtime::{ProcessManager, SessionLifecycleManager, TaskLeaseRegistry};

    // Real session lifecycle and real admission/transport permits, but NO Git
    // process and NO integration between LifecycleManager and this test model.
    // Show which guarantees disappear when the synthetic owner is dropped.
    let root = std::env::temp_dir().join(format!(
        "optic-issue317-seal-{}",
        SessionHandle::generate().expect("unique test root").to_token()
    ));
    std::fs::create_dir_all(&root).expect("create disposable root");

    let sessions = Arc::new(SessionRegistry::new());
    let session = grant();
    sessions
        .register(session.clone())
        .expect("register test session");
    let processes = Arc::new(
        ProcessManager::new(&root, HardLimits::default(), Vec::<String>::new())
            .expect("no-job process manager"),
    );
    let lifecycle = Arc::new(SessionLifecycleManager::new(
        Arc::clone(&sessions),
        Arc::new(TaskLeaseRegistry::new()),
        processes,
    ));
    let now = MonotonicTime::from_millis(1);
    let guard = one_transport_slot();
    let authority = Authority::Session(session.handle.clone());
    let mut owners = ContractRegistry::new(1);
    let id = owners
        .reserve(
            authority.clone(),
            Some(
                sessions
                    .begin_admission(&session.handle, now)
                    .expect("admit synthetic Git command"),
            ),
            Some(guard.begin_execution(now).expect("transport permit")),
        )
        .expect("owner reserved");
    owners
        .begin_native_creation(id, &authority)
        .expect("native attempt modeled");
    owners.request_stop(id, &authority).expect("stop modeled");
    owners
        .retain_uncertain(id, &authority)
        .expect("cleanup uncertain in model");
    assert!(!owners.may_seal(&session.handle));

    // The REAL lifecycle's seal calls SessionRegistry::revoke, which waits on
    // admission. Execute it on a separate OS thread so this test can release
    // the modeled owner without deadlocking the test's main thread.
    let (result_tx, result_rx) = mpsc::channel();
    let sealing = Arc::clone(&lifecycle);
    let sealed_session = session.handle.clone();
    let worker = thread::spawn(move || {
        let result = sealing
            .seal_quiescent(&sealed_session)
            .map(|seal| seal.session().clone());
        result_tx.send(result).expect("send seal result");
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    while !matches!(
        sessions.get_active(&session.handle, now),
        Err(SessionRegistryError::Revoked)
    ) && Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(1));
    }
    let revoked_before_release = matches!(
        sessions.get_active(&session.handle, now),
        Err(SessionRegistryError::Revoked)
    );
    let seal_waited = matches!(
        result_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    );
    assert_eq!(guard.active_requests(), 1);

    // This is intentionally NOT the guarded Owner::release: it is ordinary
    // destructor release without the synthetic native/pipe completion proofs.
    drop(owners);
    let after_drop = result_rx.recv_timeout(Duration::from_secs(5));
    let joined = worker.join();
    let reaped = lifecycle.try_reap(&session.handle, now);
    let remaining = sessions.len();
    std::fs::remove_dir_all(&root).expect("remove disposable root");

    assert!(revoked_before_release, "real revoke must signal promptly");
    assert!(seal_waited, "real seal must wait while admission is retained");
    assert!(joined.is_ok(), "seal worker must complete");
    assert_eq!(
        after_drop
            .expect("seal must complete after model owner Drop")
            .expect("actual lifecycle seal"),
        session.handle
    );
    assert!(
        reaped.expect("actual lifecycle reap").session_removed,
        "real lifecycle may reap once model-owned admission disappears"
    );
    assert_eq!(remaining.expect("session count"), 0);
    assert_eq!(guard.active_requests(), 0);
}
