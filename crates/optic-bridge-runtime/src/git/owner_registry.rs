//! Research-stage Windows V1 Git owner ledger (#317).
//! Not wired to production GitRead. Native proof remains mandatory.
#![allow(dead_code)]

use crate::{SessionAdmissionPermit, SessionRegistry, SessionRegistryError};
use optic_bridge_core::{MonotonicTime, SessionHandle};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use thiserror::Error;

#[derive(Clone, Copy, Debug)]
pub(super) struct OwnerBudget {
    pub memory: u64,
    pub output: u64,
    pub cpu: u32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OwnerCeiling {
    pub count: usize,
    pub memory: u64,
    pub output: u64,
    pub cpu: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum OwnerDomain {
    Session(SessionHandle),
    HostStartup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct OwnerId(u64);

#[derive(Debug, Error)]
pub(super) enum OwnerError {
    #[error("invalid or exceeded Git owner reservation limit")]
    Capacity,
    #[error("Git owner identity exhausted")]
    IdExhausted,
    #[error("Git owner state unavailable")]
    Unavailable,
    #[error("Git owner unknown or wrong authority")]
    WrongOwner,
    #[error("Git owner already contains a native payload")]
    Adopted,
    #[error("Git owner terminal observation is already running")]
    ObservationBusy,
    #[error("Git owner does not contain a native payload")]
    NotAdopted,
    #[error("session revoked before native launch")]
    Revoked,
    #[error(transparent)]
    Session(#[from] SessionRegistryError),
}

enum Stage<T> {
    Reserved,
    Adopted {
        owner: Arc<Mutex<T>>,
        observing: Arc<AtomicBool>,
    },
}

// An interrupted or panicking observer must never leave the record permanently
// busy. A panicked payload mutex stays poisoned and retains capacity.
struct ObservationFlag(Arc<AtomicBool>);

impl Drop for ObservationFlag {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

struct Record<T> {
    domain: OwnerDomain,
    revoked: Option<Arc<AtomicBool>>,
    budget: OwnerBudget,
    stop: Arc<AtomicBool>,
    stage: Stage<T>,
}

struct State<T> {
    next: u64,
    memory: u64,
    output: u64,
    cpu: u32,
    records: BTreeMap<OwnerId, Record<T>>,
}

pub(super) struct GitOwnerRegistry<T> {
    ceiling: OwnerCeiling,
    state: Mutex<State<T>>,
}

impl<T> GitOwnerRegistry<T> {
    pub(super) fn new(ceiling: OwnerCeiling) -> Result<Self, OwnerError> {
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

    // The caller owns a genuine service-worker admission, not just a stale
    // SessionHandle. Native adoption/resume must still recheck revocation.
    pub(super) fn reserve_session(
        &self,
        sessions: &SessionRegistry,
        admission: &SessionAdmissionPermit<'_>,
        now: MonotonicTime,
        budget: OwnerBudget,
    ) -> Result<OwnerId, OwnerError> {
        let grant = admission.grant();
        let (flag, current) = sessions.watch_active(&grant.handle, now)?;
        if current.principal != grant.principal
            || current.project != grant.project
            || current.policy_epoch != grant.policy_epoch
        {
            return Err(OwnerError::Revoked);
        }
        self.reserve(
            OwnerDomain::Session(grant.handle.clone()),
            Some(flag),
            budget,
        )
    }

    pub(super) fn reserve_host_startup(&self, budget: OwnerBudget) -> Result<OwnerId, OwnerError> {
        self.reserve(OwnerDomain::HostStartup, None, budget)
    }

    fn reserve(
        &self,
        domain: OwnerDomain,
        revoked: Option<Arc<AtomicBool>>,
        budget: OwnerBudget,
    ) -> Result<OwnerId, OwnerError> {
        if budget.memory == 0 || budget.output == 0 || !(1..=100).contains(&budget.cpu) {
            return Err(OwnerError::Capacity);
        }
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        if revoked.as_ref().is_some_and(|x| x.load(Ordering::Acquire)) {
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
        let next = state.next.checked_add(1).ok_or(OwnerError::IdExhausted)?;
        let id = OwnerId(next);
        state.next = next;
        state.memory = memory;
        state.output = output;
        state.cpu = cpu;
        state.records.insert(
            id,
            Record {
                domain,
                revoked,
                budget,
                stop: Arc::new(AtomicBool::new(false)),
                stage: Stage::Reserved,
            },
        );
        Ok(id)
    }

    // Commit the transfer only after every fallible registry check. On error
    // the service worker still owns the native payload and must recover it.
    // Taking T by value here would run its Drop on rejected adoption.
    pub(super) fn adopt(
        &self,
        id: OwnerId,
        domain: &OwnerDomain,
        owner: &mut Option<T>,
    ) -> Result<(), OwnerError> {
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get_mut(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.domain != domain {
            return Err(OwnerError::WrongOwner);
        }
        if !matches!(&record.stage, Stage::Reserved) {
            return Err(OwnerError::Adopted);
        }
        record.stage = Stage::Adopted {
            owner: Arc::new(Mutex::new(owner.take().ok_or(OwnerError::NotAdopted)?)),
            observing: Arc::new(AtomicBool::new(false)),
        };
        Ok(())
    }

    pub(super) fn should_stop(
        &self,
        id: OwnerId,
        domain: &OwnerDomain,
    ) -> Result<bool, OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.domain != domain {
            return Err(OwnerError::WrongOwner);
        }
        Ok(record.stop.load(Ordering::Acquire)
            || record
                .revoked
                .as_ref()
                .is_some_and(|f| f.load(Ordering::Acquire)))
    }

    // A native owner watchdog may hold this signal without entering the
    // payload lock or taking an operation-admission slot. It is not proof
    // of successful native termination; the service must still observe it.
    pub(super) fn stop_signal(
        &self,
        id: OwnerId,
        domain: &OwnerDomain,
    ) -> Result<Arc<AtomicBool>, OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.domain != domain {
            return Err(OwnerError::WrongOwner);
        }
        Ok(Arc::clone(&record.stop))
    }

    // Stop is independent of normal operation admission/capacity.
    pub(super) fn stop_session(&self, session: &SessionHandle) -> Result<usize, OwnerError> {
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let mut count = 0;
        for record in state.records.values_mut() {
            if record.domain == OwnerDomain::Session(session.clone())
                && !record.stop.swap(true, Ordering::AcqRel)
            {
                count += 1;
            }
        }
        Ok(count)
    }

    pub(super) fn outstanding(&self, session: &SessionHandle) -> Result<usize, OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        Ok(state
            .records
            .values()
            .filter(|r| r.domain == OwnerDomain::Session(session.clone()))
            .count())
    }

    // Observation can block on native I/O. It holds only the per-operation
    // payload lock, never the global reservation/stop mutex. The observer flag
    // admits one proof at a time and is released even on callback unwind.
    // False, error, panic or poisoned owner retains the slot and budget.
    pub(super) fn reap_if_proven<E>(
        &self,
        id: OwnerId,
        domain: &OwnerDomain,
        prove: impl FnOnce(&mut T) -> Result<bool, E>,
    ) -> Result<Result<bool, E>, OwnerError> {
        let (owner, _observation) = {
            let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
            let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
            if &record.domain != domain {
                return Err(OwnerError::WrongOwner);
            }
            let Stage::Adopted { owner, observing } = &record.stage else {
                return Err(OwnerError::NotAdopted);
            };
            if observing
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return Err(OwnerError::ObservationBusy);
            }
            (Arc::clone(owner), ObservationFlag(Arc::clone(observing)))
        };

        let result = {
            let mut native = owner.lock().map_err(|_| OwnerError::Unavailable)?;
            prove(&mut *native)
        };

        if matches!(&result, Ok(true)) {
            let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
            let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
            if &record.domain != domain {
                return Err(OwnerError::WrongOwner);
            }
            match &record.stage {
                Stage::Adopted {
                    owner: retained, ..
                } if Arc::ptr_eq(retained, &owner) => {}
                _ => return Err(OwnerError::NotAdopted),
            }
            let released = state.records.remove(&id).expect("registered");
            state.memory -= released.budget.memory;
            state.output -= released.budget.output;
            state.cpu -= released.budget.cpu;
        }
        Ok(result)
    }

    // Only a service worker that has proven spawn never began may call this.
    // NEVER call from request Drop or an uncertain native spawn failure.
    pub(super) fn confirm_no_spawn(
        &self,
        id: OwnerId,
        domain: &OwnerDomain,
    ) -> Result<(), OwnerError> {
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.domain != domain {
            return Err(OwnerError::WrongOwner);
        }
        if !matches!(&record.stage, Stage::Reserved) {
            return Err(OwnerError::Adopted);
        }
        let released = state.records.remove(&id).expect("registered");
        state.memory -= released.budget.memory;
        state.output -= released.budget.output;
        state.cpu -= released.budget.cpu;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_bridge_core::{Capability, PrincipalId, ProjectId, SessionGrant};
    use std::collections::BTreeSet;

    #[test]
    fn rejected_adoption_must_not_destroy_native_owner() {
        use std::sync::atomic::AtomicUsize;
        struct Payload(Arc<AtomicUsize>);
        impl Drop for Payload {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let owners = GitOwnerRegistry::new(OwnerCeiling {
            count: 1,
            memory: 100,
            output: 40,
            cpu: 80,
        })
        .unwrap();
        let id = owners.reserve_host_startup(budget()).unwrap();
        let drops = Arc::new(AtomicUsize::new(0));
        let mut payload = Some(Payload(Arc::clone(&drops)));
        assert!(matches!(
            owners.adopt(
                id,
                &OwnerDomain::Session(grant("wrong").handle),
                &mut payload
            ),
            Err(OwnerError::WrongOwner)
        ));
        assert_eq!(
            drops.load(Ordering::SeqCst),
            0,
            "rejected native resources must remain owned by the caller"
        );
        assert!(payload.is_some());
        owners
            .adopt(id, &OwnerDomain::HostStartup, &mut payload)
            .unwrap();
        assert!(payload.is_none());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        owners
            .reap_if_proven(id, &OwnerDomain::HostStartup, |_| Ok::<bool, ()>(true))
            .unwrap()
            .unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_adoption_keeps_payload_for_duplicate_unknown_and_poisoned_registry() {
        let owners = ledger();
        let id = owners.reserve_host_startup(budget()).unwrap();
        let domain = OwnerDomain::HostStartup;
        let mut missing = None;
        assert!(matches!(
            owners.adopt(id, &domain, &mut missing),
            Err(OwnerError::NotAdopted)
        ));
        let mut payload = Some("original");
        owners.adopt(id, &domain, &mut payload).unwrap();
        assert!(payload.is_none());
        let mut duplicate = Some("must survive");
        assert!(matches!(
            owners.adopt(id, &domain, &mut duplicate),
            Err(OwnerError::Adopted)
        ));
        assert_eq!(duplicate, Some("must survive"));
        assert!(matches!(
            owners.adopt(OwnerId(u64::MAX), &domain, &mut duplicate),
            Err(OwnerError::WrongOwner)
        ));
        assert_eq!(duplicate, Some("must survive"));
        assert!(
            std::panic::catch_unwind(|| {
                let _guard = owners.state.lock().unwrap();
                panic!("synthetic registry poisoning");
            })
            .is_err()
        );
        assert!(matches!(
            owners.adopt(id, &domain, &mut duplicate),
            Err(OwnerError::Unavailable)
        ));
        assert_eq!(duplicate, Some("must survive"));
        // The poisoned registry itself deliberately retains the original;
        // this test does not authorize dropping a live native registry.
    }

    // Run only under scripts/research/issue317_registry_native.py. The gate
    // ensures the independent supervisor owns this process tree before spawn.
    #[test]
    #[ignore = "requires independent Windows Job supervisor"]
    fn native_rejected_adoption_retains_owner_until_tree_and_pipes_end() {
        use optic_bridge_windows::LimitedJobObject;
        use process_wrap::tokio::CommandWrap;
        use std::{
            env,
            process::Stdio,
            thread,
            time::{Duration, Instant},
        };
        use tokio::{io::AsyncReadExt, process::Command};

        let gate = std::path::PathBuf::from(
            env::var_os("OPTIC317_REGISTRY_GATE").expect("supervisor gate"),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while !gate.exists() {
            assert!(Instant::now() < until, "outer ownership gate absent");
            thread::sleep(Duration::from_millis(5));
        }
        let started = gate.with_extension("started");
        let sessions = SessionRegistry::new();
        let a = grant("native");
        sessions.register(a.clone()).unwrap();
        let admission = sessions
            .begin_admission(&a.handle, MonotonicTime::from_millis(1))
            .unwrap();
        let owners = GitOwnerRegistry::new(OwnerCeiling {
            count: 1,
            memory: 512 * 1024 * 1024,
            output: 16384,
            cpu: 25,
        })
        .unwrap();
        let budget = OwnerBudget {
            memory: 512 * 1024 * 1024,
            output: 16384,
            cpu: 25,
        };
        let id = owners
            .reserve_session(&sessions, &admission, MonotonicTime::from_millis(1), budget)
            .unwrap();
        let domain = OwnerDomain::Session(a.handle.clone());
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut command = Command::new(env::current_exe().unwrap());
            command.args(["--exact", "git_owner_registry::tests::native_registry_fixture_child", "--ignored", "--nocapture"])
                .env_clear().env("OPTIC317_REGISTRY_CHILD", "1")
                .env("OPTIC317_REGISTRY_STARTED", &started)
                .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
            if let Some(root) = env::var_os("SystemRoot") { command.env("SystemRoot", root); }
            let mut command = CommandWrap::from(command);
            command.wrap(LimitedJobObject::new(1, budget.memory, budget.cpu).unwrap());
            let mut child = command.spawn().unwrap();
            let start_deadline = Instant::now() + Duration::from_secs(3);
            while !started.exists() {
                assert!(Instant::now() < start_deadline, "native child did not execute");
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            let out = child.stdout().take().unwrap();
            let err = child.stderr().take().unwrap();
            let mut payload = Some((child, out, err));
            assert!(matches!(owners.adopt(id, &OwnerDomain::HostStartup, &mut payload), Err(OwnerError::WrongOwner)));
            assert!(payload.as_mut().unwrap().0.try_wait().unwrap().is_none(), "rejected child still owned, not killed by Drop");
            owners.adopt(id, &domain, &mut payload).unwrap();
            assert!(payload.is_none());
            let (reply_tx, reply_rx) = std::sync::mpsc::channel::<()>();
            drop(reply_rx);
            assert!(reply_tx.send(()).is_err());
            assert_eq!(owners.outstanding(&a.handle).unwrap(), 1);
            assert!(matches!(owners.reserve_session(&sessions, &admission, MonotonicTime::from_millis(1), budget), Err(OwnerError::Capacity)));
            owners.stop_session(&a.handle).unwrap();
            assert!(owners.stop_signal(id, &domain).unwrap().load(Ordering::Acquire));
            // Stop is a request, not proof. The observer keeps the real child
            // and pipes registered after requesting native termination.
            assert!(!owners.reap_if_proven(id, &domain, |(child, _, _)| {
                child.start_kill()?;
                Ok::<bool, std::io::Error>(false)
            }).unwrap().unwrap());
            assert_eq!(owners.outstanding(&a.handle).unwrap(), 1);
            let proof = owners.reap_if_proven(id, &domain, |(child, out, err)| {
                // Drive the synchronous proof outside Tokio's worker context;
                // never hold the global registry lock while waiting for I/O.
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        tokio::time::timeout(Duration::from_secs(3), async {
                            loop {
                                if child.try_wait()?.is_some() { break; }
                                tokio::time::sleep(Duration::from_millis(5)).await;
                            }
                            for pipe in [&mut *out as &mut (dyn tokio::io::AsyncRead + Unpin), &mut *err] {
                                let mut buffer = [0_u8; 512];
                                let mut total = 0;
                                loop {
                                    let n = pipe.read(&mut buffer).await?;
                                    if n == 0 { break; }
                                    total += n;
                                    if total > 8192 { return Err(std::io::Error::other("fixture output cap")); }
                                }
                            }
                            Ok::<bool, std::io::Error>(true)
                        }).await.map_err(|_| std::io::Error::other("native proof timed out"))?
                    })
                })
            }).unwrap().unwrap();
            assert!(proof);
            assert_eq!(owners.outstanding(&a.handle).unwrap(), 0);
            println!("OPTIC317_REGISTRY_NATIVE tree_empty=true pipes_eof=true rejected_payload_retained=true capacity_released_after_proof=true");
        });
    }

    #[test]
    #[ignore = "synthetic child selected only by independent supervisor"]
    fn native_registry_fixture_child() {
        assert_eq!(std::env::var("OPTIC317_REGISTRY_CHILD").as_deref(), Ok("1"));
        std::fs::write(
            std::env::var_os("OPTIC317_REGISTRY_STARTED").expect("started marker"),
            b"executed",
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(8));
    }

    fn grant(name: &str) -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().unwrap(),
            principal: PrincipalId::new(name).unwrap(),
            project: ProjectId::new("synthetic").unwrap(),
            capabilities: BTreeSet::from([Capability::FileRead]),
            expires_at: MonotonicTime::from_millis(1_000),
            policy_epoch: 1,
        }
    }

    fn ledger() -> GitOwnerRegistry<&'static str> {
        GitOwnerRegistry::new(OwnerCeiling {
            count: 2,
            memory: 100,
            output: 40,
            cpu: 80,
        })
        .unwrap()
    }
    fn budget() -> OwnerBudget {
        OwnerBudget {
            memory: 50,
            output: 20,
            cpu: 40,
        }
    }

