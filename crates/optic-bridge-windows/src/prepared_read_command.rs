//! Research-stage native lifetime primitive; not wired to production Git.
//! Stdin is always null. Policy, executable pins and reservations belong to
//! the service. Keep this owner registered until native AND pipe proof succeeds.
use super::{LimitedJobObject, NativeJobDeadline, OwnedJob, resume_process_threads};
use crate::CapturedOutputPipe;
use std::{
    io::{Error, ErrorKind, Result},
    os::windows::{io::AsRawHandle, process::CommandExt},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::HANDLE,
    System::{JobObjects::AssignProcessToJobObject, Threading::CREATE_SUSPENDED},
};

/// One prepared command. No process exists at preparation; adopt this value
/// into the service registry before start. Rejected/error starts retain it.
pub struct PreparedReadCommand {
    // Join the finite watchdog before closing the final owning Job handle.
    watchdog: NativeJobDeadline,
    job: OwnedJob,
    child: Option<Child>,
    out: CapturedOutputPipe,
    err: CapturedOutputPipe,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    output_limit: usize,
    output_exceeded: bool,
    out_eof: bool,
    err_eof: bool,
    attempted: bool,
    assigned: bool,
    stopped: bool,
    terminal: bool,
    outcome_taken: bool,
    deadline_expired: bool,
    cancelled: bool,
    status: Option<ExitStatus>,
    stop_flags: Vec<Arc<AtomicBool>>,
    until: Instant,
}

#[derive(Debug)]
pub struct ReadCommandOutcome {
    /// Raw root status, not proof that the overall operation succeeded.
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub deadline_expired: bool,
    pub output_limit_exceeded: bool,
    pub cancelled: bool,
}

impl ReadCommandOutcome {
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status.is_some_and(|status| status.success())
            && !self.cancelled
            && !self.deadline_expired
            && !self.output_limit_exceeded
    }
}

