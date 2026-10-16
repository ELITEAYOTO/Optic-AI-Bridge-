//! Authorized preparation only. No process can be launched by this API.
use crate::{ProjectInputCaptureError, ProjectInputCaptureService};
use optic_bridge_core::{ActionEnvelope, ProjectInputManifest};
#[cfg(windows)]
use optic_bridge_core::{ProjectInputError, SessionHandle, TaskLeaseId};
#[cfg(windows)]
use optic_bridge_windows::{
    AppContainerProfile, WindowsFileIdentity, inspect_directory_no_reparse, pin_directory_chain,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[cfg(windows)]
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[cfg(windows)]
const MAX_PREPARED_STAGES: usize = 4;

/// Shares four slots across all callers of this instance, including uncertain
/// cleanup. The HTTP/process adapter must own one service, not recreate it per call.
pub struct PrivateProjectStageService {
    capture: ProjectInputCaptureService,
    slots: Arc<AtomicUsize>,
}

impl PrivateProjectStageService {
    #[must_use]
    pub fn new(capture: ProjectInputCaptureService) -> Self {
        Self {
            capture,
            slots: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[must_use]
    pub fn occupied_slots(&self) -> usize {
        self.slots.load(Ordering::Acquire)
    }

    pub fn prepare(
        &self,
        manifest: ProjectInputManifest,
        actions: &[ActionEnvelope],
    ) -> Result<PreparedPrivateProject, PrivateProjectStageError> {
        let first = actions
            .first()
            .ok_or(ProjectInputCaptureError::EffectMismatch)?;
        // Outer admission remains live after capture returns, through private
        // copying and the final authorization check. No source is reopened.
        let (_admission, started) = self.capture.capture_admission(first)?;
        let snapshot = self.capture.capture(manifest, actions)?;
        #[cfg(not(windows))]
        {
            let _ = (snapshot, started);
            Err(PrivateProjectStageError::UnsupportedPlatform)
        }
        #[cfg(windows)]
        {
            self.slots
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                    (used < MAX_PREPARED_STAGES).then_some(used + 1)
                })
                .map_err(|_| PrivateProjectStageError::Capacity)?;
            let slot = StageSlot {
                slots: self.slots.clone(),
                released: false,
            };
            let profile = AppContainerProfile::create_ephemeral()?;
            let private = profile.private_storage_root()?;
            let _private_pin = pin_directory_chain(&private, None)
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            let root = private.join("inputs");
            fs::create_dir(&root)?;
            let identity = inspect_directory_no_reparse(&root)
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?
                .identity;
            let mut stage = PreparedPrivateProject {
                root,
                identity,
                files: Vec::new(),
                directories: BTreeSet::new(),
                _profile: profile,
                slot: Some(slot),
                manifest: None,
            };
            let _root_pin = pin_directory_chain(&stage.root, None)
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            for (input, action) in snapshot.manifest().files().iter().zip(actions) {
                self.capture.authorize(action, started)?;
                let relative = Path::new(input.path.as_str());
                let mut parent = PathBuf::new();
                for part in relative.parent().unwrap_or(Path::new("")).components() {
                    let _pin = pin_directory_chain(&stage.root, Some(&parent))
                        .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
                    parent.push(part);
                    if !stage.directories.contains(&parent) {
                        fs::create_dir(stage.root.join(&parent))?;
                        stage.directories.insert(parent.clone());
                    }
                }
                let _parent_pin = pin_directory_chain(&stage.root, relative.parent())
                    .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(stage.root.join(relative))?;
                stage.files.push(relative.to_path_buf());
                file.write_all(snapshot.content(&input.path).expect("sealed input"))?;
            }
            for action in actions {
                self.capture.authorize(action, started)?;
            }
            stage.manifest = Some(snapshot.into_manifest());
            Ok(stage)
        }
    }
}

/// The profile is deliberately private and there is no execution API. Dropping
/// this value cannot race an owned child: this tranche never starts one.
pub struct PreparedPrivateProject {
    #[cfg(windows)]
    manifest: Option<ProjectInputManifest>,
    #[cfg(windows)]
    root: PathBuf,
    #[cfg(windows)]
    identity: WindowsFileIdentity,
    #[cfg(windows)]
    files: Vec<PathBuf>,
    #[cfg(windows)]
    directories: BTreeSet<PathBuf>,
    #[cfg(windows)]
    _profile: AppContainerProfile,
    #[cfg(windows)]
    slot: Option<StageSlot>,
}

impl std::fmt::Debug for PreparedPrivateProject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedPrivateProject")
            .finish_non_exhaustive()
    }
}