    #[test]
    fn capacity_owner_is_retained_until_proven() {
        let sessions = SessionRegistry::new();
        let a = grant("a");
        let b = grant("b");
        sessions.register(a.clone()).unwrap();
        sessions.register(b.clone()).unwrap();
        let now = MonotonicTime::from_millis(1);
        let permit_a = sessions.begin_admission(&a.handle, now).unwrap();
        let permit_b = sessions.begin_admission(&b.handle, now).unwrap();
        let owners = ledger();
        let aid = owners
            .reserve_session(&sessions, &permit_a, now, budget())
            .unwrap();
        let bid = owners
            .reserve_session(&sessions, &permit_b, now, budget())
            .unwrap();
        assert!(matches!(
            owners.reserve_session(&sessions, &permit_a, now, budget()),
            Err(OwnerError::Capacity)
        ));
        let ad = OwnerDomain::Session(a.handle.clone());
        let bd = OwnerDomain::Session(b.handle.clone());
        owners
            .adopt(aid, &ad, &mut Some("real owner retained here"))
            .unwrap();
        owners
            .adopt(bid, &bd, &mut Some("independent owner"))
            .unwrap();
        assert!(matches!(
            owners.should_stop(aid, &bd),
            Err(OwnerError::WrongOwner)
        ));
        assert_eq!(owners.stop_session(&a.handle).unwrap(), 1);
        assert!(owners.should_stop(aid, &ad).unwrap());
        assert!(!owners.should_stop(bid, &bd).unwrap());
        assert!(
            !owners
                .reap_if_proven(aid, &ad, |_| Ok::<_, ()>(false))
                .unwrap()
                .unwrap()
        );
        assert!(
            owners
                .reap_if_proven(aid, &ad, |_| Err::<bool, _>("native observation failed"))
                .unwrap()
                .is_err()
        );
        assert_eq!(owners.outstanding(&a.handle).unwrap(), 1);
        assert!(
            owners
                .reap_if_proven(aid, &ad, |_| Ok::<_, ()>(true))
                .unwrap()
                .unwrap()
        );
        assert_eq!(owners.outstanding(&a.handle).unwrap(), 0);
        assert_eq!(owners.outstanding(&b.handle).unwrap(), 1);
        owners
            .reap_if_proven(bid, &bd, |_| Ok::<_, ()>(true))
            .unwrap()
            .unwrap();
    }

