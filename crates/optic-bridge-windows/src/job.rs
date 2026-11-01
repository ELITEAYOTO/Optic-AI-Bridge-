use std::{
    ffi::c_void,
    fs::File,
    future::Future,
    io::{Error, ErrorKind, Result},
    mem::size_of,
    os::windows::io::{AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    pin::Pin,
    process::ExitStatus,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use crate::output_pipe::CapturedOutputPipe;
use process_wrap::tokio::{ChildWrapper, CommandWrap, CommandWrapper};
use tokio::process::Command;
use windows::{
    Win32::{
        Foundation::{
            CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_NO_MORE_FILES, HANDLE,
            WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
                JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
                JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
                JOBOBJECT_CPU_RATE_CONTROL_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JobObjectBasicAccountingInformation, JobObjectCpuRateControlInformation,
                JobObjectExtendedLimitInformation, QueryInformationJobObject,
                SetInformationJobObject, TerminateJobObject,
            },
            Threading::{
                CREATE_NO_WINDOW, CREATE_SUSPENDED, GetCurrentProcess, GetProcessId, OpenThread,
                ResumeThread, THREAD_SUSPEND_RESUME, WaitForSingleObject,
            },
        },
    },
    core::{BOOL, HRESULT},
};

const JOB_EMPTY_POLL: Duration = Duration::from_millis(10);

/// Windows Job Object containment configured from an already-authorized process budget.
///
/// The wrapper forces temporary suspended creation, creates/configures the Job Object,
/// assigns the child, and only then resumes the child threads. This closes the ordinary
/// spawn-before-assignment window while keeping the public process API structured.
#[derive(Clone, Copy, Debug)]
pub struct LimitedJobObject {
    active_process_limit: u32,
    job_memory_limit: usize,
    cpu_percent: u32,
    without_console: bool,
}

impl LimitedJobObject {
    pub fn new(active_process_limit: u32, job_memory_limit: u64, cpu_percent: u32) -> Result<Self> {
        if active_process_limit == 0
            || job_memory_limit == 0
            || cpu_percent == 0
            || cpu_percent > 100
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Windows Job Object limits must be non-zero and CPU percent must be at most 100",
            ));
        }
        let job_memory_limit = usize::try_from(job_memory_limit).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                "Windows Job Object memory limit does not fit the platform address size",
            )
        })?;
        Ok(Self {
            active_process_limit,
            job_memory_limit,
            cpu_percent,
            without_console: false,
        })
    }

    /// Keep a private non-interactive helper out of its parent's console.
    /// This changes no Job limit or suspension/assignment/resume ordering.
    #[must_use]
    pub fn without_console(mut self) -> Self {
        self.without_console = true;
        self
    }
}

impl CommandWrapper for LimitedJobObject {
    fn pre_spawn(&mut self, command: &mut Command, _core: &CommandWrap) -> Result<()> {
        let flags = CREATE_SUSPENDED.0
            | if self.without_console {
                CREATE_NO_WINDOW.0
            } else {
                0
            };
        command.creation_flags(flags);
        Ok(())
    }

    fn wrap_child(
        &mut self,
        mut inner: Box<dyn ChildWrapper>,
        _core: &CommandWrap,
    ) -> Result<Box<dyn ChildWrapper>> {
        let process = child_process_handle(inner.as_ref()).ok_or_else(|| {
            Error::new(
                ErrorKind::Unsupported,
                "child wrapper does not expose a Windows process handle",
            )
        })?;
        let process = HANDLE(process.as_raw_handle());

        let job = match OwnedJob::create(
            self.active_process_limit,
            self.job_memory_limit,
            self.cpu_percent,
        ) {
            Ok(job) => job,
            Err(error) => {
                let _ = inner.start_kill();
                return Err(error);
            }
        };

        // SAFETY: `job.raw()` is an owned Job Object handle and `process` is the live
        // child process handle borrowed from `inner`. Both remain valid for this call.
        if let Err(error) = unsafe { AssignProcessToJobObject(job.raw(), process) } {
            let _ = inner.start_kill();
            return Err(Error::other(error));
        }

        if let Err(error) = resume_process_threads(process) {
            let _ = job.terminate(1);
            let _ = inner.start_kill();
            return Err(error);
        }

        Ok(Box::new(LimitedJobChild {
            inner,
            job,
            root_status: None,
        }))
    }
}

#[derive(Debug)]
struct OwnedJob(HANDLE);

// SAFETY: Win32 kernel object handles are process-wide values that may be used from
// different threads. Ownership remains unique in `OwnedJob`, and Drop closes it once.
unsafe impl Send for OwnedJob {}
// SAFETY: operations used through `&OwnedJob` do not mutate Rust memory and the kernel
// synchronizes access to the underlying Job Object.
unsafe impl Sync for OwnedJob {}

