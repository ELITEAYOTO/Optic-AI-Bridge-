//! Authorized private inputs and a host-only experimental Node task controller.
use crate::{ProjectInputCaptureError, ProjectInputCaptureService};
use optic_bridge_core::{ActionEnvelope, ProjectInputError, ProjectInputManifest};
#[cfg(windows)]
use optic_bridge_core::{SessionHandle, TaskLeaseId};
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
        self.capture.preflight(&manifest, actions, started)?;
        #[cfg(not(windows))]
        {
            Err(PrivateProjectStageError::UnsupportedPlatform)
        }
        #[cfg(windows)]
        {
            let slot = self.reserve_slot()?;
            self.materialize(manifest, actions, slot, started)
        }
    }
    #[cfg(windows)]
    fn reserve_slot(&self) -> Result<StageSlot, PrivateProjectStageError> {
        self.slots
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (used < MAX_PREPARED_STAGES).then_some(used + 1)
            })
            .map_err(|_| PrivateProjectStageError::Capacity)?;
        Ok(StageSlot {
            slots: self.slots.clone(),
            released: false,
            retain_on_drop: false,
        })
    }

    /// Keeps the native profile on its creating worker rather than declaring
    /// native SID pointers Send/Sync. Cancellation closes the result channel;
    /// the worker still owns effect permits through copying and then cleans up.
    #[cfg(windows)]
    pub async fn prepare_background(
        self: Arc<Self>,
        manifest: ProjectInputManifest,
        actions: Vec<ActionEnvelope>,
        request: crate::RequestPermit,
        io: crate::BlockingIoGovernor,
    ) -> Result<PreparedPrivateProjectLease, PrivateProjectStageError> {
        let first = actions
            .first()
            .ok_or(ProjectInputCaptureError::EffectMismatch)?;
        let (admission, started) = self.capture.capture_admission(first)?;
        self.capture.preflight(&manifest, &actions, started)?;
        if request.is_expired_at(started) {
            return Err(ProjectInputCaptureError::Deadline.into());
        }
        let session = first.session.clone();
        let task = first
            .task_lease
            .clone()
            .ok_or(ProjectInputCaptureError::EffectMismatch)?;
        let epoch = first.policy_epoch;
        let slot = self.reserve_slot()?;
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| PrivateProjectStageError::WorkerUnavailable)?;
        // Reserve before awaiting I/O capacity; cancellation here has no native
        // allocation and StageSlot returns the reservation automatically.
        let io_permit = io.acquire_owned().await?;
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (control_tx, control_rx) = std::sync::mpsc::sync_channel(1);
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let service = Arc::clone(&self);
        std::thread::Builder::new()
            .name("optic-private-inputs".into())
            .spawn(move || {
                let result = (|| {
                    if ready_tx.is_closed() {
                        return Err(PrivateProjectStageError::WorkerUnavailable);
                    }
                    let (_admission, now) = service.capture.capture_admission(&actions[0])?;
                    if ready_tx.is_closed() {
                        return Err(PrivateProjectStageError::WorkerUnavailable);
                    }
                    if request.is_expired_at(now) {
                        return Err(ProjectInputCaptureError::Deadline.into());
                    }
                    service.capture.preflight(&manifest, &actions, now)?;
                    let stage = service.materialize(manifest, &actions, slot, now)?;
                    let (_final_admission, finished) =
                        service.capture.capture_admission(&actions[0])?;
                    if request.is_expired_at(finished) {
                        return Err(ProjectInputCaptureError::Deadline.into());
                    }
                    Ok(stage)
                })();
                // Result delivery never frees capacity before the synchronous effect
                // has ended. The idle prepared lease holds only its stage slot.
                drop(request);
                drop(io_permit);
                match result {
                    Ok(stage) => {
                        let lease = PreparedPrivateProjectLease {
                            root: stage.input_root().to_path_buf(),
                            session,
                            task,
                            epoch,
                            io: io.clone(),
                            control: Some(control_tx),
                            done: Some(done_rx),
                        };
                        // An abandoned reply drops its lease/sender. recv then wakes
                        // disconnected, just as it does on ordinary lease Drop.
                        let _ = ready_tx.send(Ok(lease));
                        if let Ok(PrivateStageCommand::RunNode(command)) = control_rx.recv() {
                            let outcome = crate::private_node_task::execute(
                                stage,
                                &service.capture,
                                &actions,
                                command,
                                &io,
                                &runtime,
                            );
                            let _ = done_tx.send(outcome);
                            return;
                        }
                        let outcome = match runtime.block_on(io.acquire_owned()) {
                            Ok(_cleanup_permit) => stage.discard(),
                            Err(error) => {
                                // No ungoverned cleanup when the owner is unavailable.
                                // The four-slot reservation remains retained.
                                std::mem::forget(stage);
                                Err(error.into())
                            }
                        };
                        let _ = done_tx.send(outcome);
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                }
            })?;
        drop(admission);
        ready_rx
            .await
            .map_err(|_| PrivateProjectStageError::WorkerUnavailable)?
    }

    #[cfg(windows)]
    fn materialize(
        &self,
        manifest: ProjectInputManifest,
        actions: &[ActionEnvelope],
        mut slot: StageSlot,
        started: optic_bridge_core::MonotonicTime,
    ) -> Result<PreparedPrivateProject, PrivateProjectStageError> {
        // Reserve before opening sources, limiting simultaneous captured
        // bytes as well as prepared disk inputs. Pure capture failure frees
        // its slot; once native allocation begins, uncertainty retains it.
        let snapshot = self.capture.capture(manifest, actions)?;
        slot.retain_on_drop = true;
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
            _profile: Some(profile),
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
    _profile: Option<AppContainerProfile>,
    #[cfg(windows)]
    slot: Option<StageSlot>,
}

