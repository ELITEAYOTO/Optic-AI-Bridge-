//! #317 research-only: session-owned native GitRead resource reservations.
//! No GitRead production caller yet. An owner must be adopted BEFORE native spawn.
//! This ledger is not a grant to execute commands or release uncertain children.
#![allow(dead_code)]

use crate::{OwnedSessionAdmissionPermit, SessionRegistry, SessionRegistryError};
use optic_bridge_core::{MonotonicTime, SessionHandle};
use optic_bridge_windows::{PreparedReadCommand, ReadCommandOutcome};
use std::{
    collections::BTreeMap,
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use thiserror::Error;

#[derive(Clone, Copy)]
struct Budget {
    memory: u64,
    output: u64,
    cpu: u32,
}

#[derive(Clone, Copy)]
struct Ceiling {
    count: usize,
    memory: u64,
    output: u64,
    cpu: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct OwnerId(u64);

#[derive(Debug, Error)]
enum OwnerError {
    #[error("owner reservation capacity exceeded")]
    Capacity,
    #[error("owner registry state unavailable")]
    Unavailable,
    #[error("owner identity exhausted")]
    IdExhausted,
    #[error("native owner absent or wrong session")]
    WrongOwner,
    #[error("native owner already adopted")]
    AlreadyAdopted,
    #[error("native owner payload missing")]
    MissingPayload,
    #[error("native owner observation in progress")]
    ObservationBusy,
    #[error("prepared native owner does not match session stop signals or was already started")]
    UnboundNativeOwner,
    #[error("session was revoked before owner adoption")]
    Revoked,
    #[error(transparent)]
    Session(#[from] SessionRegistryError),
    #[error(transparent)]
    Native(#[from] io::Error),
}

struct Record {
    session: SessionHandle,
    // The counted admission is owned by the service, never the request thread.
    // Dropping it while the Job is uncertain would let revoke drain too early.
    admission: OwnedSessionAdmissionPermit,
    budget: Budget,
    stop: Arc<AtomicBool>,
    revoked: Arc<AtomicBool>,
    native: Option<Arc<Mutex<PreparedReadCommand>>>,
    observing: Arc<AtomicBool>,
}

struct State {
    next: u64,
    memory: u64,
    output: u64,
    cpu: u32,
    records: BTreeMap<OwnerId, Record>,
}

struct Observation(Arc<AtomicBool>);

impl Drop for Observation {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

struct NativeGitOwners {
    ceiling: Ceiling,
    state: Mutex<State>,
}

impl NativeGitOwners {
    fn new(ceiling: Ceiling) -> Result<Self, OwnerError> {
        if ceiling.count == 0
            || ceiling.memory == 0
            || ceiling.output == 0
            || !(1..=100).contains(&ceiling.cpu)
        {
            return Err(OwnerError::Capacity);
        }
        Ok(Self {
            ceiling,
            state: Mutex::new(State {
                next: 0,
                memory: 0,
                output: 0,
                cpu: 0,
                records: BTreeMap::new(),
            }),
        })
    }

    // The admission and all three ceilings are committed BEFORE a process can
    // exist. GitRead policy/task leases and executable pins remain separate
    // mandatory checks for a future production adapter.
    fn reserve(
        &self,
        sessions: &Arc<SessionRegistry>,
        session: &SessionHandle,
        now: MonotonicTime,
        budget: Budget,
    ) -> Result<OwnerId, OwnerError> {
        let admission = sessions.begin_owned_admission(session, now)?;
        let (revoked, current) = sessions.watch_active(session, now)?;
        let grant = admission.grant();
        if current.principal != grant.principal
            || current.project != grant.project
            || current.policy_epoch != grant.policy_epoch
        {
            return Err(OwnerError::Revoked);
        }
        if budget.memory == 0 || budget.output == 0 || !(1..=100).contains(&budget.cpu) {
            return Err(OwnerError::Capacity);
        }
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        if revoked.load(Ordering::Acquire) {
            return Err(OwnerError::Revoked);
        }
        let memory = state
            .memory
            .checked_add(budget.memory)
            .ok_or(OwnerError::Capacity)?;
        let output = state
            .output
            .checked_add(budget.output)
            .ok_or(OwnerError::Capacity)?;
        let cpu = state
            .cpu
            .checked_add(budget.cpu)
            .ok_or(OwnerError::Capacity)?;
        if state.records.len() >= self.ceiling.count
            || memory > self.ceiling.memory
            || output > self.ceiling.output
            || cpu > self.ceiling.cpu
        {
            return Err(OwnerError::Capacity);
        }
        let id = OwnerId(state.next.checked_add(1).ok_or(OwnerError::IdExhausted)?);
        state.next = id.0;
        state.memory = memory;
        state.output = output;
        state.cpu = cpu;
        state.records.insert(
            id,
            Record {
                session: session.clone(),
                admission,
                budget,
                stop: Arc::new(AtomicBool::new(false)),
                revoked,
                native: None,
                observing: Arc::new(AtomicBool::new(false)),
            },
        );
        Ok(id)
    }

    // Two independent out-of-band signals for the native watchdog.
    // Neither signal implies a successful native termination.
    fn stop_signals(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<Vec<Arc<AtomicBool>>, OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.session != session {
            return Err(OwnerError::WrongOwner);
        }
        Ok(vec![Arc::clone(&record.stop), Arc::clone(&record.revoked)])
    }

    fn adopt(
        &self,
        id: OwnerId,
        session: &SessionHandle,
        owner: &mut Option<PreparedReadCommand>,
    ) -> Result<(), OwnerError> {
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get_mut(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.session != session {
            return Err(OwnerError::WrongOwner);
        }
        if record.native.is_some() {
            return Err(OwnerError::AlreadyAdopted);
        }
        // Adopting an *unstarted* native owner is cleanup ownership, NOT
        // authorization to launch. Even after revoke, it must be possible to
        // retain and prove the no-spawn Job/EOF receipt before releasing the
        // counted service admission. Accept ONLY the exact pre-reserved stop
        // atomics so a substituted owner cannot ignore revocation.
        let native = owner.as_ref().ok_or(OwnerError::MissingPayload)?;
        if !native.matches_unstarted_owner_signals(&record.stop, &record.revoked) {
            return Err(OwnerError::UnboundNativeOwner);
        }
        if record.revoked.load(Ordering::Acquire) {
            record.stop.store(true, Ordering::Release);
        }
        // Check all fallible conditions before taking the caller-owned value.
        let native = owner.take().expect("prepared owner already validated");
        record.native = Some(Arc::new(Mutex::new(native)));
        Ok(())
    }

    fn stop_session(&self, session: &SessionHandle) -> Result<usize, OwnerError> {
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let mut count = 0;
        for record in state.records.values_mut() {
            if &record.session == session && !record.stop.swap(true, Ordering::AcqRel) {
                count += 1;
            }
        }
        Ok(count)
    }

    fn outstanding(&self, session: &SessionHandle) -> Result<usize, OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        Ok(state
            .records
            .values()
            .filter(|r| &r.session == session)
            .count())
    }

    // Does not hold the global budget mutex during potentially blocking native
    // operations; an external cancellation can still set the stop signal.
    fn with_owner<R>(
        &self,
        id: OwnerId,
        session: &SessionHandle,
        visit: impl FnOnce(&mut PreparedReadCommand) -> io::Result<R>,
    ) -> Result<R, OwnerError> {
        let (owner, _observation) = self.claim_owner(id, session)?;
        let mut native = owner.lock().map_err(|_| OwnerError::Unavailable)?;
        Ok(visit(&mut native)?)
    }

    // Both native start and terminal proof share ONE per-owner observer gate.
    // Neither operation keeps the global ledger locked while using the OS.
    fn claim_owner(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<(Arc<Mutex<PreparedReadCommand>>, Observation), OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.session != session {
            return Err(OwnerError::WrongOwner);
        }
        let owner = record.native.as_ref().ok_or(OwnerError::MissingPayload)?;
        if record
            .observing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(OwnerError::ObservationBusy);
        }
        Ok((
            Arc::clone(owner),
            Observation(Arc::clone(&record.observing)),
        ))
    }

    // No caller-supplied success boolean: the only release path requires the
    // staged native owner's own root+Job-empty+both-EOF terminal receipt and a
    // successfully claimed outcome. Any native error retains reservations.
    fn reap_terminal(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<Option<ReadCommandOutcome>, OwnerError> {
        let (owner, _observation) = self.claim_owner(id, session)?;
        let outcome = {
            let mut native = owner.lock().map_err(|_| OwnerError::Unavailable)?;
            if !native.poll_terminal()? {
                return Ok(None);
            }
            native.take_outcome()?
        };
        // Only an identical proven native payload may release its admission.
        let released = {
            let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
            let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
            if &record.session != session
                || !record
                    .native
                    .as_ref()
                    .is_some_and(|p| Arc::ptr_eq(p, &owner))
            {
                return Err(OwnerError::WrongOwner);
            }
            let record = state.records.remove(&id).expect("proven record exists");
            state.memory -= record.budget.memory;
            state.output -= record.budget.output;
            state.cpu -= record.budget.cpu;
            record
        };
        // Releasing the counted session admission follows the kernel receipt,
        // and occurs OUTSIDE the resource ledger mutex.
        drop(released);
        Ok(Some(outcome))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_bridge_core::{Capability, PrincipalId, ProjectId, SessionGrant};
    use optic_bridge_windows::LimitedJobObject;
    use std::{
        collections::BTreeSet,
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    fn grant(name: &str) -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().expect("session entropy"),
            principal: PrincipalId::new(name).expect("principal"),
            project: ProjectId::new("native-ledger-test").expect("project"),
            capabilities: BTreeSet::from([Capability::FileRead]),
            expires_at: MonotonicTime::from_millis(20000),
            policy_epoch: 1,
        }
    }

    fn budget() -> Budget {
        Budget {
            memory: 512 * 1024 * 1024,
            output: 16384,
            cpu: 25,
        }
    }

    fn ledger() -> NativeGitOwners {
        let b = budget();
        NativeGitOwners::new(Ceiling {
            count: 1,
            memory: b.memory,
            output: b.output,
            cpu: b.cpu,
        })
        .expect("limits")
    }

    #[test]
    fn revoked_before_adoption_is_owned_and_proven_no_spawn_before_releasing_admission() {
        use std::process::Command;

        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("revoked-before-adoption");
        sessions.register(grant.clone()).expect("register");
        let owners = ledger();
        let id = owners
            .reserve(
                &sessions,
                &grant.handle,
                MonotonicTime::from_millis(1),
                budget(),
            )
            .expect("reserve counted admission before native preparation");
        let signals = owners
            .stop_signals(id, &grant.handle)
            .expect("stop/revoke watch");
        let mut prepared = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("job"),
                budget().output as usize,
                3000,
                signals.clone(),
            )
            .expect("prepare native job and streams without launching"),
        );

        let sessions_for_revoke = Arc::clone(&sessions);
        let handle = grant.handle.clone();
        let (tx, rx) = mpsc::channel();
        let revoker = thread::spawn(move || {
            tx.send(sessions_for_revoke.revoke(&handle))
                .expect("send revoke result");
        });
        let started = Instant::now();
        while !signals[1].load(Ordering::Acquire) {
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "revoke bit never published"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let held_during_revoke = owners.outstanding(&grant.handle).expect("reservation");
        let drain_pending = matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty));
        let not_adopted = matches!(
            owners.reap_terminal(id, &grant.handle),
            Err(OwnerError::MissingPayload)
        );
        // Revoke does NOT grant permission to spawn, but must not strand the
        // counted reservation with no OS owner capable of proving no-spawn.
        owners
            .adopt(id, &grant.handle, &mut prepared)
            .expect("adopt stopped preparation solely for native cleanup");
        let transferred = prepared.is_none();
        let mut command = Command::new(std::env::current_exe().expect("test binary"));
        let launch_error = owners
            .with_owner(id, &grant.handle, |native| native.start(&mut command))
            .expect_err("revoked native process MUST NOT be spawned");
        let interrupted = matches!(
            launch_error,
            OwnerError::Native(ref e) if e.kind() == io::ErrorKind::Interrupted
        );
        let job_empty = owners
            .with_owner(id, &grant.handle, |native| native.native_active_processes())
            .expect("native Job accounting");
        let receipt = owners
            .reap_terminal(id, &grant.handle)
            .expect("full Job and EOF no-spawn proof")
            .expect("terminal native evidence");
        let capacity_released = owners
            .outstanding(&grant.handle)
            .expect("released reservation");
        let revoke_result = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("revoke must drain after native proof")
            .expect("revoke result");
        revoker.join().expect("revoke thread");

        assert_eq!(held_during_revoke, 1);
        assert!(drain_pending && not_adopted && transferred && interrupted);
        assert_eq!(job_empty, 0);
        assert!(
            receipt.status.is_none(),
            "no native root can have been created"
        );
        assert!(receipt.cancelled && !receipt.is_success());
        assert!(receipt.stdout.is_empty() && receipt.stderr.is_empty());
        assert_eq!(capacity_released, 0);
        assert!(revoke_result);
    }

    #[test]
    fn mismatched_native_stop_signals_cannot_be_adopted_or_release_capacity() {
        use std::process::Command;

        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("native-signals-bound-to-session");
        sessions.register(grant.clone()).expect("register");
        let owners = ledger();
        let id = owners
            .reserve(
                &sessions,
                &grant.handle,
                MonotonicTime::from_millis(1),
                budget(),
            )
            .expect("reserve");
        let signals = owners
            .stop_signals(id, &grant.handle)
            .expect("owner signals");
        let unrelated = vec![
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ];
        let mut foreign = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("foreign job"),
                budget().output as usize,
                3000,
                unrelated,
            )
            .expect("prepare unrelated native owner"),
        );
        let rejected = matches!(
            owners.adopt(id, &grant.handle, &mut foreign),
            Err(OwnerError::UnboundNativeOwner)
        );
        let caller_kept_foreign = foreign.is_some();
        let capacity_still_held = owners.outstanding(&grant.handle).expect("reservation");
        let no_owner_adopted = matches!(
            owners.reap_terminal(id, &grant.handle),
            Err(OwnerError::MissingPayload)
        );
        // Complete this valid no-spawn case using the actual bound atomics.
        let mut correctly_bound = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("job"),
                budget().output as usize,
                3000,
                signals,
            )
            .expect("prepare actual session owner"),
        );
        owners
            .adopt(id, &grant.handle, &mut correctly_bound)
            .expect("adopt only exact stop/revoke identity");
        let stopped = owners
            .stop_session(&grant.handle)
            .expect("session-scoped stop");
        let mut command = Command::new(std::env::current_exe().expect("test binary"));
        let start_rejected = matches!(
            owners.with_owner(id, &grant.handle, |native| native.start(&mut command)),
            Err(OwnerError::Native(ref e)) if e.kind() == io::ErrorKind::Interrupted
        );
        let receipt = owners
            .reap_terminal(id, &grant.handle)
            .expect("prove no native child and EOF")
            .expect("terminal receipt");
        let released = owners
            .outstanding(&grant.handle)
            .expect("capacity released");

        assert!(rejected && caller_kept_foreign && no_owner_adopted);
        assert_eq!(capacity_still_held, 1);
        assert_eq!(stopped, 1);
        assert!(start_rejected);
        assert!(receipt.cancelled && receipt.status.is_none());
        assert!(!receipt.is_success());
        assert_eq!(released, 0);
    }

    #[test]
    fn reserved_owner_retains_admission_until_native_quiescence() {
        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("owner-a");
        sessions.register(grant.clone()).expect("register");
        let now = MonotonicTime::from_millis(1);
        let owners = ledger();
        let id = owners
            .reserve(&sessions, &grant.handle, now, budget())
            .expect("reserve before spawn");
        let signals = owners
            .stop_signals(id, &grant.handle)
            .expect("native watchdog signals");
        let mut native = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("job"),
                budget().output as usize,
                3000,
                signals,
            )
            .expect("native preparation before child creation"),
        );
        owners
            .adopt(id, &grant.handle, &mut native)
            .expect("adopt BEFORE spawn");
        assert!(native.is_none());

        let sessions_for_revoke = Arc::clone(&sessions);
        let handle_for_revoke = grant.handle.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let revoker = thread::spawn(move || {
            done_tx
                .send(sessions_for_revoke.revoke(&handle_for_revoke))
                .expect("send revoke");
        });
        let start = Instant::now();
        while !owners.stop_signals(id, &grant.handle).expect("signals")[1].load(Ordering::Acquire) {
            assert!(
                start.elapsed() < Duration::from_secs(2),
                "revocation signal absent"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert_eq!(owners.outstanding(&grant.handle).expect("reserved"), 1);

        let result = owners
            .reap_terminal(id, &grant.handle)
            .expect("native observation");
        let receipt = result.expect("no-spawn revoked Job and EOF must be proven");
        assert!(receipt.cancelled && receipt.status.is_none());
        assert!(!receipt.is_success());
        assert_eq!(owners.outstanding(&grant.handle).expect("released"), 0);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("drained session")
                .expect("revoke")
        );
        revoker.join().expect("join revoke");
    }

    // Spawned only inside the synthetic, service-owned Windows Job. This is
    // not a GitRead or arbitrary executable path, and it performs no I/O to
    // user projects, network, secrets or external process IDs.
    #[test]
    fn owned_native_running_child_fixture() {
        if std::env::var_os("OPTIC317_LEDGER_CHILD").is_none() {
            return;
        }
        use std::io::Write;
        println!("OPTIC317_LEDGER_CHILD_ACTUALLY_RUNNING");
        std::io::stdout().flush().expect("synthetic child flush");
        let path = std::path::PathBuf::from(
            std::env::var_os("OPTIC317_LEDGER_STARTED").expect("synthetic startup marker"),
        );
        std::fs::write(path, b"alive before revoke").expect("mark synthetic child running");
        thread::sleep(Duration::from_secs(5));
    }

    #[test]
    fn live_native_owner_survives_revoke_until_job_empty_and_pipe_eof() {
        use std::{env, process::Command};

        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("live-owned-process");
        sessions
            .register(grant.clone())
            .expect("register synthetic session");
        let now = MonotonicTime::from_millis(1);
        let owners = ledger();
        let id = owners
            .reserve(&sessions, &grant.handle, now, budget())
            .expect("reserve counted owner before native creation");
        let signals = owners
            .stop_signals(id, &grant.handle)
            .expect("obtain independent stop signals");

        let mut native = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("native limits"),
                budget().output as usize,
                8_000,
                signals,
            )
            .expect("allocate native Job and pipes before child creation"),
        );
        owners
            .adopt(id, &grant.handle, &mut native)
            .expect("adopt prepared native resources before spawn");
        assert!(native.is_none());

        let gate = env::temp_dir().join(format!("optic317-service-ledger-{}", std::process::id()));
        let _ = std::fs::remove_file(&gate);
        let mut command = Command::new(env::current_exe().expect("synthetic test executable"));
        command
            .args([
                "--exact",
                "git_owner_registry::tests::owned_native_running_child_fixture",
                "--nocapture",
            ])
            .env_clear()
            .env("OPTIC317_LEDGER_CHILD", "1")
            .env("OPTIC317_LEDGER_STARTED", &gate);
        if let Some(system_root) = env::var_os("SystemRoot") {
            command.env("SystemRoot", system_root);
        }
        owners
            .with_owner(id, &grant.handle, |owner| owner.start(&mut command))
            .expect("registered owner for native spawn");

        let started = Instant::now();
        while !gate.is_file() {
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "owned synthetic child did not confirm its live execution"
            );
            thread::sleep(Duration::from_millis(5));
        }

        let active_before_revoke = owners
            .with_owner(id, &grant.handle, |owner| owner.native_active_processes())
            .expect("native Job accounting before revoke");
        let pending_before_revoke = owners
            .reap_terminal(id, &grant.handle)
            .expect("native owner still running")
            .is_none();
        let reservation_before_revoke = owners
            .outstanding(&grant.handle)
            .expect("counted budget retained");

        let (tx, rx) = mpsc::channel();
        let revoked_sessions = Arc::clone(&sessions);
        let revoked_handle = grant.handle.clone();
        let revoker = thread::spawn(move || {
            tx.send(revoked_sessions.revoke(&revoked_handle))
                .expect("send completed revoke");
        });

        let observing = Instant::now();
        while !owners
            .stop_signals(id, &grant.handle)
            .expect("stop signals")[1]
            .load(Ordering::Acquire)
        {
            assert!(
                observing.elapsed() < Duration::from_secs(2),
                "native owner did not see session revoke"
            );
            thread::sleep(Duration::from_millis(5));
        }

        let admission_still_blocks_revoke = matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty));
        let reservation_during_revoke = owners
            .outstanding(&grant.handle)
            .expect("hold reservation until native receipt");

        let stopping = Instant::now();
        let outcome = loop {
            if let Some(receipt) = owners
                .reap_terminal(id, &grant.handle)
                .expect("native terminal observation")
            {
                break receipt;
            }
            assert!(
                stopping.elapsed() < Duration::from_secs(3),
                "owned native termination and EOF not confirmed in finite time"
            );
            thread::sleep(Duration::from_millis(5));
        };

        let reservation_after_proof = owners
            .outstanding(&grant.handle)
            .expect("native confirmed release");
        let revoke_result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("session revoke must complete after native receipt")
            .expect("session registry revoke");
        revoker.join().expect("join revoke worker");
        let _ = std::fs::remove_file(&gate);

        // Assert collected evidence after the revoker has been joined.
        assert_eq!(active_before_revoke, 1, "actual native Job member required");
        assert!(
            pending_before_revoke,
            "running native child is not terminal"
        );
        assert_eq!(reservation_before_revoke, 1);
        assert!(admission_still_blocks_revoke);
        assert_eq!(reservation_during_revoke, 1);
        assert!(
            stopping.elapsed() < Duration::from_secs(5),
            "native cancellation must precede synthetic 5-second natural exit"
        );
        assert!(outcome.cancelled && !outcome.is_success());
        assert!(
            !outcome.deadline_expired,
            "external revoke, not 8s deadline"
        );
        assert!(
            outcome.status.is_some(),
            "owned native root must be observed"
        );
        assert!(
            String::from_utf8_lossy(&outcome.stdout)
                .contains("OPTIC317_LEDGER_CHILD_ACTUALLY_RUNNING")
        );
        assert_eq!(reservation_after_proof, 0);
        assert!(revoke_result);
    }

    #[test]
    fn reservation_rejects_cross_session_access_and_preserves_capacity() {
        let sessions = Arc::new(SessionRegistry::new());
        let a = grant("owner-a");
        let b = grant("owner-b");
        sessions.register(a.clone()).expect("register A");
        sessions.register(b.clone()).expect("register B");
        let owners = ledger();
        let now = MonotonicTime::from_millis(1);
        let id = owners
            .reserve(&sessions, &a.handle, now, budget())
            .expect("reserve A");
        assert!(matches!(
            owners.stop_signals(id, &b.handle),
            Err(OwnerError::WrongOwner)
        ));
        assert!(matches!(
            owners.reap_terminal(id, &b.handle),
            Err(OwnerError::WrongOwner)
        ));
        assert!(matches!(
            owners.reserve(&sessions, &b.handle, now, budget()),
            Err(OwnerError::Capacity)
        ));
        assert_eq!(owners.stop_session(&b.handle).expect("foreign stop"), 0);
        assert_eq!(owners.stop_session(&a.handle).expect("owned stop"), 1);
        assert_eq!(
            owners.outstanding(&a.handle).expect("retained reservation"),
            1
        );
    }
}
