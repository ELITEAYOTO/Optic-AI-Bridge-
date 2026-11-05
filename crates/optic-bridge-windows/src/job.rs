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
                CREATE_NO_WINDOW, CREATE_SUSPENDED, GetCurrentProcess, GetProcessId,
                GetProcessIdOfThread, OpenThread, ResumeThread, THREAD_QUERY_LIMITED_INFORMATION,
                THREAD_SUSPEND_RESUME, TerminateProcess, WaitForSingleObject,
            },
        },
    },
    core::{BOOL, HRESULT},
};

const JOB_EMPTY_POLL: Duration = Duration::from_millis(10);

/// Typed evidence that a post-CreateProcessW wrapper setup error has NO
/// confirmed native root exit. Neither a successful wrapper stop request nor
/// an unconfirmed kernel stop allows consumers to release process quotas.
/// This is evidence classification, NOT proof of a still-running process.
#[derive(Debug)]
pub struct UnconfirmedWindowsChildTermination {
    detail: String,
}

impl std::fmt::Display for UnconfirmedWindowsChildTermination {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for UnconfirmedWindowsChildTermination {}

fn uncertain_child_termination(message: String) -> Error {
    Error::other(UnconfirmedWindowsChildTermination { detail: message })
}

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
        inner: Box<dyn ChildWrapper>,
        _core: &CommandWrap,
    ) -> Result<Box<dyn ChildWrapper>> {
        self.wrap_child_with_before_resume(inner, |_job, _process| {})
    }
}

impl LimitedJobObject {
    // Production invokes a no-op. The interleaving hook is used by a
    // controlled real-kernel Windows regression at this exact boundary.
    fn wrap_child_with_before_resume(
        &mut self,
        mut inner: Box<dyn ChildWrapper>,
        before_resume: impl FnOnce(&mut OwnedJob, HANDLE),
    ) -> Result<Box<dyn ChildWrapper>> {
        let process = match child_process_handle(inner.as_ref()) {
            Some(handle) => handle,
            None => {
                // pre_spawn already requested CREATE_SUSPENDED; dropping a
                // wrapper without an exposed process HANDLE does not kill
                // its child. Ask the ORIGINAL wrapper to stop that suspended
                // process before we lose its unique owner.
                // An accepted stop request is not evidence of kernel exit:
                // the wrapper may acknowledge it while the suspended process
                // remains live. With no original PROCESS HANDLE there is no
                // trustworthy physical observation, even when start_kill()
                // returns Ok. Do not classify this as ordinary Unsupported.
                let stop = inner.start_kill();
                return Err(uncertain_child_termination(format!(
                    "child wrapper has no Windows process handle; suspended child termination uncertain: wrapper stop={stop:?}; original kernel process exit unobservable"
                )));
            }
        };
        let process = HANDLE(process.as_raw_handle());

        let mut job = match OwnedJob::create(
            self.active_process_limit,
            self.job_memory_limit,
            self.cpu_percent,
        ) {
            Ok(job) => job,
            Err(error) => {
                return Err(terminate_original_suspended_root_after_job_error(
                    process,
                    inner.as_mut(),
                    error,
                ));
            }
        };

        // SAFETY: `job.raw()` is an owned Job Object handle and `process` is the live
        // child process handle borrowed from `inner`. Both remain valid for this call.
        if let Err(error) = unsafe { AssignProcessToJobObject(job.raw(), process) } {
            // A failed kernel assignment leaves this CREATE_SUSPENDED root
            // outside the new Job. Its fallible wrapper stop is not enough:
            // retain the ORIGINAL process HANDLE until physical exit is
            // observed, or return explicit termination uncertainty.
            return Err(terminate_original_suspended_root_after_job_error(
                process,
                inner.as_mut(),
                Error::other(error),
            ));
        }

        before_resume(&mut job, process);
        if let Err(error) = resume_process_threads(process) {
            return Err(terminate_assigned_root_after_resume_error(
                &job,
                process,
                inner.as_mut(),
                error,
            ));
        }

        Ok(Box::new(LimitedJobChild {
            inner,
            job,
            root_status: None,
        }))
    }
}

// ResumeThread may have partially modified a real thread before failing.
// A successful TerminateJobObject *request* or ChildWrapper::start_kill
// acknowledgement is never an observation of the root's physical exit.
// A failed cleanup must be distinctly typed so ProcessManager retains the
// original quota/JobRecord rather than treating it as a no-child spawn error.
fn terminate_assigned_root_after_resume_error(
    job: &OwnedJob,
    process: HANDLE,
    child: &mut dyn ChildWrapper,
    resume_error: Error,
) -> Error {
    let kernel_stop = job.terminate(1);
    // Observe the ORIGINAL borrowed HANDLE before invoking a fallible
    // ChildWrapper hook which could itself alter its stored process handle.
    // SAFETY: the original ChildWrapper is still held by this call.
    let wait_ms = if kernel_stop.is_ok() { 3_000 } else { 0 };
    let root_wait = unsafe { WaitForSingleObject(process, wait_ms) };
    // The original Job might contain descendants even after root exit.
    let active = job.active_processes();
    // These bounded checks only classify a pre-return setup failure, NOT a
    // complete native guardian receipt including two independently read EOFs
    // or the missing no-future-launch authority.
    if kernel_stop.is_ok() && root_wait == WAIT_OBJECT_0 && matches!(active.as_ref(), Ok(&0)) {
        return resume_error;
    }
    // An acknowledged wrapper stop remains only a fallback request, not
    // evidence that the pinned original Job/root have physically stopped.
    let wrapper_stop = child.start_kill();
    uncertain_child_termination(format!(
        "native resume failed ({resume_error}); assigned root termination uncertain: original Job stop={kernel_stop:?}, root wait={root_wait:?}, original Job active={active:?}, wrapper fallback={wrapper_stop:?}"
    ))
}

// A Job creation OR process-to-Job assignment failure occurs AFTER CommandWrap
// created the suspended root but BEFORE that root belongs to this new Job.
// Do not depend solely on a ChildWrapper's fallible termination request: the
// original live PROCESS HANDLE
// is still borrowed from that wrapper and can be used to stop the exact root.
// Never turn an unsuccessful OS stop/wait into ordinary setup failure.
fn terminate_original_suspended_root_after_job_error(
    process: HANDLE,
    child: &mut dyn ChildWrapper,
    setup_error: Error,
) -> Error {
    // SAFETY: process is the original suspended child's live PROCESS HANDLE,
    // borrowed from child for this entire synchronous call.
    match unsafe { TerminateProcess(process, 1) } {
        Ok(()) => {
            // SAFETY: wait only on the same still-owned original PROCESS HANDLE.
            let state = unsafe { WaitForSingleObject(process, 3_000) };
            if state == WAIT_OBJECT_0 {
                return setup_error;
            }
            let fallback = child.start_kill();
            uncertain_child_termination(format!(
                "Windows Job setup failed ({setup_error}); suspended root termination uncertain: process wait={state:?}, wrapper fallback={fallback:?}"
            ))
        }
        Err(stop_error) => {
            // A process can have exited independently before our stop request.
            // SAFETY: zero-time observation of the exact original PROCESS HANDLE.
            if unsafe { WaitForSingleObject(process, 0) } == WAIT_OBJECT_0 {
                return setup_error;
            }
            let fallback = child.start_kill();
            uncertain_child_termination(format!(
                "Windows Job setup failed ({setup_error}); suspended root termination uncertain: native stop={stop_error}, wrapper fallback={fallback:?}"
            ))
        }
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

        // SetInformationJobObject succeeding is not proof that the kernel
        // retained our requested hard caps. Independently query the exact
        // Job *before* returning the private owner, when no root can exist.
        // A missing/looser or differently configured cap is fail-closed.
        job.verify_configured_limits(active_process_limit, job_memory_limit, cpu_percent)?;
        Ok(job)
    }

    /// Read back the actual original kernel Job policy rather than inferring
    /// it from SetInformationJobObject returning success. This verifies the
    /// configuration, not the number/identity of future live processes.
    /// A Job may later report more active processes than this requested
    /// limit: that is distinct evidence requiring a real OS investigation.
    fn verify_configured_limits(
        &self,
        expected_active_processes: u32,
        expected_memory_bytes: usize,
        expected_cpu_percent: u32,
    ) -> Result<()> {
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        // SAFETY: the concrete JobObjectExtendedLimitInformation structure is
        // fully initialized, correctly sized, and the original Job stays owned.
        unsafe {
            QueryInformationJobObject(
                Some(self.raw()),
                JobObjectExtendedLimitInformation,
                (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                None,
            )
        }
        .map_err(job_query_io_error)?;
        let expected_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_JOB_MEMORY;
        if limits.BasicLimitInformation.LimitFlags != expected_flags
            || limits.BasicLimitInformation.ActiveProcessLimit != expected_active_processes
            || limits.JobMemoryLimit != expected_memory_bytes
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "original native Job hard-limit policy differs from requested kernel caps",
            ));
        }
        let mut cpu = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION::default();
        // SAFETY: the information class, buffer size and live original Job
        // match and no user-controlled pointers or executable data are read.
        unsafe {
            QueryInformationJobObject(
                Some(self.raw()),
                JobObjectCpuRateControlInformation,
                (&mut cpu as *mut JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
                None,
            )
        }
        .map_err(job_query_io_error)?;
        if cpu.ControlFlags
            != (JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP)
            // SAFETY: CpuRate is the active union field under HARD_CAP.
            || unsafe { cpu.Anonymous.CpuRate } != expected_cpu_percent * 100
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "original native Job CPU hard cap differs from requested kernel rate",
            ));
        }
        Ok(())
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