impl OwnedJob {
    fn create(
        active_process_limit: u32,
        job_memory_limit: usize,
        cpu_percent: u32,
    ) -> Result<Self> {
        // SAFETY: null security attributes/name request a new unnamed Job Object.
        let handle = unsafe { CreateJobObjectW(None, None) }.map_err(Error::other)?;
        let job = Self(handle);

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_JOB_MEMORY;
        limits.BasicLimitInformation.ActiveProcessLimit = active_process_limit;
        limits.JobMemoryLimit = job_memory_limit;

        // SAFETY: the information class matches the concrete structure and its exact size;
        // `job` owns a valid Job Object handle for the duration of the call.
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(Error::other)?;

        let mut cpu = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
            ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
            ..Default::default()
        };
        cpu.Anonymous.CpuRate = cpu_percent * 100;
        // SAFETY: the CPU information class matches the concrete structure and exact size;
        // the hard cap is configured before any child is assigned or resumed.
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectCpuRateControlInformation,
                (&cpu as *const JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            )
        }
        .map_err(Error::other)?;

        Ok(job)
    }

    const fn raw(&self) -> HANDLE {
        self.0
    }

    fn terminate(&self, exit_code: u32) -> Result<()> {
        // SAFETY: `self.0` is the valid Job Object handle uniquely owned by this value.
        unsafe { TerminateJobObject(self.0, exit_code) }.map_err(Error::other)
    }

    fn active_processes(&self) -> Result<u32> {
        query_active_processes(self.0)
    }
}

