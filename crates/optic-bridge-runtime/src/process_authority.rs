use std::{fs, io::{self, Read}, path::Path, time::{Duration, Instant}};

#[cfg(not(windows))]
use std::fs::File;

use optic_bridge_core::{
    ContentVersion, ContentVersionReadError, ProcessExecutionClass, TaskLeaseId,
};
use thiserror::Error;

#[cfg(windows)]
use optic_bridge_windows::{PinnedExecutableFile, open_pinned_executable};

pub const MAX_PROCESS_EXECUTABLE_IDENTITY_BYTES: u64 = 512 * 1024 * 1024;

/// Diagnostic-only reader that partitions native read time from the
/// remaining bounded content-hash work without inspecting or logging bytes.
struct TimedIdentityReader<'a, R: Read> {
    inner: &'a mut R,
    read_elapsed: Duration,
    read_calls: u32,
}

impl<R: Read> Read for TimedIdentityReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let began = Instant::now();
        let result = self.inner.read(buffer);
        self.read_elapsed += began.elapsed();
        self.read_calls = self.read_calls.saturating_add(1);
        result
    }
}


#[cfg(windows)]
type ExecutablePin = PinnedExecutableFile;

#[cfg(not(windows))]
#[derive(Debug)]
struct ExecutablePin {
    file: File,
}

#[cfg(not(windows))]
impl ExecutablePin {
    fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessExecutableIdentity {
    canonical_path: String,
    version: ContentVersion,
}

impl ProcessExecutableIdentity {
    pub fn capture(executable: &str) -> Result<Self, ProcessAuthorityError> {
        let (identity, _pin) =
            Self::capture_with_pin(executable, MAX_PROCESS_EXECUTABLE_IDENTITY_BYTES)?;
        Ok(identity)
    }

    #[cfg(test)]
    fn capture_with_limit(executable: &str, max_bytes: u64) -> Result<Self, ProcessAuthorityError> {
        let (identity, _pin) = Self::capture_with_pin(executable, max_bytes)?;
        Ok(identity)
    }

    fn capture_with_pin(
        executable: &str,
        max_bytes: u64,
    ) -> Result<(Self, ExecutablePin), ProcessAuthorityError> {
        // Record only fixed phase durations when the operator opted into
        // startup timings. Never reveal an executable path, file contents,
        // identity hash, or additional authority through this diagnostic.
        let started = Instant::now();
        let canonical_path = canonical_executable(executable)?;
        let path_done = started.elapsed();
        let mut pin = open_identity_pin(Path::new(&canonical_path))?;
        let pin_done = started.elapsed();
        let timing_enabled =
            std::env::var("OPTIC_STARTUP_TIMING").ok().as_deref() == Some("1");
        let version = if timing_enabled {
            let hash_started = Instant::now();
            let mut timed = TimedIdentityReader {
                inner: pin.file_mut(),
                read_elapsed: Duration::ZERO,
                read_calls: 0,
            };
            let observed = ContentVersion::from_reader_bounded(&mut timed, max_bytes)?;
            let hash_elapsed = hash_started.elapsed();
            eprintln!(
                "OPTIC_PROCESS_IDENTITY_IO_TIMING read_ms={} nonread_ms={} total_ms={} calls={}",
                timed.read_elapsed.as_millis(),
                hash_elapsed.saturating_sub(timed.read_elapsed).as_millis(),
                hash_elapsed.as_millis(),
                timed.read_calls,
            );
            observed
        } else {
            ContentVersion::from_reader_bounded(pin.file_mut(), max_bytes)?
        };
        let total = started.elapsed();
        if timing_enabled {
            eprintln!(
                "OPTIC_PROCESS_IDENTITY_TIMING path_ms={} pin_ms={} hash_ms={} total_ms={}",
                path_done.as_millis(),
                pin_done.saturating_sub(path_done).as_millis(),
                total.saturating_sub(pin_done).as_millis(),
                total.as_millis(),
            );
        }
        Ok((
            Self {
                canonical_path,
                version,
            },
            pin,
        ))
    }

    #[must_use]
    pub fn canonical_path(&self) -> &str {
        &self.canonical_path
    }

