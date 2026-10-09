use std::{
    collections::HashMap,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Mutex,
    time::Duration,
};

use optic_bridge_core::{GitObjectId, GitObjectIdError, HardLimits, SessionHandle};
use thiserror::Error;

use crate::{
    GitBlobBatchError, GitReadError, GitReadService, GitTreeEntryKind, GitTreeManifest,
    GitTreeManifestError, HardenedCommandError, HardenedCommandRunner, HardenedCommandSpec,
    SessionBlobBatch, SessionChangeSet, SessionConflictError, SessionConflictReport,
    git_blob_batch::{ExpectedGitBlob, expected_batch_output_bytes, parse_cat_file_batch},
    git_change_set::{build_change_set, detect_conflicts},
    git_tree_manifest::parse_git_tree_manifest,
    git_worktree::{
        RegisteredWorktree, WorktreeListError, git_mutation_base_args, git_mutation_environment,
        git_path_arg, parse_worktree_list, path_is_lexically_within, paths_lexically_equal,
    },
};

const CAPTURE_LIMIT_BYTES: u64 = 4 * 1024;
const DISABLED_HOOKS_DIRECTORY: &str = "hooks-disabled";

pub struct SessionWorktreeManager {
    repository_root: PathBuf,
    git_executable: PathBuf,
    worktree_root: PathBuf,
    disabled_hooks_root: PathBuf,
    runner: HardenedCommandRunner,
    max_worktrees: usize,
    recovery_entry_limit: u32,
    recovery_output_limit: u64,
    blob_batch_entry_limit: u32,
    blob_batch_input_limit: u64,
    blob_batch_output_limit: u64,
    blob_size_limit: u64,
    conflict_report_limit: u32,
    records: Mutex<HashMap<SessionHandle, SessionWorktreeRecord>>,
}

