//! Service-owned, non-executing preparation for one fixed native git status.
//! The future Windows guardian must adopt this value before native creation.
use std::path::Path;

use optic_bridge_core::{ActionEnvelope, ActionId, MonotonicTime, ResourceBudget, TaskLeaseId};
use thiserror::Error;

use crate::{
    GitReadError, GitReadService, HardenedCommandSpec, NativeGitReadAdmissionError,
    NativeGitReadAdmittedPreflight, NativeGitReadOwnedPermits, NativeGitReadTrustedContext,
    finish_owned_native_git_read_preflight,
};

/// A fixed service command and its original counted authorization resources.
/// This value is neither cloneable nor a native spawn permit. A future
/// registered guardian must retain it through real terminal OS/pipe proof.
#[must_use = "native preparation must remain owned until proven terminal"]
pub struct PreparedNativeGitStatus {
    pub(crate) command: HardenedCommandSpec,
    pub(crate) admission: NativeGitReadAdmittedPreflight,
    pub(crate) envelope: ActionEnvelope,
}

impl PreparedNativeGitStatus {
    #[must_use]
    pub fn repository_root(&self) -> &Path {
        &self.command.cwd
    }

    #[must_use]
    pub fn git_executable(&self) -> &Path {
        &self.command.executable
    }

    #[must_use]
    pub fn output_limit(&self) -> u64 {
        self.command.output_limit
    }

    #[must_use]
    pub fn action_id(&self) -> &ActionId {
        &self.envelope.action_id
    }

    #[must_use]
    pub fn resources(&self) -> ResourceBudget {
        self.envelope.resources
    }

    #[must_use]
    pub fn lease_id(&self) -> &TaskLeaseId {
        self.admission.lease_id()
    }

    /// The original absolute horizon; native preparation must not restart it.
    #[must_use]
    pub fn deadline(&self) -> MonotonicTime {
        self.admission.deadline()
    }

    /// Stop request only: false is never independent permission to spawn.
    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.admission.is_stop_required_at(now)
    }
}

