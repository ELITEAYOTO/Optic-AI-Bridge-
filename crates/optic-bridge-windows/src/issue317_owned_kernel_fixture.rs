//! Windows-only research fixture. All Win32/unsafe code MUST remain here,
//! not in the unsafe-forbidden Rust runtime. This gated module is not a
//! production process-launcher API and executes only the fixed current libtest.
use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    thread,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0},
        System::{
            JobObjects::{
                CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicAccountingInformation,
                JobObjectExtendedLimitInformation, QueryInformationJobObject,
                SetInformationJobObject, TerminateJobObject,
            },
            Threading::{
                CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
                DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
                InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
                PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION, STARTUPINFOEXW, STARTUPINFOW,
                UpdateProcThreadAttribute, WaitForSingleObject,
            },
        },
    },
    core::{PCWSTR, PWSTR},
};
pub struct ResearchKernelJob(OwnedHandle);
impl ResearchKernelJob {
    pub fn prepare() -> io::Result<Self> {
        // SAFETY: unnamed kernel Job, now owned exactly once by the Rust handle.
        let raw = unsafe { CreateJobObjectW(None, None) }.map_err(io::Error::other)?;
        let job = Self(unsafe { OwnedHandle::from_raw_handle(raw.0) });
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_JOB_MEMORY;
        limits.BasicLimitInformation.ActiveProcessLimit = 1;
        limits.JobMemoryLimit = 64 * 1024 * 1024;
        // SAFETY: exact fully-initialized structure, live Job and matching class.
        unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(io::Error::other)?;
        assert_eq!(job.active()?, 0, "real Job was not prepared empty");
        Ok(job)
    }
    fn raw(&self) -> HANDLE {
        HANDLE(self.0.as_raw_handle())
    }
    pub fn active(&self) -> io::Result<u32> {
        let mut value = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: borrowed live kernel Job, matching accounting layout.
        unsafe {
            QueryInformationJobObject(
                Some(self.raw()),
                JobObjectBasicAccountingInformation,
                (&mut value as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                None,
            )
        }
        .map_err(io::Error::other)?;
        Ok(value.ActiveProcesses)
    }
    /// Research-only deterministic OS-query fault. Does not call Win32 on
    /// the injected branch and MUST NOT be interpreted as a real OS error.
    /// A failed query is not proof of an empty Job or permission to reclaim.
    pub fn observe_active_with_injected_failure(&self, unavailable: bool) -> io::Result<u32> {
        if unavailable {
            return Err(io::Error::other(
                "injected native accounting observation failure",
            ));
        }
        self.active()
    }

    pub fn terminate(&self) -> io::Result<()> {
        // SAFETY: owned kernel Job, scoped synthetic process only.
        unsafe { TerminateJobObject(self.raw(), 87) }.map_err(io::Error::other)
    }
    pub fn await_empty(&self) -> io::Result<()> {
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            if self.active()? == 0 {
                return Ok(());
            }
            if Instant::now() >= until {
                return Err(io::Error::other("kernel Job remained nonempty"));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

struct Attrs(Vec<usize>);
impl Attrs {
    fn prepare() -> io::Result<Self> {
        let mut bytes = 0;
        // SAFETY: documented buffer-size probe only.
        let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut bytes) };
        if bytes == 0 {
            return Err(io::Error::other("missing Win32 attribute-list size"));
        }
        let mut values = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        let ptr = LPPROC_THREAD_ATTRIBUTE_LIST(values.as_mut_ptr().cast());
        // SAFETY: pointer is correctly aligned, and the backing memory remains live.
        unsafe { InitializeProcThreadAttributeList(Some(ptr), 1, None, &mut bytes) }
            .map_err(io::Error::other)?;
        Ok(Self(values))
    }
    fn raw(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.0.as_mut_ptr().cast())
    }
}
impl Drop for Attrs {
    fn drop(&mut self) {
        // SAFETY: only successfully initialized attribute lists can be constructed.
        unsafe { DeleteProcThreadAttributeList(self.raw()) };
    }
}

pub struct ResearchSuspendedChild {
    process: OwnedHandle,
    _thread: OwnedHandle,
}
impl ResearchSuspendedChild {
    pub fn create(job: &ResearchKernelJob, root: &Path) -> io::Result<Self> {
        let exe = env::current_exe()?.canonicalize()?;
        let mut app: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        // The Rust libtest child never resumes: no code or tool may run.
        let line = format!(r#""{}" --help"#, exe.display());
        let mut command: Vec<u16> = OsStr::new(&line).encode_wide().chain(Some(0)).collect();
        let cwd: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();

        let mut vars = BTreeMap::<String, std::ffi::OsString>::new();
        for key in ["SystemRoot", "WINDIR"] {
            if let Some(value) = env::var_os(key) {
                vars.insert(key.to_owned(), value);
            }
        }
        for key in ["TEMP", "TMP"] {
            vars.insert(key.to_owned(), root.as_os_str().to_owned());
        }
        let mut environment = Vec::<u16>::new();
        for (key, value) in vars {
            environment.extend(OsStr::new(&key).encode_wide());
            environment.push('=' as u16);
            environment.extend(value.encode_wide());
            environment.push(0);
        }
        environment.push(0);

        let jobs = [job.raw()];
        let mut attrs = Attrs::prepare()?;
        // SAFETY: configured existing kernel Job supplied as creation attribute;
        // both array and backing buffer outlive the CreateProcessW call.
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
        .map_err(io::Error::other)?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.lpAttributeList = attrs.raw();
        let mut info = PROCESS_INFORMATION::default();
        // SAFETY: fixed libtest executable, explicit minimal environment, no
        // inherited handles, and child is suspended in the PRECREATED Job.
        unsafe {
            CreateProcessW(
                PCWSTR(app.as_mut_ptr()),
                Some(PWSTR(command.as_mut_ptr())),
                None,
                None,
                false,
                CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
                Some(environment.as_ptr().cast()),
                PCWSTR(cwd.as_ptr()),
                &startup.StartupInfo as *const STARTUPINFOW,
                &mut info,
            )
        }
        .map_err(io::Error::other)?;
        Ok(Self {
            process: unsafe { OwnedHandle::from_raw_handle(info.hProcess.0) },
            _thread: unsafe { OwnedHandle::from_raw_handle(info.hThread.0) },
        })
    }
    pub fn await_exit(&self) -> io::Result<()> {
        // SAFETY: genuine owned child handle; 3-second bounded wait.
        let state = unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 3_000) };
        if state == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err(io::Error::other("suspended child termination unconfirmed"))
        }
    }
}
