//! #317 research: actual session latch + short commit + owned atomic Win32 child.
//! Every ignored host requires the independently pre-armed outer Job protocol.
#[path = "authority_native_owner_tests/native_children.rs"]
mod native_children;
use super::*;
use crate::{
    Clock, SessionAdmissionPermit, SessionGrantSpec, SessionLifecycleManager, SessionRegistry,
    SessionRegistryError, StdClock, TaskLeaseRegistry,
};
use optic_bridge_core::{Capability, MonotonicTime, PrincipalId, ProjectId};
use optic_bridge_windows_test_support as native;
use std::{collections::BTreeSet, env, io, sync::mpsc, thread, time::Instant};

fn wait(mut predicate: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(3);
    while !predicate() {
        assert!(
            Instant::now() < until,
            "native registration research deadline"
        );
        thread::sleep(Duration::from_millis(2));
    }
}
struct Client(Arc<AtomicBool>);
impl Drop for Client {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

// Constructed only by Root::commit after actual registration. Not a production
// authorizer: fixed fixture, no caller-selected executable or permission grant.
struct Commit {
    root: JobId,
    owner: JobId,
    deadline: MonotonicTime,
}
struct Keeper<'a> {
    admission: Option<SessionAdmissionPermit<'a>>,
    authority: Arc<AtomicBool>,
    native: native::Native,
    watch: native::StopWatch,
    id: JobId,
    record: Arc<JobRecord>,
    deadline: MonotonicTime,
    registered: bool,
    ticket: Option<Commit>,
    terminal: bool,
    failed: bool,
    timed_out: bool,
}
struct Root<'a> {
    registry: &'a SessionRegistry,
    manager: &'a ProcessManager,
    session: SessionHandle,
    clock: &'a dyn Clock,
    id: JobId,
    // Exactly one prepared/live/uncertain slot in this finite research root.
    // The host owns Root, Client never owns handles or its admission.
    slot: Option<Box<Keeper<'a>>>,
    stop: Arc<AtomicBool>,
    fail_stop: Arc<AtomicBool>,
    fail_query: Arc<AtomicBool>,
    errors: Arc<AtomicU64>,
    closing: bool,
    last_error: Option<i32>,
}
impl<'a> Root<'a> {
    fn new(
        registry: &'a SessionRegistry,
        manager: &'a ProcessManager,
        session: &SessionHandle,
        clock: &'a dyn Clock,
    ) -> (Self, Client) {
        let stop = Arc::new(AtomicBool::new(false));
        (
            Self {
                registry,
                manager,
                session: session.clone(),
                clock,
                id: JobId::generate().unwrap(),
                slot: None,
                stop: Arc::clone(&stop),
                fail_stop: Arc::new(AtomicBool::new(false)),
                fail_query: Arc::new(AtomicBool::new(false)),
                errors: Arc::new(AtomicU64::new(0)),
                closing: false,
                last_error: None,
            },
            Client(stop),
        )
    }
    fn prepare(&mut self, late_phase: impl FnOnce(), latch: bool) -> io::Result<()> {
        if self.closing || self.stop.load(Ordering::Acquire) || self.slot.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "root closed or capacity full",
            ));
        }
        let now = self.clock.now();
        assert_eq!(
            native::CPU,
            self.manager.limits.max_process_cpu_percent_per_job
        );
        assert!(
            ResourceBudget {
                timeout_ms: 4000,
                output_bytes: native::OUTPUT as u64,
                memory_bytes: native::MEMORY,
                process_count: 2
            }
            .fits_within(self.manager.limits.max_process_budget)
        );
        let admission = self
            .registry
            .begin_admission(&self.session, now)
            .map_err(io::Error::other)?;
        let authority = self
            .registry
            .research_admitted_stop_signal(&admission)
            .map_err(io::Error::other)?;
        let deadline = MonotonicTime::from_millis(
            now.as_millis()
                .checked_add(4000)
                .unwrap()
                .min(admission.grant().expires_at.as_millis()),
        );
        let record = Arc::new(JobRecord {
            owner: self.session.clone(),
            private_action: None,
            sequence: 0,
            workload_class: WorkloadClass::Standard,
            reserved_output_bytes: native::OUTPUT as u64,
            reserved_memory_bytes: native::MEMORY,
            reserved_cpu_percent: native::CPU,
            cwd_pin: Mutex::new(None),
            output: Mutex::new(OutputState {
                stdout: Vec::new(),
                stderr: Vec::new(),
                stdout_closed: false,
                stderr_closed: false,
            }),
            state: Mutex::new(JobState {
                status: ProcessStatus::Running,
                exit_code: None,
            }),
            stop_requested: Arc::new(AtomicBool::new(false)),
            output_overflow: AtomicBool::new(false),
            output_read_failed: AtomicBool::new(false),
        });
        // Empty-Job preparation is reversible and occurs OUTSIDE authority locks.
        let native = native::Native::prepare()?;
        let watched_authority = if latch {
            Arc::clone(&authority)
        } else {
            Arc::new(AtomicBool::new(false))
        };
        let watch = native::StopWatch::new(
            Arc::clone(&native.job),
            [
                Arc::clone(&record.stop_requested),
                watched_authority,
                Arc::clone(&self.stop),
            ],
            Arc::clone(&self.fail_stop),
            Instant::now()
                + Duration::from_millis(
                    deadline
                        .as_millis()
                        .saturating_sub(self.clock.now().as_millis()),
                ),
        );
        self.slot = Some(Box::new(Keeper {
            admission: Some(admission),
            authority,
            native,
            watch,
            id: JobId::generate().unwrap(),
            record,
            deadline,
            registered: false,
            ticket: None,
            terminal: false,
            failed: false,
            timed_out: false,
        }));
        late_phase(); // finite test rendezvous, still outside authority locks
        let keeper = self.slot.as_mut().unwrap();
        // Capture actual host memory OUTSIDE BOTH authority and JobStore locks.
        // Then apply the existing pure headroom formula to CURRENT reservations.
        // No fake provider or weakened ceiling. Like the production sample, this
        // is an admission observation, not a promise about future system memory.
        let memory = self
            .manager
            .host_memory_provider
            .snapshot()
            .map_err(io::Error::other)?;
        if memory.total_physical_bytes == 0
            || memory.available_physical_bytes > memory.total_physical_bytes
        {
            return Err(io::Error::other("host memory unavailable"));
        }
        let mut store = self.manager.jobs.lock().unwrap();
        let headroom =
            effective_host_memory_headroom_bytes(&self.manager.limits, memory.total_physical_bytes);
        let required = headroom
            .checked_add(reserved_process_memory_bytes(&store).map_err(io::Error::other)?)
            .and_then(|n| n.checked_add(native::MEMORY))
            .ok_or_else(|| io::Error::other("headroom overflow"))?;
        if memory.available_physical_bytes < required {
            return Err(io::Error::other("host memory headroom exceeded"));
        }
        self.manager
            .prepare_store_for_start_with_workload(
                &mut store,
                &self.session,
                WorkloadClass::Standard,
                native::OUTPUT as u64,
                native::MEMORY,
                native::CPU,
            )
            .map_err(io::Error::other)?;
        assert!(
            store
                .jobs
                .insert(keeper.id.clone(), Arc::clone(&keeper.record))
                .is_none()
        );
        keeper.registered = true;
        if keeper.authority.load(Ordering::Acquire) {
            keeper.record.stop_requested.store(true, Ordering::Release);
        }
        Ok(())
    }
    fn commit(&mut self) -> Result<(), SessionRegistryError> {
        let keeper = self.slot.as_mut().unwrap();
        assert!(keeper.registered && keeper.ticket.is_none() && !keeper.terminal);
        let ticket =
            self.registry
                .research_commit_owner(&self.session, self.clock.now(), || {
                    // TRUSTED local callback only. No native work, memory sample, JobStore
                    // lock or foreign callback in this short authority critical section.
                    let now = self.clock.now();
                    if now >= keeper.deadline {
                        return Err(SessionRegistryError::Expired);
                    }
                    if self.closing
                        || self.stop.load(Ordering::Acquire)
                        || keeper.record.stop_requested.load(Ordering::Acquire)
                        || keeper.authority.load(Ordering::Acquire)
                    {
                        return Err(SessionRegistryError::Revoked);
                    }
                    Ok(Commit {
                        root: self.id.clone(),
                        owner: keeper.id.clone(),
                        deadline: keeper.deadline,
                    })
                })??;
        keeper.ticket = Some(ticket);
        Ok(())
    }
    fn check_start(&mut self) -> io::Result<()> {
        let keeper = self.slot.as_mut().unwrap();
        let ticket = keeper
            .ticket
            .as_ref()
            .ok_or_else(|| io::Error::other("no committed owner"))?;
        if keeper.terminal
            || ticket.root != self.id
            || ticket.owner != keeper.id
            || self.closing
            || self.stop.load(Ordering::Acquire)
            || keeper.authority.load(Ordering::Acquire)
            || keeper.record.stop_requested.load(Ordering::Acquire)
            || self.clock.now() >= ticket.deadline
        {
            self.stop.store(true, Ordering::Release);
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "committed owner stopped or expired",
            ));
        }
        Ok(())
    }
    fn start(&mut self, root: &Path, mode: &str, paused_native: impl FnOnce()) -> io::Result<()> {
        let result = self.start_inner(root, mode, paused_native);
        if let Err(error) = &result {
            self.stop.store(true, Ordering::Release);
            if error.kind() != io::ErrorKind::Interrupted {
                self.last_error = error.raw_os_error();
                self.slot.as_mut().unwrap().failed = true;
            }
        }
        result
    }
    fn start_inner(
        &mut self,
        root: &Path,
        mode: &str,
        paused_native: impl FnOnce(),
    ) -> io::Result<()> {
        self.check_start()?;
        let native = &mut self.slot.as_mut().unwrap().native;
        native.create(root, mode, mode == "create-error")?;
        paused_native(); // proves close progress while native phase is in flight
        self.check_start()?;
        self.slot
            .as_ref()
            .unwrap()
            .native
            .resume(mode == "resume-error")
    }
    fn request_shutdown(&mut self) -> bool {
        self.closing = true;
        self.stop.store(true, Ordering::Release);
        if let Some(k) = self.slot.as_mut() {
            k.native.close_writers();
        }
        self.slot.as_ref().is_none_or(|k| k.terminal)
    }
    fn observe(&mut self) -> io::Result<bool> {
        let result = self.observe_inner();
        if let Err(error) = &result {
            self.last_error = error.raw_os_error();
            let _ = self
                .errors
                .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    Some(n.saturating_add(1))
                });
            if let Some(k) = self.slot.as_mut() {
                k.failed = true;
                k.record.state.lock().unwrap().status = ProcessStatus::TerminationUncertain;
            }
        }
        result
    }
    fn observe_inner(&mut self) -> io::Result<bool> {
        let Some(k) = self.slot.as_mut() else {
            return Ok(true);
        };
        if k.terminal {
            return Ok(true);
        }
        if self.clock.now() >= k.deadline {
            k.timed_out = true;
            self.stop.store(true, Ordering::Release);
        }
        if self.fail_query.load(Ordering::Acquire) {
            native::Native::rejected_query_probe()?;
        }
        k.native.capture()?;
        if k.native.overflow {
            self.stop.store(true, Ordering::Release);
        }
        if !k.native.terminal()? {
            if k.watch.errors.load(Ordering::Acquire) > 0 {
                k.failed = true;
                k.record.state.lock().unwrap().status = ProcessStatus::TerminationUncertain;
            }
            return Ok(false);
        }
        if !k.watch.try_finish() {
            return Ok(false);
        }
        // All fallible observations BEFORE publication, as proved in c1a9c32.
        let code = k.native.root_code()?.map(|n| n as i32);
        let mut out = k.record.output.lock().unwrap();
        let mut state = k.record.state.lock().unwrap();
        out.stdout = std::mem::take(&mut k.native.stdout);
        out.stderr = std::mem::take(&mut k.native.stderr);
        out.stdout_closed = true;
        out.stderr_closed = true;
        k.record
            .output_overflow
            .store(k.native.overflow, Ordering::Release);
        state.status = if k.failed || k.watch.errors.load(Ordering::Acquire) > 0 {
            ProcessStatus::Failed
        } else if k.timed_out || k.watch.deadline_expired.load(Ordering::Acquire) {
            ProcessStatus::TimedOut
        } else if k.native.overflow {
            ProcessStatus::OutputLimitExceeded
        } else {
            ProcessStatus::Stopped
        };
        state.exit_code = code;
        drop(state);
        drop(out);
        k.terminal = true;
        k.ticket.take();
        k.admission.take();
        Ok(true)
    }
}
impl Drop for Root<'_> {
    fn drop(&mut self) {
        self.request_shutdown();
        // Explicit retaining policy. NOT a bounded production Drop. If native
        // proof remains unavailable the identified root stays HERE, with permit,
        // handles and quota; the external harness owns the host's recovery.
        loop {
            if matches!(self.observe(), Ok(true)) {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
}

#[path = "authority_native_owner_tests/lifecycle_cases.rs"]
mod lifecycle_cases;
#[path = "authority_native_owner_tests/scenarios.rs"]
mod scenarios;

#[test]
#[ignore = "requires pre-armed independent outer Job; fixed self-test processes only"]
fn authority_native_owner_host() {
    let gate = PathBuf::from(env::var_os("OPTIC317_OWNER_GATE").unwrap());
    wait(|| gate.exists());
    let path = gate.parent().unwrap();
    let mode = env::var("OPTIC317_OWNER_MODE").unwrap();
    let proof = if mode == "controller-expired" {
        lifecycle_cases::exercise(path, &mode)
    } else {
        scenarios::exercise(path, &mode)
    };
    fs::write(path.join("ready"), proof.to_string()).unwrap();
    thread::sleep(Duration::from_secs(10));
    panic!("outer supervisor did not recover fixed host");
}