#[derive(Clone, Debug)]
struct SessionWorktreeRecord {
    path: PathBuf,
    base_head: GitObjectId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionWorktree {
    pub owner: SessionHandle,
    pub path: PathBuf,
    pub base_head: GitObjectId,
    pub current_head: GitObjectId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionWorktreeRecoveryReport {
    pub removed_worktrees: u32,
}

impl SessionWorktreeManager {
    pub fn from_hard_limits(
        repository_root: impl AsRef<Path>,
        git_executable: impl AsRef<Path>,
        worktree_root: impl AsRef<Path>,
        limits: HardLimits,
    ) -> Result<Self, SessionWorktreeError> {
        let limits = limits
            .validate_nonzero()
            .map_err(|_| SessionWorktreeError::InvalidLimits)?;
        let read_service =
            GitReadService::from_hard_limits(repository_root, git_executable, limits)?;
        if !worktree_root.as_ref().is_absolute() {
            return Err(SessionWorktreeError::WorktreeRootMustBeAbsolute);
        }
        fs::create_dir_all(worktree_root.as_ref())?;
        let worktree_root = fs::canonicalize(worktree_root.as_ref())?;
        if !fs::metadata(&worktree_root)?.is_dir() {
            return Err(SessionWorktreeError::WorktreeRootNotDirectory);
        }

        let repository_root = read_service.repository_root().to_path_buf();
        if path_is_lexically_within(&repository_root, &worktree_root)
            || path_is_lexically_within(&worktree_root, &repository_root)
        {
            return Err(SessionWorktreeError::WorktreeRootOverlapsRepository);
        }

        let disabled_hooks_root = worktree_root.join(DISABLED_HOOKS_DIRECTORY);
        fs::create_dir_all(&disabled_hooks_root)?;
        if fs::read_dir(&disabled_hooks_root)?.next().is_some() {
            return Err(SessionWorktreeError::DisabledHooksDirectoryNotEmpty);
        }

        Ok(Self {
            repository_root,
            git_executable: read_service.git_executable().to_path_buf(),
            worktree_root,
            disabled_hooks_root,
            runner: HardenedCommandRunner::new(Duration::from_millis(
                limits.max_request_duration_ms,
            ))
            .map_err(|_| SessionWorktreeError::InvalidLimits)?,
            max_worktrees: usize::try_from(limits.max_sessions)
                .map_err(|_| SessionWorktreeError::InvalidLimits)?,
            recovery_entry_limit: limits.max_fs_directory_scan_entries,
            recovery_output_limit: limits.max_git_read_bytes,
            blob_batch_entry_limit: limits.max_fs_list_page_entries,
            blob_batch_input_limit: limits.max_git_read_bytes,
            blob_batch_output_limit: limits.max_active_output_ram_bytes_per_session,
            blob_size_limit: limits.max_fs_mutation_bytes,
            conflict_report_limit: limits.max_fs_list_page_entries,
            records: Mutex::new(HashMap::new()),
        })
    }

    #[must_use]
    pub fn repository_root(&self) -> &Path {
        &self.repository_root
    }

    #[must_use]
    pub fn worktree_root(&self) -> &Path {
        &self.worktree_root
    }

    pub fn provision(
        &self,
        owner: &SessionHandle,
        base_head: &GitObjectId,
    ) -> Result<SessionWorktree, SessionWorktreeError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| SessionWorktreeError::RegistryPoisoned)?;
        if records.contains_key(owner) {
            return Err(SessionWorktreeError::SessionAlreadyProvisioned);
        }
        if records.len() >= self.max_worktrees {
            return Err(SessionWorktreeError::CapacityExceeded);
        }
        self.ensure_disabled_hooks_empty()?;
        let resolved = self.resolve_commit(&self.repository_root, base_head.as_str())?;
        if &resolved != base_head {
            return Err(SessionWorktreeError::BaseHeadMismatch);
        }

        let path = self.worktree_root.join(owner.to_token());
        if path.exists() {
            return Err(SessionWorktreeError::WorktreePathAlreadyExists);
        }
        let status = self.run_status(
            &self.repository_root,
            [
                OsString::from("worktree"),
                OsString::from("add"),
                OsString::from("--detach"),
                OsString::from("--no-checkout"),
                OsString::from("--lock"),
                git_path_arg(&path),
                OsString::from(base_head.as_str()),
            ],
        )?;
        if !status.success() {
            return Err(SessionWorktreeError::WorktreeCreateFailed);
        }

        // Reserve ownership immediately after Git reports creation. If later validation
        // or cleanup becomes uncertain, retain the slot/record fail-closed instead of
        // reclaiming session capacity while an owned worktree may still exist.
        records.insert(
            owner.clone(),
            SessionWorktreeRecord {
                path: path.clone(),
                base_head: base_head.clone(),
            },
        );

        let validated = self
            .validate_owned_path(owner, &path)
            .and_then(|canonical| {
                let observed = self.resolve_commit(&canonical, "HEAD")?;
                if &observed != base_head {
                    return Err(SessionWorktreeError::BaseHeadMismatch);
                }
                Ok((canonical, observed))
            });
        let (canonical, current_head) = match validated {
            Ok(value) => value,
            Err(error) => {
                if self.remove_owned_worktree(&path).is_ok() {
                    records.remove(owner);
                    return Err(error);
                }
                return Err(SessionWorktreeError::ProvisionCleanupFailed);
            }
        };

        records.insert(
            owner.clone(),
            SessionWorktreeRecord {
                path: canonical.clone(),
                base_head: base_head.clone(),
            },
        );
        Ok(SessionWorktree {
            owner: owner.clone(),
            path: canonical,
            base_head: base_head.clone(),
            current_head,
        })
    }

    pub fn get(&self, owner: &SessionHandle) -> Result<SessionWorktree, SessionWorktreeError> {
        let records = self
            .records
            .lock()
            .map_err(|_| SessionWorktreeError::RegistryPoisoned)?;
        let record = records
            .get(owner)
            .ok_or(SessionWorktreeError::UnknownSessionWorktree)?;
        let canonical = self.validate_owned_path(owner, &record.path)?;
        let current_head = self.resolve_commit(&canonical, "HEAD")?;
        Ok(SessionWorktree {
            owner: owner.clone(),
            path: canonical,
            base_head: record.base_head.clone(),
            current_head,
        })
    }

    pub fn manifest(&self, owner: &SessionHandle) -> Result<GitTreeManifest, SessionWorktreeError> {
        let worktree = self.get(owner)?;
        self.manifest_for_head(&worktree.current_head)
    }

    pub fn change_set(
        &self,
        owner: &SessionHandle,
    ) -> Result<SessionChangeSet, SessionWorktreeError> {
        let worktree = self.get(owner)?;
        let base = self.manifest_for_head(&worktree.base_head)?;
        let current = self.manifest_for_head(&worktree.current_head)?;
        Ok(build_change_set(owner.clone(), &base, &current))
    }

