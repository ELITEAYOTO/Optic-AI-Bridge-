use std::{
    ffi::{OsStr, OsString, c_void},
    io::{Error, Result},
    marker::PhantomData,
    mem::{size_of, size_of_val},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, BorrowedHandle},
    },
    path::Path,
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, WAIT_OBJECT_0},
        Security::{
            FreeSid, GetTokenInformation,
            Isolation::{CreateAppContainerProfile, DeleteAppContainerProfile},
            PSID, SECURITY_CAPABILITIES, TOKEN_QUERY, TokenIsAppContainer,
        },
        System::Threading::{
            CREATE_NO_WINDOW, CREATE_SUSPENDED, CreateProcessW, DeleteProcThreadAttributeList,
            EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess, GetExitCodeProcess,
            InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST, OpenProcessToken,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            PROCESS_INFORMATION, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW,
            TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
    core::{HSTRING, PCWSTR, PWSTR},
};

/// Ephemeral per-user AppContainer profile owned by Optic.
///
/// This establishes an AppContainer identity only. It does not grant process
/// execution, filesystem access, or network capabilities.
#[derive(Debug)]
pub struct AppContainerProfile {
    name: HSTRING,
    sid: PSID,
}

/// Borrowed AppContainer process-creation capabilities.
///
/// The lifetime keeps the underlying profile SID alive for every safe use of
/// the raw Windows structure.
#[derive(Debug)]
pub struct AppContainerSecurityCapabilities<'a> {
    raw: SECURITY_CAPABILITIES,
    _profile: PhantomData<&'a AppContainerProfile>,
}

impl AppContainerSecurityCapabilities<'_> {
    #[must_use]
    pub fn as_raw(&self) -> &SECURITY_CAPABILITIES {
        &self.raw
    }
}

impl AppContainerProfile {
    /// Create a fresh AppContainer profile with no capabilities.
    ///
    /// Existing profiles are not reused implicitly, so stale grants cannot be
    /// inherited by a new isolation attempt.
    pub fn create(name: &str) -> Result<Self> {
        if name.is_empty() || name.len() > 64 {
            return Err(Error::other(
                "AppContainer profile name must be 1..=64 bytes",
            ));
        }

        let name = HSTRING::from(name);
        let description = HSTRING::from("Optic AI Bridge isolated process profile");
        // SAFETY: strings live through the synchronous call, no capability array
        // is supplied, and the returned SID is owned by this value.
        let sid = unsafe {
            CreateAppContainerProfile(&name, &name, &description, None).map_err(Error::other)?
        };

        Ok(Self { name, sid })
    }

    /// Borrow security capabilities for process creation in this profile.
    ///
    /// No capability SID is attached; in particular no network capability is granted.
    /// The returned value cannot outlive this profile in safe Rust.
    #[must_use]
    pub fn security_capabilities(&self) -> AppContainerSecurityCapabilities<'_> {
        AppContainerSecurityCapabilities {
            raw: SECURITY_CAPABILITIES {
                AppContainerSid: self.sid,
                Capabilities: std::ptr::null_mut(),
                CapabilityCount: 0,
                Reserved: 0,
            },
            _profile: PhantomData,
        }
    }
}

impl Drop for AppContainerProfile {
    fn drop(&mut self) {
        // SAFETY: `sid` came from CreateAppContainerProfile and is released once.
        // Profile cleanup is best-effort in Drop and cannot weaken a running child.
        unsafe {
            let _ = FreeSid(self.sid);
            let _ = DeleteAppContainerProfile(&self.name);
        }
    }
}

/// Explicit standard handles inherited by an AppContainer child.
///
/// The launcher duplicates these handles as inheritable handles and places only
/// those duplicates in `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`.
#[derive(Clone, Copy, Debug)]
pub struct AppContainerStdio<'a> {
    pub stdin: BorrowedHandle<'a>,
    pub stdout: BorrowedHandle<'a>,
    pub stderr: BorrowedHandle<'a>,
}

#[derive(Debug)]
pub struct SuspendedAppContainerProcess {
    process: HANDLE,
    thread: HANDLE,
    completed: bool,
}

