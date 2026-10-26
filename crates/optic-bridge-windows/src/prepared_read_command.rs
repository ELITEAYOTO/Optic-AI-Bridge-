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
    // Positive native Job accounting observed other active members after root exit.
    // Used for a synthetic regression only; a start marker is insufficient.
    post_root_active_observed: bool,
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
            post_root_active_observed: false,
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
    /// Acceptable for service ownership only while never started and wired to
    /// the exact session stop/revoke atomics. A caller cannot substitute an
    /// unrelated owner that would ignore revocation or carry an existing child.
    pub fn matches_unstarted_owner_signals(
        &self,
        stop: &Arc<AtomicBool>,
        revoked: &Arc<AtomicBool>,
        lease_revoked: Option<&Arc<AtomicBool>>,
    ) -> bool {
        !self.attempted
            && self.child.is_none()
            && self.stop_flags.len() == 2 + usize::from(lease_revoked.is_some())
            && Arc::ptr_eq(&self.stop_flags[0], stop)
            && Arc::ptr_eq(&self.stop_flags[1], revoked)
            && lease_revoked.is_none_or(|signal| Arc::ptr_eq(&self.stop_flags[2], signal))
    }

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
            // A successful root status does not authorize successful completion
            // when native accounting still shows active members of its Job.
            // Explicitly record that termination of the remaining tree was
            // requested: even if it succeeds, the whole command did not.
            self.post_root_active_observed = true;
            self.cancelled = true;
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
    /// Read-only native Job accounting for independent owner/regression checks.
    /// A nonzero count is NOT itself a command success or termination receipt.
    pub fn native_active_processes(&self) -> Result<u32> {
        self.job.active_processes()
    }

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
    use std::{env, fs, io::Write, thread};
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
    fn synthetic_native_read_fixture() {
        if env::var_os("OPTIC317_PREPARED_SYNTHETIC").is_none() {
            return;
        }
        println!("OPTIC317_PREPARED_CAPTURED_STDOUT");
        eprintln!("OPTIC317_PREPARED_CAPTURED_STDERR");
    }

    #[test]
    fn prepared_owner_proves_root_job_empty_and_both_pipe_eofs() {
        let mut owner = prepared();
        let executable = env::current_exe().expect("synthetic test binary");
        let mut command = Command::new(executable);
        command
            .args([
                "--exact",
                "job::prepared_read_command::tests::synthetic_native_read_fixture",
                "--nocapture",
            ])
            .env_clear()
            .env("OPTIC317_PREPARED_SYNTHETIC", "1");
        if let Some(root) = env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        owner
            .start(&mut command)
            .expect("create suspended assigned child");
        let started = Instant::now();
        while !owner.poll_terminal().expect("native/pipe observation") {
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "synthetic native owner failed to prove quiescence"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let outcome = owner
            .take_outcome()
            .expect("terminal native ownership proof");
        assert!(outcome.is_success(), "normal synthetic child: {outcome:?}");
        assert!(
            String::from_utf8_lossy(&outcome.stdout).contains("OPTIC317_PREPARED_CAPTURED_STDOUT")
        );
        assert!(
            String::from_utf8_lossy(&outcome.stderr).contains("OPTIC317_PREPARED_CAPTURED_STDERR")
        );
        assert!(owner.take_outcome().is_err(), "outcome is single-use");
    }

    #[test]
    fn synthetic_root_that_leaves_child_holding_pipes() {
        if env::var_os("OPTIC317_PREPARED_ROOT").is_none() {
            return;
        }
        let mut descendant = Command::new(env::current_exe().expect("test fixture executable"));
        descendant
            .args([
                "--exact",
                "job::prepared_read_command::tests::synthetic_descendant_retains_pipes",
                "--nocapture",
            ])
            .env("OPTIC317_PREPARED_DESCENDANT", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let _owned_descendant = descendant.spawn().expect("spawn disposable descendant");
        let gate = std::path::PathBuf::from(
            env::var_os("OPTIC317_PREPARED_GATE").expect("synthetic handshake path"),
        );
        let until = Instant::now() + Duration::from_millis(1000);
        while !gate.is_file() {
            assert!(
                Instant::now() < until,
                "descendant did not confirm it had started"
            );
            thread::sleep(Duration::from_millis(5));
        }
        println!("OPTIC317_PREPARED_ROOT_EXITED");
        std::io::stdout()
            .flush()
            .expect("flush synthetic root output");
        std::process::exit(0);
    }

    #[test]
    fn synthetic_descendant_retains_pipes() {
        if env::var_os("OPTIC317_PREPARED_DESCENDANT").is_none() {
            return;
        }
        println!("OPTIC317_PREPARED_DESCENDANT_STARTED");
        std::io::stdout()
            .flush()
            .expect("flush synthetic descendant output");
        let gate = std::path::PathBuf::from(
            env::var_os("OPTIC317_PREPARED_GATE").expect("synthetic handshake path"),
        );
        fs::write(&gate, b"synthetic descendant confirmed alive")
            .expect("write disposable synthetic gate");
        thread::sleep(Duration::from_millis(2500));
    }

    #[test]
    fn native_terminal_receipt_after_descendant_start_marker() {
        let mut owner = PreparedReadCommand::prepare(
            LimitedJobObject::new(4, 512 * 1024 * 1024, 25).expect("limits"),
            16 * 1024,
            1600,
            vec![],
        )
        .expect("prepare before child spawn");
        let gate = env::temp_dir().join(format!(
            "optic317-prepared-native-descendant-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&gate);
        let mut command = Command::new(env::current_exe().expect("test fixture executable"));
        command
            .args([
                "--exact",
                "job::prepared_read_command::tests::synthetic_root_that_leaves_child_holding_pipes",
                "--nocapture",
            ])
            .env_clear()
            .env("OPTIC317_PREPARED_ROOT", "1")
            .env("OPTIC317_PREPARED_GATE", &gate);
        if let Some(root) = env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        let started = Instant::now();
        owner.start(&mut command).expect("suspended contained root");
        while !owner.poll_terminal().expect("native Job/pipe observation") {
            assert!(
                started.elapsed() < Duration::from_millis(2100),
                "native Job/pipes not confirmed before synthetic descendant's 2500ms natural exit"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let outcome = owner.take_outcome().expect("native terminal evidence");
        assert!(
            started.elapsed() < Duration::from_millis(2100),
            "retained synthetic child exceeded controlled deadline"
        );
        assert!(
            String::from_utf8_lossy(&outcome.stdout).contains("OPTIC317_PREPARED_ROOT_EXITED"),
            "root fixture did not demonstrate a successful early exit"
        );
        assert!(
            String::from_utf8_lossy(&outcome.stdout)
                .contains("OPTIC317_PREPARED_DESCENDANT_STARTED"),
            "descendant must be positively observed before any ownership claim"
        );
        println!(
            "ISSUE317_PREPARED_NATIVE_EVIDENCE root={:?} cancelled={} deadline_expired={} active_after_terminal={} out_eof={} err_eof={} elapsed_ms={} descendant_marked={}",
            outcome.status,
            outcome.cancelled,
            outcome.deadline_expired,
            owner.job.active_processes().expect("native Job accounting"),
            owner.out_eof,
            owner.err_eof,
            started.elapsed().as_millis(),
            String::from_utf8_lossy(&outcome.stdout)
                .contains("OPTIC317_PREPARED_DESCENDANT_STARTED")
        );
        // A startup marker alone cannot prove post-root lifetime. The
        // stronger evidence is the *native Job accounting observation* made
        // after the root process returned a terminal status.
        assert!(
            owner.post_root_active_observed,
            "no native evidence of a still-active Job after root exit"
        );
        assert!(
            outcome.cancelled,
            "forced post-root Job stop must be classified"
        );
        assert!(
            !outcome.is_success(),
            "root exit 0 must not mask native termination of remaining Job members"
        );
        // Do not release terminal ownership based on the request alone.
        assert_eq!(
            owner
                .job
                .active_processes()
                .expect("native terminal accounting"),
            0,
            "native Job must be confirmed empty"
        );
        assert!(
            owner.out_eof && owner.err_eof,
            "both inherited pipes must reach EOF"
        );
        fs::remove_file(gate).expect("remove synthetic gate marker");
    }

    #[test]
    fn pre_spawn_revocation_prevents_native_child_creation() {
        let stop = Arc::new(AtomicBool::new(false));
        let mut owner = PreparedReadCommand::prepare(
            LimitedJobObject::new(4, 512 * 1024 * 1024, 25).expect("limits"),
            16 * 1024,
            1500,
            vec![Arc::clone(&stop)],
        )
        .expect("own Job and pipes before launch");
        // Equivalent to session revocation after preparation but before the
        // service authorizes native creation: no child must be created.
        stop.store(true, Ordering::Release);
        let mut command = Command::new(env::current_exe().expect("test binary"));
        command
            .args([
                "--exact",
                "job::prepared_read_command::tests::synthetic_native_read_fixture",
                "--nocapture",
            ])
            .env_clear()
            .env("OPTIC317_PREPARED_SYNTHETIC", "1");
        let error = owner
            .start(&mut command)
            .expect_err("revoked reservation must fail closed");
        assert_eq!(error.kind(), ErrorKind::Interrupted);
        assert!(owner.child.is_none(), "no child should exist after veto");
        assert_eq!(owner.job.active_processes().expect("Job accounting"), 0);
        let started = Instant::now();
        while !owner.poll_terminal().expect("post-veto native proof") {
            assert!(
                started.elapsed() < Duration::from_millis(900),
                "pre-spawn native cleanup must finish in finite time"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let outcome = owner.take_outcome().expect("terminal no-spawn record");
        assert!(outcome.cancelled && !outcome.is_success());
        assert!(outcome.status.is_none(), "no root process was created");
        assert!(outcome.stdout.is_empty() && outcome.stderr.is_empty());
        assert!(owner.out_eof && owner.err_eof);
    }

    #[test]
    fn external_stop_signal_terminates_registered_native_child_and_pipes() {
        let stop = Arc::new(AtomicBool::new(false));
        let mut owner = PreparedReadCommand::prepare(
            LimitedJobObject::new(4, 512 * 1024 * 1024, 25).expect("limits"),
            16 * 1024,
            3000,
            vec![Arc::clone(&stop)],
        )
        .expect("prepare owner before child creation");
        let gate = env::temp_dir().join(format!("optic317-prepared-revoke-{}", std::process::id()));
        let _ = fs::remove_file(&gate);
        let mut command = Command::new(env::current_exe().expect("test binary"));
        command
            .args([
                "--exact",
                "job::prepared_read_command::tests::synthetic_descendant_retains_pipes",
                "--nocapture",
            ])
            .env_clear()
            .env("OPTIC317_PREPARED_DESCENDANT", "1")
            .env("OPTIC317_PREPARED_GATE", &gate);
        if let Some(root) = env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        owner
            .start(&mut command)
            .expect("start owned native fixture");
        let handshake = Instant::now();
        while !gate.is_file() {
            assert!(
                handshake.elapsed() < Duration::from_millis(1200),
                "native child must prove it started before external stop"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            owner.job.active_processes().expect("active native child"),
            1,
            "the native child must still be running when revocation arrives"
        );
        let cancelled_at = Instant::now();
        stop.store(true, Ordering::Release);
        while !owner
            .poll_terminal()
            .expect("stop and native EOF observation")
        {
            assert!(
                cancelled_at.elapsed() < Duration::from_millis(1300),
                "stopped native child cannot hold both pipes until its 2500ms natural exit"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let outcome = owner.take_outcome().expect("confirmed native cleanup");
        assert!(outcome.cancelled && !outcome.is_success());
        assert!(
            !outcome.deadline_expired,
            "test requires external cancellation before the 3000ms deadline"
        );
        assert_eq!(owner.job.active_processes().expect("Job empty"), 0);
        assert!(
            owner.out_eof && owner.err_eof,
            "both native pipes must reach EOF"
        );
        assert!(
            String::from_utf8_lossy(&outcome.stdout)
                .contains("OPTIC317_PREPARED_DESCENDANT_STARTED"),
            "the already-running native child must have actually executed"
        );
        fs::remove_file(gate).expect("remove disposable native stop marker");
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
