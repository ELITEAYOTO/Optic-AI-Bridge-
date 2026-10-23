//! Experimental service root. Not wired to GitRead; authorization minting is
//! intentionally absent. Keep this root alive when try_shutdown reports pending.
//! Drop requests stop then JOINS; permanent uncertainty can block Drop forever.
//! This fail-closed fallback is not an accepted application shutdown policy.
#![allow(dead_code)]
use super::*;
use crate::{SessionRegistry, SessionRegistryError};
use optic_bridge_core::MonotonicTime;
use optic_bridge_windows::{PreparedReadCommand, ReadCommandOutcome};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    process::Command as NativeCommand,
    thread::{self, JoinHandle},
};

// No production constructor: a future authorizer must mint this AFTER checking
// GitRead/task leases, executable/cwd pins and the monotonic effective deadline.
// Do not substitute a ProcessRun or HostStartup grant for that authority.
pub(crate) struct AuthorizedRead {
    session: SessionHandle,
    now: MonotonicTime,
    command: NativeCommand,
    resources: ResourceBudget,
    workload: WorkloadClass,
    #[cfg(test)]
    faults: Arc<TestFaults>,
}

#[derive(Default)]
#[cfg(test)]
struct TestFaults {
    hold_start: AtomicBool,
    observation_error: AtomicBool,
    panic_once: AtomicBool,
    panics_caught: AtomicU64,
}

