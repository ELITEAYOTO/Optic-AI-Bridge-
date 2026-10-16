use std::{
    ffi::c_void,
    future::Future,
    io::{Error, ErrorKind, Result},
    mem::size_of,
    os::windows::io::{AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    pin::Pin,
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use process_wrap::tokio::{ChildWrapper, CommandWrap, CommandWrapper};
use tokio::process::Command;
use windows::{
    Win32::{
        Foundation::{
            CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_NO_MORE_FILES, HANDLE,
        },
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE,
                JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
                JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicAccountingInformation,
                JobObjectCpuRateControlInformation, JobObjectExtendedLimitInformation,
                QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
            },
            Threading::{
                CREATE_SUSPENDED, GetCurrentProcess, GetProcessId, OpenThread,
                PROCESS_CREATION_FLAGS, ResumeThread, THREAD_SUSPEND_RESUME,
            },
        },
    },
    core::HRESULT,
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
        })
    }
}

impl CommandWrapper for LimitedJobObject {
    fn pre_spawn(&mut self, command: &mut Command, _core: &CommandWrap) -> Result<()> {
        command.creation_flags(PROCESS_CREATION_FLAGS(CREATE_SUSPENDED.0).0);
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
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: the requested information class matches `accounting` and the buffer
        // remains valid for the entire synchronous call.
        unsafe {
            QueryInformationJobObject(
                Some(self.0),
                JobObjectBasicAccountingInformation,
                (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                None,
            )
        }
        .map_err(Error::other)?;
        Ok(accounting.ActiveProcesses)
    }
}

impl Drop for OwnedJob {
    fn drop(&mut self) {
        // SAFETY: this value owns the handle and Drop runs once. KILL_ON_JOB_CLOSE makes
        // closing the final Job Object handle the containment backstop for descendants.
        let _ = unsafe { CloseHandle(self.0) };
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
}
impl NativeJobDeadline {
    fn new(job: &OwnedJob, timeout_ms: u32, stop_flags: Vec<Arc<AtomicBool>>) -> Result<Self> {
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
        let until = std::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
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
                        let _ = unsafe { TerminateJobObject(HANDLE(handle.as_raw_handle()), 1) };
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

/// Native-test-only containment for the owned suspended AppContainer probe.
/// Never accepts a PID or exposes an uncontained process to the tool surface.
#[cfg(test)]
pub(crate) struct AppContainerProbeJob(OwnedJob);

#[cfg(test)]
impl AppContainerProbeJob {
    /// Inspect only live members of this owned kernel job. PID membership is
    /// rechecked on the opened handle before token access to fence PID reuse.
    pub(crate) fn verify_active_tokens(&self, profile: &crate::AppContainerProfile) -> Result<u32> {
        use std::mem::size_of_val;
        use windows::Win32::{
            Security::{
                EqualSid, GetTokenInformation, TOKEN_APPCONTAINER_INFORMATION, TOKEN_QUERY,
                TokenAppContainerSid, TokenCapabilities, TokenIsAppContainer,
            },
            System::{
                JobObjects::{IsProcessInJob, JobObjectBasicProcessIdList},
                Threading::{
                    GetExitCodeProcess, OpenProcess, OpenProcessToken,
                    PROCESS_QUERY_LIMITED_INFORMATION,
                },
            },
        };
        #[repr(C)]
        struct ProcessIds {
            assigned: u32,
            listed: u32,
            ids: [usize; 16],
        }
        struct ProbeHandle(HANDLE);
        impl Drop for ProbeHandle {
            fn drop(&mut self) {
                // SAFETY: each successful open creates one uniquely owned handle.
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
        let mut ids = ProcessIds {
            assigned: 0,
            listed: 0,
            ids: [0; 16],
        };
        // SAFETY: exact native layout, fixed capacity equals the configured job
        // process ceiling; Windows refuses a snapshot larger than this buffer.
        unsafe {
            QueryInformationJobObject(
                Some(self.0.raw()),
                JobObjectBasicProcessIdList,
                (&mut ids as *mut ProcessIds).cast(),
                size_of::<ProcessIds>() as u32,
                None,
            )
        }
        .map_err(Error::other)?;
        if ids.assigned > 16 || ids.listed > 16 {
            return Err(Error::other("owned probe process ceiling"));
        }
        let mut verified = 0;
        for value in &ids.ids[..ids.listed as usize] {
            let pid = u32::try_from(*value)
                .map_err(|_| Error::other("invalid owned process identity"))?;
            // SAFETY: PID comes only from this owned job snapshot. No memory or
            // token is read until current handle membership is confirmed below.
            let process = match unsafe {
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            } {
                Ok(handle) => ProbeHandle(handle),
                Err(error) if error.code() == windows::core::HRESULT::from_win32(87) => continue,
                Err(error) => return Err(Error::other(error)),
            };
            let mut member = windows::core::BOOL::default();
            // SAFETY: opened process, owned job and exact BOOL output.
            unsafe { IsProcessInJob(process.0, Some(self.0.raw()), &mut member) }
                .map_err(Error::other)?;
            if !member.as_bool() {
                continue;
            }
            let mut exit = 0;
            // SAFETY: owned job member handle has query rights and valid output.
            unsafe { GetExitCodeProcess(process.0, &mut exit) }.map_err(Error::other)?;
            if exit != 259 {
                continue;
            }
            let mut token = HANDLE::default();
            // SAFETY: current member process and writable handle output.
            if let Err(error) = unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut token) } {
                // A short-lived member may exit between the liveness check
                // and opening its token. Do not mistake that for a live escape.
                unsafe { GetExitCodeProcess(process.0, &mut exit) }.map_err(Error::other)?;
                if exit != 259 {
                    continue;
                }
                return Err(Error::other(error));
            }
            let token = ProbeHandle(token);
            let mut is_container = 0_u32;
            let mut returned = 0_u32;
            // SAFETY: each information class matches a fixed, aligned bounded
            // output buffer that stays live through all pointer/SID use.
            unsafe {
                GetTokenInformation(
                    token.0,
                    TokenIsAppContainer,
                    Some((&mut is_container as *mut u32).cast()),
                    4,
                    &mut returned,
                )
                .map_err(Error::other)?;
                if is_container == 0 {
                    return Err(Error::other("owned descendant lost AppContainer isolation"));
                }
                let mut sid_buffer = [0_usize; 16];
                GetTokenInformation(
                    token.0,
                    TokenAppContainerSid,
                    Some(sid_buffer.as_mut_ptr().cast()),
                    size_of_val(&sid_buffer) as u32,
                    &mut returned,
                )
                .map_err(Error::other)?;
                let info = &*sid_buffer.as_ptr().cast::<TOKEN_APPCONTAINER_INFORMATION>();
                EqualSid(info.TokenAppContainer, profile.package_sid())
                    .map_err(|_| Error::other("owned descendant changed container identity"))?;
                let mut caps = [0_u64; 64];
                GetTokenInformation(
                    token.0,
                    TokenCapabilities,
                    Some(caps.as_mut_ptr().cast()),
                    size_of_val(&caps) as u32,
                    &mut returned,
                )
                .map_err(Error::other)?;
                if *caps.as_ptr().cast::<u32>() != 0 {
                    return Err(Error::other("owned descendant gained capability SIDs"));
                }
            }
            verified += 1;
        }
        Ok(verified)
    }

    pub(crate) fn assign(process: BorrowedHandle<'_>) -> Result<Self> {
        let job = OwnedJob::create(16, 1024 * 1024 * 1024, 50)?;
        // SAFETY: process is borrowed from the owned, still-suspended test child;
        // limits are configured before any test application code can run.
        unsafe { AssignProcessToJobObject(job.raw(), HANDLE(process.as_raw_handle())) }
            .map_err(Error::other)?;
        Ok(Self(job))
    }

    pub(crate) fn terminate_and_confirm_empty(&self) -> Result<()> {
        self.0.terminate(1)?;
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while self.0.active_processes()? != 0 {
            if std::time::Instant::now() >= until {
                return Err(Error::other("owned probe job did not become empty"));
            }
            std::thread::sleep(JOB_EMPTY_POLL);
        }
        Ok(())
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
