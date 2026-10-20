//! Authorized native capture for private build inputs; no staging or execution.
use std::{path::Path, sync::Arc};

use optic_bridge_core::{
    ActionEnvelope, Effect, HardLimits, MonotonicTime, ProjectInputError, ProjectInputManifest,
    ProjectInputSnapshot,
};
use optic_bridge_policy::{PolicyDecision, PolicyEngine, PolicyReason};
use thiserror::Error;

use crate::{
    BoundedFileSystem, Clock, FileSystemError, SessionRegistry, SessionRegistryError,
    TaskLeaseRegistry, TaskLeaseRegistryError,
};
#[cfg(windows)]
use optic_bridge_windows::{
    WindowsFileIdentity, inspect_directory_no_reparse, pin_directory_chain,
};

/// Synchronous bounded service. A transport must retain its own execution and
/// blocking-I/O permits through this call, including when its caller disconnects.
pub struct ProjectInputCaptureService {
    #[cfg_attr(not(windows), allow(dead_code))]
    filesystem: BoundedFileSystem,
    sessions: Arc<SessionRegistry>,
    leases: Arc<TaskLeaseRegistry>,
    clock: Arc<dyn Clock>,
    max_duration_ms: u64,
    #[cfg(windows)]
    root_identity: WindowsFileIdentity,
}