fn query_active_processes(job: HANDLE) -> Result<u32> {
    let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
    // SAFETY: the requested information class matches `accounting` and the buffer
    // remains valid for the entire synchronous call.
    unsafe {
        QueryInformationJobObject(
            Some(job),
            JobObjectBasicAccountingInformation,
            (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast::<c_void>(),
            size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
            None,
        )
    }
    .map_err(job_query_io_error)?;
    Ok(accounting.ActiveProcesses)
}

fn job_query_io_error(error: windows::core::Error) -> Error {
    // Win32 failures in windows-rs are wrapped as FACILITY_WIN32 HRESULTs.
    // Recover only an explicitly recognized Win32 mapping. For any other
    // HRESULT preserve the entire code rather than truncating its low bits.
    match windows::Win32::Foundation::WIN32_ERROR::from_error(&error) {
        Some(code) => Error::from_raw_os_error(code.0 as i32),
        None => Error::from(error),
    }
}

impl Drop for OwnedJob {
    fn drop(&mut self) {
        // SAFETY: this value owns the handle and Drop runs once. KILL_ON_JOB_CLOSE makes
        // closing the final Job Object handle the containment backstop for descendants.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Inert Windows GitRead custody: real kernel Job, bounded anonymous pipes and
/// a nulled stdin are opened *before* any root could exist. This type has no
/// create/spawn/resume method and is never proof of a completed child.
#[must_use = "retain native handles until unstarted cleanup or later terminal proof"]
/// A service-pinned original root HANDLE, not a PID lookup or a launch grant.
struct PinnedOriginalRoot {
    handle: OwnedHandle,
    pid: u32,
}

/// Success certifies only four physically observed facts for the pinned root:
/// root exit, kernel Job empty, physical stdout EOF and physical stderr EOF.
/// The counts are bytes discarded while completing *cleanup*, never command
/// output exposed to an MCP caller. This is NOT a no-future-create/resume fence.
#[derive(Debug)]
#[must_use = "retain the completed physical observations with their original custody"]
pub struct NativePhysicalStopReceipt {
    // An external caller must not be able to construct an arbitrary success
    // receipt by filling in public fields. Only the four-fact OS observation
    // inside PrecreatedNativeReadCustody may construct this type.
    root_pid: u32,
    discarded_stdout_bytes: u64,
    discarded_stderr_bytes: u64,
}

impl NativePhysicalStopReceipt {
    #[must_use]
    pub fn root_pid(&self) -> u32 {
        self.root_pid
    }

    #[must_use]
    pub fn discarded_stdout_bytes(&self) -> u64 {
        self.discarded_stdout_bytes
    }

    #[must_use]
    pub fn discarded_stderr_bytes(&self) -> u64 {
        self.discarded_stderr_bytes
    }
}

pub struct PrecreatedNativeReadCustody {
    // Disarm and join the independent watchdog before closing the owned Job.
    // A watchdog is optional until a future trusted native launch owner arms it.
    deadline: Option<NativeJobDeadline>,
    // The Job uses KILL_ON_JOB_CLOSE. No process can be added through this API.
    job: OwnedJob,
    original_root: Option<PinnedOriginalRoot>,
    _stdin: File,
    _stdout: CapturedOutputPipe,
    _stderr: CapturedOutputPipe,
}

impl PrecreatedNativeReadCustody {
    /// Pre-create exact Win32 ownership under an already configured resource
    /// ceiling. If an intermediate open fails, RAII closes all prior handles.
    /// No child or Job assignment occurs on this path.
    pub fn new(limits: LimitedJobObject) -> Result<Self> {
        let job = OwnedJob::create(
            limits.active_process_limit,
            limits.job_memory_limit,
            limits.cpu_percent,
        )?;
        let stdout = CapturedOutputPipe::new()?;
        let stderr = CapturedOutputPipe::new()?;
        let stdin = File::open("NUL")?;
        Ok(Self {
            deadline: None,
            job,
            original_root: None,
            _stdin: stdin,
            _stdout: stdout,
            _stderr: stderr,
        })
    }

    /// Install a nonrenewable watchdog on this *original* private kernel Job,
    /// before any future native launcher is allowed to use it. Its separate
    /// worker holds a noninheritable duplicate of the exact Job handle and
    /// observes up to four original authority flags, plus the native owner's
    /// separately retained internal fail-closed stop when explicitly bound.
    ///
    /// This type still cannot create, resume or admit a process. An armed
    /// watchdog is neither a cross-registry launch grant nor proof that a
    /// process tree or either output pipe has physically terminated. Future
    /// native custody must retain the Job and both streams until that proof.
    pub fn arm_watchdog(
        &mut self,
        timeout_ms: u32,
        stop_flags: Vec<Arc<AtomicBool>>,
    ) -> Result<()> {
        let until = std::time::Instant::now()
            .checked_add(Duration::from_millis(u64::from(timeout_ms)))
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "watchdog horizon overflow"))?;
        self.arm_watchdog_until(until, stop_flags)
    }

    /// Preserve a service-owned absolute horizon through handle duplication and
    /// worker startup. The existing caller-owned interface still rejects more
    /// than four original authority stops.
    pub fn arm_watchdog_until(
        &mut self,
        until: std::time::Instant,
        stop_flags: Vec<Arc<AtomicBool>>,
    ) -> Result<()> {
        if stop_flags.len() > 4 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "native read watchdog accepts at most four original stop flags",
            ));
        }
        self.arm_watchdog_until_owned(until, stop_flags)
    }

    /// The inert native GitRead owner supplies exactly its four retained
    /// original authority stops plus one private internal ledger stop. This
    /// fifth signal can only request original Job termination, not launch.
    /// The ordinary watchdog interface above keeps its existing four-source
    /// limit, so no unrelated caller gains a detached stop-source slot.
    pub fn arm_watchdog_until_with_internal_stop(
        &mut self,
        until: std::time::Instant,
        mut original_stops: Vec<Arc<AtomicBool>>,
        internal_stop: Arc<AtomicBool>,
    ) -> Result<()> {
        if original_stops.len() != 4 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "native read internal watchdog requires exactly four original stop flags",
            ));
        }
        original_stops.push(internal_stop);
        self.arm_watchdog_until_owned(until, original_stops)
    }

    fn arm_watchdog_until_owned(
        &mut self,
        until: std::time::Instant,
        stop_flags: Vec<Arc<AtomicBool>>,
    ) -> Result<()> {
        if until <= std::time::Instant::now() || self.deadline.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "native read watchdog requires an unarmed Job and a future horizon",
            ));
        }
        self.deadline = Some(NativeJobDeadline::new_until(&self.job, until, stop_flags)?);
        Ok(())
    }

    /// Worker observed stop/deadline and returned. This is NOT proof of
    /// successful Job termination, Job-empty, root exit or either pipe EOF.
    #[must_use]
    pub fn watchdog_stop_observed(&self) -> bool {
        self.deadline
            .as_ref()
            .and_then(|deadline| deadline.worker.as_ref())
            .is_some_and(std::thread::JoinHandle::is_finished)
    }

    /// Exact completed TerminateJobObject attempt on the original Job. None
    /// means no result has been published (including an unarmed watchdog).
    /// Err retains the Windows HRESULT without converting it into success.
    /// Even Ok is NOT root exit, Job-empty, pipe EOF or future launch closure.
    #[must_use]
    pub fn watchdog_termination_result(&self) -> Option<std::result::Result<(), i32>> {
        self.deadline
            .as_ref()
            .and_then(|deadline| deadline.termination_result.get().copied())
    }

    /// The watchdog's deadline flag is a timeout observation, not a Job-empty,
    /// root-exit, pipe-EOF or successful TerminateJobObject receipt.
    #[must_use]
    pub fn watchdog_deadline_expired(&self) -> bool {
        self.deadline
            .as_ref()
            .is_some_and(|deadline| deadline.expired.load(Ordering::Acquire))
    }

    /// Retain the exact root HANDLE at its first verified Job-associated,
    /// still-alive observation. The caller owns the original CreateProcessW
    /// handles; this owner holds a noninheritable duplicate until Drop.
    ///
    /// This method NEVER issues CreateProcessW/ResumeThread, permits execution
    /// or makes a post-revocation suspended creation safe. A future service
    /// launcher must separately synchronize original generation revocation
    /// before it can resume even a successfully pinned child.
    pub fn pin_original_job_root(&mut self, root: BorrowedHandle<'_>) -> Result<()> {
        if self.original_root.is_some() || self.deadline.is_none() || self.watchdog_stop_observed()
        {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "native root requires an unbound owner with its original armed watchdog",
            ));
        }
        let process = HANDLE(root.as_raw_handle());
        let mut belongs = BOOL(0);
        // SAFETY: root is borrowed and the exact configured Job is retained.
        unsafe { IsProcessInJob(process, Some(self.job.raw()), &mut belongs) }
            .map_err(Error::other)?;
        if !belongs.as_bool() {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "native root is not a member of its original Job",
            ));
        }
        // SAFETY: zero-duration wait interrogates the borrowed HANDLE only.
        if unsafe { WaitForSingleObject(process, 0) } != WAIT_TIMEOUT {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "native root is not a verified live Job member",
            ));
        }
        // SAFETY: the same borrowed HANDLE remains valid throughout this call.
        let pid = unsafe { GetProcessId(process) };
        if pid == 0 {
            return Err(Error::last_os_error());
        }
        let mut duplicate = HANDLE::default();
        // SAFETY: duplicate into the current process, explicitly noninheritable.
        // The original root HANDLE cannot be substituted by PID reuse.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                process,
                GetCurrentProcess(),
                &mut duplicate,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(Error::other)?;
        // SAFETY: a successful DuplicateHandle provides one unique owned value.
        self.original_root = Some(PinnedOriginalRoot {
            handle: unsafe { OwnedHandle::from_raw_handle(duplicate.0) },
            pid,
        });
        Ok(())
    }

    /// After the ORIGINAL Job watchdog reports a successful termination call,
    /// independently observe the retained root handle, kernel active count and
    /// each original pipe's physical EOF. Discard output in bounded chunks,
    /// never joining potentially stuck reader threads or buffering unbounded
    /// bytes. This method adds NO execution grace: the supplied cleanup horizon
    /// is absolute and cannot be renewed internally.
    ///
    /// Failures preserve ownership for the caller's fail-closed cleanup/Drop.
    /// In particular a completed TerminateJobObject call, an empty Job, an
    /// already signaled *foreign* root or only one EOF are never sufficient.
    /// Future CreateProcessW/ResumeThread must be sealed by a separate issuer.
    pub fn confirm_post_watchdog_physical_stop(
        &mut self,
        cleanup_until: Instant,
        discard_limit: u64,
    ) -> Result<NativePhysicalStopReceipt> {
        if discard_limit == 0 || cleanup_until <= Instant::now() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid native cleanup budget",
            ));
        }
        let root = self.original_root.as_ref().ok_or_else(|| {
            Error::new(
                ErrorKind::PermissionDenied,
                "no original root HANDLE pinned",
            )
        })?;
        if self.watchdog_termination_result() != Some(Ok(())) {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "original watchdog has not reported successful Job termination",
            ));
        }
        // Parent-held writer duplicates would prevent physical EOF even after
        // a successful kill. Closing them here is safe only after watchdog stop.
        self._stdout.close_parent_writer();
        self._stderr.close_parent_writer();
        let mut stdout_eof = false;
        let mut stderr_eof = false;
        let mut stdout_bytes = 0_u64;
        let mut stderr_bytes = 0_u64;
        loop {
            // SAFETY: a duplicated original root HANDLE stays valid for this
            // whole method; no PID lookup or substituted process is involved.
            let wait = unsafe { WaitForSingleObject(HANDLE(root.handle.as_raw_handle()), 0) };
            let root_exited = match wait {
                WAIT_OBJECT_0 => true,
                WAIT_TIMEOUT => false,
                _ => return Err(Error::last_os_error()),
            };
            let job_empty = self.job.active_processes()? == 0;
            if !stdout_eof {
                let chunk = self._stdout.try_read(8 * 1024)?;
                stdout_eof = chunk.eof;
                stdout_bytes = stdout_bytes
                    .checked_add(u64::try_from(chunk.bytes.len()).unwrap_or(u64::MAX))
                    .ok_or_else(|| Error::other("native stdout accounting overflow"))?;
            }
            if !stderr_eof {
                let chunk = self._stderr.try_read(8 * 1024)?;
                stderr_eof = chunk.eof;
                stderr_bytes = stderr_bytes
                    .checked_add(u64::try_from(chunk.bytes.len()).unwrap_or(u64::MAX))
                    .ok_or_else(|| Error::other("native stderr accounting overflow"))?;
            }
            if stdout_bytes
                .checked_add(stderr_bytes)
                .is_none_or(|total| total > discard_limit)
            {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "native physical cleanup exceeded its original discard ceiling",
                ));
            }
            if root_exited && job_empty && stdout_eof && stderr_eof {
                return Ok(NativePhysicalStopReceipt {
                    root_pid: root.pid,
                    discarded_stdout_bytes: stdout_bytes,
                    discarded_stderr_bytes: stderr_bytes,
                });
            }
            if Instant::now() >= cleanup_until {
                return Err(Error::new(
                    ErrorKind::TimedOut,
                    "native shutdown did not prove root exit, Job empty and BOTH pipe EOFs",
                ));
            }
            std::thread::sleep(JOB_EMPTY_POLL);
        }
    }

    /// Read the actual kernel Job's active-process count. Zero here is only
    /// a no-child precreation observation, not root-exit/EOF terminal evidence.
    pub fn active_processes(&self) -> Result<u32> {
        self.job.active_processes()
    }
}

