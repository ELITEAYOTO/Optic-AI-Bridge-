//! Fixed Win32 research fixtures; not a process launcher API.
use optic_bridge_windows::CapturedOutputPipe;
use std::{
    env,
    ffi::{OsStr, c_void},
    fs::File,
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsHandle, AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{
            HANDLE, HANDLE_FLAG_INHERIT, SetHandleInformation, WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        System::{
            JobObjects::{
                CreateJobObjectW, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE,
                JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
                JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicAccountingInformation,
                JobObjectCpuRateControlInformation, JobObjectExtendedLimitInformation,
                QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
            },
            Threading::{
                CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
                DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
                GetExitCodeProcess, GetProcessHandleCount, InitializeProcThreadAttributeList,
                LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION, ResumeThread,
                STARTF_USESTDHANDLES, STARTUPINFOEXW, STARTUPINFOW, UpdateProcThreadAttribute,
                WaitForSingleObject,
            },
        },
    },
    core::{PCWSTR, PWSTR},
};

fn native_error(error: windows::core::Error) -> io::Error {
    let hresult = error.code().0 as u32;
    if hresult & 0xffff0000 == 0x80070000 {
        io::Error::from_raw_os_error((hresult & 0xffff) as i32)
    } else {
        io::Error::other(error)
    }
}

pub struct Job(OwnedHandle);
impl Job {
    fn new() -> io::Result<Self> {
        // SAFETY: creates an unnamed non-inheritable Job; successful result owned once.
        let raw = unsafe { CreateJobObjectW(None, None) }.map_err(native_error)?;
        let job = Self(unsafe { OwnedHandle::from_raw_handle(raw.0) });
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_JOB_MEMORY;
        limits.BasicLimitInformation.ActiveProcessLimit = 2;
        limits.JobMemoryLimit = super::MEMORY as usize;
        // SAFETY: exact initialized structure/class/size with an owned live handle.
        unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(native_error)?;
        let mut cpu = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
            ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
            ..Default::default()
        };
        cpu.Anonymous.CpuRate = super::CPU * 100;
        // SAFETY: matching CPU information structure, configured before child creation.
        unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectCpuRateControlInformation,
                (&cpu as *const JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast(),
                size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            )
        }
        .map_err(native_error)?;
        Ok(job)
    }
    fn raw(&self) -> HANDLE {
        HANDLE(self.0.as_raw_handle())
    }
    pub fn active(&self) -> io::Result<u32> {
        let mut value = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: live owned Job and matching initialized accounting buffer.
        unsafe {
            QueryInformationJobObject(
                Some(self.raw()),
                JobObjectBasicAccountingInformation,
                (&mut value as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                None,
            )
        }
        .map_err(native_error)?;
        Ok(value.ActiveProcesses)
    }
    fn stop(&self, inject_bad_handle: bool) -> io::Result<()> {
        let handle = if inject_bad_handle {
            HANDLE::default()
        } else {
            self.raw()
        };
        // SAFETY: owned Job normally; null handle only in explicit native error test.
        unsafe { TerminateJobObject(handle, 87) }.map_err(native_error)
    }
}

struct Attributes(Vec<usize>);
impl Attributes {
    fn new() -> io::Result<Self> {
        let mut bytes = 0;
        // SAFETY: documented size query, then aligned, sufficiently sized backing memory.
        let _ = unsafe { InitializeProcThreadAttributeList(None, 2, None, &mut bytes) };
        if bytes == 0 {
            return Err(io::Error::other("missing startup attribute size"));
        }
        let mut memory = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(memory.as_mut_ptr().cast());
        unsafe { InitializeProcThreadAttributeList(Some(list), 2, None, &mut bytes) }
            .map_err(native_error)?;
        Ok(Self(memory))
    }
    fn raw(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.0.as_mut_ptr().cast())
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        // SAFETY: every successfully returned Attributes initialized this exact buffer.
        unsafe { DeleteProcThreadAttributeList(self.raw()) };
    }
}

struct Child {
    process: OwnedHandle,
    thread: OwnedHandle,
}
impl Child {
    fn exited(&self) -> io::Result<bool> {
        // SAFETY: retained exact process handle; zero-time query only.
        let result = unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 0) };
        if result == WAIT_OBJECT_0 {
            Ok(true)
        } else if result == WAIT_TIMEOUT {
            Ok(false)
        } else {
            Err(io::Error::last_os_error())
        }
    }
    fn code(&self) -> io::Result<u32> {
        let mut code = 0;
        // SAFETY: live exact process handle and writable output.
        unsafe { GetExitCodeProcess(HANDLE(self.process.as_raw_handle()), &mut code) }
            .map_err(native_error)?;
        Ok(code)
    }
}

