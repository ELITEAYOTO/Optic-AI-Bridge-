use std::{io, path::Path};

#[cfg(not(windows))]
use std::fs::File;

use optic_bridge_core::{ContentVersion, ContentVersionReadError, JobId, TaskLeaseId};
use thiserror::Error;

use crate::{ProcessError, ProcessManager, ProcessStartSpec};

#[cfg(windows)]
use optic_bridge_windows::{PinnedExecutableFile, open_pinned_executable};

pub const MAX_PROCESS_EXECUTABLE_IDENTITY_BYTES: u64 = 512 * 1024 * 1024;

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
    pub fn capture(
        processes: &ProcessManager,
        executable: &str,
    ) -> Result<Self, ProcessAuthorityError> {
        Self::capture_with_limit(processes, executable, MAX_PROCESS_EXECUTABLE_IDENTITY_BYTES)
    }

    fn capture_with_limit(
        processes: &ProcessManager,
        executable: &str,
        max_bytes: u64,
    ) -> Result<Self, ProcessAuthorityError> {
        let canonical_path = processes
            .canonicalize_executable(executable)
            .map_err(ProcessAuthorityError::ExecutableObservation)?;
        let mut pin = open_identity_pin(Path::new(&canonical_path))?;
        let version = ContentVersion::from_reader_bounded(pin.file_mut(), max_bytes)?;
        Ok(Self {
            canonical_path,
            version,
        })
    }

    #[must_use]
    pub fn canonical_path(&self) -> &str {
        &self.canonical_path
    }

    fn verify_and_pin(
        &self,
        processes: &ProcessManager,
        executable: &str,
    ) -> Result<ExecutablePin, ProcessAuthorityError> {
        let canonical = processes
            .canonicalize_executable(executable)
            .map_err(ProcessAuthorityError::ExecutableObservation)?;
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
        Ok(pin)
    }
}

#[derive(Clone, Debug)]
pub struct ProcessAuthority {
    lease_id: TaskLeaseId,
    identity: ProcessExecutableIdentity,
}

impl ProcessAuthority {
    pub fn capture(
        processes: &ProcessManager,
        lease_id: TaskLeaseId,
        executable: &str,
    ) -> Result<Self, ProcessAuthorityError> {
        Ok(Self {
            lease_id,
            identity: ProcessExecutableIdentity::capture(processes, executable)?,
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

    /// Verify the executable identity and keep its pin alive through process creation.
    pub fn start(
        &self,
        processes: &ProcessManager,
        mut spec: ProcessStartSpec,
    ) -> Result<JobId, ProcessAuthorityError> {
        let pin = self.identity.verify_and_pin(processes, &spec.executable)?;
        spec.executable = self.identity.canonical_path.clone();
        let result = processes.start(spec);
        drop(pin);
        result.map_err(ProcessAuthorityError::Process)
    }
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
    #[error("process executable could not be observed for identity: {0}")]
    ExecutableObservation(ProcessError),
    #[error("process runtime rejected verified executable authority: {0}")]
    Process(ProcessError),
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

    use optic_bridge_core::HardLimits;

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
        let manager =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let executable = executable.to_string_lossy().into_owned();
        let identity = ProcessExecutableIdentity::capture_with_limit(&manager, &executable, 1024)
            .expect("capture identity");

        fs::write(&executable, b"second-tool").expect("replace fixture contents");
        assert!(matches!(
            identity.verify_and_pin(&manager, &executable),
            Err(ProcessAuthorityError::ExecutableIdentityChanged)
        ));

        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn identity_accepts_unchanged_content() {
        let root = workspace("stable");
        let executable = root.join("tool.bin");
        fs::write(&executable, b"stable-tool").expect("write fixture");
        let manager =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let executable = executable.to_string_lossy().into_owned();
        let identity = ProcessExecutableIdentity::capture_with_limit(&manager, &executable, 1024)
            .expect("capture identity");

        let pin = identity
            .verify_and_pin(&manager, &executable)
            .expect("verify unchanged identity");
        drop(pin);

        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn identity_observation_is_bounded() {
        let root = workspace("bounded");
        let executable = root.join("tool.bin");
        fs::write(&executable, b"too-large").expect("write fixture");
        let manager =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let executable = executable.to_string_lossy().into_owned();
        assert!(matches!(
            ProcessExecutableIdentity::capture_with_limit(&manager, &executable, 4),
            Err(ProcessAuthorityError::IdentityRead(
                ContentVersionReadError::LimitExceeded
            ))
        ));
        fs::remove_dir_all(root).expect("remove fixture root");
    }
}
