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
        let memory = state.memory.checked_add(budget.memory).ok_or(OwnerError::Capacity)?;
        let output = state.output.checked_add(budget.output).ok_or(OwnerError::Capacity)?;
        let cpu = state.cpu.checked_add(budget.cpu).ok_or(OwnerError::Capacity)?;
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
        if record.stop.load(Ordering::Acquire) || record.revoked.load(Ordering::Acquire) {
            return Err(OwnerError::Revoked);
        }
        // Check all fallible conditions before taking the caller-owned value.
        let native = owner.take().ok_or(OwnerError::MissingPayload)?;
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
        Ok(state.records.values().filter(|r| &r.session == session).count())
    }

    // Does not hold the global budget mutex during potentially blocking native
    // operations; an external cancellation can still set the stop signal.
    fn with_owner<R>(
        &self,
        id: OwnerId,
        session: &SessionHandle,
        visit: impl FnOnce(&mut PreparedReadCommand) -> io::Result<R>,
    ) -> Result<R, OwnerError> {
        let owner = self.get_owner(id, session)?;
        let mut native = owner.lock().map_err(|_| OwnerError::Unavailable)?;
        Ok(visit(&mut native)?)
    }

    fn get_owner(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<Arc<Mutex<PreparedReadCommand>>, OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.session != session {
            return Err(OwnerError::WrongOwner);
        }
        record.native.as_ref().cloned().ok_or(OwnerError::MissingPayload)
    }

    // No caller-supplied success boolean: the only release path requires the
    // staged native owner's own root+Job-empty+both-EOF terminal receipt and a
    // successfully claimed outcome. Any native error retains reservations.
    fn reap_terminal(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<Option<ReadCommandOutcome>, OwnerError> {
        let (owner, _observation) = {
            let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
            let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
            if &record.session != session {
                return Err(OwnerError::WrongOwner);
            }
            if record
                .observing
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return Err(OwnerError::ObservationBusy);
            }
            let observation = Observation(Arc::clone(&record.observing));
            let Some(owner) = record.native.as_ref() else {
                return Err(OwnerError::MissingPayload);
            };
            (Arc::clone(owner), observation)
        };
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
                || !record.native.as_ref().is_some_and(|p| Arc::ptr_eq(p, &owner))
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
        Budget { memory: 512 * 1024 * 1024, output: 16384, cpu: 25 }
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
    fn reserved_owner_retains_admission_until_native_quiescence() {
        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("owner-a");
        sessions.register(grant.clone()).expect("register");
        let now = MonotonicTime::from_millis(1);
        let owners = ledger();
        let id = owners.reserve(&sessions, &grant.handle, now, budget()).expect("reserve before spawn");
        let signals = owners.stop_signals(id, &grant.handle).expect("native watchdog signals");
        let mut native = Some(PreparedReadCommand::prepare(
            LimitedJobObject::new(1, budget().memory, budget().cpu).expect("job"),
            budget().output as usize,
            3000,
            signals,
        ).expect("native preparation before child creation"));
        owners.adopt(id, &grant.handle, &mut native).expect("adopt BEFORE spawn");
        assert!(native.is_none());

        let sessions_for_revoke = Arc::clone(&sessions);
        let handle_for_revoke = grant.handle.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let revoker = thread::spawn(move || {
            done_tx.send(sessions_for_revoke.revoke(&handle_for_revoke)).expect("send revoke");
        });
        let start = Instant::now();
        while !owners.stop_signals(id, &grant.handle).expect("signals")[1].load(Ordering::Acquire) {
            assert!(start.elapsed() < Duration::from_secs(2), "revocation signal absent");
            thread::sleep(Duration::from_millis(5));
        }
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert_eq!(owners.outstanding(&grant.handle).expect("reserved"), 1);

        let result = owners.reap_terminal(id, &grant.handle).expect("native observation");
        let receipt = result.expect("no-spawn revoked Job and EOF must be proven");
        assert!(receipt.cancelled && receipt.status.is_none());
        assert!(!receipt.is_success());
        assert_eq!(owners.outstanding(&grant.handle).expect("released"), 0);
        assert_eq!(done_rx.recv_timeout(Duration::from_secs(3)).expect("drained session").expect("revoke"), true);
        revoker.join().expect("join revoke");
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
        let id = owners.reserve(&sessions, &a.handle, now, budget()).expect("reserve A");
        assert!(matches!(owners.stop_signals(id, &b.handle), Err(OwnerError::WrongOwner)));
        assert!(matches!(owners.reap_terminal(id, &b.handle), Err(OwnerError::WrongOwner)));
        assert!(matches!(owners.reserve(&sessions, &b.handle, now, budget()), Err(OwnerError::Capacity)));
        assert_eq!(owners.stop_session(&b.handle).expect("foreign stop"), 0);
        assert_eq!(owners.stop_session(&a.handle).expect("owned stop"), 1);
        assert_eq!(owners.outstanding(&a.handle).expect("retained reservation"), 1);
    }
}