    pub fn conflicts(
        &self,
        left: &SessionHandle,
        right: &SessionHandle,
    ) -> Result<SessionConflictReport, SessionWorktreeError> {
        let left = self.change_set(left)?;
        let right = self.change_set(right)?;
        detect_conflicts(&left, &right, self.conflict_report_limit)
            .map_err(SessionWorktreeError::Conflict)
    }

    fn manifest_for_head(
        &self,
        head: &GitObjectId,
    ) -> Result<GitTreeManifest, SessionWorktreeError> {
        let mut args = git_mutation_base_args(&self.disabled_hooks_root);
        args.extend([
            OsString::from("ls-tree"),
            OsString::from("-r"),
            OsString::from("-z"),
            OsString::from("--full-tree"),
            OsString::from("-l"),
            OsString::from(head.as_str()),
        ]);
        let mut env = git_mutation_environment();
        env.insert(OsString::from("GIT_OPTIONAL_LOCKS"), OsString::from("0"));
        let spec = HardenedCommandSpec {
            executable: self.git_executable.clone(),
            cwd: self.repository_root.clone(),
            args,
            env,
            output_limit: self.recovery_output_limit,
        };
        let output = self.runner.run(&spec).map_err(map_runner_error)?;
        if !output.status.success() {
            return Err(SessionWorktreeError::GitCommandFailed);
        }
        parse_git_tree_manifest(head.clone(), &output.stdout, self.recovery_entry_limit)
            .map_err(SessionWorktreeError::TreeManifest)
    }

    pub fn read_blob_batch(
        &self,
        owner: &SessionHandle,
        paths: &[optic_bridge_core::WorkspacePath],
    ) -> Result<SessionBlobBatch, SessionWorktreeError> {
        if paths.is_empty() {
            return Err(SessionWorktreeError::EmptyBlobBatch);
        }
        if paths.len() > usize::try_from(self.blob_batch_entry_limit).unwrap_or(usize::MAX) {
            return Err(SessionWorktreeError::BlobBatchEntryLimitExceeded);
        }

        let manifest = self.manifest(owner)?;
        let mut expected = Vec::with_capacity(paths.len());
        let mut input = Vec::new();
        let mut previous = None::<&str>;
        let mut sorted_paths = paths.iter().collect::<Vec<_>>();
        sorted_paths.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        for path in &sorted_paths {
            if previous == Some(path.as_str()) {
                return Err(SessionWorktreeError::DuplicateBlobBatchPath);
            }
            previous = Some(path.as_str());
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.path.as_str() == path.as_str())
                .ok_or(SessionWorktreeError::UnknownBlobBatchPath)?;
            let executable = match entry.kind {
                GitTreeEntryKind::RegularFile => false,
                GitTreeEntryKind::ExecutableFile => true,
                GitTreeEntryKind::Symlink | GitTreeEntryKind::Gitlink => {
                    return Err(SessionWorktreeError::UnsupportedBlobBatchEntry);
                }
            };
            let size = entry
                .blob_size
                .ok_or(SessionWorktreeError::UnsupportedBlobBatchEntry)?;
            if size > self.blob_size_limit {
                return Err(SessionWorktreeError::BlobTooLarge);
            }
            input.extend_from_slice(entry.object.as_str().as_bytes());
            input.push(b'\n');
            expected.push(ExpectedGitBlob {
                path: entry.path.clone(),
                object: entry.object.clone(),
                size,
                executable,
            });
        }
        if u64::try_from(input.len()).unwrap_or(u64::MAX) > self.blob_batch_input_limit {
            return Err(SessionWorktreeError::BlobBatchInputLimitExceeded);
        }
        let output_limit = expected_batch_output_bytes(&expected)?;
        if output_limit == 0 || output_limit > self.blob_batch_output_limit {
            return Err(SessionWorktreeError::BlobBatchOutputLimitExceeded);
        }

