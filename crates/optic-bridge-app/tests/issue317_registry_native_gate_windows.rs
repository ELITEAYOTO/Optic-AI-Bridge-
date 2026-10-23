#![cfg(windows)]
//! #317 RESEARCH ONLY: actual SessionRegistry + Win32 ResumeThread ordering.
//! Requires an independently armed outer Windows Job, managed by a CI-only
//! supervisor. This is NOT a production process launch/close implementation.
use optic_bridge_core::{
    Capability, MonotonicTime, PrincipalId, ProjectId, SessionGrant, SessionHandle,
};
use optic_bridge_runtime::{SessionRegistry, SessionRegistryError};
use std::{
    collections::BTreeSet,
    ffi::{OsStr, c_void},
    fs, io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::{
            JobObjects::{
                CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicAccountingInformation,
                JobObjectExtendedLimitInformation, QueryInformationJobObject,
                SetInformationJobObject, TerminateJobObject,
            },
            Threading::{
                CREATE_SUSPENDED, CreateProcessW, DeleteProcThreadAttributeList,
                EXTENDED_STARTUPINFO_PRESENT, InitializeProcThreadAttributeList,
                LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION,
                ResumeThread, STARTUPINFOEXW, STARTUPINFOW, TerminateProcess,
                UpdateProcThreadAttribute,
            },
        },
    },
    core::{PCWSTR, PWSTR},
};

fn until(mut predicate: impl FnMut() -> bool, context: &str) {
    let limit = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(Instant::now() < limit, "deadline: {context}");
        thread::sleep(Duration::from_millis(5));
    }
}

fn signal(path: &Path, value: &str) {
    let mut tmp = path.to_path_buf();
    tmp.set_extension("tmp");
    fs::write(&tmp, value).expect("fixed synthetic readiness");
    fs::rename(tmp, path).expect("atomic readiness publish");
}

struct Job(HANDLE);
impl Job {
    fn create() -> io::Result<Self> {
        let raw = unsafe { CreateJobObjectW(None, None) }.map_err(io::Error::other)?;
        let job = Self(raw);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_JOB_MEMORY;
        limits.BasicLimitInformation.ActiveProcessLimit = 1;
        limits.JobMemoryLimit = 512 * 1024 * 1024;
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(io::Error::other)?;
        Ok(job)
    }
    fn count(&self) -> io::Result<u32> {
        let mut state = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        unsafe {
            QueryInformationJobObject(
                Some(self.0),
                JobObjectBasicAccountingInformation,
                (&mut state as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                None,
            )
        }
        .map_err(io::Error::other)?;
        Ok(state.ActiveProcesses)
    }
    fn terminate(&self) -> io::Result<()> {
        unsafe { TerminateJobObject(self.0, 87) }.map_err(io::Error::other)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

struct Attributes(Vec<usize>);
impl Attributes {
    fn new() -> io::Result<Self> {
        let mut bytes = 0usize;
        let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut bytes) };
        if bytes == 0 {
            return Err(io::Error::other("native startup attribute size missing"));
        }
        let mut memory = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(memory.as_mut_ptr().cast::<c_void>());
        unsafe { InitializeProcThreadAttributeList(Some(list), 1, None, &mut bytes) }
            .map_err(io::Error::other)?;
        Ok(Self(memory))
    }
    fn raw(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.0.as_mut_ptr().cast::<c_void>())
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.raw()) };
    }
}
struct Child {
    process: OwnedHandle,
    thread: OwnedHandle,
}
impl Child {
    fn create_suspended(job: &Job) -> io::Result<Self> {
        let mut attrs = Attributes::new()?;
        let handles = [job.0];
        unsafe {
            UpdateProcThreadAttribute(
                attrs.raw(),
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                Some(handles.as_ptr().cast::<c_void>()),
                size_of::<HANDLE>(),
                None,
                None,
            )
        }
        .map_err(io::Error::other)?;
        let exe = std::env::current_exe()?;
        let mut path: Vec<u16> = exe.as_os_str().encode_wide().collect();
        path.push(0);
        let cmd = format!(
            "\"{}\" --exact synthetic_native_resume_child --ignored --nocapture",
            exe.display()
        );
        let mut args: Vec<u16> = OsStr::new(&cmd).encode_wide().collect();
        args.push(0);
        let mut si = STARTUPINFOEXW::default();
        si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        si.lpAttributeList = attrs.raw();
        let mut pi = PROCESS_INFORMATION::default();
        // SAFETY: fixed self-test executable/argv and valid initialized
        // attribute memory. Job membership is established during creation.
        unsafe {
            CreateProcessW(
                PCWSTR(path.as_ptr()),
                Some(PWSTR(args.as_mut_ptr())),
                None,
                None,
                false,
                CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
                None,
                PCWSTR::null(),
                &si.StartupInfo as *const STARTUPINFOW,
                &mut pi,
            )
        }
        .map_err(io::Error::other)?;
        Ok(Self {
            process: unsafe { OwnedHandle::from_raw_handle(pi.hProcess.0) },
            thread: unsafe { OwnedHandle::from_raw_handle(pi.hThread.0) },
        })
    }
    fn resume(&self) -> io::Result<()> {
        let prior = unsafe { ResumeThread(HANDLE(self.thread.as_raw_handle())) };
        if prior == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        assert_eq!(prior, 1, "exact thread was suspended once");
        Ok(())
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        let _ = unsafe { TerminateProcess(HANDLE(self.process.as_raw_handle()), 87) };
    }
}