/// Sendable host lease; its native profile remains on a bounded owner worker.
/// A single host-only Node command consumes the lease; no profile is exposed.
#[cfg(windows)]
pub struct PreparedPrivateProjectLease {
    root: PathBuf,
    session: SessionHandle,
    task: TaskLeaseId,
    epoch: u64,
    io: crate::BlockingIoGovernor,
    control: Option<std::sync::mpsc::SyncSender<PrivateStageCommand>>,
    done: Option<tokio::sync::oneshot::Receiver<Result<(), PrivateProjectStageError>>>,
}

#[cfg(windows)]
enum PrivateStageCommand {
    Discard,
    RunNode(crate::private_node_task::PrivateNodeTaskCommand),
}

#[cfg(windows)]
impl std::fmt::Debug for PreparedPrivateProjectLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedPrivateProjectLease")
            .finish_non_exhaustive()
    }
}

#[cfg(windows)]
impl PreparedPrivateProjectLease {
    /// Consumes this preparation once. A lost reply must be reconciled through
    /// the shared process manager; this lease cannot blindly start it again.
    pub async fn start_node(
        mut self,
        start: crate::PrivateNodeTaskStart,
        request: crate::RequestPermit,
    ) -> Result<optic_bridge_core::JobId, PrivateProjectStageError> {
        if start.action.session != self.session || start.action.policy_epoch != self.epoch {
            return Err(ProjectInputError::Binding.into());
        }
        let io = self.io.acquire_owned().await?;
        let (reply, response) = tokio::sync::oneshot::channel();
        let command = crate::private_node_task::PrivateNodeTaskCommand {
            start,
            request,
            io,
            reply,
        };
        self.control
            .take()
            .expect("live lease control")
            .try_send(PrivateStageCommand::RunNode(command))
            .map_err(|_| PrivateProjectStageError::WorkerUnavailable)?;
        response
            .await
            .map_err(|_| PrivateProjectStageError::WorkerUnavailable)?
    }

    #[must_use]
    pub fn input_root(&self) -> &Path {
        &self.root
    }

    pub fn require_binding(
        &self,
        session: &SessionHandle,
        task: &TaskLeaseId,
        epoch: u64,
    ) -> Result<(), ProjectInputError> {
        if session != &self.session || task != &self.task || epoch != self.epoch {
            return Err(ProjectInputError::Binding);
        }
        Ok(())
    }

    pub async fn discard(mut self) -> Result<(), PrivateProjectStageError> {
        self.control
            .take()
            .expect("live lease control")
            .try_send(PrivateStageCommand::Discard)
            .map_err(|_| PrivateProjectStageError::WorkerUnavailable)?;
        self.done
            .take()
            .expect("live lease completion")
            .await
            .map_err(|_| PrivateProjectStageError::WorkerUnavailable)?
    }
}

