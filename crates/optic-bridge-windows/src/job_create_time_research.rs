//! Fixed, test-only Win32 proof: a REAL kernel Job is created/configured
//! before child creation and supplied to PROC_THREAD_ATTRIBUTE_JOB_LIST.
//! Never used by production GitRead; no arbitrary executable or PID API.
use super::OwnedJob;
use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fs, io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows::{
    Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0},
        System::Threading::{
            CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
            DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
            InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
            PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION, ResumeThread, STARTUPINFOEXW,
            STARTUPINFOW, UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
    core::{PCWSTR, PWSTR},
};

fn win_error(error: windows::core::Error) -> io::Error {
    io::Error::other(error)
}

struct Attributes(Vec<usize>);
impl Attributes {
    fn new() -> io::Result<Self> {
        let mut bytes = 0usize;
        // SAFETY: first Win32 call is the documented size query.
        let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut bytes) };
        if bytes == 0 {
            return Err(io::Error::other("missing attribute-list size"));
        }
        let mut memory = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(memory.as_mut_ptr().cast());
        // SAFETY: aligned backing allocation remains pinned until Drop.
        unsafe { InitializeProcThreadAttributeList(Some(list), 1, None, &mut bytes) }
            .map_err(win_error)?;
        Ok(Self(memory))
    }

    fn raw(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.0.as_mut_ptr().cast())
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        // SAFETY: Attributes exists only after successful list initialization.
        unsafe { DeleteProcThreadAttributeList(self.raw()) };
    }
}

struct SuspendedFixture {
    // Field order guarantees a final kernel Job handle remains owned until
    // our test completes (or unwinds); KILL_ON_JOB_CLOSE contains a live child.
    job: OwnedJob,
    process: OwnedHandle,
    thread: OwnedHandle,
}
impl SuspendedFixture {
    fn create(marker: &Path, root: &Path) -> io::Result<Self> {
        // The kernel Job, with limits and kill-on-close, exists BEFORE CreateProcessW.
        let job = OwnedJob::create(1, 512 * 1024 * 1024, 25)?;
        if job.active_processes()? != 0 {
            return Err(io::Error::other("precreated Job is unexpectedly nonempty"));
        }
        let exe = env::current_exe()?.canonicalize()?;
        let mut app: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        let command = format!(
            "\"{}\" --exact job::create_time_research::synthetic_child --ignored --nocapture --test-threads=1",
            exe.display()
        );
        let mut args: Vec<u16> = OsStr::new(&command).encode_wide().chain(Some(0)).collect();
        let mut cwd: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();

        let mut vars = BTreeMap::<String, std::ffi::OsString>::new();
        for key in ["SystemRoot", "WINDIR"] {
            if let Some(value) = env::var_os(key) {
                vars.insert(key.to_owned(), value);
            }
        }
        for key in ["TEMP", "TMP"] {
            vars.insert(key.to_owned(), root.as_os_str().to_owned());
        }
        vars.insert(
            "OPTIC_317_CREATE_TIME_MARKER".to_owned(),
            marker.as_os_str().to_owned(),
        );
        let mut environment = Vec::<u16>::new();
        for (key, value) in vars {
            environment.extend(OsStr::new(&key).encode_wide());
            environment.push('=' as u16);
            environment.extend(value.encode_wide());
            environment.push(0);
        }
        environment.push(0);

        let jobs = [job.raw()];
        let mut attrs = Attributes::new()?;
        // SAFETY: the attribute list and the exact precreated Job handle stay
        // alive throughout CreateProcessW. No post-spawn AssignProcess call.
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
        .map_err(win_error)?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.lpAttributeList = attrs.raw();
        let mut info = PROCESS_INFORMATION::default();
        // SAFETY: fixed current test executable and hardcoded ignored child test;
        // no inherited handles, explicit restricted environment, suspended root
        // and pre-existing Job membership at the moment of kernel creation.
        unsafe {
            CreateProcessW(
                PCWSTR(app.as_mut_ptr()),
                Some(PWSTR(args.as_mut_ptr())),
                None,
                None,
                false,
                CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
                Some(environment.as_ptr().cast()),
                PCWSTR(cwd.as_mut_ptr()),
                &startup.StartupInfo as *const STARTUPINFOW,
                &mut info,
            )
        }
        .map_err(win_error)?;
        Ok(Self {
            job,
            process: unsafe { OwnedHandle::from_raw_handle(info.hProcess.0) },
            thread: unsafe { OwnedHandle::from_raw_handle(info.hThread.0) },
        })
    }