    #[test]
    fn host_startup_separate_and_no_spawn_must_be_confirmed() {
        let owners = ledger();
        let id = owners.reserve_host_startup(budget()).unwrap();
        assert!(matches!(
            owners.confirm_no_spawn(id, &OwnerDomain::Session(grant("a").handle)),
            Err(OwnerError::WrongOwner)
        ));
        owners
            .confirm_no_spawn(id, &OwnerDomain::HostStartup)
            .unwrap();
        assert!(owners.reserve_host_startup(budget()).is_ok());
    }
    #[test]
    fn blocked_observer_does_not_block_out_of_band_stop() {
        use std::{sync::mpsc, thread, time::Duration};

        let sessions = SessionRegistry::new();
        let grant = grant("blocked-observer");
        sessions.register(grant.clone()).unwrap();
        let now = MonotonicTime::from_millis(1);
        let admission = sessions.begin_admission(&grant.handle, now).unwrap();
        let owners = Arc::new(ledger());
        let id = owners
            .reserve_session(&sessions, &admission, now, budget())
            .unwrap();
        let domain = OwnerDomain::Session(grant.handle.clone());
        owners
            .adopt(id, &domain, &mut Some("held native owner"))
            .unwrap();
        let stop_signal = owners.stop_signal(id, &domain).unwrap();
        assert!(!stop_signal.load(Ordering::Acquire));

        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (resume_tx, resume_rx) = mpsc::sync_channel(1);
        let observing = Arc::clone(&owners);
        let observed_domain = domain.clone();
        let observer = thread::spawn(move || {
            observing
                .reap_if_proven(id, &observed_domain, |_| {
                    entered_tx.send(()).unwrap();
                    resume_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                    Ok::<bool, ()>(false)
                })
                .unwrap()
                .unwrap()
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();

        let (stopped_tx, stopped_rx) = mpsc::sync_channel(1);
        let stopping = Arc::clone(&owners);
        let handle = grant.handle.clone();
        let stopper = thread::spawn(move || {
            stopped_tx.send(stopping.stop_session(&handle)).unwrap();
        });
        let stopped_without_releasing_observer =
            matches!(stopped_rx.recv_timeout(Duration::from_secs(1)), Ok(Ok(1)));
        let concurrent_proof_is_rejected = stopped_without_releasing_observer
            && matches!(
                owners.reap_if_proven(id, &domain, |_| Ok::<bool, ()>(true)),
                Err(OwnerError::ObservationBusy)
            );

        // Always unblock/join helpers before assertions so even a regression
        // cannot strand test-owned threads; the native CI has an outer deadline.
        resume_tx.send(()).unwrap();
        stopper.join().unwrap();
        assert!(!observer.join().unwrap());
        assert!(stopped_without_releasing_observer);
        assert!(concurrent_proof_is_rejected);
        assert!(owners.should_stop(id, &domain).unwrap());
        assert!(stop_signal.load(Ordering::Acquire));
        assert_eq!(owners.outstanding(&grant.handle).unwrap(), 1);
        assert!(
            owners
                .reap_if_proven(id, &domain, |_| Ok::<bool, ()>(true))
                .unwrap()
                .unwrap()
        );
        assert_eq!(owners.outstanding(&grant.handle).unwrap(), 0);
    }
}