        let mut args = git_mutation_base_args(&self.disabled_hooks_root);
        args.extend([OsString::from("cat-file"), OsString::from("--batch")]);
        let mut env = git_mutation_environment();
        env.insert(OsString::from("GIT_OPTIONAL_LOCKS"), OsString::from("0"));
        let spec = HardenedCommandSpec {
            executable: self.git_executable.clone(),
            cwd: self.repository_root.clone(),
            args,
            env,
            output_limit,
        };
        let output = self
            .runner
            .run_with_input(&spec, &input, self.blob_batch_input_limit)
            .map_err(map_runner_error)?;
        if !output.status.success() {
            return Err(SessionWorktreeError::GitCommandFailed);
        }
        if !output.stderr.is_empty() {
            return Err(SessionWorktreeError::BlobBatchUnexpectedStderr);
        }
        let blobs = parse_cat_file_batch(&expected, &output.stdout)
            .map_err(SessionWorktreeError::BlobBatch)?;
        Ok(SessionBlobBatch {
            head: manifest.head,
            blobs,
        })
    }

    pub fn release(&self, owner: &SessionHandle) -> Result<(), SessionWorktreeError> {
        if self.release_if_owned(owner)? {
            Ok(())
        } else {
            Err(SessionWorktreeError::UnknownSessionWorktree)
        }
    }

    pub fn release_if_owned(&self, owner: &SessionHandle) -> Result<bool, SessionWorktreeError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| SessionWorktreeError::RegistryPoisoned)?;
        let Some(record) = records.get(owner).cloned() else {
            return Ok(false);
        };
        let canonical = self.validate_owned_path(owner, &record.path)?;
        self.remove_owned_worktree(&canonical)?;
        records.remove(owner);
        Ok(true)
    }

    pub fn recover_stale(&self) -> Result<SessionWorktreeRecoveryReport, SessionWorktreeError> {
        let records = self
            .records
            .lock()
            .map_err(|_| SessionWorktreeError::RegistryPoisoned)?;
        if !records.is_empty() {
            return Err(SessionWorktreeError::RecoveryWhileActive);
        }
        self.ensure_disabled_hooks_empty()?;

        let registered = self.registered_worktrees()?;
        let mut owned = HashMap::<String, PathBuf>::new();
        for worktree in registered {
            if !path_is_lexically_within(&self.worktree_root, &worktree.path) {
                continue;
            }
            let parent = worktree
                .path
                .parent()
                .ok_or(SessionWorktreeError::RecoveryOwnedPathMalformed)?;
            if !paths_lexically_equal(parent, &self.worktree_root) {
                return Err(SessionWorktreeError::RecoveryOwnedPathMalformed);
            }
            let token = worktree
                .path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(SessionWorktreeError::RecoveryOwnedPathMalformed)?;
            let owner = SessionHandle::from_token(token)
                .map_err(|_| SessionWorktreeError::RecoveryOwnedPathMalformed)?;
            if owner.to_token() != token || !worktree.locked {
                return Err(SessionWorktreeError::RecoveryOwnedPathMalformed);
            }
            if owned.len() >= self.max_worktrees {
                return Err(SessionWorktreeError::RecoveryOwnedLimitExceeded);
            }
            let canonical = validate_direct_owned_directory(&self.worktree_root, &worktree.path)?;
            if owned.insert(token.to_owned(), canonical).is_some() {
                return Err(SessionWorktreeError::RecoveryWorktreeListMalformed);
            }
        }

        let mut seen = 0_u32;
        let mut seen_owned = HashMap::<String, ()>::new();
        for entry in fs::read_dir(&self.worktree_root)? {
            seen = seen
                .checked_add(1)
                .ok_or(SessionWorktreeError::RecoveryEntryLimitExceeded)?;
            if seen > self.recovery_entry_limit {
                return Err(SessionWorktreeError::RecoveryEntryLimitExceeded);
            }
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| SessionWorktreeError::RecoveryUnexpectedEntry)?;
            if name == DISABLED_HOOKS_DIRECTORY {
                if entry.path() != self.disabled_hooks_root {
                    return Err(SessionWorktreeError::RecoveryUnexpectedEntry);
                }
                continue;
            }
            let owner = SessionHandle::from_token(&name)
                .map_err(|_| SessionWorktreeError::RecoveryUnexpectedEntry)?;
            if owner.to_token() != name {
                return Err(SessionWorktreeError::RecoveryUnexpectedEntry);
            }
            let Some(expected) = owned.get(&name) else {
                return Err(SessionWorktreeError::RecoveryUnregisteredOwnedPath);
            };
            let canonical = validate_direct_owned_directory(&self.worktree_root, &entry.path())?;
            if !paths_lexically_equal(&canonical, expected) {
                return Err(SessionWorktreeError::RecoveryOwnedPathUnsafe);
            }
            seen_owned.insert(name, ());
        }
        if seen_owned.len() != owned.len() {
            return Err(SessionWorktreeError::RecoveryOwnedWorktreeMissing);
        }

        let mut ordered = owned.into_iter().collect::<Vec<_>>();
        ordered.sort_by(|left, right| left.0.cmp(&right.0));
        for (_, path) in &ordered {
            self.remove_owned_worktree(path)?;
        }
        Ok(SessionWorktreeRecoveryReport {
            removed_worktrees: u32::try_from(ordered.len())
                .map_err(|_| SessionWorktreeError::RecoveryOwnedLimitExceeded)?,
        })
    }

    fn validate_owned_path(
        &self,
        owner: &SessionHandle,
        path: &Path,
    ) -> Result<PathBuf, SessionWorktreeError> {
        let expected = self.worktree_root.join(owner.to_token());
        if !paths_lexically_equal(path, &expected) {
            return Err(SessionWorktreeError::OwnershipMismatch);
        }
        let canonical = validate_direct_owned_directory(&self.worktree_root, path)?;
        let registered = self.registered_worktrees()?;
        let Some(worktree) = registered
            .iter()
            .find(|worktree| paths_lexically_equal(&worktree.path, &canonical))
        else {
            return Err(SessionWorktreeError::WorktreeNotRegistered);
        };
        if !worktree.locked {
            return Err(SessionWorktreeError::WorktreeNotLocked);
        }
        Ok(canonical)
    }

    fn ensure_disabled_hooks_empty(&self) -> Result<(), SessionWorktreeError> {
        if fs::read_dir(&self.disabled_hooks_root)?.next().is_some() {
            return Err(SessionWorktreeError::DisabledHooksDirectoryNotEmpty);
        }
        Ok(())
    }

    fn registered_worktrees(&self) -> Result<Vec<RegisteredWorktree>, SessionWorktreeError> {
        let (status, bytes) = self.run_capture(
            &self.repository_root,
            [
                OsString::from("worktree"),
                OsString::from("list"),
                OsString::from("--porcelain"),
                OsString::from("-z"),
            ],
            self.recovery_output_limit,
        )?;
        if !status.success() {
            return Err(SessionWorktreeError::GitCommandFailed);
        }
        parse_worktree_list(&bytes, self.recovery_entry_limit).map_err(|error| match error {
            WorktreeListError::Malformed => SessionWorktreeError::RecoveryWorktreeListMalformed,
            WorktreeListError::EntryLimitExceeded => {
                SessionWorktreeError::RecoveryEntryLimitExceeded
            }
        })
    }

    fn remove_owned_worktree(&self, path: &Path) -> Result<(), SessionWorktreeError> {
        let status = self.run_status(
            &self.repository_root,
            [
                OsString::from("worktree"),
                OsString::from("remove"),
                OsString::from("--force"),
                OsString::from("--force"),
                git_path_arg(path),
            ],
        )?;
        if !status.success() || path.exists() {
            return Err(SessionWorktreeError::WorktreeCleanupFailed);
        }
        Ok(())
    }

    fn resolve_commit(
        &self,
        context: &Path,
        object: &str,
    ) -> Result<GitObjectId, SessionWorktreeError> {
        let commitish = format!("{object}^{{commit}}");
        let (status, output) = self.run_capture(
            context,
            [
                OsString::from("rev-parse"),
                OsString::from("--verify"),
                OsString::from(commitish),
            ],
            CAPTURE_LIMIT_BYTES,
        )?;
        if !status.success() {
            return Err(SessionWorktreeError::ObjectNotFound);
        }
        let value = std::str::from_utf8(&output)
            .map_err(|_| SessionWorktreeError::InvalidObjectId)?
            .trim();
        Ok(GitObjectId::parse(value.to_owned())?)
    }

    fn run_status<I>(&self, context: &Path, args: I) -> Result<ExitStatus, SessionWorktreeError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut command_args = git_mutation_base_args(&self.disabled_hooks_root);
        command_args.extend(args);
        let spec = HardenedCommandSpec {
            executable: self.git_executable.clone(),
            cwd: context.to_path_buf(),
            args: command_args,
            env: git_mutation_environment(),
            output_limit: CAPTURE_LIMIT_BYTES,
        };
        Ok(self.runner.run(&spec).map_err(map_runner_error)?.status)
    }

    fn run_capture<I>(
        &self,
        context: &Path,
        args: I,
        output_limit: u64,
    ) -> Result<(ExitStatus, Vec<u8>), SessionWorktreeError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut command_args = git_mutation_base_args(&self.disabled_hooks_root);
        command_args.extend(args);
        let spec = HardenedCommandSpec {
            executable: self.git_executable.clone(),
            cwd: context.to_path_buf(),
            args: command_args,
            env: git_mutation_environment(),
            output_limit,
        };
        let output = self.runner.run(&spec).map_err(map_runner_error)?;
        Ok((output.status, output.stdout))
    }
}