#[derive(Debug, Error)]
pub enum NativeGitStatusPreparationError {
    #[error("native Git status requires UTF-8 service-owned targets")]
    NonUtf8Target,
    #[error(transparent)]
    Command(#[from] GitReadError),
    #[error(transparent)]
    Admission(#[from] NativeGitReadAdmissionError),
}

impl GitReadService {
    /// Bind the fixed status spec to the service's configured canonical
    /// repository/executable and the worker's original owned permits.
    ///
    /// No caller-provided target, command arguments or separate output budget
    /// crosses this boundary. It does not create a process, reserve native
    /// quotas or approve sensitive metadata publication. Those controls and
    /// final synchronized identity/policy/revocation checks remain mandatory.
    pub fn prepare_native_status(
        &self,
        trusted: &NativeGitReadTrustedContext<'_>,
        owned: NativeGitReadOwnedPermits,
        envelope: &ActionEnvelope,
        now: MonotonicTime,
    ) -> Result<PreparedNativeGitStatus, NativeGitStatusPreparationError> {
        let command = self.status_command_spec(envelope.resources.output_bytes)?;
        let repository = command
            .cwd
            .to_str()
            .ok_or(NativeGitStatusPreparationError::NonUtf8Target)?;
        let executable = command
            .executable
            .to_str()
            .ok_or(NativeGitStatusPreparationError::NonUtf8Target)?;
        let admission = finish_owned_native_git_read_preflight(
            trusted, owned, envelope, repository, executable, now,
        )?;
        Ok(PreparedNativeGitStatus {
            command,
            admission,
            envelope: envelope.clone(),
        })
    }
}

/// Windows-only accounting stage between admitted status preparation and a
/// future OS-native Git guardian. This is NOT a Win32 Job handle or spawn permit:
/// NativeProcessReservation records capacity and configures limits, but does
/// not create a kernel Job, process, or pipes. Only later owner code may do so.
#[cfg(windows)]
#[must_use = "keep the original permits and process accounting until safe disposal"]
pub struct RegisteredNativeGitStatus {
    // Drop the no-child record before the original session/transport permits.
    record: crate::process::NativeProcessReservation,
    prepared: PreparedNativeGitStatus,
}

#[cfg(windows)]
impl RegisteredNativeGitStatus {
    #[must_use]
    pub fn job_id(&self) -> optic_bridge_core::JobId {
        self.record.id()
    }

    #[must_use]
    pub fn deadline(&self) -> MonotonicTime {
        self.prepared.deadline()
    }

    /// A stop request, not approval to create a process or proof of cleanup.
    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.prepared.is_stop_required_at(now) || self.record.stop_requested()
    }
}

#[cfg(windows)]
#[derive(Debug, Error)]
pub enum NativeGitStatusRegistrationError {
    #[error("the original native GitRead admission has expired or been revoked")]
    StopRequired,
    #[error("GitRead service repository differs from the configured process manager root")]
    WrongProcessRoot,
    #[error("native GitRead requires UTF-8 executable and fixed arguments")]
    NonUtf8Command,
    #[error("native GitRead preparation and action output budgets differ")]
    OutputBudgetMismatch,
    #[error(transparent)]
    Process(#[from] crate::ProcessError),
}

#[cfg(windows)]
impl crate::ProcessManager {
    /// Register a private, owner-scoped action and reserve actual shared
    /// global/session process, CPU, memory and output capacity before any
    /// potential native creation. The original owned admission, transport
    /// permit, task lease witness and action horizon remain inside the result.
    ///
    /// No child, OS Job handle or pipe is created by this method. In
    /// particular, successful registration grants NO CreateProcess authority:
    /// an owner must add true native Job/pipe custody, pinned Git identity and
    /// synchronous last-boundary revocation/policy checks before any launch.
    pub fn reserve_prepared_native_status(
        &self,
        prepared: PreparedNativeGitStatus,
        now: MonotonicTime,
    ) -> Result<RegisteredNativeGitStatus, NativeGitStatusRegistrationError> {
        if prepared.is_stop_required_at(now) {
            return Err(NativeGitStatusRegistrationError::StopRequired);
        }
        if self.native_registration_root() != prepared.repository_root() {
            return Err(NativeGitStatusRegistrationError::WrongProcessRoot);
        }
        if prepared.command.output_limit != prepared.envelope.resources.output_bytes {
            return Err(NativeGitStatusRegistrationError::OutputBudgetMismatch);
        }
        let executable = prepared
            .git_executable()
            .to_str()
            .ok_or(NativeGitStatusRegistrationError::NonUtf8Command)?
            .to_owned();
        let args = prepared
            .command
            .args
            .iter()
            .map(|arg| {
                arg.to_str()
                    .map(str::to_owned)
                    .ok_or(NativeGitStatusRegistrationError::NonUtf8Command)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let spec = crate::ProcessStartSpec {
            session: prepared.envelope.session.clone(),
            class: optic_bridge_core::ProcessExecutionClass::FixedTool,
            workload_class: prepared.admission.workload_class(),
            network: optic_bridge_core::NetworkAccess::Denied,
            executable,
            args,
            cwd: None,
            workspace_read_files: Vec::new(),
            env_allowlist: Vec::new(),
            resources: prepared.envelope.resources,
        };
        let record = self.reserve_native_private(&spec, prepared.action_id())?;
        Ok(RegisteredNativeGitStatus { record, prepared })
    }
}

/// Inert, unstarted Windows kernel Job and stdio custody tied to a single
/// registered private action. Never exposes CreateProcess or child ownership.
#[cfg(windows)]
#[must_use = "hold original MCP permits and OS handles through native ownership"]
pub struct PrecreatedNativeGitStatusCustody {
    // Drop the real unstarted Job/pipes before the original counted permits.
    native: optic_bridge_windows::PrecreatedNativeReadCustody,
    // Target handles outlive the Job; prevent service identity/path substitution.
    _executable_pin: optic_bridge_windows::PinnedExecutableFile,
    _repository_pin: optic_bridge_windows::PinnedDirectoryChain,
    registered: RegisteredNativeGitStatus,
}

#[cfg(windows)]
impl PrecreatedNativeGitStatusCustody {
    #[must_use]
    pub fn job_id(&self) -> optic_bridge_core::JobId {
        self.registered.job_id()
    }

    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.registered.is_stop_required_at(now)
    }

    /// This queries a real kernel Job with no process started by this type.
    /// An empty Job during precreation is NOT terminal proof for a future run.
    pub fn native_active_processes(&self) -> std::io::Result<u32> {
        self.native.active_processes()
    }
}

#[cfg(windows)]
#[derive(Debug, Error)]
pub enum NativeGitStatusCustodyError {
    #[error("native Git status admission stopped before Job precreation")]
    StopRequired,
    #[error("native Git repository pin failed: {0}")]
    RepositoryPin(#[from] optic_bridge_windows::PinnedDirectoryError),
    #[error("native Git targets differ from the original canonical service identity")]
    TargetIdentityChanged,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(windows)]
impl RegisteredNativeGitStatus {
    /// Pins the *original* service-owned executable and repository by opened
    /// Windows handles, verifies their kernel-resolved canonical paths, and
    /// opens a real bounded Job, stdout/stderr pipes and closed stdin under
    /// the already counted admission. Any error has no child and releases
    /// the unstarted resources and original permits.
    ///
    /// This is path pin custody, NOT an independently revalidated original
    /// TaskLease content hash, synchronized launch gate, native execution,
    /// terminal/EOF proof or permission to wire MCP GitRead.
    pub fn precreate_native_custody(
        self,
        now: MonotonicTime,
    ) -> Result<PrecreatedNativeGitStatusCustody, NativeGitStatusCustodyError> {
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusCustodyError::StopRequired);
        }
        let repository = self.prepared.repository_root();
        let executable = self.prepared.git_executable();
        // Open handles that deny directory replacement and executable write/
        // delete sharing. A real post-open final path must match the trusted
        // canonical service spec, not just the original lexical path.
        let repository_pin = optic_bridge_windows::pin_native_git_read_repository_root(repository)?;
        if repository_pin.final_path() != repository {
            return Err(NativeGitStatusCustodyError::TargetIdentityChanged);
        }
        let executable_pin = optic_bridge_windows::open_pinned_executable(executable)?;
        if executable_pin.final_path() != executable {
            return Err(NativeGitStatusCustodyError::TargetIdentityChanged);
        }
        // Do not allocate another resource or establish any future custody
        // after already-observed revocation, even though no spawn exists.
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusCustodyError::StopRequired);
        }
        let native = optic_bridge_windows::PrecreatedNativeReadCustody::new(self.record.limits)?;
        // A revocation racing OS handle creation still cannot turn this
        // unstarted owner into spawn authority or fresh deadline.
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusCustodyError::StopRequired);
        }
        Ok(PrecreatedNativeGitStatusCustody {
            native,
            _executable_pin: executable_pin,
            _repository_pin: repository_pin,
            registered: self,
        })
    }
}
