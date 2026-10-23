//! Research-only integration: genuine counted admission + real JobStore entry
//! + Win32 kernel Job associated at CreateProcessW. No GitRead or real workload.
use super::{grant, reserve};
use crate::{ProcessStatus, SessionRegistry};
use optic_bridge_core::MonotonicTime;
use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fs,
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    sync::{Arc, mpsc},
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

struct KernelJob(OwnedHandle);
impl KernelJob {
    fn prepare() -> io::Result<Self> {
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
    fn active(&self) -> io::Result<u32> {
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
    fn terminate(&self) -> io::Result<()> {
        // SAFETY: owned kernel Job, scoped synthetic process only.
        unsafe { TerminateJobObject(self.raw(), 87) }.map_err(io::Error::other)
    }
    fn await_empty(&self) -> io::Result<()> {
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

struct FixedSuspendedChild {
    process: OwnedHandle,
    _thread: OwnedHandle,
}
impl FixedSuspendedChild {
    fn create(job: &KernelJob, root: &Path) -> io::Result<Self> {
        let exe = env::current_exe()?.canonicalize()?;
        let mut app: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        // The Rust libtest child never resumes: no code or tool may run.
        let line = format!("\\\"{}\\\" --help", exe.display());
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
    fn await_exit(&self) -> io::Result<()> {
        // SAFETY: genuine owned child handle; 3-second bounded wait.
        let state = unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 3_000) };
        if state == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err(io::Error::other("suspended child termination unconfirmed"))
        }
    }
}

#[test]
fn owned_registered_kernel_tree_stops_before_admission_drains() {
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (root, manager, reservation, action) = reserve(&a.handle);
    let worker_root = root.clone();

    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (observed_tx, observed_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        // Prepare a REAL kernel Job and reserve all accounting before effect.
        let job = KernelJob::prepare().expect("precreate real kill-on-close Job");
        let receipt = owned
            .research_commit_registered_owner(&reservation, now)
            .expect("short generation-bound owner commitment");
        assert!(!receipt.is_stop_required_at(now));
        let child = FixedSuspendedChild::create(&job, &worker_root)
            .expect("create fixed suspended child associated atomically with Job");
        let mut reservation = reservation;
        reservation.mark_native_created();
        assert_eq!(job.active().unwrap(), 1);
        ready_tx.send(()).unwrap();

        // This read-only signal must work WITHOUT ProcessManager::cancel_session.
        let limit = Instant::now() + Duration::from_secs(3);
        while !receipt.is_stop_required_at(now) {
            assert!(Instant::now() < limit, "owner stop not published");
            thread::yield_now();
        }
        observed_tx.send(job.active().expect("still recorded kernel child")).unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        // Keep reservation AND counted admission until POSITIVE native proof.
        job.terminate().expect("stop exact owned Job");
        child.await_exit().expect("root termination confirmed");
        job.await_empty().expect("whole native tree empty");
        reservation.finish(ProcessStatus::Stopped, None);
        drop(receipt);
        drop(reservation);
        drop(child);
        drop(job);
        drop(owned);
    });
    ready_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(
        manager
            .find_recorded_private_action(&a.handle, &action)
            .unwrap()
            .is_some()
    );
    let registry_close = Arc::clone(&registry);
    let a_close = a.handle.clone();
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let closer = thread::spawn(move || {
        done_tx.send(registry_close.revoke(&a_close)).unwrap();
    });
    assert_eq!(
        observed_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
        1,
        "worker observed stop while kernel child was STILL contained and counted"
    );
    assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert!(matches!(
        registry.remove_quiescent_inactive(&a.handle, now),
        Err(crate::SessionRegistryError::AdmissionsInFlight)
    ));
    assert!(registry.get_active(&b.handle, now).is_ok());
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    assert!(
        done_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("admission drains after positive Job-tree termination")
            .expect("registry revoke result")
    );
    closer.join().unwrap();
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}