impl std::fmt::Debug for PreparedPrivateProject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedPrivateProject")
            .finish_non_exhaustive()
    }
}

#[cfg(windows)]
impl PreparedPrivateProject {
    pub(crate) fn manifest(&self) -> &ProjectInputManifest {
        self.manifest.as_ref().expect("returned stage manifest")
    }

    pub(crate) fn execution_profile(&self) -> &AppContainerProfile {
        self._profile.as_ref().expect("live execution profile")
    }

    pub(crate) fn verify_inputs_and_pin(
        &self,
    ) -> Result<optic_bridge_windows::PinnedDirectoryChain, PrivateProjectStageError> {
        let pin = pin_directory_chain(&self.root, None)
            .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
        if inspect_directory_no_reparse(&self.root)
            .map_err(|_| PrivateProjectStageError::NativeBoundary)?
            .identity
            != self.identity
        {
            return Err(PrivateProjectStageError::NativeBoundary);
        }
        for input in self.manifest().files() {
            let relative = Path::new(input.path.as_str());
            let _parent = pin_directory_chain(&self.root, relative.parent())
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            let mut file = optic_bridge_windows::open_file_no_reparse(&self.root.join(relative))
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            let observed = optic_bridge_core::ContentVersion::from_reader_bounded(
                file.file_mut(),
                input.size_bytes.max(1),
            )
            .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            if observed != input.version {
                return Err(PrivateProjectStageError::InvalidPrivateTask);
            }
        }
        Ok(pin)
    }

    pub(crate) fn node_environment(
        &mut self,
    ) -> Result<Vec<(std::ffi::OsString, std::ffi::OsString)>, PrivateProjectStageError> {
        use std::ffi::OsString;
        // .optic is forbidden in input manifests, so this cannot replace input
        // files. No inherited HOME/PATH/NODE_OPTIONS/credentials are forwarded.
        for relative in [".optic", ".optic/tmp", ".optic/local", ".optic/roaming"] {
            let path = PathBuf::from(relative);
            let _parent = pin_directory_chain(&self.root, path.parent())
                .map_err(|_| PrivateProjectStageError::NativeBoundary)?;
            fs::create_dir(self.root.join(&path))?;
            self.directories.insert(path);
        }
        Ok(vec![
            (
                OsString::from("SystemRoot"),
                std::env::var_os("SystemRoot")
                    .ok_or(PrivateProjectStageError::InvalidPrivateTask)?,
            ),
            (
                OsString::from("TEMP"),
                self.root.join(".optic/tmp").into_os_string(),
            ),
            (
                OsString::from("TMP"),
                self.root.join(".optic/tmp").into_os_string(),
            ),
            (
                OsString::from("LOCALAPPDATA"),
                self.root.join(".optic/local").into_os_string(),
            ),
            (
                OsString::from("APPDATA"),
                self.root.join(".optic/roaming").into_os_string(),
            ),
        ])
    }

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
        self._profile
            .take()
            .expect("owned cleanup profile")
            .close()?;
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
    retain_on_drop: bool,
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
#[cfg(windows)]
impl Drop for StageSlot {
    fn drop(&mut self) {
        if !self.retain_on_drop {
            self.release();
        }
    }
}
// A slot is released explicitly only after known cleanup. A failed cleanup
// retains accounting and eventually refuses further preparation, fail closed.

#[derive(Debug, Error)]
pub enum PrivateProjectStageError {
    #[error("private task shape or prepared input version is invalid")]
    InvalidPrivateTask,
    #[error("private task binding mismatch: {0}")]
    Input(#[from] ProjectInputError),
    #[error("private process admission failed: {0}")]
    Process(#[from] crate::ProcessError),
    #[error("private runtime identity failed: {0}")]
    RuntimeIdentity(#[from] crate::ProcessAuthorityError),
    #[error("private preparation owner worker is unavailable")]
    WorkerUnavailable,
    #[error("private preparation I/O capacity unavailable: {0}")]
    BlockingIo(#[from] crate::BlockingIoError),
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