fn grant() -> SessionGrant {
    SessionGrant {
        handle: SessionHandle::generate().expect("session handle"),
        principal: PrincipalId::new("issue317-native").expect("synthetic principal"),
        project: ProjectId::new("synthetic").expect("synthetic project"),
        capabilities: BTreeSet::from([Capability::GitRead]),
        expires_at: MonotonicTime::from_millis(10_000),
        policy_epoch: 1,
    }
}

fn revoked(registry: &SessionRegistry, handle: &SessionHandle, now: MonotonicTime) -> bool {
    matches!(
        registry.get_active(handle, now),
        Err(SessionRegistryError::Revoked)
    )
}

#[test]
#[ignore = "only an independently supervised Windows CI test may execute this child"]
fn synthetic_native_resume_child() {
    let marker = std::env::var_os("OPTIC317_REGISTRY_MARKER").expect("synthetic marker");
    signal(Path::new(&marker), "fixed-child-executed");
}

#[test]
#[ignore = "requires pre-armed external Job; never run outside supervised CI"]
fn registry_native_resume_race_host() {
    let mode = std::env::var("OPTIC317_REGISTRY_MODE").expect("fixed mode");
    assert!(matches!(
        mode.as_str(),
        "direct-revoke-race" | "close-wins" | "launch-wins"
    ));
    let outer_gate = std::env::var_os("OPTIC317_REGISTRY_GATE").expect("outer gate");
    let ready = std::env::var_os("OPTIC317_REGISTRY_READY").expect("outer ready");
    let marker = std::env::var_os("OPTIC317_REGISTRY_MARKER").expect("execution marker");
    let outer_gate = Path::new(&outer_gate);
    let ready = Path::new(&ready);
    let marker = Path::new(&marker);
    until(|| outer_gate.exists(), "independent external Job armed");
    assert!(!marker.exists());

    let registry = Arc::new(SessionRegistry::new());
    let grant = grant();
    registry.register(grant.clone()).expect("register grant");
    let session = grant.handle;
    let now = MonotonicTime::from_millis(1);
    // This permit covers native launch/stop proof only, not unbounded
    // service observation. The current registry revoke remains synchronous.
    let permit = registry
        .begin_admission(&session, now)
        .expect("real admission");
    let job = Job::create().expect("pre-configure limited Job before process");
    let child = Child::create_suspended(&job).expect("atomically Job-owned child");
    assert_eq!(job.count().expect("suspended child membership"), 1);
    let shared_gate = Arc::new(Mutex::new(true));
    let (done_tx, done_rx) = mpsc::channel();

    if mode == "direct-revoke-race" {
        // Negative control: direct production revoke publishes closure
        // after a stale active check, but before native resume.
        assert!(registry.get_active(&session, now).is_ok());
        let cloned = Arc::clone(&registry);
        let handle = session.clone();
        let closer = thread::spawn(move || {
            done_tx
                .send(cloned.revoke(&handle))
                .expect("revoke outcome");
        });
        until(
            || revoked(&registry, &session, now),
            "real registry revoked",
        );
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        // Intentionally unsafe old pattern: resumes AFTER actual revocation
        // has already been published, proving a real authority gap.
        child.resume().expect("reproduce stale-check resume");
        until(|| marker.exists(), "executed after real revoke");
        assert!(marker.exists());
        job.terminate().expect("owned Job stop");
        until(
            || job.count().expect("native accounting") == 0,
            "native stop",
        );
        drop(permit);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("revoke result")
                .expect("first revoke")
        );
        closer.join().expect("closer join");
    } else if mode == "close-wins" {
        let cloned = Arc::clone(&registry);
        let handle = session.clone();
        let closer_gate = Arc::clone(&shared_gate);
        let closer = thread::spawn(move || {
            // Same gate guards both actual close request and native commit.
            *closer_gate.lock().expect("shared close gate") = false;
            done_tx
                .send(cloned.revoke(&handle))
                .expect("revoke outcome");
        });
        until(|| revoked(&registry, &session, now), "real registry closed");
        {
            let open = shared_gate.lock().expect("shared launch gate");
            assert!(!*open, "a completed close cannot grant a new resume");
            assert!(matches!(
                registry.begin_admission(&session, now),
                Err(SessionRegistryError::Revoked)
            ));
        }
        assert!(!marker.exists(), "suspended child did not execute");
        job.terminate().expect("reclaim suspended child");
        until(|| job.count().expect("native accounting") == 0, "Job empty");
        assert!(!marker.exists(), "close-first prevented execution");
        drop(permit);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("revoke result")
                .expect("first revoke")
        );
        closer.join().expect("closer join");
    } else {
        // Launch wins while holding a session-scoped TEST-ONLY shared gate;
        // production registry is NOT yet wired through such a gate.
        {
            let mut open = shared_gate.lock().expect("shared launch gate");
            assert!(*open);
            assert!(registry.get_active(&session, now).is_ok());
            child.resume().expect("native resume under shared gate");
            *open = true;
        }
        let cloned = Arc::clone(&registry);
        let handle = session.clone();
        let closer_gate = Arc::clone(&shared_gate);
        let closer = thread::spawn(move || {
            *closer_gate.lock().expect("shared close gate") = false;
            done_tx
                .send(cloned.revoke(&handle))
                .expect("revoke outcome");
        });
        until(|| revoked(&registry, &session, now), "real registry closed");
        until(|| marker.exists(), "pre-authorized launch executed");
        job.terminate().expect("stop owned Job");
        until(|| job.count().expect("native accounting") == 0, "Job empty");
        drop(permit);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("revoke result")
                .expect("first revoke")
        );
        closer.join().expect("closer join");
    }
    assert!(revoked(&registry, &session, now));
    assert_eq!(job.count().expect("terminal Job proof"), 0);
    drop(child);
    let summary = match mode.as_str() {
        "direct-revoke-race" => {
            "mode=direct-revoke-race;real-close-before-resume=1;marker=1;job-empty=1"
        }
        "close-wins" => "mode=close-wins;real-close=1;not-resumed=1;marker-absent=1;job-empty=1",
        "launch-wins" => "mode=launch-wins;native-resume-before-real-close=1;marker=1;job-empty=1",
        _ => unreachable!(),
    };
    signal(ready, summary);
    // Independent external supervisor kills this exact host after receipt.
    thread::sleep(Duration::from_secs(10));
    panic!("independent supervisor failed to terminate research host");
}