impl PreparedReadCommand {
    pub fn prepare(
        limits: LimitedJobObject,
        output_limit: usize,
        timeout_ms: u32,
        stop_flags: Vec<Arc<AtomicBool>>,
    ) -> Result<Self> {
        if output_limit == 0 || timeout_ms == 0 || stop_flags.len() > 4 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "finite command budgets required",
            ));
        }
        let job = OwnedJob::create(
            limits.active_process_limit,
            limits.job_memory_limit,
            limits.cpu_percent,
        )?;
        let out = CapturedOutputPipe::new()?;
        let err = CapturedOutputPipe::new()?;
        let until = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        let watchdog = NativeJobDeadline::new(&job, timeout_ms, stop_flags.clone())?;
        Ok(Self {
            watchdog,
            job,
            child: None,
            out,
            err,
            stdout: Vec::new(),
            stderr: Vec::new(),
            output_limit,
            output_exceeded: false,
            out_eof: false,
            err_eof: false,
            attempted: false,
            assigned: false,
            stopped: false,
            terminal: false,
            status: None,
            stop_flags,
            until,
            outcome_taken: false,
            deadline_expired: false,
            cancelled: false,
        })
    }

    fn stop_requested(&self) -> bool {
        self.stopped
            || Instant::now() >= self.until
            || self
                .stop_flags
                .iter()
                .any(|flag| flag.load(Ordering::Acquire))
    }

    /// The caller must supply an already-authorized structured command. No
    /// caller-supplied stdio/creation flags survive this boundary. Any returned
    /// error can own a suspended child: retain and recover this SAME owner.
    pub fn start(&mut self, command: &mut Command) -> Result<()> {
        self.start_with_operations(
            command,
            |job, process| {
                // SAFETY: retained owned handles, root created suspended below.
                unsafe { AssignProcessToJobObject(job, process) }.map_err(Error::other)
            },
            resume_process_threads,
        )
    }

    // Private fault seams; production can select only the operations above.
    fn start_with_operations(
        &mut self,
        command: &mut Command,
        assign: impl FnOnce(HANDLE, HANDLE) -> Result<()>,
        resume: impl FnOnce(HANDLE) -> Result<()>,
    ) -> Result<()> {
        if self.attempted {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                "command start already attempted",
            ));
        }
        self.attempted = true;
        let result = (|| {
            if self.stop_requested() {
                return Err(Error::new(
                    ErrorKind::Interrupted,
                    "command stopped before creation",
                ));
            }
            command
                .stdin(Stdio::null())
                .stdout(Stdio::from(self.out.writer_handle()?.try_clone_to_owned()?))
                .stderr(Stdio::from(self.err.writer_handle()?.try_clone_to_owned()?))
                .creation_flags(CREATE_SUSPENDED.0);
            self.child = Some(command.spawn()?);
            let process = HANDLE(self.child.as_ref().expect("retained child").as_raw_handle());
            assign(self.job.raw(), process)?;
            self.assigned = true;
            if self.stop_requested() {
                return Err(Error::new(
                    ErrorKind::Interrupted,
                    "command stopped before resume",
                ));
            }
            resume(process)
        })();
        // Command retains configured Stdio handles for reuse, even on failed
        // spawn. Remove those duplicates too, otherwise our own EOF can stall.
        command.stdout(Stdio::null()).stderr(Stdio::null());
        self.out.close_parent_writer();
        self.err.close_parent_writer();
        result
    }

    /// A request only. Errors do not clear native handles or capacity.
    pub fn request_stop(&mut self) -> Result<()> {
        self.cancelled = true;
        self.stop_native()
    }

    fn stop_native(&mut self) -> Result<()> {
        self.request_stop_with(|job| {
            // SAFETY: this exact owner retains the Job handle.
            unsafe { windows::Win32::System::JobObjects::TerminateJobObject(job, 1) }
                .map_err(Error::other)
        })
    }

    fn request_stop_with(&mut self, terminate: impl FnOnce(HANDLE) -> Result<()>) -> Result<()> {
        self.stopped = true;
        self.attempted = true;
        self.out.close_parent_writer();
        self.err.close_parent_writer();
        let result = terminate(self.job.raw());
        // A failed assignment can leave the owned root suspended outside Job.
        // It cannot execute; still retain/kill/query that exact Child handle.
        if !self.assigned
            && let Some(child) = &mut self.child
        {
            self.status = child.try_wait()?;
            if self.status.is_none() {
                child.kill()?;
            }
        }
        result
    }

    fn capture(&mut self) -> Result<()> {
        let mut used = self.stdout.len() + self.stderr.len();
        for (pipe, bytes, eof) in [
            (&mut self.out, &mut self.stdout, &mut self.out_eof),
            (&mut self.err, &mut self.stderr, &mut self.err_eof),
        ] {
            if *eof {
                continue;
            }
            let chunk = pipe.try_read(8192)?;
            // One combined ceiling across both outputs, never one per pipe.
            let remaining = self.output_limit.saturating_sub(used);
            let accepted = remaining.min(chunk.bytes.len());
            self.output_exceeded |= chunk.bytes.len() > remaining;
            bytes.try_reserve_exact(accepted).map_err(Error::other)?;
            bytes.extend_from_slice(&chunk.bytes[..accepted]);
            used += accepted;
            *eof = chunk.eof;
        }
        Ok(())
    }

    /// One bounded polling step. No reader/writer thread and no pipe join.
    /// Native/pipe errors retain all resources; caller must retry or quarantine.
    pub fn poll_terminal(&mut self) -> Result<bool> {
        if self.terminal {
            return Ok(true);
        }
        self.deadline_expired |=
            Instant::now() >= self.until || self.watchdog.expired.load(Ordering::Acquire);
        self.cancelled |= self
            .stop_flags
            .iter()
            .any(|flag| flag.load(Ordering::Acquire));
        if self.stop_requested() || self.output_exceeded {
            self.stop_native()?;
        }
        self.capture()?;
        if self.output_exceeded {
            self.stop_native()?;
        }
        if self.status.is_none()
            && let Some(child) = &mut self.child
        {
            self.status = child.try_wait()?;
        }
        let active = self.job.active_processes()?;
        // The command's root completed; background descendants are not an
        // independent authorized operation and must not retain its pipes.
        if self.status.is_some() && active != 0 {
            self.stop_native()?;
        }
        self.terminal = self.attempted
            && (self.child.is_none() || self.status.is_some())
            && active == 0
            && self.out_eof
            && self.err_eof;
        Ok(self.terminal)
    }

    /// Only after full proof. A no-spawn failure has status None, not success.
    pub fn take_outcome(&mut self) -> Result<ReadCommandOutcome> {
        if !self.terminal || self.outcome_taken {
            return Err(Error::other(
                "native/pipe termination unproven or outcome already taken",
            ));
        }
        self.outcome_taken = true;
        Ok(ReadCommandOutcome {
            status: self.status,
            stdout: std::mem::take(&mut self.stdout),
            stderr: std::mem::take(&mut self.stderr),
            deadline_expired: self.deadline_expired,
            output_limit_exceeded: self.output_exceeded,
            cancelled: self.cancelled,
        })
    }
}

