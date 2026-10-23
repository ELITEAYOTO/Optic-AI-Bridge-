//! Test-only Win32 CreateProcessW / PROC_THREAD_ATTRIBUTE_JOB_LIST proof.
//!
//! A separate supervisor owns this test process in a KILL_ON_JOB_CLOSE Job
//! BEFORE unlocking its gate. It kills only this synthetic test host and
//! accounts for the suspended child. No bridge production API is exported.
//! See docs/research/ISSUE317_NATIVE_LIFECYCLE_DECISION_2026-10-23.md.

use super::OwnedJob;
use std::{
    ffi::{OsStr, c_void},
    fs, io,
    mem::size_of,
    os::windows::{ffi::OsStrExt, io::{AsRawHandle, FromRawHandle, OwnedHandle}},
    path::Path,
    thread,
    time::{Duration, Instant},
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::HANDLE,
        System::Threading::{
            CreateProcessW, DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
            InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
            PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_JOB_LIST,
            STARTUPINFOEXW, STARTUPINFOW, UpdateProcThreadAttribute,
            CREATE_SUSPENDED, TerminateProcess,
        },
    },
};

struct NativeAttributes {
    // usize-aligned backing memory, stable until DeleteProcThreadAttributeList.
    storage: Vec<usize>,
}

impl NativeAttributes {
    fn new() -> io::Result<Self> {
        let mut bytes = 0usize;
        // First call intentionally returns ERROR_INSUFFICIENT_BUFFER, with
        // the exact byte count needed for one process creation attribute.
        let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut bytes) };
        if bytes == 0 {
            return Err(io::Error::other("attribute size unavailable"));
        }
        let words = bytes.div_ceil(size_of::<usize>());
        let mut storage = Vec::new();
        storage.try_reserve_exact(words).map_err(io::Error::other)?;
        storage.resize(words, 0usize);
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast::<c_void>());
        // SAFETY: backing storage is aligned and has >= bytes; it is retained
        // until the matching DeleteProcThreadAttributeList in Drop.
        unsafe { InitializeProcThreadAttributeList(Some(list), 1, None, &mut bytes) }
            .map_err(io::Error::other)?;
        Ok(Self { storage })
    }

    fn list(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.storage.as_mut_ptr().cast::<c_void>())
    }
}

impl Drop for NativeAttributes {
    fn drop(&mut self) {
        // SAFETY: initialization succeeded; storage outlives this call.
        unsafe { DeleteProcThreadAttributeList(self.list()) };
    }
}

struct SuspendedChild {
    process: OwnedHandle,
    _thread: OwnedHandle,
}

impl Drop for SuspendedChild {
    fn drop(&mut self) {
        // Exact owned process handle only; outer independent Job is the
        // backstop in case this synthetic host terminates unexpectedly.
        // SAFETY: OwnedHandle holds a valid PROCESS_INFORMATION handle.
        let _ = unsafe { TerminateProcess(HANDLE(self.process.as_raw_handle()), 1) };
    }
}

fn launch_suspended_with_job(job_handle: HANDLE) -> io::Result<SuspendedChild> {
    let mut attributes = NativeAttributes::new()?;
    let handles = [job_handle];
    // SAFETY: the array holds a single HANDLE for the lifetime of the
    // attribute list and CreateProcessW call. Its pointer/size match Win32.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.list(), 0, PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            Some(handles.as_ptr().cast::<c_void>()), size_of::<HANDLE>(),
            None, None,
        )
    }.map_err(io::Error::other)?;

    // Never take executable/argv from untrusted test input. This launches
    // this exact synthetic Rust test binary, suspended; no thread is resumed.
    let executable = std::env::current_exe()?;
    let mut executable_wide: Vec<u16> = executable.as_os_str().encode_wide().collect();
    executable_wide.push(0);
    // Explicit harmless target if unexpectedly resumed; no arbitrary tests.
    let command_line = format!(
        "\"{}\" --exact optic317_never_execute_synthetic_child",
        executable.display()
    );
    let mut command_wide: Vec<u16> = OsStr::new(&command_line).encode_wide().collect();
    command_wide.push(0);
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.lpAttributeList = attributes.list();
    let mut process_info = PROCESS_INFORMATION::default();
    // SAFETY: both UTF-16 strings are NUL-terminated, startup contains a
    // live initialized attribute list, and inherited handles are disabled.
    // No fallback to conventional spawn/AssignProcessToJobObject is allowed.
    unsafe {
        CreateProcessW(
            PCWSTR(executable_wide.as_ptr()),
            Some(PWSTR(command_wide.as_mut_ptr())),
            None, None, false,
            CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
            None, PCWSTR::null(),
            (&startup.StartupInfo as *const STARTUPINFOW),
            &mut process_info,
        )
    }.map_err(io::Error::other)?;
    // SAFETY: on successful CreateProcessW both handles are owned by us.
    // The exact native child remains suspended and can be killed in Drop.
    let process = unsafe { OwnedHandle::from_raw_handle(process_info.hProcess.0) };
    let thread = unsafe { OwnedHandle::from_raw_handle(process_info.hThread.0) };
    Ok(SuspendedChild { process, _thread: thread })
}

fn wait_for_gate(path: &Path) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(7);
    while !path.exists() {
        if Instant::now() >= deadline {
            return Err(io::Error::other("external ownership gate missing"));
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[test]
#[ignore = "must be run under independently armed Windows Job supervisor"]
fn native_atomic_job_list_host_fixture() {
    let mode = std::env::var("OPTIC317_RUST_MODE").expect("supervised mode");
    assert!(matches!(mode.as_str(), "atomic" | "invalid"));
    let gate = std::env::var_os("OPTIC317_RUST_GATE").expect("outer Job gate");
    let ready = std::env::var_os("OPTIC317_RUST_READY").expect("outer Job ready path");
    let gate = Path::new(&gate);
    let ready = Path::new(&ready);
    wait_for_gate(gate).expect("supervisor must own host before native effects");

    let job = OwnedJob::create(1, 512 * 1024 * 1024, 30)
        .expect("actual CPU/memory/process limits and KILL_ON_JOB_CLOSE");
    let _owned_child = if mode == "atomic" {
        let child = launch_suspended_with_job(job.raw()).expect("atomic creation");
        assert_eq!(job.active_processes().expect("native accounting"), 1);
        Some(child)
    } else {
        // No fallback path: invalid Job must reject creation.
        let invalid = HANDLE(0x7fff_ffffusize as *mut c_void);
        assert!(launch_suspended_with_job(invalid).is_err());
        assert_eq!(job.active_processes().expect("no child created"), 0);
        None
    };

    let signal = if mode == "atomic" {
        "mode=atomic;inner_active=1;suspended=1"
    } else {
        "mode=invalid;inner_active=0;no_spawn=1"
    };
    // Atomic rename prevents the supervisor observing a partially written
    // message, preserving the earlier red test's specific regression.
    let mut pending = ready.to_path_buf();
    pending.set_extension("tmp");
    fs::write(&pending, signal).expect("write finite fixed proof");
    fs::rename(&pending, ready).expect("atomically announce native state");

    // The external Job supervisor kills THIS host through its owned handle.
    // No fixture child is resumed. Outer supervisor has its own deadline.
    thread::sleep(Duration::from_secs(10));
    panic!("independent supervisor did not terminate the synthetic host");
}