#[derive(Debug, Error)]
pub(crate) enum ServiceError {
    #[error("read service is stopping")]
    Stopping,
    #[error(transparent)]
    Session(#[from] SessionRegistryError),
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error("read service native or thread operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("read service bookkeeping unavailable")]
    Unavailable,
    #[error("read service bookkeeping busy; shutdown requested, owner retained")]
    Busy,
}

pub(crate) struct ReadServiceRoot {
    processes: Arc<ProcessManager>,
    sessions: Arc<SessionRegistry>,
    stopping: Arc<AtomicBool>,
    workers: Mutex<HashMap<JobId, JoinHandle<()>>>,
}

impl ReadServiceRoot {
    pub(crate) fn new(processes: Arc<ProcessManager>, sessions: Arc<SessionRegistry>) -> Self {
        Self {
            processes,
            sessions,
            stopping: Arc::new(AtomicBool::new(false)),
            workers: Mutex::new(HashMap::new()),
        }
    }

    // Serialize submissions with shutdown and bound retained thread handles.
    // Workers never enter this mutex, JobStore or SessionRegistry while polling.
    pub(crate) fn submit(&self, read: AuthorizedRead) -> Result<JobId, ServiceError> {
        let mut workers = self.workers.lock().map_err(|_| ServiceError::Unavailable)?;
        reap_finished(&mut workers)?;
        if self.stopping.load(Ordering::Acquire) {
            return Err(ServiceError::Stopping);
        }
        // Reservation/submission fence ONLY: this permit is not transferred to
        // the worker. Revoke may return with an owner still retained. This is
        // an integration gate, not the synchronous GitRead admission contract.
        let _admission = self.sessions.begin_admission(&read.session, read.now)?;
        let (revoked, _) = self.sessions.watch_active(&read.session, read.now)?;
        if workers.len() >= self.processes.limits.max_active_process_jobs as usize {
            return Err(ProcessError::TooManyActiveJobs.into());
        }
        // No fallible map allocation may remain after the native owner has
        // moved into a running worker. Reserve its root index before effects.
        workers
            .try_reserve(1)
            .map_err(|_| ServiceError::Unavailable)?;
        let resources = read
            .resources
            .validate_nonzero()
            .map_err(ProcessError::InvalidLimits)?;
        if !resources.fits_within(self.processes.limits.max_process_budget)
            || resources.timeout_ms > u64::from(u32::MAX)
        {
            return Err(ProcessError::ResourceBudgetExceeded.into());
        }
        let native_limits = LimitedJobObject::new(
            resources.process_count,
            resources.memory_bytes,
            self.processes.limits.max_process_cpu_percent_per_job,
        )?;
        let output_limit = usize::try_from(resources.output_bytes)
            .map_err(|_| ProcessError::ResourceBudgetExceeded)?;
        let id = JobId::generate().map_err(|_| ProcessError::JobIdUnavailable)?;
        let record = Arc::new(JobRecord {
            owner: read.session.clone(),
            private_action: None,
            sequence: self.processes.sequence.fetch_add(1, Ordering::AcqRel),
            workload_class: read.workload,
            reserved_output_bytes: resources.output_bytes,
            reserved_memory_bytes: resources.memory_bytes,
            reserved_cpu_percent: self.processes.limits.max_process_cpu_percent_per_job,
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
        {
            let mut store = self
                .processes
                .jobs
                .lock()
                .map_err(|_| ProcessError::StateUnavailable)?;
            if store.jobs.contains_key(&id) || workers.contains_key(&id) {
                return Err(ServiceError::Unavailable);
            }
            self.processes.prepare_store_for_start_with_workload(
                &mut store,
                &read.session,
                read.workload,
                resources.output_bytes,
                resources.memory_bytes,
                self.processes.limits.max_process_cpu_percent_per_job,
            )?;
            if revoked.load(Ordering::Acquire) {
                return Err(ServiceError::Session(SessionRegistryError::Revoked));
            }
            store.jobs.insert(id.clone(), Arc::clone(&record));
        }
        let cancellation = [
            Arc::clone(&record.stop_requested),
            revoked,
            Arc::clone(&self.stopping),
        ];
        let native = PreparedReadCommand::prepare(
            native_limits,
            output_limit,
            resources.timeout_ms as u32,
            cancellation.to_vec(),
        );
        let native = match native {
            Ok(native) => native,
            Err(error) => {
                finish_no_spawn(&record);
                return Err(error.into());
            }
        };
        let retained = Arc::clone(&record);
        let spawn = thread::Builder::new()
            .name("optic-git-read-owner".into())
            .spawn(move || {
                // This local outlives each catch_unwind boundary. Never put the
                // owner inside a callback whose unwind would destroy it.
                let mut driver = Driver {
                    native,
                    record,
                    command: Some(read.command),
                    outcome: None,
                    launch_error: None,
                    launch_cancelled: false,
                    cancellation,
                    #[cfg(test)]
                    faults: read.faults,
                };
                loop {
                    match catch_unwind(AssertUnwindSafe(|| driver.step())) {
                        Ok(Ok(true)) => return,
                        Ok(Ok(false)) => {}
                        Err(_) => {
                            #[cfg(test)]
                            driver.faults.panics_caught.fetch_add(1, Ordering::AcqRel);
                            driver.retain_uncertain();
                        }
                        Ok(Err(_)) => driver.retain_uncertain(),
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            });
        match spawn {
            Ok(worker) => {
                workers.insert(id.clone(), worker);
                Ok(id)
            }
            Err(error) => {
                // The closure never ran: no native process could be created.
                // Its prepared handles were dropped by thread::spawn, safely
                // before any native effect. Existing history stays bounded.
                finish_no_spawn(&retained);
                Err(error.into())
            }
        }
    }

    // A count of retained worker responsibilities, not native process count.
    // Pending means SELF is still required as the identified root owner.
    pub(crate) fn try_shutdown(&self) -> Result<usize, ServiceError> {
        self.stopping.store(true, Ordering::Release);
        let mut workers = self.workers.try_lock().map_err(|error| match error {
            std::sync::TryLockError::WouldBlock => ServiceError::Busy,
            std::sync::TryLockError::Poisoned(_) => ServiceError::Unavailable,
        })?;
        reap_finished(&mut workers)?;
        Ok(workers.len())
    }
}

fn finish_no_spawn(record: &JobRecord) {
    mark_stream_closed(record, ProcessStream::Stdout);
    mark_stream_closed(record, ProcessStream::Stderr);
    if let Ok(mut state) = record.state.lock() {
        state.status = ProcessStatus::Failed;
    }
    // Poison does not become a success: admission/lifecycle stay fail-closed.
}

fn reap_finished(workers: &mut HashMap<JobId, JoinHandle<()>>) -> Result<(), ServiceError> {
    // The map is bounded by shared job admission plus finished handles since
    // the previous submit. Reap only finished threads; no active worker join.
    let finished: Vec<_> = workers
        .iter()
        .filter(|(_, h)| h.is_finished())
        .map(|(id, _)| id.clone())
        .collect();
    for id in finished {
        if let Some(worker) = workers.remove(&id) {
            worker.join().map_err(|_| ServiceError::Unavailable)?;
        }
    }
    Ok(())
}

impl Drop for ReadServiceRoot {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        // Only cleanup uses poisoned contents: never admit another operation.
        let workers = self
            .workers
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner());
        for (_, worker) in workers.drain() {
            let _ = worker.join();
        }
    }
}

struct Driver {
    native: PreparedReadCommand,
    record: Arc<JobRecord>,
    command: Option<NativeCommand>,
    outcome: Option<ReadCommandOutcome>,
    launch_error: Option<std::io::ErrorKind>,
    launch_cancelled: bool,
    cancellation: [Arc<AtomicBool>; 3],
    #[cfg(test)]
    faults: Arc<TestFaults>,
}

impl Driver {
    fn retain_uncertain(&self) {
        self.record
            .output_read_failed
            .store(true, Ordering::Release);
        self.record.stop_requested.store(true, Ordering::Release);
        if let Ok(mut state) = self.record.state.lock() {
            state.status = ProcessStatus::TerminationUncertain;
        }
    }

    fn step(&mut self) -> Result<bool, std::io::Error> {
        #[cfg(test)]
        if self.command.is_some() && self.faults.hold_start.load(Ordering::Acquire) {
            return Ok(false);
        }
        if let Some(mut command) = self.command.take()
            && let Err(error) = self.native.start(&mut command)
        {
            self.launch_error = Some(error.kind());
            // Snapshot the REAL owner signals before requesting cleanup;
            // our cleanup flag must not turn an actual launch I/O error
            // into an apparent user cancellation.
            self.launch_cancelled = self
                .cancellation
                .iter()
                .any(|flag| flag.load(Ordering::Acquire));
            self.record.stop_requested.store(true, Ordering::Release);
        }
        #[cfg(test)]
        {
            if self.faults.panic_once.swap(false, Ordering::AcqRel) {
                panic!("synthetic worker observation panic");
            }
            if self.faults.observation_error.load(Ordering::Acquire) {
                return Err(std::io::Error::other("synthetic observation unavailable"));
            }
        }
        if self.outcome.is_none() {
            if !self.native.poll_terminal()? {
                return Ok(false);
            }
            self.outcome = Some(self.native.take_outcome()?);
        }
        let outcome = self
            .outcome
            .as_mut()
            .expect("outcome installed after native proof");
        // Locks may fail, but outcome/native remain in this same driver.
        // Move buffers instead of copying a second output reservation.
        let mut output = self
            .record
            .output
            .lock()
            .map_err(|_| std::io::Error::other("output state unavailable"))?;
        let mut state = self
            .record
            .state
            .lock()
            .map_err(|_| std::io::Error::other("job state unavailable"))?;
        output.stdout = std::mem::take(&mut outcome.stdout);
        output.stderr = std::mem::take(&mut outcome.stderr);
        output.stdout_closed = true;
        output.stderr_closed = true;
        self.record
            .output_overflow
            .store(outcome.output_limit_exceeded, Ordering::Release);
        state.exit_code = outcome.status.and_then(|status| status.code());
        let owner_interrupted_start = self.launch_error == Some(std::io::ErrorKind::Interrupted)
            && (self.launch_cancelled || outcome.deadline_expired);
        state.status = if (self.launch_error.is_some() && !owner_interrupted_start)
            || self.record.output_read_failed.load(Ordering::Acquire)
        {
            ProcessStatus::Failed
        } else if outcome.deadline_expired {
            ProcessStatus::TimedOut
        } else if outcome.output_limit_exceeded {
            ProcessStatus::OutputLimitExceeded
        } else if outcome.cancelled {
            ProcessStatus::Stopped
        } else {
            ProcessStatus::Exited
        };
        Ok(true)
    }
}

#[cfg(test)]
#[path = "read_service_tests.rs"]
mod tests;