pub struct Native {
    pub job: Arc<Job>,
    child: Option<Child>,
    out: CapturedOutputPipe,
    err: CapturedOutputPipe,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub out_eof: bool,
    pub err_eof: bool,
    pub overflow: bool,
    pub association_observed: bool,
    attempted: bool,
}
impl Native {
    pub fn rejected_query_probe() -> io::Result<()> {
        let mut value = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // NULL means the caller's current Job and would NOT be an invalid query.
        // SAFETY: use a PROCESS pseudo-handle (wrong object type) with valid
        // writable buffer. Never close/modify it or the real owned Job.
        let wrong_type = unsafe { GetCurrentProcess() };
        unsafe {
            QueryInformationJobObject(
                Some(wrong_type),
                JobObjectBasicAccountingInformation,
                (&mut value as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                None,
            )
        }
        .map_err(native_error)
    }
    pub fn current_handle_count() -> io::Result<u32> {
        let mut count = 0;
        // SAFETY: current-process pseudo-handle and writable count only.
        unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) }.map_err(native_error)?;
        Ok(count)
    }
    pub fn prepare() -> io::Result<Self> {
        Ok(Self {
            job: Arc::new(Job::new()?),
            child: None,
            out: CapturedOutputPipe::new()?,
            err: CapturedOutputPipe::new()?,
            stdout: Vec::new(),
            stderr: Vec::new(),
            out_eof: false,
            err_eof: false,
            overflow: false,
            association_observed: false,
            attempted: false,
        })
    }
    pub fn create(&mut self, root: &Path, mode: &str, missing: bool) -> io::Result<()> {
        if self.attempted {
            return Err(io::Error::other("creation already attempted"));
        }
        self.attempted = true;
        let result = (|| {
            let exe = if missing {
                root.join("absent-fixture.exe")
            } else {
                env::current_exe()?
            };
            if missing {
                assert!(!exe.exists());
            }
            let mut path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
            let text = format!(
                "\"{}\" --exact process::authority_native_owner_tests::native_children::synthetic_root --ignored --nocapture --test-threads=1",
                exe.display()
            );
            let mut command: Vec<u16> = OsStr::new(&text).encode_wide().chain(Some(0)).collect();
            let cwd: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
            // Minimal explicit child environment, never ambient credentials/config.
            let mut vars = std::collections::BTreeMap::new();
            for key in ["SystemRoot", "WINDIR"] {
                if let Some(value) = env::var_os(key) {
                    vars.insert(key.to_string(), value);
                }
            }
            for key in ["TEMP", "TMP", "OPTIC317_OWNER_ROOT"] {
                vars.insert(key.into(), root.as_os_str().to_owned());
            }
            vars.insert("OPTIC317_OWNER_MODE".into(), mode.into());
            let mut environment = Vec::<u16>::new();
            for (key, value) in vars {
                environment.extend(OsStr::new(&key).encode_wide());
                environment.push(b'=' as u16);
                environment.extend(value.encode_wide());
                environment.push(0);
            }
            environment.push(0);
            let input = File::open("NUL")?;
            let input = input.as_handle().try_clone_to_owned()?;
            let out = self.out.writer_handle()?.try_clone_to_owned()?;
            let err = self.err.writer_handle()?.try_clone_to_owned()?;
            let streams = [
                HANDLE(input.as_raw_handle()),
                HANDLE(out.as_raw_handle()),
                HANDLE(err.as_raw_handle()),
            ];
            for handle in streams {
                // SAFETY: exact locally-owned handles; only these three are allowlisted.
                unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAG_INHERIT) }
                    .map_err(native_error)?;
            }
            let jobs = [self.job.raw()];
            let mut attrs = Attributes::new()?;
            // SAFETY: both arrays stay live through CreateProcessW; initialized 2-entry list.
            unsafe {
                UpdateProcThreadAttribute(
                    attrs.raw(),
                    0,
                    PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                    Some(jobs.as_ptr().cast()),
                    size_of::<HANDLE>(),
                    None,
                    None,
                )
            }
            .map_err(native_error)?;
            unsafe {
                UpdateProcThreadAttribute(
                    attrs.raw(),
                    0,
                    PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                    Some(streams.as_ptr().cast()),
                    size_of::<[HANDLE; 3]>(),
                    None,
                    None,
                )
            }
            .map_err(native_error)?;
            let mut startup = STARTUPINFOEXW::default();
            startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
            startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
            startup.StartupInfo.hStdInput = streams[0];
            startup.StartupInfo.hStdOutput = streams[1];
            startup.StartupInfo.hStdError = streams[2];
            startup.lpAttributeList = attrs.raw();
            let mut info = PROCESS_INFORMATION::default();
            // SAFETY: fixed self-test executable/arguments, explicit environment/cwd,
            // pre-configured Job association and exactly selected stdio during creation.
            unsafe {
                CreateProcessW(
                    PCWSTR(path.as_mut_ptr()),
                    Some(PWSTR(command.as_mut_ptr())),
                    None,
                    None,
                    true,
                    CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
                    Some(environment.as_ptr().cast::<c_void>()),
                    PCWSTR(cwd.as_ptr()),
                    &startup.StartupInfo as *const STARTUPINFOW,
                    &mut info,
                )
            }
            .map_err(native_error)?;
            self.child = Some(Child {
                process: unsafe { OwnedHandle::from_raw_handle(info.hProcess.0) },
                thread: unsafe { OwnedHandle::from_raw_handle(info.hThread.0) },
            });
            if self.job.active()? != 1 {
                return Err(io::Error::other(
                    "new suspended root not observed in configured Job",
                ));
            }
            self.association_observed = true;
            Ok(())
        })();
        self.close_writers();
        result
    }
    pub fn resume(&self, inject_bad_handle: bool) -> io::Result<()> {
        let child = self
            .child
            .as_ref()
            .ok_or_else(|| io::Error::other("missing owned child"))?;
        let handle = if inject_bad_handle {
            HANDLE::default()
        } else {
            HANDLE(child.thread.as_raw_handle())
        };
        // SAFETY: retained suspended thread; null only for the supervised error regression.
        let result = unsafe { ResumeThread(handle) };
        if result == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        if result != 1 {
            return Err(io::Error::other("unexpected suspend count"));
        }
        Ok(())
    }
    pub fn close_writers(&mut self) {
        self.out.close_parent_writer();
        self.err.close_parent_writer();
    }
    pub fn capture(&mut self) -> io::Result<()> {
        let mut used = self.stdout.len() + self.stderr.len();
        for (pipe, bytes, eof) in [
            (&mut self.out, &mut self.stdout, &mut self.out_eof),
            (&mut self.err, &mut self.stderr, &mut self.err_eof),
        ] {
            if *eof {
                continue;
            }
            let chunk = pipe.try_read(1024)?;
            let remaining = super::OUTPUT.saturating_sub(used);
            let accepted = remaining.min(chunk.bytes.len());
            self.overflow |= accepted != chunk.bytes.len();
            bytes.extend_from_slice(&chunk.bytes[..accepted]);
            used += accepted;
            *eof = chunk.eof;
        }
        Ok(())
    }
    pub fn root_exited(&self) -> io::Result<bool> {
        self.child.as_ref().map_or(Ok(true), Child::exited)
    }
    pub fn root_code(&self) -> io::Result<Option<u32>> {
        self.child.as_ref().map(Child::code).transpose()
    }
    pub fn terminal(&self) -> io::Result<bool> {
        Ok(self.root_exited()? && self.job.active()? == 0 && self.out_eof && self.err_eof)
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        // Test-failure fallback, NOT terminal proof. The independently owned
        // outer Job remains responsible until it confirms its whole tree empty.
        let _ = self.job.stop(false);
    }
}