fn validate_direct_owned_directory(
    root: &Path,
    path: &Path,
) -> Result<PathBuf, SessionWorktreeError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => SessionWorktreeError::RecoveryOwnedWorktreeMissing,
        _ => SessionWorktreeError::Io(error),
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(SessionWorktreeError::RecoveryOwnedPathUnsafe);
    }
    let canonical = fs::canonicalize(path)?;
    if !path_is_lexically_within(root, &canonical)
        || canonical
            .parent()
            .is_none_or(|parent| !paths_lexically_equal(parent, root))
    {
        return Err(SessionWorktreeError::RecoveryOwnedPathUnsafe);
    }
    Ok(canonical)
}

fn map_runner_error(error: HardenedCommandError) -> SessionWorktreeError {
    match error {
        HardenedCommandError::TimedOut => SessionWorktreeError::CommandTimedOut,
        HardenedCommandError::OutputLimitExceeded { .. } => {
            SessionWorktreeError::CommandOutputTooLarge
        }
        HardenedCommandError::MissingChildPipe => SessionWorktreeError::MissingChildPipe,
        HardenedCommandError::ReaderThreadPanicked | HardenedCommandError::WriterThreadPanicked => {
            SessionWorktreeError::ReaderThreadPanicked
        }
        HardenedCommandError::Io(error) => SessionWorktreeError::Io(error),
        HardenedCommandError::ExecutableMustBeAbsolute
        | HardenedCommandError::WorkingDirectoryMustBeAbsolute
        | HardenedCommandError::ZeroOutputLimit
        | HardenedCommandError::ZeroTimeout
        | HardenedCommandError::InputLimitExceeded { .. } => SessionWorktreeError::InvalidLimits,
    }
}

