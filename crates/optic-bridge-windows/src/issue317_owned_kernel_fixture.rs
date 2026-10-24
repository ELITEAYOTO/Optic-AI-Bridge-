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
        Foundation::{
            DUPLICATE_HANDLE_OPTIONS, DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, WAIT_OBJECT_0,
        },
        System::{
            JobObjects::{
                CreateJobObjectW, IsProcessInJob, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
                JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
                QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
            },
            Threading::{
                CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
                DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
                InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
                PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION, STARTUPINFOEXW, STARTUPINFOW,
                TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
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

    /// Research-only negative permission assertion, performed while the
    /// delegated Job is empty. The launcher must NOT have TERMINATE rights.
    pub fn prove_terminate_denied(&self) -> io::Result<()> {
        // SAFETY: only the fixed fixture's limited borrowed Job handle is used.
        // The test invokes this before child creation, so even a regression
        // accidentally granting termination cannot kill an unrelated process.
        match unsafe { TerminateJobObject(self.raw(), 87) } {
            Err(err) if err.code().0 as u32 == 0x8007_0005 => Ok(()),
            Err(err) => Err(io::Error::other(format!(
                "expected access denied for launcher Job termination: {err}"
            ))),
            Ok(()) => Err(io::Error::other(
                "launcher Job handle unexpectedly has terminate permission",
            )),
        }
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
        Self::create_fixed(Some(job), root, false)
    }
    fn create_fixed(
        job: Option<&ResearchKernelJob>,
        root: &Path,
        launcher: bool,
    ) -> io::Result<Self> {
        let exe = env::current_exe()?.canonicalize()?;
        let mut app: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        // The Rust libtest child never resumes: no code or tool may run.
        let line = if launcher {
            format!(
                r#""{}" --exact session_registry::owned_native_research_tests::launcher_crash::launcher_host --ignored --nocapture --test-threads=1"#,
                exe.display()
            )
        } else {
            format!(r#""{}" --help"#, exe.display())
        };
        let mut command: Vec<u16> = OsStr::new(&line).encode_wide().chain(Some(0)).collect();
        let cwd: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();

        let mut vars = BTreeMap::<String, std::ffi::OsString>::new();
        for key in ["SystemRoot", "WINDIR"] {
            if let Some(value) = env::var_os(key) {
                vars.insert(key.to_owned(), value);
            }
        }
        for key in ["TEMP", "TMP", "OPTIC317_TRANSFER_ROOT"] {
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

        let mut attrs = if job.is_some() {
            Some(Attrs::prepare()?)
        } else {
            None
        };
        let jobs = job.map(|j| [j.raw()]);
        if let (Some(attrs), Some(jobs)) = (attrs.as_mut(), jobs.as_ref()) {
            // SAFETY: exact owned Job supplied at creation; arrays live through syscall.
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
        }
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = if job.is_some() {
            size_of::<STARTUPINFOEXW>()
        } else {
            size_of::<STARTUPINFOW>()
        } as u32;
        if let Some(attrs) = attrs.as_mut() {
            startup.lpAttributeList = attrs.raw();
        }
        let flags = if launcher {
            CREATE_UNICODE_ENVIRONMENT
        } else {
            CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT
        };
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
                flags,
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

/// Feature-gated fixed launcher, owned by the surviving service test host.
/// No PID lookup, caller-selected executable, shell, or caller creation flags.
pub struct ResearchCrashLauncher(ResearchSuspendedChild);
impl ResearchCrashLauncher {
    pub fn create(root: &Path) -> io::Result<Self> {
        ResearchSuspendedChild::create_fixed(None, root, true).map(Self)
    }
    pub fn terminate_and_wait(&self) -> io::Result<()> {
        // SAFETY: exact CreateProcessW result owned by this wrapper, not a PID.
        unsafe { TerminateProcess(HANDLE(self.0.process.as_raw_handle()), 93) }
            .map_err(io::Error::other)?;
        self.0.await_exit()
    }
    /// Research-only: duplicate a guardian-owned, already limited native Job
    /// INTO this fixed launcher process. The caller must retain its original
    /// Job handle; the returned integer is valid ONLY in the launcher process.
    /// A delivered value is not an ACK or terminal-cleanup proof.
    fn duplicate_preowned_job_into(
        job: &ResearchKernelJob,
        destination: HANDLE,
    ) -> windows::core::Result<u64> {
        // Win32 JOB_OBJECT_ASSIGN_PROCESS=0x0001, JOB_OBJECT_QUERY=0x0004.
        // The pinned windows crate does not export these symbolic masks.
        const LIMITED_LAUNCHER_JOB_ACCESS: u32 = 0x0001 | 0x0004;
        let mut target = HANDLE::default();
        // SAFETY: the source is this guardian's owned Job; the caller supplies
        // a target process handle for the *fixed synthetic* launcher only.
        // Neither DUPLICATE_SAME_ACCESS nor CLOSE_SOURCE may be used here.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                job.raw(),
                destination,
                &mut target,
                LIMITED_LAUNCHER_JOB_ACCESS,
                false,
                DUPLICATE_HANDLE_OPTIONS(0),
            )?;
        }
        Ok(target.0 as usize as u64)
    }

    pub fn offer_preowned_job(&self, job: &ResearchKernelJob) -> io::Result<u64> {
        Self::duplicate_preowned_job_into(job, HANDLE(self.0.process.as_raw_handle()))
            .map_err(io::Error::other)
    }

    /// Research negative: call exactly the same DuplicateHandle path with a
    /// deliberately INVALID destination process handle. On this supported
    /// hosted Win32 path the observed rejection is ERROR_INVALID_PARAMETER
    /// (HRESULT 0x80070057); any other result remains a test failure.
    /// No offer is issued,
    /// and the real guardian-owned Job must remain alive and observable.
    pub fn prove_invalid_job_destination_rejected(
        &self,
        job: &ResearchKernelJob,
    ) -> io::Result<()> {
        match Self::duplicate_preowned_job_into(job, HANDLE::default()) {
            Err(err) if err.code().0 as u32 == 0x8007_0057 => Ok(()),
            Err(err) => Err(io::Error::other(format!(
                "expected ERROR_INVALID_PARAMETER for failed native delegation: {err}"
            ))),
            Ok(_) => Err(io::Error::other(
                "invalid native delegation destination unexpectedly accepted",
            )),
        }
    }
    fn duplicate(&self, value: u64) -> io::Result<OwnedHandle> {
        let value = usize::try_from(value).map_err(io::Error::other)?;
        if value == 0 || value > isize::MAX as usize {
            return Err(io::Error::other("invalid offered handle"));
        }
        let mut result = HANDLE::default();
        // SAFETY: source is the exact owned launcher process. No CLOSE_SOURCE:
        // the launcher retains originals until acknowledgement or process death.
        unsafe {
            DuplicateHandle(
                HANDLE(self.0.process.as_raw_handle()),
                HANDLE(value as *mut _),
                GetCurrentProcess(),
                &mut result,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(io::Error::other)?;
        Ok(unsafe { OwnedHandle::from_raw_handle(result.0) })
    }
    pub fn accept_job(&self, value: u64) -> io::Result<ResearchKernelJob> {
        let job = ResearchKernelJob(self.duplicate(value)?);
        job.active()?; // Reject a non-Job object before treating it as authority.
        Ok(job)
    }
    pub fn accept_child(
        &self,
        value: u64,
        job: &ResearchKernelJob,
    ) -> io::Result<ResearchTransferredChild> {
        let process = self.duplicate(value)?;
        let mut member = Default::default();
        // SAFETY: duplicate process handle and validated owned Job. Kernel checks
        // membership by object handles; no PID number is an identity receipt.
        unsafe {
            IsProcessInJob(
                HANDLE(process.as_raw_handle()),
                Some(job.raw()),
                &mut member,
            )
        }
        .map_err(io::Error::other)?;
        if !member.as_bool() {
            return Err(io::Error::other("offered child not in accepted Job"));
        }
        Ok(ResearchTransferredChild(process))
    }
}
impl ResearchKernelJob {
    /// Opaque values in SOURCE process only; not handles in the recipient.
    /// Fixed private fixture writes these to its bounded test mailbox.
    /// Only the fixed research launcher consumes a *previously duplicated*
    /// handle received through its bounded private fixture mailbox. It does
    /// not take ownership from the guardian, which retains its original Job.
    pub fn adopt_preowned_job_offer(value: u64) -> io::Result<Self> {
        let raw = usize::try_from(value).map_err(io::Error::other)?;
        if raw == 0 || raw > isize::MAX as usize {
            return Err(io::Error::other("invalid preowned Job offer"));
        }
        // SAFETY: in this closed research harness the service duplicated this
        // Job into our process and transfers exactly one target-owned handle.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(raw as *mut _) });
        job.active()?; // Validate that the handle really refers to a Job.
        Ok(job)
    }

    pub fn transfer_offer(&self, child: &ResearchSuspendedChild) -> [u64; 2] {
        [
            self.0.as_raw_handle() as usize as u64,
            child.process.as_raw_handle() as usize as u64,
        ]
    }
}
pub struct ResearchTransferredChild(OwnedHandle);
impl ResearchTransferredChild {
    pub fn await_exit(&self) -> io::Result<()> {
        // SAFETY: validated owned process duplicate, bounded wait.
        if unsafe { WaitForSingleObject(HANDLE(self.0.as_raw_handle()), 3000) } == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err(io::Error::other("transferred root exit unconfirmed"))
        }
    }
}