/// Inert Windows GitRead custody: real kernel Job, bounded anonymous pipes
/// and nulled stdin are opened before any root exists. There is no process
/// create/spawn/resume method and no independent execution authorization.
#[must_use = "retain native handles until unstarted cleanup or later terminal proof"]
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
        self.pin_original_job_root_with_before_retention(root, || {})
    }

    // The callback is always empty in production. Test-only race fixtures
    // place an original stop exactly after a REAL DuplicateHandle but before
    // the root handle is published into its permanent native custody.
    fn pin_original_job_root_with_before_retention(
        &mut self,
        root: BorrowedHandle<'_>,
        before_retention: impl FnOnce(),
    ) -> Result<()> {
        if self.original_root.is_some() || self.root_retention_stopped() {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "native root requires an unbound, live owner with its original armed watchdog",
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
        let pinned = PinnedOriginalRoot {
            handle: unsafe { OwnedHandle::from_raw_handle(duplicate.0) },
            pid,
        };
        before_retention();
        // The root may have PHYSICALLY exited between our first liveness
        // observation and DuplicateHandle. Recheck the exact duplicated
        // kernel HANDLE, not a PID, immediately before permanent retention.
        // A signaled/invalid root cannot be installed as the live root of
        // this original native Job even if no stop flag has been published.
        // This observation is NOT an atomic native Create/Resume issuer:
        // a later exit still requires the original OS/EOF guardian.
        match unsafe { WaitForSingleObject(HANDLE(pinned.handle.as_raw_handle()), 0) } {
            WAIT_TIMEOUT => {}
            WAIT_OBJECT_0 => {
                return Err(Error::new(
                    ErrorKind::PermissionDenied,
                    "native root exited after HANDLE duplication but before retention",
                ));
            }
            _ => return Err(Error::last_os_error()),
        }
        // Reobserve the very same ORIGINAL stop sources and absolute horizon
        // after the actual native HANDLE and liveness checks, just before
        // retaining this root. Later stop still belongs to the original Job
        // watchdog; this is NOT an atomic CreateProcessW/ResumeThread gate.
        if self.root_retention_stopped() {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "original native root stop or deadline won before handle retention",
            ));
        }
        self.original_root = Some(pinned);
        Ok(())
    }

    fn root_retention_stopped(&self) -> bool {
        self.deadline
            .as_ref()
            .is_none_or(NativeJobDeadline::stop_or_expired)
            || self.watchdog_stop_observed()
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
        self.confirm_post_watchdog_physical_stop_with_clock(
            cleanup_until,
            discard_limit,
            Instant::now,
        )
    }

    // The clock can be controlled only from this module's Windows test build.
    // Production always passes Instant::now. This lets a real physical-EOF
    // fixture prove expiry precisely between the last OS read and publication.
    fn confirm_post_watchdog_physical_stop_with_clock(
        &mut self,
        cleanup_until: Instant,
        discard_limit: u64,
        mut now: impl FnMut() -> Instant,
    ) -> Result<NativePhysicalStopReceipt> {
        if discard_limit == 0 || cleanup_until <= now() {
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
            // An already-complete physical snapshot is not sufficient if the
            // original absolute cleanup horizon expired while obtaining it.
            // This must precede even the successful four-fact receipt.
            if now() >= cleanup_until {
                return Err(Error::new(
                    ErrorKind::TimedOut,
                    "native shutdown did not prove root exit, Job empty and BOTH pipe EOFs within its absolute cleanup horizon",
                ));
            }
            if root_exited && job_empty && stdout_eof && stderr_eof {
                return Ok(NativePhysicalStopReceipt {
                    root_pid: root.pid,
                    discarded_stdout_bytes: stdout_bytes,
                    discarded_stderr_bytes: stderr_bytes,
                });
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
    fn real_suspended_job_root_cannot_be_pinned_after_original_stop_wins() {
        use std::{
            os::windows::{io::AsRawHandle, process::CommandExt},
            path::PathBuf,
            process::{Child, Command as NativeCommand, Stdio},
        };
        // A real Windows kernel child is created SUSPENDED under a fixed
        // System32 executable, assigned to the ORIGINAL Job and NEVER resumed.
        // This is not a simulation of CreateProcessW or process ownership.
        struct KillOnDrop(Child);
        impl Drop for KillOnDrop {
            fn drop(&mut self) {
                let _ = self.0.kill();
            }
        }

        let root_dir =
            PathBuf::from(std::env::var_os("SystemRoot").expect("fixed Windows system root"));
        let executable = root_dir.join("System32").join("cmd.exe");
        assert!(
            executable.is_file(),
            "fixed signed Windows fixture executable"
        );
        let limits =
            LimitedJobObject::new(1, 256 * 1024 * 1024, 50).expect("bounded native Job caps");
        let mut owner =
            PrecreatedNativeReadCustody::new(limits).expect("real owned Job and original pipes");
        let original_stop = Arc::new(AtomicBool::new(false));
        owner
            .arm_watchdog(20_000, vec![Arc::clone(&original_stop)])
            .expect("original Job watchdog armed before kernel root creation");

        let mut command = NativeCommand::new(executable);
        command
            .arg("/d")
            .arg("/c")
            .arg("exit /b 0")
            .current_dir(std::env::temp_dir())
            .env_clear()
            .env("SystemRoot", root_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0);
        let mut root = KillOnDrop(command.spawn().expect("real suspended Win32 root"));
        drop(command);
        let process = HANDLE(root.0.as_raw_handle());
        // SAFETY: original suspended process and private kernel Job remain owned.
        unsafe { AssignProcessToJobObject(owner.job.raw(), process) }
            .expect("real Job membership before any root execution");
        assert_eq!(owner.active_processes().expect("real kernel Job count"), 1);
        assert!(
            root.0.try_wait().expect("real suspended status").is_none(),
            "root must be suspended and live before original revoke"
        );

        let reached_native_boundary = AtomicBool::new(false);
        // The callback is injected only after the REAL process HANDLE has
        // been duplicated. Revocation therefore wins before custody can
        // publish its retained root; this is a deterministic OS-handle race.
        // SAFETY: original Child retains the real kernel HANDLE for this call.
        let result = owner.pin_original_job_root_with_before_retention(
            unsafe { BorrowedHandle::borrow_raw(root.0.as_raw_handle()) },
            || {
                reached_native_boundary.store(true, Ordering::Release);
                original_stop.store(true, Ordering::Release);
            },
        );
        assert!(
            reached_native_boundary.load(Ordering::Acquire),
            "test must execute the actual post-DuplicateHandle commitment"
        );
        assert!(
            matches!(result, Err(ref error) if error.kind() == ErrorKind::PermissionDenied),
            "original stop won before root retention: must fail closed, result={result:?}"
        );
        assert!(
            owner.original_root.is_none(),
            "a revoked owner must never publish the pinned root"
        );

        let until = Instant::now() + Duration::from_secs(5);
        while owner.active_processes().expect("owned Job accounting") != 0 && Instant::now() < until
        {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            owner.active_processes().expect("real post-stop Job query"),
            0,
            "the original watchdog must terminate the suspended kernel root"
        );
    }

    #[test]
    fn real_exited_root_after_handle_duplication_must_not_be_retained() {
        use std::{
            os::windows::{io::AsRawHandle, process::CommandExt},
            path::PathBuf,
            process::{Child, Command as NativeCommand, Stdio},
        };
        use windows::Win32::System::Threading::TerminateProcess;

        // Actual suspended Win32 process, real original kernel Job and a real
        // duplicated process HANDLE. It is NEVER resumed and never runs Git.
        // The test kills the real root precisely after DuplicateHandle but
        // BEFORE owner custody can publish its pinned root.
        struct KillOnDrop(Child);
        impl Drop for KillOnDrop {
            fn drop(&mut self) {
                let _ = self.0.kill();
            }
        }

        let root_dir =
            PathBuf::from(std::env::var_os("SystemRoot").expect("fixed Windows system root"));
        let executable = root_dir.join("System32").join("cmd.exe");
        assert!(executable.is_file(), "fixed Windows kernel fixture target");
        let limits = LimitedJobObject::new(1, 256 * 1024 * 1024, 50).expect("bounded original Job");
        let mut owner =
            PrecreatedNativeReadCustody::new(limits).expect("real Job and original stdio custody");
        let original_stop = Arc::new(AtomicBool::new(false));
        owner
            .arm_watchdog(20_000, vec![Arc::clone(&original_stop)])
            .expect("independent original watchdog");

        let mut command = NativeCommand::new(executable);
        command
            .arg("/d")
            .arg("/c")
            .arg("exit /b 0")
            .current_dir(std::env::temp_dir())
            .env_clear()
            .env("SystemRoot", root_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0);
        let root = KillOnDrop(command.spawn().expect("real CREATE_SUSPENDED root"));
        drop(command);
        let process = HANDLE(root.0.as_raw_handle());
        // SAFETY: private Job and real suspended root are alive and owned.
        unsafe { AssignProcessToJobObject(owner.job.raw(), process) }
            .expect("assign actual suspended root to original Job");
        assert_eq!(owner.active_processes().expect("real Job count"), 1);
        assert_eq!(
            unsafe { WaitForSingleObject(process, 0) },
            WAIT_TIMEOUT,
            "the original kernel root must be alive at the initial observation"
        );
        let reached_native_boundary = AtomicBool::new(false);
        // SAFETY: the original Child retains this exact process HANDLE
        // throughout pin_original_job_root_with_before_retention.
        let result = owner.pin_original_job_root_with_before_retention(
            unsafe { BorrowedHandle::borrow_raw(root.0.as_raw_handle()) },
            || {
                reached_native_boundary.store(true, Ordering::Release);
                // SAFETY: TerminateProcess acts on the still-owned exact
                // original root HANDLE, not a PID or detached process lookup.
                unsafe { TerminateProcess(process, 93) }
                    .expect("terminate real suspended root after duplicate");
                assert_eq!(
                    unsafe { WaitForSingleObject(process, 3000) },
                    WAIT_OBJECT_0,
                    "kernel MUST show root physically exited before root retention"
                );
            },
        );
        assert!(
            reached_native_boundary.load(Ordering::Acquire),
            "test must reach the real post-DuplicateHandle retention boundary"
        );
        assert!(
            !original_stop.load(Ordering::Acquire),
            "no original revoke: refusal must be caused by real root exit"
        );
        assert!(
            matches!(result, Err(ref error) if error.kind() == ErrorKind::PermissionDenied),
            "exited kernel root cannot become a new pinned live root: {result:?}"
        );
        assert!(
            owner.original_root.is_none(),
            "a physically exited root must not be installed in native custody"
        );

        let until = Instant::now() + Duration::from_secs(3);
        while owner.active_processes().expect("original kernel Job") != 0 {
            assert!(
                Instant::now() < until,
                "kernel Job did not drain exited root"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(owner.watchdog_termination_result(), None);
        assert!(!owner.watchdog_stop_observed());
        assert_eq!(
            unsafe { WaitForSingleObject(process, 0) },
            WAIT_OBJECT_0,
            "original root remains physically exited, never resumed"
        );
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

    /// Only the selected synthetic child process performs IO and waits.
    /// Normal cargo test runs execute this as a zero-effect fixture.
    #[test]
    fn native_317_physical_stop_fixture_child() {
        use std::io::Write;
        if std::env::var("OPTIC_317_SYNTHETIC_CHILD").as_deref() != Ok("real-native-job-child-v1") {
            return;
        }
        std::io::stdout()
            .write_all(b"OPTIC_317_REAL_STDOUT\n")
            .expect("fixture writes stdout");
        std::io::stdout().flush().expect("flush stdout");
        std::io::stderr()
            .write_all(b"OPTIC_317_REAL_STDERR\n")
            .expect("fixture writes stderr");
        std::io::stderr().flush().expect("flush stderr");
        // Parent must terminate via its actual Job, not via this fixture.
        std::thread::sleep(Duration::from_secs(20));
    }

    /// This is a native containment/cleanup proof, NOT a Git launch owner
    /// or a revocation-vs-CreateProcessW linearization claim.
    #[test]
    fn native_317_real_suspended_root_job_and_both_physical_pipe_eofs() {
        use std::{
            os::windows::{io::AsRawHandle, process::CommandExt},
            process::{Command as NativeCommand, Stdio},
        };

        fn child_writer(pipe: &CapturedOutputPipe) -> File {
            let mut duplicate = HANDLE::default();
            // SAFETY: the pre-created writer belongs to the retained custody,
            // and the duplicated writer is explicitly inheritable for std
            // Command's child stdio. Parent closes each duplicate on spawn.
            unsafe {
                DuplicateHandle(
                    GetCurrentProcess(),
                    HANDLE(pipe.writer_handle().expect("live writer").as_raw_handle()),
                    GetCurrentProcess(),
                    &mut duplicate,
                    0,
                    true,
                    DUPLICATE_SAME_ACCESS,
                )
            }
            .expect("duplicate the *original* writer for the synthetic child");
            // SAFETY: DuplicateHandle succeeded and the returned handle is uniquely owned.
            File::from(unsafe { OwnedHandle::from_raw_handle(duplicate.0) })
        }

        let limits = LimitedJobObject::new(1, 256 * 1024 * 1024, 100).expect("real job limits");
        let mut owner = PrecreatedNativeReadCustody::new(limits).expect("private job and pipes");
        let stop = Arc::new(AtomicBool::new(false));
        owner
            .arm_watchdog(15_000, vec![Arc::clone(&stop)])
            .expect("independent original watchdog");
        let executable = std::env::current_exe().expect("test binary only");
        let mut command = NativeCommand::new(executable);
        command
            .arg("native_317_physical_stop_fixture_child")
            .arg("--nocapture")
            .env("OPTIC_317_SYNTHETIC_CHILD", "real-native-job-child-v1")
            .stdin(Stdio::null())
            .stdout(Stdio::from(child_writer(&owner._stdout)))
            .stderr(Stdio::from(child_writer(&owner._stderr)))
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0);
        let mut root = command.spawn().expect("create suspended synthetic root");
        // Release the test parent's original Stdio duplicates immediately;
        // they must not hold EOF open after Job termination.
        drop(command);
        // SAFETY: the original std::process::Child owns a live process handle.
        let process = HANDLE(root.as_raw_handle());
        // SAFETY: a suspended root and original job are live and owned.
        unsafe { AssignProcessToJobObject(owner.job.raw(), process) }
            .expect("assign before resume");
        // SAFETY: raw handle is borrowed only for the immediately following call.
        owner
            .pin_original_job_root(unsafe { BorrowedHandle::borrow_raw(root.as_raw_handle()) })
            .expect("pin the exact live root and job identity");
        assert_eq!(owner.active_processes().expect("real job active"), 1);
        assert!(matches!(
            owner.confirm_post_watchdog_physical_stop(
                Instant::now() + Duration::from_millis(100),
                64 * 1024
            ),
            Err(ref error) if error.kind() == ErrorKind::PermissionDenied
        ));
        resume_process_threads(process).expect("resume only after assignment and pin");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let ready_until = Instant::now() + Duration::from_secs(8);
        while !stdout
            .windows(b"OPTIC_317_REAL_STDOUT\n".len())
            .any(|part| part == b"OPTIC_317_REAL_STDOUT\n")
            || !stderr
                .windows(b"OPTIC_317_REAL_STDERR\n".len())
                .any(|part| part == b"OPTIC_317_REAL_STDERR\n")
        {
            assert!(
                Instant::now() < ready_until,
                "root never wrote both original pipes"
            );
            stdout.extend(owner._stdout.try_read(8 * 1024).expect("read stdout").bytes);
            stderr.extend(owner._stderr.try_read(8 * 1024).expect("read stderr").bytes);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(root.try_wait().expect("root still alive").is_none());
        // Simulate an independently retained pipe writer. A completed Job
        // termination must NOT be reported as full physical EOF while this
        // original-pipe duplicate is still open in the parent.
        let retained_writer = child_writer(&owner._stdout);
        let retained_stderr_writer = child_writer(&owner._stderr);
        stop.store(true, Ordering::Release);
        let stopped_until = Instant::now() + Duration::from_secs(5);
        while !owner.watchdog_stop_observed() {
            assert!(Instant::now() < stopped_until, "watchdog failed to stop");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(owner.watchdog_termination_result(), Some(Ok(())));
        // TerminateJobObject returning success is NOT synchronous root exit.
        // Keep the original watchdog's single bounded five-second horizon:
        // the runner may observe the kernel exit and Job-empty later.
        while root
            .try_wait()
            .expect("observe original root exit")
            .is_none()
            || owner.active_processes().expect("observe original Job") != 0
        {
            assert!(
                Instant::now() < stopped_until,
                "watchdog stop never reached root-exit AND Job-empty"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        // Now the two timeout outcomes below specifically prove missing pipe
        // EOF, not a still-terminating root or nonempty Job.
        assert!(
            matches!(
                owner.confirm_post_watchdog_physical_stop(
                    Instant::now() + Duration::from_millis(120),
                    64 * 1024,
                ),
                Err(ref error) if error.kind() == ErrorKind::TimedOut
            ),
            "Job-empty plus root exit MUST NOT substitute for a missing stdout EOF"
        );
        drop(retained_writer);
        assert!(
            matches!(
                owner.confirm_post_watchdog_physical_stop(
                    Instant::now() + Duration::from_millis(120),
                    64 * 1024,
                ),
                Err(ref error) if error.kind() == ErrorKind::TimedOut
            ),
            "stdout EOF alone MUST NOT substitute for missing stderr EOF"
        );
        drop(retained_stderr_writer);
        let receipt = owner
            .confirm_post_watchdog_physical_stop(Instant::now() + Duration::from_secs(5), 64 * 1024)
            .expect("pinned root exit, original Job empty and both physical EOFs");
        assert_eq!(receipt.root_pid(), root.id());
        assert_eq!(owner.active_processes().expect("post-stop Job"), 0);
        // Real root / kernel Job / both original EOFs are already complete.
        // A controlled *late decision clock* must still reject a terminal
        // receipt: expiry must be checked AFTER its physical observations.
        let decision_deadline = Instant::now() + Duration::from_secs(3);
        let mut decision_reads = 0usize;
        let late = owner.confirm_post_watchdog_physical_stop_with_clock(
            decision_deadline,
            64 * 1024,
            || {
                decision_reads += 1;
                if decision_reads == 1 {
                    decision_deadline - Duration::from_millis(1)
                } else {
                    decision_deadline + Duration::from_millis(1)
                }
            },
        );
        assert!(
            matches!(late, Err(ref error) if error.kind() == ErrorKind::TimedOut),
            "even completed real OS/EOF proof cannot publish beyond cleanup deadline: {late:?}"
        );
        assert!(
            decision_reads >= 2,
            "must observe expiry after terminal reads"
        );
        let ended = root.wait().expect("reap original root");
        assert!(
            !ended.success(),
            "Job termination must kill the sleeping root"
        );
        // The original stdio bytes were observed through custody's actual pipes,
        // not an independent or mocked output stream.
        assert!(
            stdout
                .windows(b"OPTIC_317_REAL_STDOUT\n".len())
                .any(|part| part == b"OPTIC_317_REAL_STDOUT\n")
        );
        assert!(
            stderr
                .windows(b"OPTIC_317_REAL_STDERR\n".len())
                .any(|part| part == b"OPTIC_317_REAL_STDERR\n")
        );
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
    // Retain the exact Arc generations and original monotonic deadline in
    // the owner as well as the independent watcher. A delayed watcher is not
    // a substitute for a synchronous stop observation at root retention.
    original_stops: Vec<Arc<AtomicBool>>,
    original_until: Instant,
    expired: Arc<AtomicBool>,
    termination_result: Arc<OnceLock<std::result::Result<(), i32>>>,
}
impl NativeJobDeadline {
    fn stop_or_expired(&self) -> bool {
        Instant::now() >= self.original_until
            || self
                .original_stops
                .iter()
                .any(|signal| signal.load(Ordering::Acquire))
    }

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
        // Both views observe the same immutable source generations; only the
        // independent worker issues TerminateJobObject on the retained Job.
        let original_stops = stop_flags.clone();
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
            original_stops,
            original_until: until,
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
            // The ToolHelp snapshot's owner PID is not sufficient: a TID can
            // become stale before OpenThread. Request the minimal right needed
            // to query the owner of the actual HANDLE before resuming it.
            let thread = unsafe {
                OpenThread(
                    THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                    false,
                    entry.th32ThreadID,
                )
            }
            .map_err(Error::other)?;
            let thread = ScopedHandle(thread);
            resumed |= resume_owned_root_thread(thread.0, pid)?;
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

// Verify the owner of the ACTUAL kernel thread HANDLE, not merely the stale
// ToolHelp snapshot's TID/PID association. This remains an identity check,
// not the future atomic original-generation revoke/ResumeThread issuer.
fn resume_owned_root_thread(thread: HANDLE, original_pid: u32) -> Result<bool> {
    // SAFETY: the caller holds a live kernel thread HANDLE with query rights.
    let actual_pid = unsafe { GetProcessIdOfThread(thread) };
    if actual_pid == 0 {
        return Err(Error::last_os_error());
    }
    if actual_pid != original_pid {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "opened thread belongs to a different process than the original root",
        ));
    }
    // SAFETY: this exact queried thread HANDLE also has suspend/resume rights.
    let previous = unsafe { ResumeThread(thread) };
    match previous {
        u32::MAX => Err(Error::last_os_error()),
        0 => Ok(false), // Already running: this call did not release a suspension.
        1 => Ok(true),  // Exactly one suspension was released: now runnable.
        _ => Err(Error::new(
            ErrorKind::PermissionDenied,
            "original native primary thread is still suspended after ResumeThread",
        )),
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

    // Intentionally RED research acceptance. This synthetic original stop is
    // published AFTER kernel Job assignment but BEFORE the original ResumeThread
    // call. Unlike GitRead's already-known revoke-before-spawn acceptance, this
    // checks the distinct suspended-root -> resumed-code boundary.
    //
    // Current production LimitedJobObject does NOT bind a session or TaskLease
    // stop generation. Do not turn an ordinary preflight snapshot into a launch
    // authorization: the eventual service-owned issuer must implement the
    // actual stop/OS-call critical section, not just make this fixture green.
    #[test]
    #[ignore = "known RED: service-owned original-stop/ResumeThread issuer not yet implemented; run deliberately under external bounded Windows LAB supervision"]
    fn research_poststop_suspended_root_must_not_execute_after_resume_boundary() {
        use std::{
            fs,
            os::windows::{
                io::{AsHandle, AsRawHandle},
                process::CommandExt,
            },
            path::PathBuf,
            process::{Child as StdChild, Command as StdCommand, Stdio},
            sync::Arc,
            time::{SystemTime, UNIX_EPOCH},
        };

        #[derive(Debug)]
        struct FixedRoot(StdChild);
        impl ChildWrapper for FixedRoot {
            fn inner(&self) -> &dyn ChildWrapper {
                self
            }
            fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
                self
            }
            fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
                self
            }
            fn process_handle(&self) -> Option<BorrowedHandle<'_>> {
                Some(self.0.as_handle())
            }
            fn start_kill(&mut self) -> Result<()> {
                self.0.kill()
            }
        }

        struct Emergency {
            original: OwnedHandle,
            marker: PathBuf,
        }
        impl Drop for Emergency {
            fn drop(&mut self) {
                let original = HANDLE(self.original.as_raw_handle());
                // SAFETY: test-only independently pinned exact kernel root.
                let _ = unsafe { TerminateProcess(original, 91) };
                let _ = unsafe { WaitForSingleObject(original, 5_000) };
                let _ = fs::remove_file(&self.marker);
            }
        }

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("monotonic test marker ID")
            .as_nanos();
        let marker = std::env::temp_dir().join(format!(
            "optic-p0-revoke-before-resume-{}-{unique}.txt",
            std::process::id()
        ));
        assert!(!marker.exists(), "fresh isolated temporary marker");
        let system_root =
            PathBuf::from(std::env::var_os("SystemRoot").expect("fixed Windows SystemRoot"));
        let cmd = system_root.join("System32").join("cmd.exe");
        assert!(cmd.is_file());
        // Create a *zero-length* file in the test parent before launching.
        // Bind its exact stdout HANDLE to the real child rather than rely on
        // cmd.exe command-line redirection/quoting as a marker side effect.
        let child_stdout = fs::File::create(&marker).expect("empty stdout marker");
        let child = StdCommand::new(&cmd)
            .args(["/d", "/c", "echo REAL_CHILD_ENTERED"])
            .env_clear()
            .env("SystemRoot", &system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::from(child_stdout))
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0)
            .spawn()
            .expect("real CREATE_SUSPENDED root in disposable fixture");

        let mut copy = HANDLE::default();
        // SAFETY: duplicate ORIGINAL process handle, noninheritable and owned by
        // the test emergency supervisor independent of the wrapper under test.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(child.as_raw_handle()),
                GetCurrentProcess(),
                &mut copy,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .expect("emergency duplicate of original Windows PROCESS handle");
        let emergency = Emergency {
            original: unsafe { OwnedHandle::from_raw_handle(copy.0) },
            marker,
        };
        let original = HANDLE(emergency.original.as_raw_handle());
        let original_pid = unsafe { GetProcessId(original) };
        assert_ne!(original_pid, 0);
        assert_eq!(unsafe { WaitForSingleObject(original, 0) }, WAIT_TIMEOUT);

        let stop = Arc::new(AtomicBool::new(false));
        let at_boundary = Arc::clone(&stop);
        let mut wrapper =
            LimitedJobObject::new(1, 128 * 1024 * 1024, 25).expect("real limited Job");
        let outcome =
            wrapper.wrap_child_with_before_resume(Box::new(FixedRoot(child)), |job, process| {
                assert_eq!(unsafe { GetProcessId(process) }, original_pid);
                assert_eq!(job.active_processes().unwrap(), 1);
                assert_eq!(unsafe { WaitForSingleObject(original, 0) }, WAIT_TIMEOUT);
                assert_eq!(
                    fs::metadata(&emergency.marker).unwrap().len(),
                    0,
                    "actual root has emitted no stdout before ResumeThread"
                );
                // Publish the model original-generation stop after ACTUAL Job
                // assignment and before the wrapper's ACTUAL ResumeThread.
                at_boundary.store(true, Ordering::Release);
            });
        assert!(stop.load(Ordering::Acquire));
        match outcome {
            Err(error) => {
                assert_eq!(
                    fs::metadata(&emergency.marker).unwrap().len(),
                    0,
                    "no child stdout may be written after a denied resume"
                );
                eprintln!(
                    "ISSUE323_POSTSTOP_RESUME_DENIED: original_pid={original_pid},                      original_stop_published=true, launch_rejected={error}"
                );
            }
            Ok(owned_child) => {
                // Held Job owner remains alive while the original real root
                // writes the observable marker. This is NOT a simulated PID.
                assert_eq!(
                    unsafe { WaitForSingleObject(original, 3_000) },
                    WAIT_OBJECT_0,
                    "original real root must exit within independent test bound"
                );
                let marker = fs::read_to_string(&emergency.marker)
                    .expect("actual post-stop child code must write temp marker");
                assert!(marker.contains("REAL_CHILD_ENTERED"));
                eprintln!(
                    "ISSUE323_POSTSTOP_RESUME_EXECUTED: original_pid={original_pid},                      original_stop_published=true, real_child_marker={:?}",
                    marker.trim()
                );
                drop(owned_child);
                panic!(
                    "original stop won before native ResumeThread, but the                      already-Job-assigned Windows root still executed code"
                );
            }
        }
    }

    #[test]
    fn real_resume_failure_and_denied_job_stop_must_retain_uncertain_root() {
        use std::{
            os::windows::{
                io::{AsHandle, AsRawHandle},
                process::CommandExt,
            },
            path::PathBuf,
            process::{Child as StdChild, Command as StdCommand, Stdio},
        };
        use windows::Win32::{
            Foundation::DUPLICATE_HANDLE_OPTIONS, System::Threading::SuspendThread,
        };

        #[derive(Debug)]
        struct RefusingStopChild(StdChild);
        impl ChildWrapper for RefusingStopChild {
            fn inner(&self) -> &dyn ChildWrapper {
                self
            }
            fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
                self
            }
            fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
                self
            }
            fn process_handle(&self) -> Option<BorrowedHandle<'_>> {
                Some(self.0.as_handle())
            }
            fn start_kill(&mut self) -> Result<()> {
                Err(Error::other("test wrapper refuses to stop a real root"))
            }
        }

        // Emergency test-only custody prevents a leaked child if an assertion
        // fails. The production wrapper is deliberately deprived of the
        // genuine kernel JOB_OBJECT_TERMINATE right after assignment.
        struct EmergencyCleanup {
            root: OwnedHandle,
            full_job: Option<OwnedHandle>,
        }
        impl Drop for EmergencyCleanup {
            fn drop(&mut self) {
                if let Some(job) = &self.full_job {
                    let _ = unsafe { TerminateJobObject(HANDLE(job.as_raw_handle()), 81) };
                }
                let root = HANDLE(self.root.as_raw_handle());
                let _ = unsafe { TerminateProcess(root, 81) };
                let _ = unsafe { WaitForSingleObject(root, 5_000) };
            }
        }

        let system_root = PathBuf::from(std::env::var_os("SystemRoot").unwrap());
        let executable = system_root.join("System32").join("cmd.exe");
        assert!(executable.is_file());
        let child = StdCommand::new(&executable)
            .args(["/d", "/c", "exit /b 0"])
            .env_clear()
            .env("SystemRoot", &system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0)
            .spawn()
            .expect("real fixed Windows suspended child");
        let mut duplicate = HANDLE::default();
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(child.as_raw_handle()),
                GetCurrentProcess(),
                &mut duplicate,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .expect("independent original PROCESS handle");
        let mut emergency = EmergencyCleanup {
            root: unsafe { OwnedHandle::from_raw_handle(duplicate.0) },
            full_job: None,
        };
        let root = HANDLE(emergency.root.as_raw_handle());
        assert_eq!(unsafe { WaitForSingleObject(root, 0) }, WAIT_TIMEOUT);

        let mut limits = LimitedJobObject::new(1, 128 * 1024 * 1024, 25).unwrap();
        let result = limits.wrap_child_with_before_resume(
            Box::new(RefusingStopChild(child)),
            |job, process| {
                let pid = unsafe { GetProcessId(root) };
                assert_ne!(pid, 0);
                assert_eq!(unsafe { GetProcessId(process) }, pid);
                assert_eq!(job.active_processes().unwrap(), 1);

                // Actual kernel SuspendThread raises the genuine suspended
                // count from one to two. The production ResumeThread then
                // reports previous=2 and fails, without making root runnable.
                let snapshot = ScopedHandle(
                    unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }.unwrap(),
                );
                let mut entry = THREADENTRY32 {
                    dwSize: size_of::<THREADENTRY32>() as u32,
                    ..Default::default()
                };
                unsafe { Thread32First(snapshot.0, &mut entry) }.unwrap();
                let mut suspended = 0_u32;
                loop {
                    if entry.th32OwnerProcessID == pid {
                        let thread = unsafe {
                            OpenThread(
                                THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                                false,
                                entry.th32ThreadID,
                            )
                        }
                        .expect("original primary THREAD handle");
                        let thread = ScopedHandle(thread);
                        assert_eq!(unsafe { GetProcessIdOfThread(thread.0) }, pid);
                        assert_eq!(
                            unsafe { SuspendThread(thread.0) },
                            1,
                            "suspend the original already-suspended root thread"
                        );
                        suspended += 1;
                    }
                    match unsafe { Thread32Next(snapshot.0, &mut entry) } {
                        Ok(()) => {}
                        Err(error)
                            if error.code() == HRESULT::from_win32(ERROR_NO_MORE_FILES.0) =>
                        {
                            break;
                        }
                        Err(error) => panic!("original thread enumeration failed: {error}"),
                    }
                }
                assert_eq!(
                    suspended, 1,
                    "fixed suspended cmd root has exactly one thread"
                );

                // Preserve a full-rights duplicate of the very same kernel
                // Job for emergency cleanup, then give the wrapper a QUERY-
                // only duplicate. Real TerminateJobObject now fails.
                let mut full = HANDLE::default();
                unsafe {
                    DuplicateHandle(
                        GetCurrentProcess(),
                        job.raw(),
                        GetCurrentProcess(),
                        &mut full,
                        0,
                        false,
                        DUPLICATE_SAME_ACCESS,
                    )
                }
                .expect("retain full-rights original Job");
                emergency.full_job = Some(unsafe { OwnedHandle::from_raw_handle(full.0) });

                const JOB_OBJECT_QUERY: u32 = 0x0004;
                let mut restricted = HANDLE::default();
                unsafe {
                    DuplicateHandle(
                        GetCurrentProcess(),
                        job.raw(),
                        GetCurrentProcess(),
                        &mut restricted,
                        JOB_OBJECT_QUERY,
                        false,
                        DUPLICATE_HANDLE_OPTIONS(0),
                    )
                }
                .expect("real QUERY-only Job handle");
                let original = std::mem::replace(&mut job.0, restricted);
                unsafe { CloseHandle(original) }.unwrap();
                assert_eq!(job.active_processes().unwrap(), 1);
                assert!(
                    job.terminate(81).is_err(),
                    "must first observe actual Windows Job termination denial"
                );
                assert_eq!(unsafe { WaitForSingleObject(root, 0) }, WAIT_TIMEOUT);
            },
        );
        let error = result.expect_err("the real double-suspended thread must reject Resume");
        assert!(emergency.full_job.is_some());
        assert_eq!(
            unsafe { WaitForSingleObject(root, 0) },
            WAIT_TIMEOUT,
            "the original Windows child is still genuinely alive"
        );
        assert!(
            error
                .get_ref()
                .and_then(|source| source.downcast_ref::<UnconfirmedWindowsChildTermination>())
                .is_some(),
            "a live original Job/root and denied native termination require a TYPED uncertain error, not an ordinary resume failure: {error:?}"
        );
    }

    #[test]
    fn job_creation_failure_kills_original_suspended_root_even_if_wrapper_stop_fails() {
        use std::{
            os::windows::{
                io::{AsHandle, AsRawHandle},
                process::CommandExt,
            },
            path::PathBuf,
            process::{Child as StdChild, Command as StdCommand, Stdio},
        };
        use windows::Win32::System::Threading::TerminateProcess;

        #[derive(Debug)]
        struct RefusingStopChild(StdChild);
        impl ChildWrapper for RefusingStopChild {
            fn inner(&self) -> &dyn ChildWrapper {
                self
            }
            fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
                self
            }
            fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
                self
            }
            fn process_handle(&self) -> Option<BorrowedHandle<'_>> {
                Some(self.0.as_handle())
            }
            fn start_kill(&mut self) -> Result<()> {
                Err(Error::other("injected wrapper termination refusal"))
            }
        }

        // The test's independently retained kernel handle guarantees cleanup
        // even when the assertion fails on the unpatched implementation.
        struct EmergencyRootCleanup(OwnedHandle);
        impl Drop for EmergencyRootCleanup {
            fn drop(&mut self) {
                let root = HANDLE(self.0.as_raw_handle());
                let _ = unsafe { TerminateProcess(root, 1) };
                let _ = unsafe { WaitForSingleObject(root, 5_000) };
            }
        }

        let system_root = PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"));
        let executable = system_root.join("System32").join("cmd.exe");
        assert!(executable.is_file());
        let child = StdCommand::new(&executable)
            .args(["/d", "/c", "exit /b 0"])
            .env_clear()
            .env("SystemRoot", &system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0)
            .spawn()
            .expect("real fixed suspended Win32 root");

        let mut duplicate = HANDLE::default();
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(child.as_raw_handle()),
                GetCurrentProcess(),
                &mut duplicate,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .expect("independent original PROCESS handle");
        let guard = EmergencyRootCleanup(unsafe { OwnedHandle::from_raw_handle(duplicate.0) });
        let kernel_root = HANDLE(guard.0.as_raw_handle());
        assert_eq!(unsafe { WaitForSingleObject(kernel_root, 0) }, WAIT_TIMEOUT);

        // The public constructor rightly rejects zero CPU budget. A direct
        // invalid test-only field forces an actual Win32 Job configuration
        // error AFTER the suspended child exists, not a synthetic mock.
        let mut limits = LimitedJobObject {
            active_process_limit: 1,
            job_memory_limit: 128 * 1024 * 1024,
            cpu_percent: 0,
            without_console: true,
        };
        let core = CommandWrap::with_new(&executable, |_| {});
        let failed = limits.wrap_child(Box::new(RefusingStopChild(child)), &core);
        assert!(failed.is_err(), "invalid real kernel Job budget must fail");
        assert_eq!(
            unsafe { WaitForSingleObject(kernel_root, 3_000) },
            WAIT_OBJECT_0,
            "a failed Job creation must not abandon a live suspended root when wrapper stop fails"
        );
    }

    #[test]
    fn failed_job_assignment_stops_original_root_even_if_wrapper_refuses_to_stop() {
        use std::{
            os::windows::{
                io::{AsHandle, AsRawHandle},
                process::CommandExt,
            },
            path::PathBuf,
            process::{Child as StdChild, Command as StdCommand, Stdio},
        };
        use windows::Win32::{
            Foundation::DUPLICATE_HANDLE_OPTIONS,
            System::Threading::{PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE},
        };

        // Expose the SAME original kernel process, with enough native rights
        // to query, kill and wait, but not PROCESS_SET_QUOTA (required for Job
        // assignment). This forces an actual Windows API failure without a
        // fake return value, invalid Job, PID reopen or foreign process.
        #[derive(Debug)]
        struct RestrictedProcessChild {
            _original: StdChild, // owns the exact root for the entire wrapper lifetime
            restricted: OwnedHandle,
        }
        impl ChildWrapper for RestrictedProcessChild {
            fn inner(&self) -> &dyn ChildWrapper {
                self
            }
            fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
                self
            }
            fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
                self
            }
            fn process_handle(&self) -> Option<BorrowedHandle<'_>> {
                Some(self.restricted.as_handle())
            }
            fn start_kill(&mut self) -> Result<()> {
                Err(Error::other("injected wrapper stop refusal"))
            }
        }
        // Original independently duplicated full-rights handle proves exit
        // and guarantees emergency cleanup if the RED assertion fails.
        struct EmergencyRootCleanup(OwnedHandle);
        impl Drop for EmergencyRootCleanup {
            fn drop(&mut self) {
                let root = HANDLE(self.0.as_raw_handle());
                let _ = unsafe { TerminateProcess(root, 1) };
                let _ = unsafe { WaitForSingleObject(root, 5_000) };
            }
        }

        let system_root = PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"));
        let executable = system_root.join("System32").join("cmd.exe");
        assert!(executable.is_file(), "fixed local Windows fixture");
        let original = StdCommand::new(&executable)
            .args(["/d", "/c", "exit /b 0"])
            .env_clear()
            .env("SystemRoot", &system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0)
            .spawn()
            .expect("actual suspended native root");

        let mut full = HANDLE::default();
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(original.as_raw_handle()),
                GetCurrentProcess(),
                &mut full,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .expect("retain original independent process handle");
        let guard = EmergencyRootCleanup(unsafe { OwnedHandle::from_raw_handle(full.0) });
        let root = HANDLE(guard.0.as_raw_handle());
        assert_eq!(unsafe { WaitForSingleObject(root, 0) }, WAIT_TIMEOUT);

        let mut restricted = HANDLE::default();
        // SYNCHRONIZE is the standard Win32 access bit 0x0010_0000.
        let rights = PROCESS_TERMINATE.0 | PROCESS_QUERY_LIMITED_INFORMATION.0 | 0x0010_0000;
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(original.as_raw_handle()),
                GetCurrentProcess(),
                &mut restricted,
                rights,
                false,
                DUPLICATE_HANDLE_OPTIONS(0),
            )
        }
        .expect("make actual reduced-rights handle of SAME suspended root");
        let restricted = unsafe { OwnedHandle::from_raw_handle(restricted.0) };
        let restricted_raw = HANDLE(restricted.as_raw_handle());
        assert_eq!(unsafe { GetProcessId(restricted_raw) }, original.id());
        assert_eq!(
            unsafe { WaitForSingleObject(restricted_raw, 0) },
            WAIT_TIMEOUT
        );

        // Independent native preflight proves that a correctly configured
        // real Job is created but the reduced PROCESS HANDLE cannot be added.
        let job =
            OwnedJob::create(1, 128 * 1024 * 1024, 25).expect("valid actual kernel Job setup");
        let actual_denial = unsafe { AssignProcessToJobObject(job.raw(), restricted_raw) };
        assert!(
            actual_denial.is_err(),
            "fixture MUST prove actual Windows Job assignment denial before testing wrapper"
        );
        assert_eq!(unsafe { WaitForSingleObject(root, 0) }, WAIT_TIMEOUT);
        drop(job);

        let mut limits =
            LimitedJobObject::new(1, 128 * 1024 * 1024, 25).expect("legitimate Job limits");
        let core = CommandWrap::with_new(&executable, |_| {});
        let refused = RestrictedProcessChild {
            _original: original,
            restricted,
        };
        let result = limits.wrap_child(Box::new(refused), &core);
        assert!(result.is_err(), "native Job assignment must be denied");
        assert_eq!(
            unsafe { WaitForSingleObject(root, 3_000) },
            WAIT_OBJECT_0,
            "real denied assignment plus refusing wrapper must not abandon the original suspended root"
        );
    }

    #[test]
    fn real_opaque_wrapper_acknowledged_stop_does_not_prove_suspended_root_exit() {
        use std::{
            os::windows::{io::AsRawHandle, process::CommandExt},
            path::PathBuf,
            process::{Child as StdChild, Command as StdCommand, Stdio},
        };

        // A real ChildWrapper can hide the PROCESS handle and acknowledge a
        // stop request without actually terminating its original child.
        // This is intentionally a broken wrapper, NOT a mocked OS process.
        #[derive(Debug)]
        struct AcknowledgedButLiveChild(StdChild);
        impl ChildWrapper for AcknowledgedButLiveChild {
            fn inner(&self) -> &dyn ChildWrapper {
                self
            }
            fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
                self
            }
            fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
                self
            }
            fn start_kill(&mut self) -> Result<()> {
                Ok(())
            }
            fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
                self.0.try_wait()
            }
        }
        // Hold a separate original kernel PROCESS handle to safely terminate
        // the real suspended fixture even if the old code misclassifies it.
        struct EmergencyOriginalRoot(OwnedHandle);
        impl Drop for EmergencyOriginalRoot {
            fn drop(&mut self) {
                let root = HANDLE(self.0.as_raw_handle());
                let _ = unsafe { TerminateProcess(root, 1) };
                let _ = unsafe { WaitForSingleObject(root, 5_000) };
            }
        }

        let system_root = PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"));
        let executable = system_root.join("System32").join("cmd.exe");
        assert!(executable.is_file(), "fixed Windows executable");
        let child = StdCommand::new(&executable)
            .args(["/d", "/c", "exit /b 0"])
            .env_clear()
            .env("SystemRoot", &system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0)
            .spawn()
            .expect("real suspended Windows root");
        let mut duplicate = HANDLE::default();
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(child.as_raw_handle()),
                GetCurrentProcess(),
                &mut duplicate,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .expect("independently retain exact kernel PROCESS handle");
        let original = EmergencyOriginalRoot(unsafe { OwnedHandle::from_raw_handle(duplicate.0) });
        let root = HANDLE(original.0.as_raw_handle());
        assert_eq!(unsafe { WaitForSingleObject(root, 0) }, WAIT_TIMEOUT);

        let mut limits = LimitedJobObject::new(1, 128 * 1024 * 1024, 25).unwrap();
        let core = CommandWrap::with_new(&executable, |_| {});
        let failure = limits.wrap_child(Box::new(AcknowledgedButLiveChild(child)), &core);
        assert!(
            matches!(failure, Err(ref error) if
                error.to_string().contains("termination uncertain")),
            "an acknowledged but ineffectual stop of a real live suspended root must be UNKNOWN, not an ordinary unsupported-wrapper error: {failure:?}"
        );
        assert_eq!(
            unsafe { WaitForSingleObject(root, 0) },
            WAIT_TIMEOUT,
            "the original Windows root is still genuinely suspended and alive"
        );
    }

    #[test]
    fn real_suspended_opaque_child_is_killed_when_wrapper_cannot_obtain_process_handle() {
        use std::{
            os::windows::{io::AsRawHandle, process::CommandExt},
            path::PathBuf,
            process::{Child as StdChild, Command as StdCommand, Stdio},
        };
        use windows::Win32::System::Threading::TerminateProcess;

        // A valid terminal ChildWrapper may deliberately hide a raw process
        // HANDLE. No Tokio native child is discoverable in this wrapper chain.
        #[derive(Debug)]
        struct OpaqueSuspendedChild(StdChild);
        impl ChildWrapper for OpaqueSuspendedChild {
            fn inner(&self) -> &dyn ChildWrapper {
                self
            }
            fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
                self
            }
            fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
                self
            }
            fn start_kill(&mut self) -> Result<()> {
                self.0.kill()
            }
        }
        // Preserve independent kernel ownership through wrapper failure. The
        // guard is only a test emergency cleanup, never the production fix.
        struct TestRootCleanup(OwnedHandle);
        impl Drop for TestRootCleanup {
            fn drop(&mut self) {
                let process = HANDLE(self.0.as_raw_handle());
                let _ = unsafe { TerminateProcess(process, 1) };
                let _ = unsafe { WaitForSingleObject(process, 5_000) };
            }
        }

        let system_root =
            PathBuf::from(std::env::var_os("SystemRoot").expect("Windows system root"));
        let executable = system_root.join("System32").join("cmd.exe");
        assert!(executable.is_file());
        let child = StdCommand::new(&executable)
            .args(["/d", "/c", "exit /b 0"])
            .env_clear()
            .env("SystemRoot", &system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0)
            .spawn()
            .expect("real suspended fixed System32 child");
        let mut duplicate = HANDLE::default();
        // SAFETY: original std Child and current process handles are live;
        // the test retains a distinct non-inheritable PROCESS handle.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(child.as_raw_handle()),
                GetCurrentProcess(),
                &mut duplicate,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .expect("duplicate exact original child HANDLE for physical observation");
        let root = TestRootCleanup(unsafe { OwnedHandle::from_raw_handle(duplicate.0) });
        let process = HANDLE(root.0.as_raw_handle());
        assert_eq!(
            unsafe { WaitForSingleObject(process, 0) },
            WAIT_TIMEOUT,
            "the suspended test process has not run"
        );

        let mut limits = LimitedJobObject::new(1, 128 * 1024 * 1024, 25).unwrap();
        let core = CommandWrap::with_new(&executable, |_| {});
        let result = limits.wrap_child(Box::new(OpaqueSuspendedChild(child)), &core);
        assert!(
            matches!(result, Err(ref error) if error.to_string().contains("termination uncertain")),
            "opaque process must retain explicit termination uncertainty even if its wrapper really killed the child: {result:?}"
        );
        assert_eq!(
            unsafe { WaitForSingleObject(process, 3_000) },
            WAIT_OBJECT_0,
            "a failed containment wrapper MUST physically terminate the owned suspended child"
        );
    }

    #[test]
    fn real_foreign_suspended_thread_must_not_be_resumed_by_original_root() {
        use std::{
            os::windows::{io::AsRawHandle, process::CommandExt},
            path::PathBuf,
            process::{Child, Command as StdCommand, Stdio},
        };
        use windows::Win32::System::Threading::GetCurrentProcessId;

        struct KillOnDrop(Child);
        impl Drop for KillOnDrop {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        // The process from which this test runs is not the suspended child.
        // The borrowed thread HANDLE is a real kernel object, not a fake TID.
        let system_root =
            PathBuf::from(std::env::var_os("SystemRoot").expect("fixed Windows system root"));
        let executable = system_root.join("System32").join("cmd.exe");
        assert!(executable.is_file(), "fixed system executable must exist");
        let mut command = StdCommand::new(executable);
        command
            .arg("/d")
            .arg("/c")
            .arg("exit /b 0")
            .env_clear()
            .env("SystemRoot", system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0);
        let foreign = KillOnDrop(command.spawn().expect("real suspended process"));
        let foreign_pid = foreign.0.id();
        // SAFETY: this is a real kernel pid belonging to the suspended child.
        let original_pid = unsafe { GetCurrentProcessId() };
        assert_ne!(foreign_pid, original_pid);

        // Enumerate the actual foreign suspended thread and retain its HANDLE.
        // This models the foreign-owner outcome of a stale ToolHelp TID;
        // it does not claim to force an actual TID-reuse race.
        let snapshot = ScopedHandle(
            unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }
                .expect("real kernel thread enumeration"),
        );
        let mut entry = THREADENTRY32 {
            dwSize: size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        unsafe { Thread32First(snapshot.0, &mut entry) }.expect("first thread");
        let foreign_thread = loop {
            if entry.th32OwnerProcessID == foreign_pid {
                let thread = unsafe {
                    OpenThread(
                        THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                        false,
                        entry.th32ThreadID,
                    )
                }
                .expect("open the real suspended foreign thread");
                break ScopedHandle(thread);
            }
            unsafe { Thread32Next(snapshot.0, &mut entry) }
                .expect("find foreign suspended fixture thread");
        };

        let outcome = resume_owned_root_thread(foreign_thread.0, original_pid);
        assert!(
            matches!(outcome, Err(ref error) if error.kind() == ErrorKind::PermissionDenied && error.to_string().contains("different process")),
            "a foreign kernel thread must be refused before ResumeThread; got {outcome:?}"
        );
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(foreign.0.as_raw_handle()), 0) },
            WAIT_TIMEOUT,
            "foreign suspended process must stay unexecuted"
        );

        // The SAME real thread HANDLE must still be resumable when its true
        // owner PID is supplied, proving rejection was identity-based rather
        // than a missing query right or a dead fixture.
        assert!(
            resume_owned_root_thread(foreign_thread.0, foreign_pid)
                .expect("real original thread owner must permit one resume")
        );
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(foreign.0.as_raw_handle()), 5_000) },
            WAIT_OBJECT_0,
            "matching owner resumes real suspended cmd.exe to normal exit"
        );
    }

    #[test]
    fn real_original_thread_still_suspended_after_one_resume_must_not_certify_success() {
        use std::{
            os::windows::{io::AsRawHandle, process::CommandExt},
            path::PathBuf,
            process::{Child, Command as StdCommand, Stdio},
        };
        use windows::Win32::System::Threading::SuspendThread;

        struct KillOnDrop(Child);
        impl Drop for KillOnDrop {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        // Create a real Windows primary thread suspended exactly once.
        // Use only fixed System32 cmd.exe; no project code or network input.
        let system_root =
            PathBuf::from(std::env::var_os("SystemRoot").expect("fixed Windows root"));
        let executable = system_root.join("System32").join("cmd.exe");
        assert!(executable.is_file(), "fixed system executable is required");
        let mut command = StdCommand::new(executable);
        command
            .args(["/d", "/c", "exit /b 0"])
            .env_clear()
            .env("SystemRoot", system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0);
        let root = KillOnDrop(command.spawn().expect("real suspended root"));
        let pid = root.0.id();

        // Obtain the real original primary thread HANDLE. The process stays
        // alive and suspended until this fixture explicitly allows exit.
        let snapshot = ScopedHandle(
            unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }
                .expect("real kernel thread snapshot"),
        );
        let mut entry = THREADENTRY32 {
            dwSize: size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        unsafe { Thread32First(snapshot.0, &mut entry) }.expect("first thread");
        let thread = loop {
            if entry.th32OwnerProcessID == pid {
                let owned = unsafe {
                    OpenThread(
                        THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                        false,
                        entry.th32ThreadID,
                    )
                }
                .expect("open real original primary thread");
                break ScopedHandle(owned);
            }
            unsafe { Thread32Next(snapshot.0, &mut entry) }
                .expect("locate suspended original thread");
        };

        // SAFETY: this test owns the exact live original HANDLE. Increase
        // suspend count 1 -> 2; the next native ResumeThread does NOT execute
        // the child, and must never be reported as a successful launch.
        assert_eq!(
            unsafe { SuspendThread(thread.0) },
            1,
            "real primary thread must initially have one suspend count"
        );
        let denied = resume_owned_root_thread(thread.0, pid);
        assert!(
            matches!(denied, Err(ref e) if e.kind() == ErrorKind::PermissionDenied && e.to_string().contains("still suspended")),
            "native resume must reject a genuine old suspend count of 2; got {denied:?}"
        );
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(root.0.as_raw_handle()), 0) },
            WAIT_TIMEOUT,
            "the original process is physically still suspended"
        );

        // The first failed authorization consumed exactly one suspend count;
        // the next explicit owner recovery may actually release the root.
        assert!(
            resume_owned_root_thread(thread.0, pid)
                .expect("matching original thread with exactly one count")
        );
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(root.0.as_raw_handle()), 5_000) },
            WAIT_OBJECT_0,
            "real original root must exit after actual final resume"
        );
    }

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
    fn kernel_readback_attests_all_three_native_limits_and_hard_cpu_cap() {
        let job = OwnedJob::create(1, 256 * 1024 * 1024, 20).expect("create verified Job");
        job.verify_configured_limits(1, 256 * 1024 * 1024, 20)
            .expect("query exact original kernel policy");
        assert_eq!(job.active_processes().expect("empty prelaunch Job"), 0);
        assert!(matches!(
            job.verify_configured_limits(2, 256 * 1024 * 1024, 20),
            Err(ref error) if error.kind() == ErrorKind::InvalidData
        ));
        assert!(matches!(
            job.verify_configured_limits(1, 512 * 1024 * 1024, 20),
            Err(ref error) if error.kind() == ErrorKind::InvalidData
        ));
        assert!(matches!(
            job.verify_configured_limits(1, 256 * 1024 * 1024, 25),
            Err(ref error) if error.kind() == ErrorKind::InvalidData
        ));
    }

    #[test]
    fn changed_original_kernel_active_process_limit_fails_closed() {
        let job = OwnedJob::create(1, 256 * 1024 * 1024, 20).expect("original Job");
        let mut changed = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        changed.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_JOB_MEMORY;
        changed.BasicLimitInformation.ActiveProcessLimit = 2; // Deliberately loosen the cap.
        changed.JobMemoryLimit = 256 * 1024 * 1024;
        // SAFETY: this is a fresh, empty test-only kernel Job, with the same
        // valid buffer and size as the production setter. No root is started.
        unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectExtendedLimitInformation,
                (&changed as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .expect("fixture must genuinely change kernel Job policy");
        assert!(matches!(
            job.verify_configured_limits(1, 256 * 1024 * 1024, 20),
            Err(ref error) if error.kind() == ErrorKind::InvalidData
        ));
        assert_eq!(job.active_processes().expect("still no actual root"), 0);
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
