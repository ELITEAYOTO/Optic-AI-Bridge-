#![forbid(unsafe_code)]

#[cfg(windows)]
mod windows_launcher {
    use std::{
        error::Error,
        fs::{self, OpenOptions},
        io::{Error as IoError, ErrorKind, Read},
        os::windows::{ffi::OsStrExt, io::AsHandle},
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use optic_bridge_core::HardLimits;
    use optic_bridge_windows::{
        AppContainerProfile, AppContainerStdio, spawn_appcontainer_suspended,
    };
    use serde::Deserialize;

    const PROTOCOL_VERSION: u32 = 1;
    const MAX_REQUEST_BYTES: usize = 64 * 1024;
    const MAX_ARGS: usize = 128;
    const MAX_COMMAND_WORST_CASE_UTF16_UNITS: usize = 30_000;
    const INTERNAL_FAILURE_EXIT: i32 = 126;

    type LauncherResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LauncherRequest {
        version: u32,
        executable: String,
        args: Vec<String>,
        cwd: String,
        timeout_ms: u64,
    }

    pub fn entry() -> i32 {
        match run() {
            Ok(exit_code) => exit_code,
            Err(error) => {
                eprintln!("optic isolation launcher failed: {error}");
                INTERNAL_FAILURE_EXIT
            }
        }
    }

    fn run() -> LauncherResult<i32> {
        let request = read_request(std::io::stdin().lock())?;
        let executable = canonical_file(&request.executable)?;
        let cwd = canonical_directory(&request.cwd)?;
        validate_command_size(&executable, &request.args)?;

        let profile = AppContainerProfile::create(&unique_profile_name()?)?;
        let null_stdin = OpenOptions::new().read(true).open("NUL")?;
        let stdout = std::io::stdout();
        let stderr = std::io::stderr();
        let args = request.args.into_iter().map(Into::into).collect::<Vec<_>>();

        let mut child = spawn_appcontainer_suspended(
            &profile,
            &executable,
            &args,
            &cwd,
            AppContainerStdio {
                stdin: null_stdin.as_handle(),
                stdout: stdout.as_handle(),
                stderr: stderr.as_handle(),
            },
        )?;
        if !child.is_appcontainer()? {
            return Err(IoError::other("child token is not an AppContainer token").into());
        }
        child.resume()?;
        let timeout_ms = u32::try_from(request.timeout_ms).map_err(|_| {
            IoError::new(ErrorKind::InvalidInput, "launcher timeout does not fit u32")
        })?;
        Ok(child.wait_exit(timeout_ms)? as i32)
    }

    fn read_request(reader: impl Read) -> LauncherResult<LauncherRequest> {
        let mut bytes = Vec::new();
        reader
            .take((MAX_REQUEST_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "isolation launcher request exceeds byte limit",
            )
            .into());
        }
        let request: LauncherRequest = serde_json::from_slice(&bytes)?;
        validate_request(&request)?;
        Ok(request)
    }

    fn validate_request(request: &LauncherRequest) -> LauncherResult<()> {
        if request.version != PROTOCOL_VERSION {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "unsupported isolation launcher protocol version",
            )
            .into());
        }
        if request.args.len() > MAX_ARGS {
            return Err(IoError::new(ErrorKind::InvalidInput, "too many process arguments").into());
        }
        if request.executable.contains('\0')
            || request.cwd.contains('\0')
            || request.args.iter().any(|arg| arg.contains('\0'))
        {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "launcher request contains an embedded NUL",
            )
            .into());
        }
        let max_timeout = HardLimits::default().max_process_budget.timeout_ms;
        if request.timeout_ms == 0 || request.timeout_ms > max_timeout {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "launcher timeout is outside the compiled process ceiling",
            )
            .into());
        }
        Ok(())
    }

    fn canonical_file(value: &str) -> LauncherResult<PathBuf> {
        let path = Path::new(value);
        if !path.is_absolute() {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "launcher executable must be absolute",
            )
            .into());
        }
        let canonical = fs::canonicalize(path)?;
        if !canonical.is_file() {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "launcher executable must resolve to a file",
            )
            .into());
        }
        Ok(canonical)
    }

    fn canonical_directory(value: &str) -> LauncherResult<PathBuf> {
        let path = Path::new(value);
        if !path.is_absolute() {
            return Err(
                IoError::new(ErrorKind::InvalidInput, "launcher cwd must be absolute").into(),
            );
        }
        let canonical = fs::canonicalize(path)?;
        if !canonical.is_dir() {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "launcher cwd must resolve to a directory",
            )
            .into());
        }
        Ok(canonical)
    }

    fn validate_command_size(executable: &Path, args: &[String]) -> LauncherResult<()> {
        let executable_units = executable.as_os_str().encode_wide().count();
        let worst_case = args
            .iter()
            .fold(executable_units.saturating_mul(2) + 2, |sum, arg| {
                sum.saturating_add(arg.encode_utf16().count().saturating_mul(2) + 3)
            });
        if worst_case > MAX_COMMAND_WORST_CASE_UTF16_UNITS {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "launcher command line exceeds conservative Windows bound",
            )
            .into());
        }
        Ok(())
    }

    fn unique_profile_name() -> LauncherResult<String> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(IoError::other)?
            .as_nanos();
        Ok(format!("Optic.Launcher.{}.{nanos}", std::process::id()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn oversized_request_fails_before_json_parse() {
            let oversized = std::io::repeat(b'x').take((MAX_REQUEST_BYTES + 1) as u64);
            assert!(read_request(oversized).is_err());
        }

        #[test]
        fn unknown_fields_fail_closed() {
            let request = b"{\"version\":1,\"executable\":\"C:\\\\Windows\\\\System32\\\\cmd.exe\",\"args\":[],\"cwd\":\"C:\\\\Windows\\\\System32\",\"timeout_ms\":1000,\"extra\":true}";
            assert!(read_request(request.as_slice()).is_err());
        }

        #[test]
        fn zero_timeout_fails_closed() {
            let request = b"{\"version\":1,\"executable\":\"C:\\\\Windows\\\\System32\\\\cmd.exe\",\"args\":[],\"cwd\":\"C:\\\\Windows\\\\System32\",\"timeout_ms\":0}";
            assert!(read_request(request.as_slice()).is_err());
        }
    }
}

#[cfg(windows)]
fn main() {
    std::process::exit(windows_launcher::entry());
}

#[cfg(not(windows))]
fn main() {
    eprintln!("optic-bridge-isolation-launcher is Windows-only");
    std::process::exit(126);
}