#[cfg(windows)]
impl PreparedPrivateProject {
    pub fn require_binding(
        &self,
        session: &SessionHandle,
        task: &TaskLeaseId,
        epoch: u64,
    ) -> Result<(), ProjectInputError> {
        self.manifest
            .as_ref()
            .expect("returned stage has manifest")
            .require_binding(session, task, epoch)
    }

    /// Host-side prepared input location, not a process/capability grant.
    #[must_use]
    pub fn input_root(&self) -> &Path {
        &self.root
    }

    pub fn discard(mut self) -> Result<(), PrivateProjectStageError> {
        self.cleanup()
    }

    fn cleanup(&mut self) -> Result<(), PrivateProjectStageError> {
        if self.slot.is_none() {
            return Ok(());
        }
        let pin = pin_directory_chain(&self.root, None)
            .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
        if inspect_directory_no_reparse(&self.root)
            .map_err(|_| PrivateProjectStageError::NativeBoundary)?
            .identity
            != self.identity
        {
            return Err(PrivateProjectStageError::NativeBoundary);
        }
        // Never enumerate or recursively delete a caller-controlled tree.
        while let Some(relative) = self.files.last() {
            let _parent = pin_directory_chain(&self.root, relative.parent())
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            match optic_bridge_windows::open_file_for_delete_no_reparse(&self.root.join(relative)) {
                Ok(file) => file
                    .delete()
                    .map_err(|_| PrivateProjectStageError::NativeBoundary)?,
                Err(_) if !self.root.join(relative).try_exists()? => (),
                Err(_) => return Err(PrivateProjectStageError::NativeBoundary),
            }
            self.files.pop();
        }
        // Reverse lexical order puts each declared child before its parent.
        while let Some(relative) = self.directories.last() {
            let _parent = pin_directory_chain(&self.root, relative.parent())
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            inspect_directory_no_reparse(&self.root.join(relative))
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            fs::remove_dir(self.root.join(relative))?;
            self.directories.pop_last();
        }
        drop(pin);
        fs::remove_dir(&self.root)?;
        if let Some(mut slot) = self.slot.take() {
            slot.release();
        }
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for PreparedPrivateProject {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(windows)]
struct StageSlot {
    slots: Arc<AtomicUsize>,
    released: bool,
}
#[cfg(windows)]
impl StageSlot {
    fn release(&mut self) {
        if !self.released {
            self.slots.fetch_sub(1, Ordering::AcqRel);
            self.released = true;
        }
    }
}
// A slot is released explicitly only after known cleanup. A failed cleanup
// retains accounting and eventually refuses further preparation, fail closed.

#[derive(Debug, Error)]
pub enum PrivateProjectStageError {
    #[error("private project stage requires the validated Windows boundary")]
    UnsupportedPlatform,
    #[error("private project stage capacity includes live or uncertain preparations")]
    Capacity,
    #[error("private project stage native identity/cleanup check failed")]
    NativeBoundary,
    #[error("private project input capture failed: {0}")]
    Capture(#[from] ProjectInputCaptureError),
    #[error("private project stage I/O failed: {0}")]
    Io(#[from] std::io::Error),
}
