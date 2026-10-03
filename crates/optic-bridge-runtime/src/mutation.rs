use std::path::Path;

#[cfg(any(windows, all(test, not(windows))))]
use std::path::PathBuf;
#[cfg(windows)]
use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use optic_bridge_core::{ActionId, ContentVersion, ExpectedState, HardLimits, WorkspacePath};
use thiserror::Error;

use crate::{BoundedFileSystem, FileSystemError, MutationError};

#[cfg(windows)]
use optic_bridge_windows::{
    WindowsFileError, WindowsFileIdentity, inspect_directory_no_reparse,
    open_file_for_delete_no_reparse, open_file_no_reparse, replace_file_atomically,
};

#[cfg(windows)]
const STAGING_PREFIX: &str = ".optic-";
#[cfg(windows)]
const STAGING_SUFFIX: &str = ".staged";

#[derive(Debug)]
pub struct AtomicMutationService {
    filesystem: BoundedFileSystem,
    max_mutation_bytes: u64,
    #[cfg(windows)]
    windows_root_final_path: PathBuf,
}

impl AtomicMutationService {
    pub fn from_hard_limits(
        root: impl AsRef<Path>,
        limits: HardLimits,
    ) -> Result<Self, AtomicMutationError> {
        let filesystem = BoundedFileSystem::from_hard_limits(root, limits)?;
        #[cfg(windows)]
        let windows_root_final_path = {
            let root = inspect_directory_no_reparse(filesystem.root())?;
            root.final_path
        };

        Ok(Self {
            filesystem,
            max_mutation_bytes: limits.max_fs_mutation_bytes,
            #[cfg(windows)]
            windows_root_final_path,
        })
    }

    pub fn prepare_write(
        &self,
        path: &WorkspacePath,
        expected: ExpectedState,
    ) -> Result<PreparedMutation, AtomicMutationError> {
        let observation = self.filesystem.require_expected_state(path, expected)?;

        #[cfg(windows)]
        let windows_guard = self.prepare_windows_guard(&observation.canonical_path, expected)?;

        Ok(PreparedMutation {
            canonical_path: observation.canonical_path,
            expected,
            #[cfg(windows)]
            windows_guard,
        })
    }

    pub fn prepare_delete(
        &self,
        path: &WorkspacePath,
        expected: ContentVersion,
    ) -> Result<PreparedDelete, AtomicMutationError> {
        let expected_state = ExpectedState::Content(expected);
        let observation = self
            .filesystem
            .require_expected_state(path, expected_state)?;

        #[cfg(windows)]
        let windows_identity =
            match self.prepare_windows_guard(&observation.canonical_path, expected_state)? {
                WindowsMutationGuard::Existing { identity } => identity,
                WindowsMutationGuard::Absent { .. } => {
                    return Err(AtomicMutationError::InvalidPreparedMutation);
                }
            };

        Ok(PreparedDelete {
            canonical_path: observation.canonical_path,
            expected,
            #[cfg(windows)]
            windows_identity,
        })
    }

    pub fn commit_write(
        &self,
        plan: &PreparedMutation,
        content: &[u8],
    ) -> Result<MutationCommit, AtomicMutationError> {
        #[cfg(windows)]
        {
            let action_id =
                ActionId::generate().map_err(|_| AtomicMutationError::TempNameEntropy)?;
            self.commit_write_for_action(plan, content, &action_id)
        }

        #[cfg(not(windows))]
        {
            let _ = (plan, content);
            Err(AtomicMutationError::UnsupportedPlatform)
        }
    }

    pub fn commit_write_for_action(
        &self,
        plan: &PreparedMutation,
        content: &[u8],
        action_id: &ActionId,
    ) -> Result<MutationCommit, AtomicMutationError> {
        let content_len =
            u64::try_from(content.len()).map_err(|_| AtomicMutationError::NewContentTooLarge {
                limit: self.max_mutation_bytes,
            })?;
        if content_len > self.max_mutation_bytes {
            return Err(AtomicMutationError::NewContentTooLarge {
                limit: self.max_mutation_bytes,
            });
        }

        #[cfg(windows)]
        {
            self.commit_write_windows(plan, content, action_id)
        }

        #[cfg(not(windows))]
        {
            let _ = (plan, content, action_id);
            Err(AtomicMutationError::UnsupportedPlatform)
        }
    }