    fn resume(&self) -> io::Result<()> {
        // SAFETY: uniquely owned real primary thread from CreateProcessW.
        let value = unsafe { ResumeThread(HANDLE(self.thread.as_raw_handle())) };
        if value != 1 {
            return Err(io::Error::other("resume of suspended root failed"));
        }
        Ok(())
    }

    fn await_exit(&self) -> io::Result<()> {
        // SAFETY: retained process handle. Finite outer wait prevents a hang.
        let status = unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 5_000) };
        if status != WAIT_OBJECT_0 {
            return Err(io::Error::other("child did not exit within bounded wait"));
        }
        Ok(())
    }

    fn await_empty(&self) -> io::Result<()> {
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            if self.job.active_processes()? == 0 {
                return Ok(());
            }
            if Instant::now() >= until {
                return Err(io::Error::other("native Job not proven empty"));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn fixture_root(tag: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "optic-317-joblist-{tag}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create isolated fixture");
    root
}

#[test]
#[ignore = "only invoked as an explicitly controlled suspended child"]
fn synthetic_child() {
    let marker = env::var_os("OPTIC_317_CREATE_TIME_MARKER").expect("fixed fixture environment");
    fs::write(marker, b"child resumed").expect("write synthetic marker");
}

#[test]
fn precreated_kernel_job_owns_suspended_child_at_create_time() {
    let root = fixture_root("resume");
    let marker = root.join("resumed");
    let fixture = SuspendedFixture::create(&marker, &root).expect("fixed CreateProcessW");
    assert_eq!(fixture.job.active_processes().unwrap(), 1);
    assert!(!marker.exists(), "suspended child unexpectedly executed");
    fixture.resume().expect("resume fixed child");
    fixture.await_exit().expect("wait for fixed child");
    fixture.await_empty().expect("whole kernel Job empty");
    assert!(marker.exists(), "controlled child did not run");
    drop(fixture);
    fs::remove_dir_all(root).expect("clean isolated fixture");
}

#[test]
fn precreated_kernel_job_stops_suspended_child_without_resume() {
    let root = fixture_root("stop");
    let marker = root.join("must-not-run");
    let fixture = SuspendedFixture::create(&marker, &root).expect("fixed CreateProcessW");
    assert_eq!(fixture.job.active_processes().unwrap(), 1);
    assert!(!marker.exists());
    fixture
        .job
        .terminate(87)
        .expect("terminate exact owned kernel Job");
    fixture.await_exit().expect("wait for stopped child");
    fixture.await_empty().expect("prove kernel Job empty");
    assert!(
        !marker.exists(),
        "terminated suspended child ran unexpectedly"
    );
    drop(fixture);
    fs::remove_dir_all(root).expect("clean isolated fixture");
}

#[test]
#[ignore = "only an independent outer Job supervisor may kill this fixed host"]
fn synthetic_creator_host_for_supervisor() {
    let root = PathBuf::from(env::var_os("OPTIC_317_HOST_ROOT").expect("supervised root"));
    let gate = root.join("outer-owned-gate");
    let until = Instant::now() + Duration::from_secs(10);
    while !gate.exists() {
        assert!(Instant::now() < until, "outer ownership gate never arrived");
        thread::sleep(Duration::from_millis(5));
    }
    let marker = root.join("must-never-run");
    let fixture = SuspendedFixture::create(&marker, &root).expect("create-time Job association");
    assert_eq!(fixture.job.active_processes().unwrap(), 1);
    assert!(!marker.exists(), "suspended fixture must not run");
    fs::write(root.join("inner-suspended-ready"), b"kernel-job-associated")
        .expect("publish exact post-creation state");

    // The supervisor terminates THIS exact host process while its child
    // remains suspended. No destructor or best-effort cleanup can be relied on
    // in this host. A finite expiry prevents an abandoned indefinite fixture.
    let until = Instant::now() + Duration::from_secs(20);
    while Instant::now() < until {
        thread::sleep(Duration::from_millis(25));
    }
    panic!("independent supervisor failed to terminate the synthetic host");
}