#[cfg(test)]
mod precreated_native_read_tests {
    use super::*;

    #[test]
    fn unstarted_native_job_and_both_pipes_are_real_and_idle() {
        let limits =
            LimitedJobObject::new(1, 64 * 1024 * 1024, 10).expect("positive kernel limits");
        let mut owned = PrecreatedNativeReadCustody::new(limits)
            .expect("open actual Job and 3 stdin/out/err resources");
        assert_eq!(owned.active_processes().expect("real kernel accounting"), 0);
        let stdout = owned._stdout.try_read(64).expect("real stdout pipe");
        let stderr = owned._stderr.try_read(64).expect("real stderr pipe");
        assert!(stdout.bytes.is_empty() && !stdout.eof);
        assert!(stderr.bytes.is_empty() && !stderr.eof);
        // File::open("NUL") in the constructor proved stdin opened; device
        // handles need not support ordinary filesystem metadata queries.
        // No root existed; no EOF or OS termination receipt is claimed.
    }

    #[test]
    fn unstarted_native_read_watchdog_is_nonrenewable_and_deadline_bounded() {
        let limits =
            LimitedJobObject::new(1, 64 * 1024 * 1024, 10).expect("positive kernel limits");
        let mut owned =
            PrecreatedNativeReadCustody::new(limits).expect("real unstarted kernel custody");

        assert!(matches!(
            owned.arm_watchdog(0, Vec::new()),
            Err(ref error) if error.kind() == ErrorKind::InvalidInput
        ));
        let too_many = (0..5).map(|_| Arc::new(AtomicBool::new(false))).collect();
        assert!(matches!(
            owned.arm_watchdog(200, too_many),
            Err(ref error) if error.kind() == ErrorKind::InvalidInput
        ));
        assert!(!owned.watchdog_deadline_expired());
        assert_eq!(owned.watchdog_termination_result(), None);
        assert_eq!(owned.active_processes().expect("real Job accounting"), 0);

        let stop = Arc::new(AtomicBool::new(false));
        owned
            .arm_watchdog(75, vec![stop])
            .expect("arm once with an original owner stop signal");
        assert!(matches!(
            owned.arm_watchdog(1_000, Vec::new()),
            Err(ref error) if error.kind() == ErrorKind::InvalidInput
        ));

        // This checks the actual independent timer signal. No child exists:
        // it deliberately does NOT claim terminal process or pipe evidence.
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while !owned.watchdog_deadline_expired() {
            assert!(
                std::time::Instant::now() < until,
                "native Job watchdog deadline was not observed"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        while !owned.watchdog_stop_observed() {
            assert!(std::time::Instant::now() < until, "watchdog did not return");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(owned.watchdog_termination_result(), Some(Ok(())));
        assert_eq!(owned.active_processes().expect("real Job accounting"), 0);
    }

    #[test]
    fn original_stop_retains_completed_job_termination_result() {
        let limits = LimitedJobObject::new(1, 64 * 1024 * 1024, 10).expect("limits");
        let mut owned = PrecreatedNativeReadCustody::new(limits).expect("original Job");
        let stop = Arc::new(AtomicBool::new(true));
        owned
            .arm_watchdog(30_000, vec![stop])
            .expect("original stop");
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while !owned.watchdog_stop_observed() {
            assert!(
                std::time::Instant::now() < until,
                "stop worker did not return"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!owned.watchdog_deadline_expired());
        assert_eq!(owned.watchdog_termination_result(), Some(Ok(())));
        assert_eq!(owned.active_processes().expect("separate Job query"), 0);
        // Success on a physically EMPTY Job never certifies an original root
        // or both EOFs. The owner must retain a verified real root HANDLE.
        assert!(matches!(
            owned.confirm_post_watchdog_physical_stop(
                Instant::now() + Duration::from_secs(1),
                1024
            ),
            Err(ref e) if e.kind() == ErrorKind::PermissionDenied
        ));
    }

    #[test]
    fn watchdog_retains_real_access_denied_for_stop_and_expiry() {
        use windows::Win32::Foundation::ERROR_ACCESS_DENIED;
        // Win32 JOB_OBJECT_QUERY, from SystemServices in the SDK. Keep this
        // fixture-only mask local rather than enabling another binding feature.
        const JOB_OBJECT_QUERY: u32 = 0x0004;

        for stop_requested in [true, false] {
            let job = OwnedJob::create(1, 64 * 1024 * 1024, 10).expect("original empty Job");
            let mut duplicate = HANDLE::default();
            // SAFETY: duplicate our live fixture Job into this process, without
            // inheritance and with QUERY only. No ACL or original right changes.
            unsafe {
                DuplicateHandle(
                    GetCurrentProcess(),
                    job.raw(),
                    GetCurrentProcess(),
                    &mut duplicate,
                    JOB_OBJECT_QUERY,
                    false,
                    Default::default(),
                )
            }
            .expect("query-only Job duplicate");
            // Exactly this fresh handle is owned; retain the full original Job
            // separately so failure/drop cannot destroy its kernel object.
            let query_only = OwnedJob(duplicate);
            assert_eq!(query_only.active_processes().expect("QUERY right"), 0);
            let stop = Arc::new(AtomicBool::new(stop_requested));
            let deadline = NativeJobDeadline::new(
                &query_only,
                if stop_requested { 30_000 } else { 25 },
                vec![stop],
            )
            .expect("duplicate original restricted handle into real worker");
            let bound = std::time::Instant::now() + Duration::from_secs(5);
            while !deadline.worker.as_ref().unwrap().is_finished() {
                assert!(
                    std::time::Instant::now() < bound,
                    "denied worker must return"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            let denied = windows::core::HRESULT::from_win32(ERROR_ACCESS_DENIED.0).0;
            assert_eq!(deadline.termination_result.get(), Some(&Err(denied)));
            assert_eq!(deadline.expired.load(Ordering::Acquire), !stop_requested);
            assert_eq!(job.active_processes().expect("original Job still owned"), 0);
            // The denied receipt is retained after joining/cancelling the worker.
            let receipt = Arc::clone(&deadline.termination_result);
            drop(deadline);
            assert_eq!(receipt.get(), Some(&Err(denied)));
            assert_eq!(
                query_only
                    .active_processes()
                    .expect("restricted owner retained"),
                0
            );
        }
    }

    #[test]
    fn cancelled_unstarted_watchdog_does_not_publish_termination_success() {
        let job = OwnedJob::create(1, 64 * 1024 * 1024, 10).expect("original Job");
        let deadline = NativeJobDeadline::new(&job, 30_000, Vec::new()).expect("watchdog");
        let receipt = Arc::clone(&deadline.termination_result);
        drop(deadline); // cancel and join while the horizon is still future
        assert_eq!(receipt.get(), None);
        assert_eq!(job.active_processes().expect("separate Job query"), 0);
    }
}

/// Owns one AppContainer root and its bounded kernel process tree. Policy and
/// resource admission remain the caller's responsibility. No PID is accepted.
pub struct ContainedAppContainerProcess {
    // Disarm/join before the final owning job handle is closed.
    deadline: Option<NativeJobDeadline>,
    job: OwnedJob,
    child: crate::SuspendedAppContainerProcess,
}

impl ContainedAppContainerProcess {
    pub fn contain(
        child: crate::SuspendedAppContainerProcess,
        limits: LimitedJobObject,
    ) -> Result<Self> {
        if child.has_resumed() || !child.is_appcontainer()? {
            return Err(Error::other(
                "containment requires an owned suspended AppContainer root",
            ));
        }
        let job = OwnedJob::create(
            limits.active_process_limit,
            limits.job_memory_limit,
            limits.cpu_percent,
        )?;
        // SAFETY: consumes the uniquely owned, not-yet-resumed child. Kernel
        // limits are set before assignment and before any application code runs.
        unsafe {
            AssignProcessToJobObject(job.raw(), HANDLE(child.borrowed_process().as_raw_handle()))
        }
        .map_err(Error::other)?;
        Ok(Self {
            deadline: None,
            job,
            child,
        })
    }

    /// One nonrenewable deadline, armed while suspended. It requests termination
    /// independently of the owner worker's polling or filesystem operations.
    /// Managed admission must bound the number of simultaneously armed jobs.
    pub fn arm_deadline(&mut self, timeout_ms: u32) -> Result<()> {
        self.arm_watchdog(timeout_ms, Vec::new())
    }

    /// At most four already-owner-bound stop/revocation flags. No callbacks,
    /// policy I/O or locks run in the watchdog; its deadline cannot be renewed.
    pub fn arm_watchdog(
        &mut self,
        timeout_ms: u32,
        stop_flags: Vec<Arc<AtomicBool>>,
    ) -> Result<()> {
        if timeout_ms == 0
            || stop_flags.len() > 4
            || self.child.has_resumed()
            || self.deadline.is_some()
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "deadline requires one positive suspended-job budget",
            ));
        }
        self.deadline = Some(NativeJobDeadline::new(&self.job, timeout_ms, stop_flags)?);
        Ok(())
    }

    #[must_use]
    pub fn deadline_expired(&self) -> bool {
        self.deadline
            .as_ref()
            .is_some_and(|deadline| deadline.expired.load(Ordering::Acquire))
    }

    pub fn resume(&self) -> Result<()> {
        self.child.resume()
    }

    /// A root exit alone is insufficient: descendants must also be gone.
    pub fn try_wait(&mut self) -> Result<Option<u32>> {
        match self.child.try_wait_exit()? {
            Some(code) if self.job.active_processes()? == 0 => Ok(Some(code)),
            _ => Ok(None),
        }
    }

    pub fn terminate_and_confirm_empty(&mut self, timeout_ms: u32) -> Result<()> {
        if !(1..=5000).contains(&timeout_ms) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "owned termination confirmation requires 1..=5000 ms",
            ));
        }
        // Even a failed termination request can race natural exit. Only the
        // owned kernel accounting below establishes an empty process tree.
        let _ = self.job.terminate(1);
        let until = std::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        loop {
            if self.job.active_processes()? == 0 && self.child.try_wait_exit()?.is_some() {
                return Ok(());
            }
            if std::time::Instant::now() >= until {
                return Err(Error::other(
                    "owned AppContainer tree termination uncertain",
                ));
            }
            std::thread::sleep(JOB_EMPTY_POLL);
        }
    }
}

