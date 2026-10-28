//! Research-only Windows kernel fixture for #317.
//! No production GitRead launch, policy change, or native owner is provided.
//! These fixtures exercise both an unresumed root and a tightly fixed,
//! resumed system root that exits without any project input or Git operation.

use super::*;
use std::{
    ffi::{OsString, c_void},
    fs::File,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::PathBuf,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, WAIT_OBJECT_0},
        System::JobObjects::IsProcessInJob,
        System::Threading::{
            CREATE_NO_WINDOW, CREATE_SUSPENDED, CreateProcessW, DeleteProcThreadAttributeList,
            EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess, GetExitCodeProcess,
            InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION,
            ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess,
            UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
    core::{BOOL, PCWSTR, PWSTR},
};

/// Pointer-aligned buffer whose lifetime covers the entire CreateProcessW call.
struct AttributeList {
    storage: Vec<usize>,
}

impl AttributeList {
    fn new(attribute_count: u32) -> Result<Self> {
        let mut bytes = 0usize;
        // SAFETY: the null first call asks Windows for the required buffer size.
        let _ =
            unsafe { InitializeProcThreadAttributeList(None, attribute_count, None, &mut bytes) };
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
                attribute_count,
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
        // The fixture may have resumed its primary thread; termination is safe
        // both for a still-suspended child and one that has already exited.
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
    let mut attributes = AttributeList::new(1)?;
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
    let mut attributes = AttributeList::new(1)?;
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

#[test]
fn resumed_root_exits_and_kernel_job_drains_after_job_list_creation() -> Result<()> {
    // Research-only physical execution proof. No Git, user project, arbitrary
    // command, network input, or service-owned native GitRead owner is involved.
    // /D disables cmd AutoRun commands, and the fixed payload only exits zero.
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| Error::other("SystemRoot unavailable to hosted Windows test"))?;
    let executable = PathBuf::from(system_root).join("System32").join("cmd.exe");
    if !executable.is_file() {
        return Err(Error::other("synthetic system executable unavailable"));
    }
    let application: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
    let mut command = OsString::from("\"");
    command.push(executable.as_os_str());
    command.push("\" /D /Q /C exit /b 0");
    let mut command_line: Vec<u16> = command.encode_wide().chain([0]).collect();

    let job = OwnedJob::create(1, 256 * 1024 * 1024, 20)?;
    assert_eq!(job.active_processes()?, 0);

    let job_handles = [job.raw()];
    let mut attributes = AttributeList::new(1)?;
    // SAFETY: the Job handle array and aligned attribute buffer outlive Win32
    // creation, and neither is shared with an untrusted caller.
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

    // SAFETY: all wide buffers and Job list stay alive for CreateProcessW.
    // Handle inheritance is disabled. The root is initially suspended, so
    // there is no execution before the Job membership assertion.
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
    // SAFETY: both OS handles remain valid for the lifetime of this check.
    unsafe { IsProcessInJob(process.0.hProcess, Some(job.raw()), &mut belongs) }
        .map_err(Error::other)?;
    assert!(
        belongs.as_bool(),
        "the root must be in the Job before resume"
    );
    assert_eq!(job.active_processes()?, 1);

    // SAFETY: a live primary thread handle is owned by this fixture; resume
    // exactly this previously suspended root, never an arbitrary PID/thread.
    let previous = unsafe { ResumeThread(process.0.hThread) };
    if previous == u32::MAX {
        return Err(Error::last_os_error());
    }
    assert!(
        previous > 0,
        "the test must actually resume a suspended root"
    );

    // A timeout is a test failure, not a fabricated exit or empty-Job proof.
    // SuspendedFixtureProcess::drop still requests a bounded root termination.
    assert_eq!(
        unsafe { WaitForSingleObject(process.0.hProcess, 5000) },
        WAIT_OBJECT_0,
        "resumed fixed system root did not exit within the deadline"
    );
    let mut code = u32::MAX;
    // SAFETY: the process handle is live and signaled, and the output buffer
    // is writable for the duration of GetExitCodeProcess.
    unsafe { GetExitCodeProcess(process.0.hProcess, &mut code) }.map_err(Error::other)?;
    assert_eq!(code, 0, "the fixed exit command must complete successfully");

    // Root-exit and kernel Job-empty are independent OS facts.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if job.active_processes()? == 0 {
            break;
        }
        if Instant::now() >= deadline {
            return Err(Error::other(
                "Job remained nonempty after resumed root exit",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[test]
fn resumed_job_root_produces_two_physical_pipe_eofs() -> Result<()> {
    // Research-only physical root and stdio proof, not production GitRead.
    // All child bytes are fixed literals and no project/user input is passed.
    // cmd /D suppresses AutoRun; the single shell command only echoes markers.
    fn inheritable_copy(handle: HANDLE) -> Result<OwnedHandle> {
        let mut duplicated = HANDLE::default();
        // SAFETY: the source belongs to this process. DuplicateHandle creates
        // a newly owned, explicitly inheritable OS handle for one fixed child.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                handle,
                GetCurrentProcess(),
                &mut duplicated,
                0,
                true,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(Error::other)?;
        // SAFETY: successful duplication transfers exclusive handle ownership.
        Ok(unsafe { OwnedHandle::from_raw_handle(duplicated.0) })
    }

    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| Error::other("SystemRoot unavailable to hosted Windows test"))?;
    let system_directory = PathBuf::from(system_root).join("System32");
    let executable = system_directory.join("cmd.exe");
    if !executable.is_file() {
        return Err(Error::other("fixed Windows system image unavailable"));
    }
    let application: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
    let working_directory: Vec<u16> = system_directory
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect();
    let mut fixed_command = OsString::from("\"");
    fixed_command.push(executable.as_os_str());
    fixed_command.push("\" /D /Q /C \"echo OPTIC_STDOUT & echo OPTIC_STDERR 1>&2\"");
    let mut command_line: Vec<u16> = fixed_command.encode_wide().chain([0]).collect();

    let job = OwnedJob::create(1, 256 * 1024 * 1024, 20)?;
    assert_eq!(job.active_processes()?, 0);
    let mut stdout = CapturedOutputPipe::new()?;
    let mut stderr = CapturedOutputPipe::new()?;
    let null_input = File::open("NUL")?;
    let child_stdin = inheritable_copy(HANDLE(null_input.as_raw_handle()))?;
    let child_stdout = inheritable_copy(HANDLE(stdout.writer_handle()?.as_raw_handle()))?;
    let child_stderr = inheritable_copy(HANDLE(stderr.writer_handle()?.as_raw_handle()))?;
    let inherited = [
        HANDLE(child_stdin.as_raw_handle()),
        HANDLE(child_stdout.as_raw_handle()),
        HANDLE(child_stderr.as_raw_handle()),
    ];

    // Both attributes are essential: the root must be in the configured Job
    // when CreateProcess returns, and only these three stdio handles may be
    // inherited. No blanket parent handle inheritance is intended.
    let job_handles = [job.raw()];
    let mut attributes = AttributeList::new(2)?;
    // SAFETY: Job and stdio handle arrays are live, stable in memory and
    // fully initialized. The attribute buffer outlives CreateProcessW.
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
    // SAFETY: the three handles have explicitly inheritable duplicates.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.raw(),
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(inherited.as_ptr().cast::<c_void>()),
            size_of::<HANDLE>() * inherited.len(),
            None,
            None,
        )
    }
    .map_err(Error::other)?;

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = inherited[0];
    startup.StartupInfo.hStdOutput = inherited[1];
    startup.StartupInfo.hStdError = inherited[2];
    startup.lpAttributeList = attributes.raw();
    let mut info = PROCESS_INFORMATION::default();

    // SAFETY: all paths, mutable command bytes and both attribute/handle
    // arrays outlive this call. CREATE_SUSPENDED prevents running before
    // verifying the Job. TRUE inheritance is restricted by HANDLE_LIST.
    unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            true,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW,
            None,
            PCWSTR(working_directory.as_ptr()),
            &startup.StartupInfo,
            &mut info,
        )
    }
    .map_err(Error::other)?;
    let process = SuspendedFixtureProcess(info);

    let mut belongs = BOOL(0);
    // SAFETY: the kernel Job and unique root process handle remain live.
    unsafe { IsProcessInJob(process.0.hProcess, Some(job.raw()), &mut belongs) }
        .map_err(Error::other)?;
    assert!(
        belongs.as_bool(),
        "root must be Job-associated before resume"
    );
    assert_eq!(job.active_processes()?, 1);

    // Parent-owned original writers and all temporary inherited duplicates
    // must be closed before EOF could be observed. Retain only the readers.
    stdout.close_parent_writer();
    stderr.close_parent_writer();
    drop((child_stdin, child_stdout, child_stderr));

    // SAFETY: this is the primary thread handle of our suspended child only.
    let resume_count = unsafe { ResumeThread(process.0.hThread) };
    if resume_count == u32::MAX {
        return Err(Error::last_os_error());
    }
    assert!(resume_count > 0, "the root must have genuinely resumed");

    // Root exit, Job empty and each pipe EOF are independent physical facts.
    assert_eq!(
        unsafe { WaitForSingleObject(process.0.hProcess, 5000) },
        WAIT_OBJECT_0,
        "resumed root did not exit within the fixture deadline"
    );
    let mut exit_code = u32::MAX;
    // SAFETY: the signaled process handle and writable output are valid.
    unsafe { GetExitCodeProcess(process.0.hProcess, &mut exit_code) }.map_err(Error::other)?;
    assert_eq!(exit_code, 0, "fixed echo command must exit successfully");

    let until = Instant::now() + Duration::from_secs(5);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut out_eof = false;
    let mut err_eof = false;
    while !out_eof || !err_eof || job.active_processes()? != 0 {
        if !out_eof {
            let chunk = stdout.try_read(1024)?;
            out.extend(chunk.bytes);
            out_eof = chunk.eof;
        }
        if !err_eof {
            let chunk = stderr.try_read(1024)?;
            err.extend(chunk.bytes);
            err_eof = chunk.eof;
        }
        if Instant::now() >= until {
            return Err(Error::other(
                "root exited but Job-empty and both physical pipe EOFs were not observed",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(
        String::from_utf8_lossy(&out).contains("OPTIC_STDOUT"),
        "the stdout pipe must contain its real child marker"
    );
    assert!(
        String::from_utf8_lossy(&err).contains("OPTIC_STDERR"),
        "the stderr pipe must contain its independent real child marker"
    );
    Ok(())
}

#[test]
fn descendant_keeps_both_pipes_open_after_root_exit_until_job_termination() -> Result<()> {
    // Hosted-Windows research fixture, not a GitRead service launcher. A
    // fixed cmd.exe root uses START /B to spawn a long-lived system PowerShell
    // child with inherited stdout/stderr; the root itself exits promptly.
    // No project/user input or remote/network operation is involved.
    fn inheritable_copy(handle: HANDLE) -> Result<OwnedHandle> {
        let mut duplicated = HANDLE::default();
        // SAFETY: the source is owned by this process and the successful
        // duplicate becomes an independently owned, inheritable OS handle.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                handle,
                GetCurrentProcess(),
                &mut duplicated,
                0,
                true,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(Error::other)?;
        // SAFETY: successful duplication transfers this unique handle.
        Ok(unsafe { OwnedHandle::from_raw_handle(duplicated.0) })
    }

    let root = PathBuf::from(
        std::env::var_os("SystemRoot")
            .ok_or_else(|| Error::other("SystemRoot unavailable to Windows test"))?,
    );
    let system_directory = root.join("System32");
    let executable = system_directory.join("cmd.exe");
    let descendant = system_directory
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if !executable.is_file() || !descendant.is_file() {
        return Err(Error::other(
            "required fixed Windows system image unavailable",
        ));
    }
    let application: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
    let working_directory: Vec<u16> = system_directory
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect();

    // START /B intentionally inherits the root's console-style standard
    // handles. The descendant sleeps, holding both pipe writers, while cmd
    // returns 0. The fixture then stops it through the containing Job.
    let mut command = OsString::from("\"");
    command.push(executable.as_os_str());
    command.push("\" /D /Q /C start \"\" /B \"");
    command.push(descendant.as_os_str());
    command.push("\" -NoLogo -NoProfile -NonInteractive -Command Start-Sleep -Seconds 45");
    let mut command_line: Vec<u16> = command.encode_wide().chain([0]).collect();

    // Leave room for the fixed root and its descendant; no arbitrary launch
    // arguments or executable identity are accepted by this test.
    let job = OwnedJob::create(4, 512 * 1024 * 1024, 20)?;
    let mut stdout = CapturedOutputPipe::new()?;
    let mut stderr = CapturedOutputPipe::new()?;
    let nul = File::open("NUL")?;
    let stdin_handle = inheritable_copy(HANDLE(nul.as_raw_handle()))?;
    let stdout_handle = inheritable_copy(HANDLE(stdout.writer_handle()?.as_raw_handle()))?;
    let stderr_handle = inheritable_copy(HANDLE(stderr.writer_handle()?.as_raw_handle()))?;
    let inherited = [
        HANDLE(stdin_handle.as_raw_handle()),
        HANDLE(stdout_handle.as_raw_handle()),
        HANDLE(stderr_handle.as_raw_handle()),
    ];

    let jobs = [job.raw()];
    let mut attributes = AttributeList::new(2)?;
    // SAFETY: the arrays and aligned attribute-list buffer remain live and
    // stable through the native creation call.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.raw(),
            0,
            PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            Some(jobs.as_ptr().cast::<c_void>()),
            size_of::<HANDLE>(),
            None,
            None,
        )
    }
    .map_err(Error::other)?;
    // SAFETY: the list restricts inheritance to only the three specifically
    // duplicated handles, never arbitrary handles from the hosting process.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.raw(),
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(inherited.as_ptr().cast::<c_void>()),
            size_of::<HANDLE>() * inherited.len(),
            None,
            None,
        )
    }
    .map_err(Error::other)?;

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = inherited[0];
    startup.StartupInfo.hStdOutput = inherited[1];
    startup.StartupInfo.hStdError = inherited[2];
    startup.lpAttributeList = attributes.raw();
    let mut info = PROCESS_INFORMATION::default();

    // SAFETY: the fixed executable, writable command line, cwd, inherited
    // handles and attributes outlive this call. The root starts suspended
    // and is assigned to the Job at creation, before it may run.
    unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            true,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW,
            None,
            PCWSTR(working_directory.as_ptr()),
            &startup.StartupInfo,
            &mut info,
        )
    }
    .map_err(Error::other)?;
    let process = SuspendedFixtureProcess(info);
    let mut belongs = BOOL(0);
    // SAFETY: the Job and suspended process handles are still owned here.
    unsafe { IsProcessInJob(process.0.hProcess, Some(job.raw()), &mut belongs) }
        .map_err(Error::other)?;
    assert!(belongs.as_bool(), "root must be in Job before resume");
    assert_eq!(job.active_processes()?, 1);

    // The only remaining writers after this point belong to processes.
    stdout.close_parent_writer();
    stderr.close_parent_writer();
    drop((stdin_handle, stdout_handle, stderr_handle));

    // SAFETY: resume exactly the owned primary thread of this suspended root.
    let previous = unsafe { ResumeThread(process.0.hThread) };
    if previous == u32::MAX {
        return Err(Error::last_os_error());
    }
    assert!(previous > 0, "root was never resumed");
    assert_eq!(
        unsafe { WaitForSingleObject(process.0.hProcess, 10000) },
        WAIT_OBJECT_0,
        "fixed root failed to exit while its descendant should be sleeping"
    );
    let mut exit_code = u32::MAX;
    // SAFETY: the root handle is signaled, live and the output is writable.
    unsafe { GetExitCodeProcess(process.0.hProcess, &mut exit_code) }.map_err(Error::other)?;
    assert_eq!(exit_code, 0, "the fixed START root must exit successfully");

    // Demonstrate the bug's essential distinction: a successful, signaled
    // root is NOT proof that its descendant or inherited pipes are finished.
    assert!(
        job.active_processes()? >= 1,
        "expected a still-live descendant after root exit"
    );
    assert!(
        !stdout.try_read(64)?.eof,
        "stdout must not report physical EOF while descendant owns a writer"
    );
    assert!(
        !stderr.try_read(64)?.eof,
        "stderr must not report physical EOF while descendant owns a writer"
    );

    // The Job, not a root-only TerminateProcess, closes the descendant. Keep
    // root exit, Job accounting and BOTH pipe EOFs as separate OS predicates.
    job.terminate(1)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut stdout_eof = false;
    let mut stderr_eof = false;
    loop {
        if !stdout_eof {
            stdout_eof = stdout.try_read(1024)?.eof;
        }
        if !stderr_eof {
            stderr_eof = stderr.try_read(1024)?.eof;
        }
        if job.active_processes()? == 0 && stdout_eof && stderr_eof {
            break;
        }
        if Instant::now() >= deadline {
            return Err(Error::other(
                "Job termination did not yield Job-empty and both physical EOFs",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