impl Drop for PreparedReadCommand {
    fn drop(&mut self) {
        // Containment backstop only. This is NOT terminal proof; a service must
        // never drop an uncertain record. No pipe workers exist to detach.
        if !self.terminal {
            let _ = self.stop_native();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, thread};
    use windows::Win32::System::{
        JobObjects::TerminateJobObject,
        Threading::{GetCurrentProcess, GetProcessHandleCount},
    };

    fn handles() -> u32 {
        let mut count = 0;
        // SAFETY: pseudohandle for this process; writable stack count.
        unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) }.unwrap();
        count
    }

    #[test]
    fn zero_root_exit_cannot_hide_cancellation_or_limits() {
        use std::os::windows::process::ExitStatusExt;
        let mut outcome = ReadCommandOutcome {
            status: Some(ExitStatus::from_raw(0)),
            stdout: vec![],
            stderr: vec![],
            deadline_expired: false,
            output_limit_exceeded: false,
            cancelled: false,
        };
        assert!(outcome.is_success());
        outcome.cancelled = true;
        assert!(!outcome.is_success());
        outcome.cancelled = false;
        outcome.deadline_expired = true;
        assert!(!outcome.is_success());
        outcome.deadline_expired = false;
        outcome.output_limit_exceeded = true;
        assert!(!outcome.is_success());
        outcome.output_limit_exceeded = false;
        outcome.status = None;
        assert!(!outcome.is_success());
    }

    fn prepared() -> PreparedReadCommand {
        PreparedReadCommand::prepare(
            LimitedJobObject::new(4, 512 * 1024 * 1024, 25).unwrap(),
            16384,
            8000,
            vec![],
        )
        .unwrap()
    }

    fn confirm(owner: &mut PreparedReadCommand) -> ReadCommandOutcome {
        let until = Instant::now() + Duration::from_secs(2);
        while !owner.poll_terminal().unwrap() {
            assert!(Instant::now() < until, "native recovery unconfirmed");
            thread::sleep(Duration::from_millis(5));
        }
        owner.take_outcome().unwrap()
    }

    #[test]
    #[ignore = "requires independent native Job supervisor"]
    fn native_partial_failure_matrix() {
        let gate = std::path::PathBuf::from(
            env::var_os("OPTIC317_REGISTRY_GATE").expect("supervisor gate"),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while !gate.exists() {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(5));
        }
        // Warm the standard-library thread bookkeeping before handle baseline.
        {
            let mut warm = prepared();
            let mut command = Command::new(env::var_os("OPTIC317_PYTHON").unwrap());
            command
                .arg(env::var_os("OPTIC317_FIXTURE").unwrap())
                .arg("normal")
                .env_clear();
            if let Some(value) = env::var_os("SystemRoot") {
                command.env("SystemRoot", value);
            }
            warm.start(&mut command).unwrap();
            assert!(confirm(&mut warm).status.unwrap().success());
        }
        let before = handles();
        for case in [
            "native-assignment-failure",
            "native-resume-failure",
            "native-termination-failure",
        ] {
            let start = Instant::now();
            let mut owner = prepared();
            let mut command = Command::new(env::var_os("OPTIC317_PYTHON").unwrap());
            command
                .arg(env::var_os("OPTIC317_FIXTURE").unwrap())
                .arg("hold")
                .env_clear();
            if let Some(value) = env::var_os("SystemRoot") {
                command.env("SystemRoot", value);
            }
            let result = match case {
                "native-assignment-failure" => owner.start_with_operations(
                    &mut command,
                    |job, _| {
                        // SAFETY: deliberately invalid process handle; Windows must
                        // reject it. Actual suspended root stays owned in `owner`.
                        unsafe { AssignProcessToJobObject(job, HANDLE::default()) }
                            .map_err(Error::other)
                    },
                    resume_process_threads,
                ),
                "native-resume-failure" => owner.start_with_operations(
                    &mut command,
                    |job, process| {
                        // SAFETY: actual retained Job and suspended root handles.
                        unsafe { AssignProcessToJobObject(job, process) }.map_err(Error::other)
                    },
                    |_| resume_process_threads(HANDLE::default()),
                ),
                _ => owner.start(&mut command),
            };
            if case == "native-termination-failure" {
                result.unwrap();
                assert_eq!(owner.job.active_processes().unwrap(), 1);
                assert!(
                    owner
                        .request_stop_with(|_| {
                            // SAFETY: invalid Job handle injection; no other process
                            // or object is named or affected by this rejected call.
                            unsafe { TerminateJobObject(HANDLE::default(), 1) }
                                .map_err(Error::other)
                        })
                        .is_err()
                );
                assert_eq!(
                    owner.job.active_processes().unwrap(),
                    1,
                    "failed termination cannot count as proof"
                );
                assert!(owner.take_outcome().is_err());
            } else {
                assert!(result.is_err());
                assert!(
                    owner.child.is_some(),
                    "native root retained on failed start"
                );
                assert!(owner.child.as_mut().unwrap().try_wait().unwrap().is_none());
            }
            owner.request_stop().unwrap();
            let outcome = confirm(&mut owner);
            assert!(outcome.status.is_some());
            assert!(outcome.stdout.is_empty() && outcome.stderr.is_empty());
            drop(owner);
            let after_owner = handles();
            drop(command);
            println!(
                "OPTIC317_PREPARED_HANDLES before={before} after_owner={after_owner} after_command={}",
                handles()
            );
            assert_eq!(
                handles(),
                before,
                "native owner/watchdog handles must be recovered"
            );
            println!(
                "OPTIC317_PREPARED_CASE {{\"case\":\"{case}\",\"elapsed_ms\":{},\"tree_empty_and_pipes_eof\":true,\"handles_before\":{before},\"handles_after\":{}}}",
                start.elapsed().as_millis(),
                handles()
            );
        }
        fs::write(
            gate.with_extension("native-complete"),
            b"native failures recovered",
        )
        .unwrap();
        println!("OPTIC317_REGISTRY_NATIVE native_failure_cases=3 handles_recovered=true");
    }
}