struct NativeJobDeadline {
    cancel: Option<mpsc::SyncSender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
    expired: Arc<AtomicBool>,
    termination_result: Arc<OnceLock<std::result::Result<(), i32>>>,
}
impl NativeJobDeadline {
    fn new(job: &OwnedJob, timeout_ms: u32, stop_flags: Vec<Arc<AtomicBool>>) -> Result<Self> {
        let until = std::time::Instant::now()
            .checked_add(Duration::from_millis(u64::from(timeout_ms)))
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "watchdog horizon overflow"))?;
        Self::new_until(job, until, stop_flags)
    }

    fn new_until(
        job: &OwnedJob,
        until: std::time::Instant,
        stop_flags: Vec<Arc<AtomicBool>>,
    ) -> Result<Self> {
        let mut duplicate = HANDLE::default();
        // SAFETY: duplicate a live owned job into this process only. It is never
        // inheritable or exposed; OwnedHandle provides thread-safe ownership.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                job.raw(),
                GetCurrentProcess(),
                &mut duplicate,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(Error::other)?;
        // SAFETY: successful duplication transfers exactly one unique handle.
        let handle = unsafe { OwnedHandle::from_raw_handle(duplicate.0) };
        let (cancel, receive) = mpsc::sync_channel(1);
        let expired = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&expired);
        let termination_result = Arc::new(OnceLock::new());
        let receipt = Arc::clone(&termination_result);
        let worker = std::thread::Builder::new()
            .name("optic-job-deadline".into())
            .spawn(move || {
                loop {
                    let timed_out = std::time::Instant::now() >= until;
                    if timed_out || stop_flags.iter().any(|flag| flag.load(Ordering::Acquire)) {
                        if timed_out {
                            observed.store(true, Ordering::Release);
                        }
                        // SAFETY: this thread owns the duplicate of this exact kernel
                        // job. It takes no PID and performs no file/profile cleanup.
                        let result =
                            unsafe { TerminateJobObject(HANDLE(handle.as_raw_handle()), 1) }
                                .map_err(|error| error.code().0);
                        // Publish only after the native call returned. A deadline
                        // flag or finished worker is not a termination receipt.
                        let _ = receipt.set(result);
                        break;
                    }
                    let remaining = until.saturating_duration_since(std::time::Instant::now());
                    match receive.recv_timeout(remaining.min(Duration::from_millis(20))) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => (),
                    }
                }
            })?;
        Ok(Self {
            cancel: Some(cancel),
            worker: Some(worker),
            expired,
            termination_result,
        })
    }
}
impl Drop for NativeJobDeadline {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.try_send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Debug)]
struct LimitedJobChild {
    inner: Box<dyn ChildWrapper>,
    job: OwnedJob,
    root_status: Option<ExitStatus>,
}

