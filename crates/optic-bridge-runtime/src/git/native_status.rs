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