    pub fn verify(&self) -> Result<(), ProcessAuthorityError> {
        let canonical = canonical_executable(&self.canonical_path)?;
        if canonical != self.canonical_path {
            return Err(ProcessAuthorityError::ExecutablePathChanged);
        }

        let mut pin = open_identity_pin(Path::new(&canonical))?;
        let observed = ContentVersion::from_reader_bounded(
            pin.file_mut(),
            MAX_PROCESS_EXECUTABLE_IDENTITY_BYTES,
        )?;
        if observed != self.version {
            return Err(ProcessAuthorityError::ExecutableIdentityChanged);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ProcessAuthority {
    lease_id: TaskLeaseId,
    class: ProcessExecutionClass,
    identity: ProcessExecutableIdentity,
    _pin: ExecutablePin,
}

impl ProcessAuthority {
    pub fn capture(
        lease_id: TaskLeaseId,
        executable: &str,
        class: ProcessExecutionClass,
    ) -> Result<Self, ProcessAuthorityError> {
        let (identity, pin) = ProcessExecutableIdentity::capture_with_pin(
            executable,
            MAX_PROCESS_EXECUTABLE_IDENTITY_BYTES,
        )?;
        Ok(Self {
            lease_id,
            class,
            identity,
            _pin: pin,
        })
    }

    #[must_use]
    pub fn lease_id(&self) -> &TaskLeaseId {
        &self.lease_id
    }

    #[must_use]
    pub fn canonical_path(&self) -> &str {
        self.identity.canonical_path()
    }

    #[must_use]
    pub const fn execution_class(&self) -> ProcessExecutionClass {
        self.class
    }

    #[must_use]
    pub const fn content_version(&self) -> ContentVersion {
        self.identity.version
    }

    pub fn verify(&self) -> Result<(), ProcessAuthorityError> {
        self.identity.verify()
    }
}

fn canonical_executable(executable: &str) -> Result<String, ProcessAuthorityError> {
    let path = Path::new(executable);
    if !path.is_absolute() {
        return Err(ProcessAuthorityError::InvalidExecutablePath);
    }
    let canonical = fs::canonicalize(path)?;
    if !canonical.is_file() {
        return Err(ProcessAuthorityError::InvalidExecutablePath);
    }
    canonical
        .to_str()
        .map(str::to_owned)
        .ok_or(ProcessAuthorityError::NonUtf8ExecutablePath)
}

#[cfg(windows)]
fn open_identity_pin(path: &Path) -> io::Result<ExecutablePin> {
    open_pinned_executable(path)
}

#[cfg(not(windows))]
fn open_identity_pin(path: &Path) -> io::Result<ExecutablePin> {
    Ok(ExecutablePin {
        file: File::open(path)?,
    })
}

#[derive(Debug, Error)]
pub enum ProcessAuthorityError {
    #[error("process executable path must be an absolute regular file")]
    InvalidExecutablePath,
    #[error("process executable canonical path is not valid UTF-8")]
    NonUtf8ExecutablePath,
    #[error("process executable identity observation failed: {0}")]
    IdentityRead(#[from] ContentVersionReadError),
    #[error("process executable identity could not be opened: {0}")]
    Io(#[from] io::Error),
    #[error("process executable canonical path changed since authorization")]
    ExecutablePathChanged,
    #[error("process executable content identity changed since authorization")]
    ExecutableIdentityChanged,
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    fn workspace(label: &str) -> std::path::PathBuf {
        let root = env::temp_dir().join(format!(
            "optic-process-authority-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    #[test]
    fn identity_rejects_same_path_after_content_change() {
        let root = workspace("change");
        let executable = root.join("tool.bin");
        fs::write(&executable, b"first-tool").expect("write fixture");
        let executable = executable.to_string_lossy().into_owned();
        let identity = ProcessExecutableIdentity::capture_with_limit(&executable, 1024)
            .expect("capture identity");

        fs::write(&executable, b"second-tool").expect("replace fixture contents");
        assert!(matches!(
            identity.verify(),
            Err(ProcessAuthorityError::ExecutableIdentityChanged)
        ));

        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn identity_accepts_unchanged_content() {
        let root = workspace("stable");
        let executable = root.join("tool.bin");
        fs::write(&executable, b"stable-tool").expect("write fixture");
        let executable = executable.to_string_lossy().into_owned();
        let identity = ProcessExecutableIdentity::capture_with_limit(&executable, 1024)
            .expect("capture identity");

        identity.verify().expect("verify unchanged identity");
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn identity_observation_is_bounded() {
        let root = workspace("bounded");
        let executable = root.join("tool.bin");
        fs::write(&executable, b"too-large").expect("write fixture");
        let executable = executable.to_string_lossy().into_owned();
        assert!(matches!(
            ProcessExecutableIdentity::capture_with_limit(&executable, 4),
            Err(ProcessAuthorityError::IdentityRead(
                ContentVersionReadError::LimitExceeded
            ))
        ));
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[cfg(windows)]
    #[test]
    fn authority_pin_prevents_same_path_rewrite_while_lease_is_alive() {
        let root = workspace("lease-pin");
        let executable = root.join("tool.exe");
        fs::write(&executable, b"pinned-tool").expect("write fixture");
        let lease_id = TaskLeaseId::generate().expect("lease id");
        let authority = ProcessAuthority::capture(
            lease_id,
            &executable.to_string_lossy(),
            ProcessExecutionClass::FixedTool,
        )
        .expect("capture authority");

        assert!(fs::write(&executable, b"replacement").is_err());
        authority.verify().expect("pinned identity remains valid");
        drop(authority);
        fs::write(&executable, b"replacement").expect("rewrite after authority drop");
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[cfg(windows)]
    #[test]
    fn authority_pin_still_allows_executable_launch() {
        let executable = env::current_exe().expect("current test executable");
        let lease_id = TaskLeaseId::generate().expect("lease id");
        let authority = ProcessAuthority::capture(
            lease_id,
            &executable.to_string_lossy(),
            ProcessExecutionClass::FixedTool,
        )
        .expect("capture executable authority");

        let status = std::process::Command::new(&executable)
            .arg("--list")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("launch pinned executable");
        assert!(status.success());
        authority
            .verify()
            .expect("identity remains valid after launch");
    }
}