impl ChildWrapper for LimitedJobChild {
    fn inner(&self) -> &dyn ChildWrapper {
        self.inner.as_ref()
    }

    fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
        self.inner.as_mut()
    }

    fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
        let LimitedJobChild { mut inner, job, .. } = *self;
        // Removing the containment wrapper must not silently detach a live process tree.
        // Fail closed: terminate the whole job, ask the root child to terminate as a
        // fallback, then drop the owned Job Object handle instead of leaking it.
        let _ = job.terminate(1);
        let _ = inner.start_kill();
        drop(job);
        inner
    }

    fn process_handle(&self) -> Option<BorrowedHandle<'_>> {
        child_process_handle(self.inner.as_ref())
    }

    fn start_kill(&mut self) -> Result<()> {
        self.job.terminate(1)
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        if self.root_status.is_none() {
            self.root_status = self.inner.try_wait()?;
        }
        match self.root_status {
            Some(status) if self.job.active_processes()? == 0 => Ok(Some(status)),
            _ => Ok(None),
        }
    }

    fn wait(&mut self) -> Pin<Box<dyn Future<Output = Result<ExitStatus>> + Send + '_>> {
        Box::pin(async move {
            let status = match self.root_status {
                Some(status) => status,
                None => {
                    let status = self.inner.wait().await?;
                    self.root_status = Some(status);
                    status
                }
            };

            while self.job.active_processes()? != 0 {
                tokio::time::sleep(JOB_EMPTY_POLL).await;
            }
            Ok(status)
        })
    }
}

