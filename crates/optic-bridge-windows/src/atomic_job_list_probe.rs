//! Test-only Win32 CreateProcessW / PROC_THREAD_ATTRIBUTE_JOB_LIST proof.
//!
//! A separate supervisor owns this test process in a KILL_ON_JOB_CLOSE Job
//! BEFORE unlocking its gate. It kills only this synthetic test host and
//! accounts for the suspended child. No bridge production API is exported.
//! See docs/research/ISSUE317_NATIVE_LIFECYCLE_DECISION_2026-10-23.md.

use super::OwnedJob;
use crate::CapturedOutputPipe;
use std::{
    ffi::{OsStr, c_void},
    fs, io,
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
        Foundation::{HANDLE, HANDLE_FLAG_INHERIT, SetHandleInformation},
        System::Threading::{
            CREATE_SUSPENDED, CreateProcessW, DeleteProcThreadAttributeList,
            EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess, GetProcessHandleCount,
            InitializeProcThreadAttributeList,
            LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION,
            STARTUPINFOEXW, STARTUPINFOW, TerminateProcess, UpdateProcThreadAttribute,
        },
    },
    core::{PCWSTR, PWSTR},
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

fn launch_suspended_with_job(job_handle: HANDLE, missing_binary: bool) -> io::Result<SuspendedChild> {
    let mut attributes = NativeAttributes::new()?;
    let handles = [job_handle];
    // SAFETY: the array holds a single HANDLE for the lifetime of the
    // attribute list and CreateProcessW call. Its pointer/size match Win32.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.list(),
            0,
            PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            Some(handles.as_ptr().cast::<c_void>()),
            size_of::<HANDLE>(),
            None,
            None,
        )
    }
    .map_err(io::Error::other)?;

    // Never take executable/argv from untrusted test input. This launches
    // this exact synthetic Rust test binary, suspended; no thread is resumed.
    let executable = if missing_binary {
        // Fixed synthetic nonexisting executable: error must occur within CreateProcessW,
        // AFTER Job attributes were prepared and BEFORE any child handle exists.
        let path = std::env::temp_dir().join(format!(
            "optic317-provably-absent-{}-fixture.exe", std::process::id()
        ));
        if path.exists() {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "fixture collision"));
        }
        path
    } else {
        std::env::current_exe()?
    };
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
            None,
            None,
            false,
            CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
            None,
            PCWSTR::null(),
            &startup.StartupInfo as *const STARTUPINFOW,
            &mut process_info,
        )
    }
    .map_err(io::Error::other)?;
    // SAFETY: on successful CreateProcessW both handles are owned by us.
    // The exact native child remains suspended and can be killed in Drop.
    let process = unsafe { OwnedHandle::from_raw_handle(process_info.hProcess.0) };
    let thread = unsafe { OwnedHandle::from_raw_handle(process_info.hThread.0) };
    Ok(SuspendedChild {
        process,
        _thread: thread,
    })
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
    assert!(matches!(mode.as_str(), "atomic" | "invalid" | "no-inherit" | "missing"));
    let gate = std::env::var_os("OPTIC317_RUST_GATE").expect("outer Job gate");
    let ready = std::env::var_os("OPTIC317_RUST_READY").expect("outer Job ready path");
    let gate = Path::new(&gate);
    let ready = Path::new(&ready);
    wait_for_gate(gate).expect("supervisor must own host before native effects");

    let job = OwnedJob::create(1, 512 * 1024 * 1024, 30)
        .expect("actual CPU/memory/process limits and KILL_ON_JOB_CLOSE");
    let _owned_child = match mode.as_str() {
        "atomic" => {
            let child = launch_suspended_with_job(job.raw(), false).expect("atomic creation");
            assert_eq!(job.active_processes().expect("native accounting"), 1);
            Some(child)
        }
        "no-inherit" => {
            // Deliberately make a spare pipe writer inheritable. Atomic
            // CreateProcessW must NOT leak it to the suspended child because
            // bInheritHandles=false. EOF while the child is still alive proves
            // the handle did not extend this particular pipe's lifetime.
            let mut pipe = CapturedOutputPipe::new().expect("private diagnostic pipe");
            let writer = pipe.writer_handle().expect("writer");
            // SAFETY: writer is valid/owned throughout this call; only its
            // inheritance bit changes. The parent closes it below.
            unsafe {
                SetHandleInformation(
                    HANDLE(writer.as_raw_handle()),
                    HANDLE_FLAG_INHERIT.0,
                    HANDLE_FLAG_INHERIT,
                )
            }
            .expect("make spare pipe writer inheritable");
            let child = launch_suspended_with_job(job.raw(), false).expect("atomic creation");
            assert_eq!(job.active_processes().expect("suspended child stays alive"), 1);
            pipe.close_parent_writer();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let chunk = pipe.try_read(128).expect("bounded EOF polling");
                assert!(chunk.bytes.is_empty(), "synthetic pipe has no payload");
                if chunk.eof {
                    break;
                }
                assert!(Instant::now() < deadline, "spare writer was inherited by child");
                thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(job.active_processes().expect("child still alive after EOF"), 1);
            Some(child)
        }
        "invalid" => {
            let invalid = HANDLE(0x7fff_ffffusize as *mut c_void);
            assert!(launch_suspended_with_job(invalid, false).is_err());
            assert_eq!(job.active_processes().expect("invalid Job has no child"), 0);
            None
        }
        "missing" => {
            // Repeated no-spawn failures must release attributes/temporary
            // handles. Compare after a first warm-up error, not cold startup.
            assert!(launch_suspended_with_job(job.raw(), true).is_err());
            let mut before = 0u32;
            // SAFETY: pseudo-handle for this process and writable count.
            unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut before) }
                .expect("native handle count before negative matrix");
            for _ in 0..16 {
                assert!(launch_suspended_with_job(job.raw(), true).is_err());
                assert_eq!(job.active_processes().expect("failed creation has no child"), 0);
            }
            let mut after = 0u32;
            // SAFETY: same process and count destination as above.
            unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut after) }
                .expect("native handle count after negative matrix");
            assert_eq!(before, after, "failed CreateProcessW leaked native handles");
            None
        }
        _ => unreachable!("validated fixed fixture mode"),
    };

    let signal = match mode.as_str() {
        "atomic" => "mode=atomic;inner_active=1;suspended=1",
        "no-inherit" => "mode=no-inherit;inner_active=1;pipe_eof=1;suspended=1",
        "invalid" => "mode=invalid;inner_active=0;no_spawn=1",
        "missing" => "mode=missing;inner_active=0;no_spawn=1;handles_stable=1",
        _ => unreachable!("validated fixed fixture mode"),
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