    pub fn commit_delete(
        &self,
        plan: &PreparedDelete,
    ) -> Result<DeleteCommit, AtomicMutationError> {
        #[cfg(windows)]
        {
            self.commit_delete_windows(plan)
        }

        #[cfg(not(windows))]
        {
            let _ = plan;
            Err(AtomicMutationError::UnsupportedPlatform)
        }
    }

    #[must_use]
    pub fn filesystem(&self) -> &BoundedFileSystem {
        &self.filesystem
    }

    #[cfg(windows)]
    fn prepare_windows_guard(
        &self,
        canonical_path: &WorkspacePath,
        expected: ExpectedState,
    ) -> Result<WindowsMutationGuard, AtomicMutationError> {
        let absolute = self.absolute_path(canonical_path);
        match expected {
            ExpectedState::Content(version) => {
                let mut opened = open_file_no_reparse(&absolute)?;
                self.ensure_windows_final_path(opened.final_path())?;
                let observed =
                    ContentVersion::from_reader_bounded(opened.file_mut(), self.max_mutation_bytes)
                        .map_err(|error| match error {
                            optic_bridge_core::ContentVersionReadError::LimitExceeded => {
                                AtomicMutationError::NewContentTooLarge {
                                    limit: self.max_mutation_bytes,
                                }
                            }
                            optic_bridge_core::ContentVersionReadError::Io(error) => {
                                AtomicMutationError::Io(error)
                            }
                        })?;
                if observed != version {
                    return Err(AtomicMutationError::StrongPreconditionMismatch);
                }
                Ok(WindowsMutationGuard::Existing {
                    identity: opened.identity(),
                })
            }
            ExpectedState::Absent => {
                let parent = absolute
                    .parent()
                    .ok_or(AtomicMutationError::InvalidTarget)?;
                let parent = inspect_directory_no_reparse(parent)?;
                self.ensure_windows_final_path(&parent.final_path)?;
                Ok(WindowsMutationGuard::Absent {
                    parent_identity: parent.identity,
                })
            }
        }
    }