pub struct StopWatch {
    cancel: mpsc::SyncSender<()>,
    worker: Option<JoinHandle<()>>,
    pub errors: Arc<AtomicU64>,
    pub first_error: Arc<AtomicU64>,
}
impl StopWatch {
    pub fn new(job: Arc<Job>, flags: [Arc<AtomicBool>; 3], fail: Arc<AtomicBool>) -> Self {
        let (cancel, receiver) = mpsc::sync_channel(1);
        let errors = Arc::new(AtomicU64::new(0));
        let counted = Arc::clone(&errors);
        let first_error = Arc::new(AtomicU64::new(0));
        let first = Arc::clone(&first_error);
        let worker = thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(4);
            loop {
                match receiver.recv_timeout(Duration::from_millis(5)) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                if (flags.iter().any(|f| f.load(Ordering::Acquire)) || Instant::now() >= until)
                    && let Err(error) = job.stop(fail.load(Ordering::Acquire))
                {
                    let code = error.raw_os_error().unwrap_or(-1) as i64 as u64;
                    let _ = first.compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
                    counted.fetch_add(1, Ordering::AcqRel);
                }
            }
        });
        Self {
            cancel,
            worker: Some(worker),
            errors,
            first_error,
        }
    }
    pub fn try_finish(&mut self) -> bool {
        let _ = self.cancel.try_send(());
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
        {
            return false;
        }
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
        true
    }
}
impl Drop for StopWatch {
    fn drop(&mut self) {
        let _ = self.cancel.try_send(());
        // Fail-closed backstop on harness failure, under the external deadline.
        // Only the explicit try_finish path proves a joined production-like end.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