impl ProjectInputCaptureService {
    #[cfg(windows)]
    pub(crate) fn cancellation_watch(
        &self,
        process: &ActionEnvelope,
        read: &ActionEnvelope,
    ) -> Result<(Vec<Arc<std::sync::atomic::AtomicBool>>, u64), ProjectInputCaptureError> {
        if process.session != read.session || process.policy_epoch != read.policy_epoch {
            return Err(ProjectInputCaptureError::EffectMismatch);
        }
        let now = self.clock.now();
        let (session_signal, session) = self.sessions.watch_active(&process.session, now)?;
        let mut flags = vec![session_signal];
        let mut expiry = session.expires_at;
        if session.policy_epoch != process.policy_epoch {
            return Err(ProjectInputCaptureError::EffectMismatch);
        }
        for action in [process, read] {
            let id = action
                .task_lease
                .as_ref()
                .ok_or(ProjectInputCaptureError::EffectMismatch)?;
            let (signal, lease) = self.leases.watch_active(id, &process.session, now)?;
            if lease.policy_epoch != process.policy_epoch {
                return Err(ProjectInputCaptureError::EffectMismatch);
            }
            expiry = expiry.min(lease.expires_at);
            flags.push(signal);
        }
        Ok((
            flags,
            expiry
                .as_millis()
                .checked_sub(now.as_millis())
                .filter(|v| *v > 0)
                .ok_or(ProjectInputCaptureError::Deadline)?,
        ))
    }
    #[cfg(windows)]
    pub(crate) fn workspace_root(&self) -> &Path {
        self.filesystem.root()
    }
    pub fn new(
        root: impl AsRef<Path>,
        limits: HardLimits,
        sessions: Arc<SessionRegistry>,
        leases: Arc<TaskLeaseRegistry>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, ProjectInputCaptureError> {
        limits
            .validate_nonzero()
            .map_err(|_| ProjectInputCaptureError::InvalidLimits)?;
        let filesystem = BoundedFileSystem::from_hard_limits(root, limits)?;
        #[cfg(windows)]
        let root_identity = inspect_directory_no_reparse(filesystem.root())
            .map_err(|_| ProjectInputCaptureError::NativeBoundary)?
            .identity;
        Ok(Self {
            filesystem,
            sessions,
            leases,
            clock,
            max_duration_ms: limits.max_request_duration_ms,
            #[cfg(windows)]
            root_identity,
        })
    }

    pub fn capture(
        &self,
        manifest: ProjectInputManifest,
        actions: &[ActionEnvelope],
    ) -> Result<ProjectInputSnapshot, ProjectInputCaptureError> {
        if actions.len() != manifest.files().len() {
            return Err(ProjectInputCaptureError::EffectMismatch);
        }
        let first = actions
            .first()
            .ok_or(ProjectInputCaptureError::EffectMismatch)?;
        let started = self.clock.now();
        let admission = self.sessions.begin_admission(&first.session, started)?;
        self.preflight(&manifest, actions, started)?;
        #[cfg(not(windows))]
        {
            drop(admission);
            Err(ProjectInputCaptureError::UnsupportedPlatform)
        }
        #[cfg(windows)]
        {
            // Keep the selected native root and ancestors pinned throughout.
            let _root = pin_directory_chain(self.filesystem.root(), None)
                .map_err(|_| ProjectInputCaptureError::NativeBoundary)?;
            if inspect_directory_no_reparse(self.filesystem.root())
                .map_err(|_| ProjectInputCaptureError::NativeBoundary)?
                .identity
                != self.root_identity
            {
                return Err(ProjectInputCaptureError::NativeBoundary);
            }
            let mut contents = Vec::with_capacity(manifest.files().len());
            for (input, action) in manifest.files().iter().zip(actions) {
                self.authorize(action, started)?;
                // Respect existing read hard limits; the manifest's larger
                // ceiling is not permission to increase them silently.
                let prepared =
                    self.filesystem
                        .prepare_read(&input.path, 0, Some(input.size_bytes.max(1)))?;
                if prepared.observation().canonical_path != input.path {
                    return Err(ProjectInputCaptureError::NativeBoundary);
                }
                self.authorize(action, started)?;
                let chunk = prepared.read()?;
                if !chunk.eof || chunk.next_offset != input.size_bytes {
                    return Err(ProjectInputError::ContentMismatch.into());
                }
                contents.push((input.path.clone(), chunk.bytes));
            }
            let snapshot = manifest.seal(contents)?;
            // Revocation/expiry while bytes were being copied discards them.
            for action in actions {
                self.authorize(action, started)?;
            }
            drop(admission);
            Ok(snapshot)
        }
    }

    pub(crate) fn preflight(
        &self,
        manifest: &ProjectInputManifest,
        actions: &[ActionEnvelope],
        started: MonotonicTime,
    ) -> Result<(), ProjectInputCaptureError> {
        if actions.len() != manifest.files().len() {
            return Err(ProjectInputCaptureError::EffectMismatch);
        }
        // All declared ownership/effects/policy must pass before any source is
        // opened. A manifest does not convert process authority into read rights.
        for (input, action) in manifest.files().iter().zip(actions) {
            if manifest.total_bytes() > action.resources.memory_bytes {
                return Err(ProjectInputCaptureError::PolicyDenied(
                    PolicyReason::ResourceBudgetExceeded,
                ));
            }
            let task = action
                .task_lease
                .as_ref()
                .ok_or(ProjectInputCaptureError::EffectMismatch)?;
            manifest.require_binding(&action.session, task, action.policy_epoch)?;
            if action.effect
                != (Effect::FileRead {
                    path: input.path.clone(),
                })
            {
                return Err(ProjectInputCaptureError::EffectMismatch);
            }
            self.authorize(action, started)?;
        }
        Ok(())
    }

    pub(crate) fn capture_admission(
        &self,
        action: &ActionEnvelope,
    ) -> Result<(crate::SessionAdmissionPermit<'_>, MonotonicTime), ProjectInputCaptureError> {
        let started = self.clock.now();
        Ok((
            self.sessions.begin_admission(&action.session, started)?,
            started,
        ))
    }

    pub(crate) fn authorize(
        &self,
        action: &ActionEnvelope,
        started: MonotonicTime,
    ) -> Result<(), ProjectInputCaptureError> {
        let now = self.clock.now();
        if now < started
            || now.as_millis() - started.as_millis()
                >= self.max_duration_ms.min(action.resources.timeout_ms)
        {
            return Err(ProjectInputCaptureError::Deadline);
        }
        self.authorize_at(action, now).map(|_| ())
    }

    #[cfg(windows)]
    pub(crate) fn live_lease(
        &self,
        action: &ActionEnvelope,
    ) -> Result<optic_bridge_core::TaskLease, ProjectInputCaptureError> {
        self.authorize_at(action, self.clock.now())
    }

    fn authorize_at(
        &self,
        action: &ActionEnvelope,
        now: MonotonicTime,
    ) -> Result<optic_bridge_core::TaskLease, ProjectInputCaptureError> {
        let session = self.sessions.get_active(&action.session, now)?;
        let id = action
            .task_lease
            .as_ref()
            .ok_or(ProjectInputCaptureError::EffectMismatch)?;
        let lease = self.leases.get_active(id, &action.session, now)?;
        match PolicyEngine.evaluate(action, &session, Some(&lease), now) {
            PolicyDecision::Allow => Ok(lease),
            PolicyDecision::Deny(reason) | PolicyDecision::RequireApproval(reason) => {
                Err(ProjectInputCaptureError::PolicyDenied(reason))
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ProjectInputCaptureError {
    #[error("project input capture has invalid hard limits")]
    InvalidLimits,
    #[error("project input capture requires the validated Windows filesystem boundary")]
    UnsupportedPlatform,
    #[error("project input capture native identity validation failed")]
    NativeBoundary,
    #[error("project input capture exceeded its monotonic deadline")]
    Deadline,
    #[error("project input capture requires one bound FileRead action per manifest entry")]
    EffectMismatch,
    #[error("project input read denied: {0:?}")]
    PolicyDenied(PolicyReason),
    #[error("project input contract failed: {0}")]
    Input(#[from] ProjectInputError),
    #[error("project input filesystem read failed: {0}")]
    FileSystem(#[from] FileSystemError),
    #[error("project input session is inactive: {0}")]
    Session(#[from] SessionRegistryError),
    #[error("project input task lease is inactive: {0}")]
    Lease(#[from] TaskLeaseRegistryError),
}

#[cfg(test)]
mod tests;