    #[cfg(windows)]
    fn commit_write_windows(
        &self,
        plan: &PreparedMutation,
        content: &[u8],
        action_id: &ActionId,
    ) -> Result<MutationCommit, AtomicMutationError> {
        self.revalidate_plan(plan)?;

        let target = self.absolute_path(&plan.canonical_path);
        let parent = target.parent().ok_or(AtomicMutationError::InvalidTarget)?;
        let mut staged = StagedFile::create(parent, content, action_id)?;

        // The staging write may take time. Revalidate again immediately before
        // the one-step namespace commit so an editor/change during staging fails closed.
        self.revalidate_plan(plan)?;

        let cleanup_succeeded = match plan.expected {
            ExpectedState::Absent => {
                match fs::hard_link(staged.path(), &target) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        return Err(AtomicMutationError::TargetAppearedDuringCommit);
                    }
                    Err(error) => return Err(AtomicMutationError::Io(error)),
                }
                let cleanup_succeeded = fs::remove_file(staged.path()).is_ok();
                if cleanup_succeeded {
                    staged.disarm();
                }
                cleanup_succeeded
            }
            ExpectedState::Content(_) => {
                replace_file_atomically(&target, staged.path())?;
                staged.disarm();
                true
            }
        };

        let expected_new = ContentVersion::from_bytes(content);
        let verification = match self
            .filesystem
            .observe_mutation_target(&plan.canonical_path)
        {
            Ok(observation)
                if observation.state == ExpectedState::Content(expected_new)
                    && observation.canonical_path == plan.canonical_path =>
            {
                CommitVerification::Verified(expected_new)
            }
            _ => CommitVerification::CommittedButUnverified,
        };

        Ok(MutationCommit {
            canonical_path: plan.canonical_path.clone(),
            previous_state: plan.expected,
            verification,
            temp_cleanup_succeeded: cleanup_succeeded,
        })
    }

    #[cfg(windows)]
    fn commit_delete_windows(
        &self,
        plan: &PreparedDelete,
    ) -> Result<DeleteCommit, AtomicMutationError> {
        let expected_state = ExpectedState::Content(plan.expected);
        let observation = self
            .filesystem
            .require_expected_state(&plan.canonical_path, expected_state)?;
        if observation.canonical_path != plan.canonical_path {
            return Err(AtomicMutationError::CanonicalTargetChanged);
        }

        let absolute = self.absolute_path(&plan.canonical_path);
        let mut opened = open_file_for_delete_no_reparse(&absolute)?;
        self.ensure_windows_final_path(opened.final_path())?;
        if opened.identity() != plan.windows_identity {
            return Err(AtomicMutationError::TargetIdentityChanged);
        }
        let observed =
            ContentVersion::from_reader_bounded(opened.file_mut(), self.max_mutation_bytes)
                .map_err(|error| match error {
                    optic_bridge_core::ContentVersionReadError::LimitExceeded => {
                        AtomicMutationError::NewContentTooLarge {
                            limit: self.max_mutation_bytes,
                        }
                    }
                    optic_bridge_core::ContentVersionReadError::Io(error) => {
                        AtomicMutationError::Io(error)
                    }
                })?;
        if observed != plan.expected {
            return Err(AtomicMutationError::StrongPreconditionMismatch);
        }

        // The same no-reparse handle whose final path, identity and bytes were
        // validated above performs the delete. No path-based delete is issued.
        opened.delete()?;

        let verification = match self
            .filesystem
            .observe_mutation_target(&plan.canonical_path)
        {
            Ok(observation)
                if observation.state == ExpectedState::Absent
                    && observation.canonical_path == plan.canonical_path =>
            {
                DeleteVerification::VerifiedAbsent
            }
            _ => DeleteVerification::CommittedButUnverified,
        };

        Ok(DeleteCommit {
            canonical_path: plan.canonical_path.clone(),
            previous_version: plan.expected,
            verification,
        })
    }

    #[cfg(windows)]
    fn revalidate_plan(&self, plan: &PreparedMutation) -> Result<(), AtomicMutationError> {
        let observation = self
            .filesystem
            .require_expected_state(&plan.canonical_path, plan.expected)?;
        if observation.canonical_path != plan.canonical_path {
            return Err(AtomicMutationError::CanonicalTargetChanged);
        }

        let absolute = self.absolute_path(&plan.canonical_path);
        match (&plan.windows_guard, plan.expected) {
            (WindowsMutationGuard::Existing { identity }, ExpectedState::Content(version)) => {
                let mut opened = open_file_no_reparse(&absolute)?;
                self.ensure_windows_final_path(opened.final_path())?;
                if opened.identity() != *identity {
                    return Err(AtomicMutationError::TargetIdentityChanged);
                }
                let observed =
                    ContentVersion::from_reader_bounded(opened.file_mut(), self.max_mutation_bytes)
                        .map_err(|error| match error {
                            optic_bridge_core::ContentVersionReadError::LimitExceeded => {
                                AtomicMutationError::NewContentTooLarge {
                                    limit: self.max_mutation_bytes,
                                }
                            }
                            optic_bridge_core::ContentVersionReadError::Io(error) => {
                                AtomicMutationError::Io(error)
                            }
                        })?;
                if observed != version {
                    return Err(AtomicMutationError::StrongPreconditionMismatch);
                }
                Ok(())
            }
            (WindowsMutationGuard::Absent { parent_identity }, ExpectedState::Absent) => {
                let parent = absolute
                    .parent()
                    .ok_or(AtomicMutationError::InvalidTarget)?;
                let current_parent = inspect_directory_no_reparse(parent)?;
                self.ensure_windows_final_path(&current_parent.final_path)?;
                if current_parent.identity != *parent_identity {
                    return Err(AtomicMutationError::ParentIdentityChanged);
                }
                match fs::symlink_metadata(&absolute) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Ok(_) => Err(AtomicMutationError::TargetAppearedDuringCommit),
                    Err(error) => Err(AtomicMutationError::Io(error)),
                }
            }
            _ => Err(AtomicMutationError::InvalidPreparedMutation),
        }
    }

    #[cfg(windows)]
    fn ensure_windows_final_path(&self, path: &Path) -> Result<(), AtomicMutationError> {
        if windows_path_is_within(&self.windows_root_final_path, path) {
            Ok(())
        } else {
            Err(AtomicMutationError::OutsideWorkspace)
        }
    }

    #[cfg(windows)]
    fn absolute_path(&self, path: &WorkspacePath) -> PathBuf {
        absolute_workspace_path(self.filesystem.root(), path)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedMutation {
    canonical_path: WorkspacePath,
    expected: ExpectedState,
    #[cfg(windows)]
    windows_guard: WindowsMutationGuard,
}

impl PreparedMutation {
    #[must_use]
    pub fn canonical_path(&self) -> &WorkspacePath {
        &self.canonical_path
    }

    #[must_use]
    pub const fn expected_state(&self) -> ExpectedState {
        self.expected
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedDelete {
    canonical_path: WorkspacePath,
    expected: ContentVersion,
    #[cfg(windows)]
    windows_identity: WindowsFileIdentity,
}

impl PreparedDelete {
    #[must_use]
    pub fn canonical_path(&self) -> &WorkspacePath {
        &self.canonical_path
    }

    #[must_use]
    pub const fn expected_version(&self) -> ContentVersion {
        self.expected
    }

    #[must_use]
    pub const fn previous_state(&self) -> ExpectedState {
        ExpectedState::Content(self.expected)
    }

    pub(crate) fn journal_plan(&self) -> PreparedMutation {
        PreparedMutation {
            canonical_path: self.canonical_path.clone(),
            expected: self.previous_state(),
            #[cfg(windows)]
            windows_guard: WindowsMutationGuard::Existing {
                identity: self.windows_identity,
            },
        }
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowsMutationGuard {
    Existing {
        identity: WindowsFileIdentity,
    },
    Absent {
        parent_identity: WindowsFileIdentity,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationCommit {
    pub canonical_path: WorkspacePath,
    pub previous_state: ExpectedState,
    pub verification: CommitVerification,
    pub temp_cleanup_succeeded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitVerification {
    Verified(ContentVersion),
    CommittedButUnverified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteCommit {
    pub canonical_path: WorkspacePath,
    pub previous_version: ContentVersion,
    pub verification: DeleteVerification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteVerification {
    VerifiedAbsent,
    CommittedButUnverified,
}

#[cfg(windows)]
struct StagedFile {
    path: PathBuf,
    armed: bool,
}

#[cfg(windows)]
impl StagedFile {
    fn create(
        parent: &Path,
        content: &[u8],
        action_id: &ActionId,
    ) -> Result<Self, AtomicMutationError> {
        let path = parent.join(staging_file_name(action_id));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = match options.open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(AtomicMutationError::StagingArtifactAlreadyExists);
            }
            Err(error) => return Err(AtomicMutationError::Io(error)),
        };
        if let Err(error) = file.write_all(content).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&path);
            return Err(AtomicMutationError::Io(error));
        }
        Ok(Self { path, armed: true })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

#[cfg(windows)]
impl Drop for StagedFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(windows)]
pub(crate) fn staging_file_name(action_id: &ActionId) -> String {
    format!("{STAGING_PREFIX}{}{STAGING_SUFFIX}", action_id.to_token())
}

#[cfg(windows)]
pub(crate) fn staging_path_for(
    filesystem: &BoundedFileSystem,
    path: &WorkspacePath,
    action_id: &ActionId,
) -> PathBuf {
    let mut target = absolute_workspace_path(filesystem.root(), path);
    let _ = target.pop();
    target.push(staging_file_name(action_id));
    target
}

#[cfg(windows)]
fn absolute_workspace_path(root: &Path, path: &WorkspacePath) -> PathBuf {
    let mut absolute = root.to_path_buf();
    for segment in path.as_str().split('/') {
        absolute.push(segment);
    }
    absolute
}

#[cfg(windows)]
fn windows_path_is_within(root: &Path, candidate: &Path) -> bool {
    fn key(path: &Path) -> String {
        path.as_os_str()
            .to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    }

    let root = key(root);
    let candidate = key(candidate);
    if candidate == root {
        return true;
    }
    candidate
        .strip_prefix(&root)
        .is_some_and(|suffix| suffix.starts_with('\\'))
}

#[derive(Debug, Error)]
pub enum AtomicMutationError {
    #[error("mutation precondition failed: {0}")]
    Mutation(#[from] MutationError),
    #[error("filesystem initialization failed: {0}")]
    FileSystem(#[from] FileSystemError),
    #[cfg(windows)]
    #[error("Windows mutation primitive failed: {0}")]
    Windows(#[from] WindowsFileError),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("new mutation content exceeds hard byte ceiling {limit}")]
    NewContentTooLarge { limit: u64 },
    #[error("prepared mutation target is invalid")]
    InvalidTarget,
    #[error("prepared mutation state is internally inconsistent")]
    InvalidPreparedMutation,
    #[error("canonical mutation target changed after planning")]
    CanonicalTargetChanged,
    #[error("Windows file identity changed after planning")]
    TargetIdentityChanged,
    #[error("Windows parent-directory identity changed after planning")]
    ParentIdentityChanged,
    #[error("strong mutation precondition no longer matches")]
    StrongPreconditionMismatch,
    #[error("mutation target appeared during create-only commit")]
    TargetAppearedDuringCommit,
    #[error("resolved Windows target escaped the canonical workspace")]
    OutsideWorkspace,
    #[error("failed to obtain entropy for a staging-file name")]
    TempNameEntropy,
    #[error("operation staging artifact already exists")]
    StagingArtifactAlreadyExists,
    #[error("durable mutation commit is not implemented on this platform yet")]
    UnsupportedPlatform,
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    fn workspace(label: &str) -> PathBuf {
        let token = ActionId::generate().expect("test entropy").to_token();
        let root = env::temp_dir().join(format!("optic-bridge-mutation-{label}-{token}"));
        fs::create_dir_all(&root).expect("create temp workspace");
        root
    }

    #[cfg(not(windows))]
    #[test]
    fn durable_commit_fails_closed_on_unsupported_platforms() {
        let root = workspace("unsupported");
        let limits = HardLimits::default();
        let service = AtomicMutationService::from_hard_limits(&root, limits).expect("service");
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = service
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");

        assert!(matches!(
            service.commit_write(&plan, b"content"),
            Err(AtomicMutationError::UnsupportedPlatform)
        ));
        assert!(!root.join("new.txt").exists());
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(not(windows))]
    #[test]
    fn durable_delete_fails_closed_on_unsupported_platforms() {
        let root = workspace("unsupported-delete");
        fs::write(root.join("target.txt"), b"alpha").expect("fixture");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_delete(&path, ContentVersion::from_bytes(b"alpha"))
            .expect("plan");

        assert!(matches!(
            service.commit_delete(&plan),
            Err(AtomicMutationError::UnsupportedPlatform)
        ));
        assert_eq!(fs::read(root.join("target.txt")).expect("target"), b"alpha");
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(windows)]
    #[test]
    fn windows_existing_write_is_revalidated_and_replaced() {
        let root = workspace("windows-replace");
        fs::write(root.join("target.txt"), b"alpha").expect("write fixture");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(
                &path,
                ExpectedState::Content(ContentVersion::from_bytes(b"alpha")),
            )
            .expect("plan");

        let commit = service.commit_write(&plan, b"beta").expect("commit");
        assert_eq!(fs::read(root.join("target.txt")).expect("read"), b"beta");
        assert_eq!(
            commit.verification,
            CommitVerification::Verified(ContentVersion::from_bytes(b"beta"))
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(windows)]
    #[test]
    fn windows_delete_uses_exact_prepared_identity_and_content() {
        let root = workspace("windows-delete");
        fs::write(root.join("target.txt"), b"alpha").expect("write fixture");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let expected = ContentVersion::from_bytes(b"alpha");
        let plan = service.prepare_delete(&path, expected).expect("plan");

        let commit = service.commit_delete(&plan).expect("delete");
        assert!(!root.join("target.txt").exists());
        assert_eq!(commit.previous_version, expected);
        assert_eq!(commit.verification, DeleteVerification::VerifiedAbsent);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(windows)]
    #[test]
    fn windows_delete_rejects_stale_content_without_removing_target() {
        let root = workspace("windows-delete-stale");
        fs::write(root.join("target.txt"), b"alpha").expect("write fixture");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_delete(&path, ContentVersion::from_bytes(b"alpha"))
            .expect("plan");
        fs::write(root.join("target.txt"), b"changed").expect("external change");

        assert!(service.commit_delete(&plan).is_err());
        assert_eq!(
            fs::read(root.join("target.txt")).expect("target"),
            b"changed"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(windows)]
    #[test]
    fn windows_delete_rejects_same_content_recreation_by_identity() {
        let root = workspace("windows-delete-identity");
        let target = root.join("target.txt");
        let backup = root.join("old-target.txt");
        fs::write(&target, b"alpha").expect("write fixture");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_delete(&path, ContentVersion::from_bytes(b"alpha"))
            .expect("plan");

        fs::rename(&target, &backup).expect("preserve original identity");
        fs::write(&target, b"alpha").expect("recreate same bytes");

        assert!(matches!(
            service.commit_delete(&plan),
            Err(AtomicMutationError::TargetIdentityChanged)
        ));
        assert_eq!(fs::read(&target).expect("recreated target"), b"alpha");
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(windows)]
    #[test]
    fn windows_action_id_owns_the_staging_name() {
        let root = workspace("windows-action-staging");
        fs::write(root.join("target.txt"), b"alpha").expect("write fixture");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(
                &path,
                ExpectedState::Content(ContentVersion::from_bytes(b"alpha")),
            )
            .expect("plan");
        let action_id = ActionId::generate().expect("action");
        let staging = staging_path_for(service.filesystem(), &path, &action_id);
        fs::write(&staging, b"foreign").expect("reserve action staging path");

        assert!(matches!(
            service.commit_write_for_action(&plan, b"beta", &action_id),
            Err(AtomicMutationError::StagingArtifactAlreadyExists)
        ));
        assert_eq!(fs::read(root.join("target.txt")).expect("target"), b"alpha");
        assert_eq!(fs::read(&staging).expect("staging"), b"foreign");
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(windows)]
    #[test]
    fn windows_same_content_recreation_is_rejected_by_identity() {
        let root = workspace("windows-identity");
        let target = root.join("target.txt");
        let backup = root.join("old-target.txt");
        fs::write(&target, b"alpha").expect("write fixture");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = service
            .prepare_write(
                &path,
                ExpectedState::Content(ContentVersion::from_bytes(b"alpha")),
            )
            .expect("plan");

        fs::rename(&target, &backup).expect("preserve original identity");
        fs::write(&target, b"alpha").expect("recreate same bytes");

        assert!(matches!(
            service.commit_write(&plan, b"beta"),
            Err(AtomicMutationError::TargetIdentityChanged)
        ));
        assert_eq!(fs::read(&target).expect("read recreated"), b"alpha");
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(windows)]
    #[test]
    fn windows_absent_create_is_create_only() {
        let root = workspace("windows-create");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = service
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");

        let commit = service.commit_write(&plan, b"created").expect("commit");
        assert_eq!(fs::read(root.join("new.txt")).expect("read"), b"created");
        assert!(matches!(
            commit.verification,
            CommitVerification::Verified(_)
        ));
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(windows)]
    #[test]
    fn windows_parent_recreation_invalidates_absent_plan() {
        let root = workspace("windows-parent-identity");
        let parent = root.join("dir");
        let old_parent = root.join("dir-old");
        fs::create_dir(&parent).expect("create parent");
        let service =
            AtomicMutationService::from_hard_limits(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("dir/new.txt").expect("path");
        let plan = service
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");

        fs::rename(&parent, &old_parent).expect("preserve old parent identity");
        fs::create_dir(&parent).expect("recreate parent");

        assert!(matches!(
            service.commit_write(&plan, b"created"),
            Err(AtomicMutationError::ParentIdentityChanged)
        ));
        assert!(!parent.join("new.txt").exists());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
