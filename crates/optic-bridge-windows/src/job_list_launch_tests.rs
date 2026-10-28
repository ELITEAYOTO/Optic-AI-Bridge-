//! Research-only Windows kernel fixture for #317.
//! No production GitRead launch, policy change, or native owner is provided.
//! This fixture creates a Windows system executable suspended and terminates
//! it without resuming its primary thread.

use super::*;
use std::{
    ffi::c_void,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0},
        System::JobObjects::IsProcessInJob,
        System::Threading::{
            CREATE_NO_WINDOW, CREATE_SUSPENDED, CreateProcessW, DeleteProcThreadAttributeList,
            EXTENDED_STARTUPINFO_PRESENT, InitializeProcThreadAttributeList,
            LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION,
            STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
    core::{BOOL, PCWSTR, PWSTR},
};

/// Pointer-aligned buffer whose lifetime covers the entire CreateProcessW call.
struct AttributeList {
    storage: Vec<usize>,
}

impl AttributeList {
    fn new() -> Result<Self> {
        let mut bytes = 0usize;
        // SAFETY: the null first call asks Windows for the required buffer size.
        let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut bytes) };
        if bytes == 0 {
            return Err(Error::other("Windows returned no attribute-list size"));
        }
        let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        // SAFETY: aligned owned storage has the size requested by Windows.
        unsafe {
            InitializeProcThreadAttributeList(
                Some(LPPROC_THREAD_ATTRIBUTE_LIST(
                    storage.as_mut_ptr().cast::<c_void>(),
                )),
                1,
                None,
                &mut bytes,
            )
        }
        .map_err(Error::other)?;
        Ok(Self { storage })
    }

    fn raw(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.storage.as_mut_ptr().cast::<c_void>())
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        // SAFETY: the initialized list still owns its aligned backing storage.
        unsafe { DeleteProcThreadAttributeList(self.raw()) };
    }
}

/// Always closes the synthetic child even if an assertion fails.
struct SuspendedFixtureProcess(PROCESS_INFORMATION);

impl Drop for SuspendedFixtureProcess {
    fn drop(&mut self) {
        // SAFETY: CreateProcessW returned two uniquely owned process handles.
        // The fixture never resumes its primary thread.
        unsafe {
            let _ = TerminateProcess(self.0.hProcess, 1);
            let _ = WaitForSingleObject(self.0.hProcess, 2000);
            let _ = CloseHandle(self.0.hThread);
            let _ = CloseHandle(self.0.hProcess);
        }
    }
}

#[test]
fn suspended_root_is_in_configured_job_at_createprocess_return() -> Result<()> {
    // A disposable Windows system program is used only as a suspended root.
    // No arbitrary project executable, network, or Git command is launched.
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| Error::other("SystemRoot unavailable to hosted Windows test"))?;
    let executable = PathBuf::from(system_root).join("System32").join("cmd.exe");
    if !executable.is_file() {
        return Err(Error::other("synthetic system executable unavailable"));
    }
    let application: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
    let mut command_line = application.clone();

    let job = OwnedJob::create(1, 256 * 1024 * 1024, 20)?;
    assert_eq!(job.active_processes()?, 0);

    // Attribute value must outlive AttributeList, which must outlive CreateProcessW.
    let job_handles = [job.raw()];
    let mut attributes = AttributeList::new()?;
    // SAFETY: the pointer references the live one-element Job HANDLE array.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.raw(),
            0,
            PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            Some(job_handles.as_ptr().cast::<c_void>()),
            size_of::<HANDLE>(),
            None,
            None,
        )
    }
    .map_err(Error::other)?;

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.lpAttributeList = attributes.raw();
    let mut info = PROCESS_INFORMATION::default();

    // SAFETY: executable and mutable command buffers are NUL-terminated,
    // attribute storage, Job and STARTUPINFOEX are live, and handle inheritance
    // is disabled. CREATE_SUSPENDED prevents the primary thread from running.
    unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW,
            None,
            PCWSTR::null(),
            &startup.StartupInfo,
            &mut info,
        )
    }
    .map_err(Error::other)?;
    let process = SuspendedFixtureProcess(info);

    let mut belongs = BOOL(0);
    // SAFETY: the suspended root and the owning Job remain open.
    unsafe { IsProcessInJob(process.0.hProcess, Some(job.raw()), &mut belongs) }
        .map_err(Error::other)?;
    assert!(
        belongs.as_bool(),
        "root must already belong to the kernel Job"
    );
    assert_eq!(job.active_processes()?, 1);

    // The test never resumes the root. A failed or delayed kernel termination
    // remains a test failure, not a fabricated Job-empty success.
    job.terminate(1)?;
    assert_eq!(
        unsafe { WaitForSingleObject(process.0.hProcess, 5000) },
        WAIT_OBJECT_0,
        "synthetic suspended root must terminate"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if job.active_processes()? == 0 {
            break;
        }
        if Instant::now() >= deadline {
            return Err(Error::other(
                "kernel Job remained nonempty after termination",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[test]
fn invalid_target_with_job_list_leaves_no_native_process() -> Result<()> {
    // Parent directory is an existing Windows system directory; only the
    // deliberately nonexistent executable is invalid. If somebody created it,
    // fail before attempting to launch anything.
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| Error::other("SystemRoot unavailable to hosted Windows test"))?;
    let missing = PathBuf::from(system_root)
        .join("System32")
        .join("__optic_issue317_missing_createprocess_probe_2026__.exe");
    if missing.exists() {
        return Err(Error::other("negative fixture unexpectedly exists"));
    }
    let application: Vec<u16> = missing.as_os_str().encode_wide().chain([0]).collect();
    let mut command_line = application.clone();

    let job = OwnedJob::create(1, 256 * 1024 * 1024, 20)?;
    assert_eq!(job.active_processes()?, 0);
    let handles = [job.raw()];
    let mut attributes = AttributeList::new()?;
    // SAFETY: the live Job HANDLE array and aligned attribute list remain
    // valid throughout CreateProcessW, including all failure paths.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.raw(),
            0,
            PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            Some(handles.as_ptr().cast::<c_void>()),
            size_of::<HANDLE>(),
            None,
            None,
        )
    }
    .map_err(Error::other)?;

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.lpAttributeList = attributes.raw();
    let mut info = PROCESS_INFORMATION::default();

    // SAFETY: the wide strings and STARTUPINFOEX backing storage remain live.
    // This test never requests handle inheritance or a resumed process.
    let attempted = unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW,
            None,
            PCWSTR::null(),
            &startup.StartupInfo,
            &mut info,
        )
    };
    if attempted.is_ok() {
        // Unexpected success must never leave a suspended synthetic child
        // behind, even when this regression fails.
        let _unexpected = SuspendedFixtureProcess(info);
        return Err(Error::other(
            "nonexistent executable unexpectedly created a child",
        ));
    }
    let failure = attempted.expect_err("invalid target must fail");
    assert_eq!(
        failure.code().0 as u32,
        0x8007_0002,
        "expected Win32 ERROR_FILE_NOT_FOUND, not an attribute, Job or access error: {failure}"
    );
    assert_eq!(
        job.active_processes()?,
        0,
        "failed CreateProcessW must not leave any process in the native Job"
    );
    Ok(())
}