fn child_process_handle(child: &dyn ChildWrapper) -> Option<BorrowedHandle<'_>> {
    child.process_handle().or_else(|| {
        child
            .try_inner_child()
            .and_then(|native| native.process_handle())
    })
}

fn resume_process_threads(process: HANDLE) -> Result<()> {
    // SAFETY: `process` is the live child process handle supplied by the process wrapper.
    let pid = unsafe { GetProcessId(process) };
    if pid == 0 {
        return Err(Error::last_os_error());
    }

    // SAFETY: documented snapshot call with no caller-owned pointer arguments.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }?;
    let snapshot = ScopedHandle(snapshot);
    let mut entry = THREADENTRY32 {
        dwSize: size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };

    // SAFETY: `snapshot` is valid and `entry` has the required size initialized.
    unsafe { Thread32First(snapshot.0, &mut entry) }.map_err(Error::other)?;
    let mut resumed = false;

    loop {
        if entry.th32OwnerProcessID == pid {
            // SAFETY: opening a thread by enumerated thread id with the minimal resume right.
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID) }
                .map_err(Error::other)?;
            let thread = ScopedHandle(thread);
            // SAFETY: `thread.0` is a valid thread handle with THREAD_SUSPEND_RESUME access.
            let previous = unsafe { ResumeThread(thread.0) };
            if previous == u32::MAX {
                return Err(Error::last_os_error());
            }
            resumed |= previous > 0;
        }

        // SAFETY: same valid snapshot and output buffer as above.
        match unsafe { Thread32Next(snapshot.0, &mut entry) } {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(ERROR_NO_MORE_FILES.0) => break,
            Err(error) => return Err(Error::other(error)),
        }
    }

    if resumed {
        Ok(())
    } else {
        Err(Error::other(
            "no suspended child thread was found to resume",
        ))
    }
}

#[derive(Debug)]
struct ScopedHandle(HANDLE);