impl SuspendedAppContainerProcess {
    pub fn is_appcontainer(&self) -> Result<bool> {
        let mut token = HANDLE::default();
        // SAFETY: process is live and token points to writable handle storage.
        unsafe { OpenProcessToken(self.process, TOKEN_QUERY, &mut token) }.map_err(Error::other)?;
        let _token_guard = HandleGuard(token);

        let mut value = 0u32;
        let mut returned = 0u32;
        // SAFETY: TokenIsAppContainer returns a DWORD into an exact-size buffer.
        unsafe {
            GetTokenInformation(
                token,
                TokenIsAppContainer,
                Some((&mut value as *mut u32).cast()),
                size_of::<u32>() as u32,
                &mut returned,
            )
        }
        .map_err(Error::other)?;
        Ok(value != 0)
    }

    pub fn resume(&self) -> Result<()> {
        // SAFETY: thread is the primary thread created suspended by CreateProcessW.
        let previous = unsafe { ResumeThread(self.thread) };
        if previous == u32::MAX {
            return Err(Error::last_os_error());
        }
        Ok(())
    }

    pub fn wait_exit(&mut self, timeout_ms: u32) -> Result<u32> {
        // SAFETY: process remains valid for the full bounded wait.
        let wait = unsafe { WaitForSingleObject(self.process, timeout_ms) };
        if wait != WAIT_OBJECT_0 {
            // SAFETY: fail-closed cleanup of the uniquely owned child process.
            let _ = unsafe { TerminateProcess(self.process, 1) };
            return Err(Error::other(format!(
                "AppContainer process did not exit within {timeout_ms} ms (wait={})",
                wait.0
            )));
        }

        let mut exit_code = 0u32;
        // SAFETY: process is signaled and output storage is valid.
        unsafe { GetExitCodeProcess(self.process, &mut exit_code) }.map_err(Error::other)?;
        self.completed = true;
        Ok(exit_code)
    }
}

impl Drop for SuspendedAppContainerProcess {
    fn drop(&mut self) {
        if !self.completed {
            // SAFETY: fail-closed cleanup; `process` is uniquely owned by this value.
            let _ = unsafe { TerminateProcess(self.process, 1) };
        }
        // SAFETY: both handles are uniquely owned and closed once.
        unsafe {
            let _ = CloseHandle(self.thread);
            let _ = CloseHandle(self.process);
        }
    }
}

#[derive(Debug)]
struct AttributeList {
    storage: Vec<usize>,
}

impl AttributeList {
    fn new(attribute_count: u32) -> Result<Self> {
        let mut bytes = 0usize;
        // SAFETY: sizing call intentionally passes a null list so Windows writes
        // the required byte count to `bytes`.
        let _ =
            unsafe { InitializeProcThreadAttributeList(None, attribute_count, None, &mut bytes) };
        if bytes == 0 {
            return Err(Error::other(
                "Windows returned an empty attribute-list size",
            ));
        }
        let words = bytes.div_ceil(size_of::<usize>());
        let mut storage = vec![0usize; words];
        // SAFETY: storage is pointer-aligned, sized by Windows, and stays alive
        // until DeleteProcThreadAttributeList runs.
        unsafe {
            InitializeProcThreadAttributeList(
                Some(LPPROC_THREAD_ATTRIBUTE_LIST(
                    storage.as_mut_ptr().cast::<c_void>(),
                )),
                attribute_count,
                None,
                &mut bytes,
            )
            .map_err(Error::other)?;
        }
        Ok(Self { storage })
    }

    fn raw(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.storage.as_mut_ptr().cast::<c_void>())
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        // SAFETY: list was initialized successfully and storage is still alive.
        unsafe { DeleteProcThreadAttributeList(self.raw()) };
    }
}

#[derive(Debug)]
struct HandleGuard(HANDLE);