#[derive(Debug, Error)]
pub enum SessionWorktreeError {
    #[error("session worktree hard limits are invalid")]
    InvalidLimits,
    #[error("Git read boundary validation failed: {0}")]
    ReadBoundary(#[from] GitReadError),
    #[error("session worktree root path must be absolute")]
    WorktreeRootMustBeAbsolute,
    #[error("canonical session worktree root is not a directory")]
    WorktreeRootNotDirectory,
    #[error("session worktree root must not overlap the configured repository")]
    WorktreeRootOverlapsRepository,
    #[error("the disabled-hooks directory must be empty")]
    DisabledHooksDirectoryNotEmpty,
    #[error("session worktree registry is poisoned")]
    RegistryPoisoned,
    #[error("this session already owns a worktree")]
    SessionAlreadyProvisioned,
    #[error("session worktree capacity is exhausted")]
    CapacityExceeded,
    #[error("session does not own a worktree")]
    UnknownSessionWorktree,
    #[error("session worktree path ownership does not match its session")]
    OwnershipMismatch,
    #[error("session worktree path already exists before provisioning")]
    WorktreePathAlreadyExists,
    #[error("Git object does not resolve to a commit")]
    ObjectNotFound,
    #[error("Git returned an invalid object id")]
    InvalidObjectId,
    #[error("session worktree base HEAD does not match the requested exact commit")]
    BaseHeadMismatch,
    #[error("failed to create the session worktree")]
    WorktreeCreateFailed,
    #[error("session worktree is not registered with Git")]
    WorktreeNotRegistered,
    #[error("session worktree is not locked")]
    WorktreeNotLocked,
    #[error("failed to remove the session worktree")]
    WorktreeCleanupFailed,
    #[error("failed to clean a partially provisioned session worktree")]
    ProvisionCleanupFailed,
    #[error("stale recovery cannot run while live session worktrees are registered")]
    RecoveryWhileActive,
    #[error("Git recovery worktree listing is malformed")]
    RecoveryWorktreeListMalformed,
    #[error("Git recovery entry count exceeded its hard ceiling")]
    RecoveryEntryLimitExceeded,
    #[error("stale session worktree count exceeded the session ceiling")]
    RecoveryOwnedLimitExceeded,
    #[error("registered session worktree path is malformed")]
    RecoveryOwnedPathMalformed,
    #[error("registered session worktree is missing from disk")]
    RecoveryOwnedWorktreeMissing,
    #[error("registered session worktree path is unsafe")]
    RecoveryOwnedPathUnsafe,
    #[error("session worktree root contains an unexpected entry")]
    RecoveryUnexpectedEntry,
    #[error("session worktree exists on disk but is not registered with Git")]
    RecoveryUnregisteredOwnedPath,
    #[error(transparent)]
    TreeManifest(#[from] GitTreeManifestError),
    #[error(transparent)]
    Conflict(#[from] SessionConflictError),
    #[error(transparent)]
    BlobBatch(#[from] GitBlobBatchError),
    #[error("Git blob batch must request at least one manifest path")]
    EmptyBlobBatch,
    #[error("Git blob batch entry count exceeded its hard ceiling")]
    BlobBatchEntryLimitExceeded,
    #[error("Git blob batch contains duplicate requested paths")]
    DuplicateBlobBatchPath,
    #[error("Git blob batch path is not present in the current session manifest")]
    UnknownBlobBatchPath,
    #[error("Git blob batch cannot materialize symlink or gitlink entries")]
    UnsupportedBlobBatchEntry,
    #[error("Git blob exceeds the single-file hard ceiling")]
    BlobTooLarge,
    #[error("Git blob batch request exceeded its input byte ceiling")]
    BlobBatchInputLimitExceeded,
    #[error("Git blob batch response would exceed the per-session RAM ceiling")]
    BlobBatchOutputLimitExceeded,
    #[error("Git blob batch wrote unexpected stderr")]
    BlobBatchUnexpectedStderr,
    #[error("Git command failed")]
    GitCommandFailed,
    #[error("Git command timed out")]
    CommandTimedOut,
    #[error("Git command exceeded its bounded output limit")]
    CommandOutputTooLarge,
    #[error("Git command did not expose the expected child pipe")]
    MissingChildPipe,
    #[error("Git command reader thread panicked")]
    ReaderThreadPanicked,
    #[error("Git command I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("Git object id is invalid: {0}")]
    GitObjectId(#[from] GitObjectIdError),
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        ffi::OsStr,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    struct RepoFixture {
        base: PathBuf,
        repo: PathBuf,
        worktrees: PathBuf,
        git: PathBuf,
        head: GitObjectId,
    }

    impl RepoFixture {
        fn new(git: PathBuf, label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let base = env::temp_dir().join(format!("optic-session-worktree-{label}-{nonce}"));
            let repo = base.join("repo");
            let worktrees = base.join("worktrees");
            fs::create_dir_all(&repo).expect("repo");
            fs::create_dir_all(&worktrees).expect("worktrees");
            run_git(
                &git,
                Some(&repo),
                [OsStr::new("init"), OsStr::new("--quiet")],
            );
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("config"),
                    OsStr::new("user.email"),
                    OsStr::new("optic@example.invalid"),
                ],
            );
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("config"),
                    OsStr::new("user.name"),
                    OsStr::new("Optic Test"),
                ],
            );
            fs::write(repo.join("tracked.txt"), b"base\n").expect("fixture file");
            run_git(&git, Some(&repo), [OsStr::new("add"), OsStr::new(".")]);
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("commit"),
                    OsStr::new("--quiet"),
                    OsStr::new("-m"),
                    OsStr::new("base"),
                ],
            );
            let bytes = Command::new(&git)
                .arg("-C")
                .arg(&repo)
                .args(["rev-parse", "HEAD"])
                .output()
                .expect("head");
            assert!(bytes.status.success());
            let head = GitObjectId::parse(
                std::str::from_utf8(&bytes.stdout)
                    .expect("utf8")
                    .trim()
                    .to_owned(),
            )
            .expect("oid");
            Self {
                base,
                repo,
                worktrees,
                git,
                head,
            }
        }