impl Drop for ScopedHandle {
    fn drop(&mut self) {
        // SAFETY: each ScopedHandle owns a temporary Win32 handle exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_limits() {
        assert!(LimitedJobObject::new(0, 1, 25).is_err());
        assert!(LimitedJobObject::new(1, 0, 25).is_err());
        assert!(LimitedJobObject::new(1, 1, 0).is_err());
        assert!(LimitedJobObject::new(1, 1, 101).is_err());
    }

    #[test]
    fn accepts_nonzero_limits() {
        assert!(LimitedJobObject::new(1, 64 * 1024 * 1024, 25).is_ok());
    }

    #[test]
    fn configures_hard_cpu_cap() {
        let job = OwnedJob::create(1, 64 * 1024 * 1024, 25).expect("create limited job");
        let mut cpu = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION::default();
        // SAFETY: the queried information class matches the concrete buffer and exact size;
        // `job` owns a valid Job Object handle for the duration of this synchronous call.
        unsafe {
            QueryInformationJobObject(
                Some(job.raw()),
                JobObjectCpuRateControlInformation,
                (&mut cpu as *mut JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
                None,
            )
        }
        .expect("query CPU rate control");
        assert_eq!(
            cpu.ControlFlags,
            JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP
        );
        // SAFETY: `CpuRate` is the active union member for hard-cap mode.
        assert_eq!(unsafe { cpu.Anonymous.CpuRate }, 2_500);
    }

    #[test]
    fn drop_fixture_child() {
        let Some(started) = std::env::var_os("OPTIC_JOB_DROP_STARTED") else {
            return;
        };
        let survived = std::env::var_os("OPTIC_JOB_DROP_SURVIVED")
            .expect("survived marker must accompany started marker");
        std::fs::write(started, b"started").expect("write started marker");
        std::thread::sleep(Duration::from_millis(700));
        std::fs::write(survived, b"survived").expect("write survived marker");
    }

    async fn spawn_delayed_marker_fixture(
        label: &str,
    ) -> (
        std::path::PathBuf,
        std::path::PathBuf,
        Box<dyn ChildWrapper>,
    ) {
        use process_wrap::tokio::CommandWrap;
        use std::time::{SystemTime, UNIX_EPOCH};

        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("optic-job-{label}-{token}"));
        std::fs::create_dir_all(&root).expect("create fixture dir");
        let started = root.join("started");
        let survived = root.join("survived");

        let mut command = Command::new(std::env::current_exe().expect("current test executable"));
        command
            .args(["--exact", "job::tests::drop_fixture_child", "--nocapture"])
            .env("OPTIC_JOB_DROP_STARTED", &started)
            .env("OPTIC_JOB_DROP_SURVIVED", &survived);
        let mut command = CommandWrap::from(command);
        command.wrap(
            LimitedJobObject::new(1, 512 * 1024 * 1024, 25).expect("valid Job Object limits"),
        );
        let child = command.spawn().expect("spawn wrapped fixture");

        for _ in 0..200 {
            if started.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(started.exists(), "fixture child did not start");
        (root, survived, child)
    }

    #[tokio::test]
    async fn dropping_job_handle_kills_running_child() {
        let (root, survived, child) = spawn_delayed_marker_fixture("drop").await;
        drop(child);
        tokio::time::sleep(Duration::from_millis(900)).await;
        assert!(
            !survived.exists(),
            "closing the kill-on-close Job Object did not terminate the child"
        );
        std::fs::remove_dir_all(root).expect("remove fixture dir");
    }

    #[tokio::test]
    async fn unwrapping_job_containment_fails_closed() {
        let (root, survived, child) = spawn_delayed_marker_fixture("unwrap").await;
        let inner = child.into_inner();
        drop(inner);
        tokio::time::sleep(Duration::from_millis(900)).await;
        assert!(
            !survived.exists(),
            "unwrapping containment allowed the child to survive"
        );
        std::fs::remove_dir_all(root).expect("remove fixture dir");
    }
}

#[cfg(test)]
mod native_error_regression {
    use super::*;

    #[test]
    fn invalid_job_accounting_preserves_original_win32_error() {
        // An invalid borrowed handle has no process or ownership to clean up.
        // QueryInformationJobObject must retain ERROR_INVALID_HANDLE (Win32 6).
        // NULL is a special API value, not a guaranteed bad handle.
        // A non-null dangling pointer with address 1 is never an owned kernel Job handle.
        let invalid_job = HANDLE(std::ptr::dangling_mut::<c_void>());
        let error = query_active_processes(invalid_job)
            .expect_err("invalid Job handle must fail native accounting");
        assert_eq!(
            error.raw_os_error(),
            Some(windows::Win32::Foundation::ERROR_INVALID_HANDLE.0 as i32),
            "native accounting failure must expose the original Win32 code"
        );
    }
    #[test]
    fn win32_hresult_24_preserves_win32_code() {
        let native =
            windows::core::Error::from(windows::Win32::Foundation::WIN32_ERROR(24).to_hresult());
        assert_eq!(job_query_io_error(native).raw_os_error(), Some(24));
    }

    #[test]
    fn non_win32_hresult_preserves_entire_status() {
        let status = HRESULT(0x8000_4005_u32 as i32);
        let native = windows::core::Error::from(status);
        assert_eq!(job_query_io_error(native).raw_os_error(), Some(status.0));
    }
}

#[cfg(test)]
#[path = "job_list_launch_tests.rs"]
mod job_list_launch_tests;