impl HandleGuard {
    fn duplicate_inheritable(source: BorrowedHandle<'_>) -> Result<Self> {
        let process = unsafe { GetCurrentProcess() };
        let mut duplicate = HANDLE::default();
        let source = HANDLE(source.as_raw_handle());
        // SAFETY: current-process pseudo handle is valid for both source and target;
        // `source` is borrowed and `duplicate` is writable output storage.
        unsafe {
            DuplicateHandle(
                process,
                source,
                process,
                &mut duplicate,
                0,
                true,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(Error::other)?;
        Ok(Self(duplicate))
    }

    const fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for HandleGuard {
    fn drop(&mut self) {
        // SAFETY: guard uniquely owns this handle.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Create an AppContainer child suspended, with only the supplied stdio handles inherited.
///
/// The caller can verify `TokenIsAppContainer` before calling `resume`. Dropping the returned
/// value before a successful bounded `wait_exit` terminates the child fail-closed.
pub fn spawn_appcontainer_suspended(
    profile: &AppContainerProfile,
    executable: &Path,
    args: &[OsString],
    cwd: &Path,
    stdio: AppContainerStdio<'_>,
) -> Result<SuspendedAppContainerProcess> {
    let executable_wide = wide_null(executable.as_os_str());
    let cwd_wide = wide_null(cwd.as_os_str());
    let mut command_line = build_command_line(executable.as_os_str(), args);

    let stdin = HandleGuard::duplicate_inheritable(stdio.stdin)?;
    let stdout = HandleGuard::duplicate_inheritable(stdio.stdout)?;
    let stderr = HandleGuard::duplicate_inheritable(stdio.stderr)?;
    let inherited_handles = [stdin.raw(), stdout.raw(), stderr.raw()];

    let mut attributes = AttributeList::new(2)?;
    let security = profile.security_capabilities();
    // SAFETY: the list and borrowed SECURITY_CAPABILITIES remain alive through
    // CreateProcessW; the attribute identifier matches the concrete structure.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.raw(),
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            Some((security.as_raw() as *const SECURITY_CAPABILITIES).cast()),
            size_of::<SECURITY_CAPABILITIES>(),
            None,
            None,
        )
    }
    .map_err(Error::other)?;
    // SAFETY: each handle in the list is a real inheritable duplicate owned above and
    // remains live until CreateProcessW returns.
    unsafe {
        UpdateProcThreadAttribute(
            attributes.raw(),
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(inherited_handles.as_ptr().cast()),
            size_of_val(&inherited_handles),
            None,
            None,
        )
    }
    .map_err(Error::other)?;

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags |= STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = stdin.raw();
    startup.StartupInfo.hStdOutput = stdout.raw();
    startup.StartupInfo.hStdError = stderr.raw();
    startup.lpAttributeList = attributes.raw();
    let mut info = PROCESS_INFORMATION::default();

    // SAFETY: all pointers reference live NUL-terminated buffers; only explicit
    // duplicated stdio handles are inheritable, and both process attributes remain live.
    unsafe {
        CreateProcessW(
            PCWSTR(executable_wide.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            true,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW,
            None,
            PCWSTR(cwd_wide.as_ptr()),
            &startup.StartupInfo,
            &mut info,
        )
    }
    .map_err(Error::other)?;

    Ok(SuspendedAppContainerProcess {
        process: info.hProcess,
        thread: info.hThread,
        completed: false,
    })
}

fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn quote_windows_arg(value: &OsStr) -> Vec<u16> {
    let source: Vec<u16> = value.encode_wide().collect();
    let needs_quotes = source.is_empty()
        || source
            .iter()
            .any(|value| matches!(*value, 0x20 | 0x09 | 0x22));
    if !needs_quotes {
        return source;
    }

    let mut out = vec![u16::from(b'"')];
    let mut slashes = 0usize;
    for value in source {
        if value == u16::from(b'\\') {
            slashes += 1;
            continue;
        }
        if value == u16::from(b'"') {
            out.extend(std::iter::repeat_n(u16::from(b'\\'), slashes * 2 + 1));
            out.push(value);
        } else {
            out.extend(std::iter::repeat_n(u16::from(b'\\'), slashes));
            out.push(value);
        }
        slashes = 0;
    }
    out.extend(std::iter::repeat_n(u16::from(b'\\'), slashes * 2));
    out.push(u16::from(b'"'));
    out
}

fn build_command_line(executable: &OsStr, args: &[OsString]) -> Vec<u16> {
    let mut command = quote_windows_arg(executable);
    for arg in args {
        command.push(u16::from(b' '));
        command.extend(quote_windows_arg(arg));
    }
    command.push(0);
    command
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::{OsStr, OsString},
        fs::{self, OpenOptions},
        io::{Read, Seek, SeekFrom},
        os::windows::io::AsHandle,
        path::PathBuf,
        process::{Command, Stdio},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static PROFILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);
    const PROCESS_WAIT_MS: u32 = 10_000;

    fn unique_profile_name() -> String {
        format!(
            "Optic.Test.{}.{}",
            std::process::id(),
            PROFILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn system32_executable(name: &str) -> PathBuf {
        let root = std::env::var_os("SystemRoot").expect("SystemRoot");
        PathBuf::from(root).join("System32").join(name)
    }

    #[test]
    fn appcontainer_token_denies_ungranted_user_file_read() {
        let profile = AppContainerProfile::create(&unique_profile_name()).expect("create profile");
        {
            let security = profile.security_capabilities();
            assert_eq!(security.as_raw().CapabilityCount, 0);
            assert!(security.as_raw().Capabilities.is_null());
        }

        let sentinel = std::env::temp_dir().join(format!(
            "optic-appcontainer-sentinel-{}-{}.txt",
            std::process::id(),
            PROFILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&sentinel, b"optic appcontainer sentinel").expect("write sentinel");
        let _sentinel_cleanup = SentinelCleanup(sentinel.clone());

        let findstr = system32_executable("findstr.exe");
        let normal = Command::new(&findstr)
            .args([
                OsStr::new("/c:optic appcontainer sentinel"),
                sentinel.as_os_str(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("run normal findstr");
        assert!(
            normal.success(),
            "control process must be able to read sentinel"
        );

        let system32 = findstr.parent().expect("system32 parent");
        let stdin = OpenOptions::new()
            .read(true)
            .open("NUL")
            .expect("open NUL stdin");
        let stdout = OpenOptions::new()
            .write(true)
            .open("NUL")
            .expect("open NUL stdout");
        let stderr = OpenOptions::new()
            .write(true)
            .open("NUL")
            .expect("open NUL stderr");
        let args = vec![
            OsString::from("/c:optic appcontainer sentinel"),
            sentinel.as_os_str().to_os_string(),
        ];
        let mut isolated = spawn_appcontainer_suspended(
            &profile,
            &findstr,
            &args,
            system32,
            AppContainerStdio {
                stdin: stdin.as_handle(),
                stdout: stdout.as_handle(),
                stderr: stderr.as_handle(),
            },
        )
        .expect("spawn AppContainer process");

        assert!(
            isolated
                .is_appcontainer()
                .expect("query AppContainer token"),
            "kernel token must report TokenIsAppContainer before the child is resumed"
        );
        isolated.resume().expect("resume AppContainer process");
        let exit_code = isolated
            .wait_exit(PROCESS_WAIT_MS)
            .expect("wait AppContainer process");
        assert_ne!(
            exit_code, 0,
            "AppContainer without grants must not read the user-owned sentinel"
        );
    }

    #[test]
    fn appcontainer_explicit_stdio_is_captured() {
        let profile = AppContainerProfile::create(&unique_profile_name()).expect("create profile");
        let output_path = std::env::temp_dir().join(format!(
            "optic-appcontainer-stdio-{}-{}.txt",
            std::process::id(),
            PROFILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _output_cleanup = SentinelCleanup(output_path.clone());
        let stdin = OpenOptions::new()
            .read(true)
            .open("NUL")
            .expect("open NUL stdin");
        let mut output = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&output_path)
            .expect("open captured output");

        let cmd = system32_executable("cmd.exe");
        let system32 = cmd.parent().expect("system32 parent");
        let args = vec![
            OsString::from("/d"),
            OsString::from("/s"),
            OsString::from("/c"),
            OsString::from("echo optic-appcontainer-stdio"),
        ];
        let mut isolated = spawn_appcontainer_suspended(
            &profile,
            &cmd,
            &args,
            system32,
            AppContainerStdio {
                stdin: stdin.as_handle(),
                stdout: output.as_handle(),
                stderr: output.as_handle(),
            },
        )
        .expect("spawn captured AppContainer process");
        assert!(
            isolated
                .is_appcontainer()
                .expect("query AppContainer token")
        );
        isolated.resume().expect("resume captured process");
        assert_eq!(
            isolated
                .wait_exit(PROCESS_WAIT_MS)
                .expect("wait captured process"),
            0
        );
        drop(isolated);

        output
            .seek(SeekFrom::Start(0))
            .expect("rewind captured output");
        let mut captured = String::new();
        output
            .read_to_string(&mut captured)
            .expect("read captured output");
        assert!(
            captured.contains("optic-appcontainer-stdio"),
            "explicit inherited stdout must remain observable"
        );
    }

    struct SentinelCleanup(PathBuf);

    impl Drop for SentinelCleanup {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
}