        fn manager(&self, limits: HardLimits) -> SessionWorktreeManager {
            SessionWorktreeManager::from_hard_limits(&self.repo, &self.git, &self.worktrees, limits)
                .expect("manager")
        }
    }

    impl Drop for RepoFixture {
        fn drop(&mut self) {
            let _ = Command::new(&self.git)
                .arg("-C")
                .arg(&self.repo)
                .args(["worktree", "prune", "--expire", "now"])
                .status();
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn find_git_executable() -> Option<PathBuf> {
        let path = env::var_os("PATH")?;
        for directory in env::split_paths(&path) {
            #[cfg(windows)]
            let candidate = directory.join("git.exe");
            #[cfg(not(windows))]
            let candidate = directory.join("git");
            if candidate.is_file() {
                return fs::canonicalize(candidate).ok();
            }
        }
        None
    }

    fn run_git<const N: usize>(git: &Path, repo: Option<&Path>, args: [&OsStr; N]) {
        let mut command = Command::new(git);
        if let Some(repo) = repo {
            command.arg("-C").arg(repo);
        }
        let status = command
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("fixture git");
        assert!(status.success());
    }

    #[test]
    fn session_worktrees_are_exact_head_owner_scoped_and_bounded() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "owner");
        let limits = HardLimits {
            max_sessions: 1,
            ..HardLimits::default()
        };
        let manager = fixture.manager(limits);
        let a = SessionHandle::generate().expect("session a");
        let b = SessionHandle::generate().expect("session b");

        let worktree = manager.provision(&a, &fixture.head).expect("provision a");
        assert_eq!(worktree.owner, a);
        assert_eq!(worktree.base_head, fixture.head);
        assert_eq!(worktree.current_head, fixture.head);
        assert!(worktree.path.is_dir());
        assert!(
            !worktree.path.join("tracked.txt").exists(),
            "Phase 4A must not materialize repository-controlled checkout content"
        );
        let manifest = manager.manifest(&a).expect("session manifest");
        assert_eq!(manifest.head, fixture.head);
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].path.as_str(), "tracked.txt");
        assert_eq!(
            manifest.entries[0].kind,
            crate::GitTreeEntryKind::RegularFile
        );
        assert_eq!(manifest.entries[0].blob_size, Some(5));
        assert_eq!(manifest.total_blob_bytes, 5);
        let blob = manager
            .read_blob_batch(
                &a,
                &[optic_bridge_core::WorkspacePath::parse("tracked.txt").expect("blob path")],
            )
            .expect("blob batch");
        assert_eq!(blob.blobs.len(), 1);
        assert_eq!(
            blob.blobs[0].bytes,
            b"base
"
        );
        assert!(!blob.blobs[0].executable);
        assert_eq!(
            worktree.path.file_name().and_then(|v| v.to_str()),
            Some(a.to_token().as_str())
        );
        assert!(matches!(
            manager.get(&b),
            Err(SessionWorktreeError::UnknownSessionWorktree)
        ));
        assert!(matches!(
            manager.provision(&b, &fixture.head),
            Err(SessionWorktreeError::CapacityExceeded)
        ));

        fs::write(worktree.path.join("tracked.txt"), b"session-a\n").expect("session edit");
        assert_eq!(
            fs::read(fixture.repo.join("tracked.txt")).expect("main file"),
            b"base\n"
        );

        manager.release(&a).expect("release a");
        assert!(!worktree.path.exists());
        let second = manager
            .provision(&b, &fixture.head)
            .expect("capacity reused");
        manager.release(&b).expect("release b");
        assert!(!second.path.exists());
    }

    #[test]
    fn stale_locked_session_worktrees_are_recovered_after_restart() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "recovery");
        let owner = SessionHandle::generate().expect("session");
        let path = {
            let manager = fixture.manager(HardLimits::default());
            manager
                .provision(&owner, &fixture.head)
                .expect("provision")
                .path
        };
        assert!(path.exists());

        let restarted = fixture.manager(HardLimits::default());
        let report = restarted.recover_stale().expect("recovery");
        assert_eq!(report.removed_worktrees, 1);
        assert!(!path.exists());
    }

    #[test]
    fn recovery_fails_closed_before_cleanup_when_root_contains_foreign_entry() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "foreign");
        let owner = SessionHandle::generate().expect("session");
        let path = {
            let manager = fixture.manager(HardLimits::default());
            manager
                .provision(&owner, &fixture.head)
                .expect("provision")
                .path
        };
        fs::write(fixture.worktrees.join("foreign.txt"), b"foreign").expect("foreign entry");

        let restarted = fixture.manager(HardLimits::default());
        assert!(matches!(
            restarted.recover_stale(),
            Err(SessionWorktreeError::RecoveryUnexpectedEntry)
        ));
        assert!(
            path.exists(),
            "fail-closed recovery must not partially clean owned worktrees"
        );
    }
}
