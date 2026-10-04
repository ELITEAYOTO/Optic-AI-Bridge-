use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use optic_bridge_core::{ActionId, GitObjectId, GitObjectIdError, HardLimits, IdError};
use thiserror::Error;

use crate::{GitReadError, GitReadService};

const INTEGRATION_REF_PREFIX: &str = "refs/optic/integration/";
const CAPTURE_LIMIT_BYTES: u64 = 4 * 1024;
const PIPE_READ_CHUNK_BYTES: usize = 8 * 1024;
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(5);
const DISABLED_HOOKS_DIRECTORY: &str = "hooks-disabled";

#[cfg(windows)]
const NULL_CONFIG_PATH: &str = "NUL";
#[cfg(not(windows))]
const NULL_CONFIG_PATH: &str = "/dev/null";

#[derive(Debug)]
pub struct GitIntegrationService {
    repository_root: PathBuf,
    git_executable: PathBuf,
    integration_root: PathBuf,
    disabled_hooks_root: PathBuf,
    target_ref: String,
    command_timeout: Duration,
    recovery_output_limit: u64,
    recovery_entry_limit: u32,
    recovery_owned_limit: u32,
}

impl GitIntegrationService {
    pub fn from_hard_limits(
        repository_root: impl AsRef<Path>,
        git_executable: impl AsRef<Path>,
        integration_root: impl AsRef<Path>,
        target_ref: impl Into<String>,
        limits: HardLimits,
    ) -> Result<Self, GitIntegrationError> {
        let limits = limits
            .validate_nonzero()
            .map_err(|_| GitIntegrationError::InvalidLimits)?;
        let read_service =
            GitReadService::from_hard_limits(repository_root, git_executable, limits)?;

        if !integration_root.as_ref().is_absolute() {
            return Err(GitIntegrationError::IntegrationRootMustBeAbsolute);
        }
        fs::create_dir_all(integration_root.as_ref())?;
        let integration_root = fs::canonicalize(integration_root.as_ref())?;
        if !fs::metadata(&integration_root)?.is_dir() {
            return Err(GitIntegrationError::IntegrationRootNotDirectory);
        }

        let repository_root = read_service.repository_root().to_path_buf();
        if integration_root.starts_with(&repository_root)
            || repository_root.starts_with(&integration_root)
        {
            return Err(GitIntegrationError::IntegrationRootOverlapsRepository);
        }

        let disabled_hooks_root = integration_root.join(DISABLED_HOOKS_DIRECTORY);
        fs::create_dir_all(&disabled_hooks_root)?;
        if fs::read_dir(&disabled_hooks_root)?.next().is_some() {
            return Err(GitIntegrationError::DisabledHooksDirectoryNotEmpty);
        }

        let service = Self {
            repository_root,
            git_executable: read_service.git_executable().to_path_buf(),
            integration_root,
            disabled_hooks_root,
            target_ref: target_ref.into(),
            command_timeout: Duration::from_millis(limits.max_request_duration_ms),
            recovery_output_limit: limits.max_git_read_bytes,
            recovery_entry_limit: limits.max_fs_directory_scan_entries,
            recovery_owned_limit: limits.max_concurrent_requests,
        };
        service.validate_target_ref()?;
        service.ensure_target_ref_is_direct()?;
        service.target_head()?;
        Ok(service)
    }

    #[must_use]
    pub fn repository_root(&self) -> &Path {
        &self.repository_root
    }

    #[must_use]
    pub fn integration_root(&self) -> &Path {
        &self.integration_root
    }

    #[must_use]
    pub fn target_ref(&self) -> &str {
        &self.target_ref
    }

    pub fn target_head(&self) -> Result<GitObjectId, GitIntegrationError> {
        self.resolve_commit(&self.repository_root, &self.target_ref)
            .map_err(|error| match error {
                GitIntegrationError::ObjectNotFound => GitIntegrationError::TargetRefMissing,
                other => other,
            })
    }

    /// Performs a deliberately narrow Phase 2D3 fast-forward integration.
    ///
    /// The operator-owned target ref must live under `refs/optic/integration/`.
    /// The caller workspace is never checked out, reset, or updated. An Optic-owned
    /// detached/locked worktree is created with `--no-checkout`, validated, fully
    /// cleaned up, and only then is the internal target ref advanced with Git's
    /// compare-and-swap `update-ref <ref> <new> <expected>` form.
    pub fn integrate_fast_forward(
        &self,
        action_id: &ActionId,
        source_head: &GitObjectId,
        expected_target_head: &GitObjectId,
    ) -> Result<GitIntegrationResult, GitIntegrationError> {
        self.ensure_expected_target(expected_target_head)?;
        self.resolve_commit(&self.repository_root, source_head.as_str())?;
        self.require_fast_forward(expected_target_head, source_head)?;

        let worktree_path = self.integration_root.join(action_id.to_token());
        if worktree_path.exists() {
            return Err(GitIntegrationError::OperationPathAlreadyExists);
        }

        self.create_locked_worktree(&worktree_path, expected_target_head)?;
        let validation = self.validate_owned_worktree(&worktree_path, expected_target_head);
        let cleanup = self.remove_owned_worktree(&worktree_path);
        cleanup?;
        validation?;

        // Revalidate after all preparatory work and immediately before the only
        // target-ref mutation. update-ref repeats the exact-head precondition
        // atomically, so a concurrent target move cannot be overwritten.
        self.ensure_expected_target(expected_target_head)?;
        self.ensure_target_ref_is_direct()?;
        self.ensure_disabled_hooks_empty()?;
        let status = self.run_status(
            &self.repository_root,
            [
                OsString::from("update-ref"),
                OsString::from("--no-deref"),
                OsString::from(&self.target_ref),
                OsString::from(source_head.as_str()),
                OsString::from(expected_target_head.as_str()),
            ],
        )?;
        if !status.success() {
            let observed = self.target_head()?;
            if &observed != expected_target_head {
                return Err(GitIntegrationError::StaleTarget {
                    expected: expected_target_head.clone(),
                    observed,
                });
            }
            return Err(GitIntegrationError::GitCommandFailed);
        }

        let observed = self.target_head()?;
        if &observed != source_head {
            return Err(GitIntegrationError::PostUpdateVerificationFailed);
        }

        Ok(GitIntegrationResult {
            action_id: action_id.clone(),
            previous_target_head: expected_target_head.clone(),
            new_target_head: source_head.clone(),
            mode: GitIntegrationMode::FastForward,
        })
    }

    /// Removes only stale worktrees that can be proven to be Optic-owned.
    ///
    /// Recovery is deliberately conservative: all registered and on-disk state is
    /// validated first, and cleanup begins only after the complete snapshot is
    /// coherent. User worktrees outside `integration_root` are ignored. A missing,
    /// unlocked, unregistered, escaped, malformed, or otherwise ambiguous owned
    /// entry fails closed and no cleanup is attempted.
    pub fn recover_owned_worktrees(
        &self,
    ) -> Result<GitIntegrationRecoveryReport, GitIntegrationError> {
        self.ensure_disabled_hooks_empty()?;
        let registered = self.registered_worktrees()?;
        let owned = self.validate_owned_recovery_snapshot(&registered)?;

        for (token, worktree) in &owned {
            self.revalidate_owned_recovery_path(token, &worktree.canonical_path)?;
            self.remove_owned_worktree(&worktree.canonical_path)?;
        }

        Ok(GitIntegrationRecoveryReport {
            removed_worktrees: u32::try_from(owned.len())
                .map_err(|_| GitIntegrationError::RecoveryEntryLimitExceeded)?,
        })
    }

    fn registered_worktrees(&self) -> Result<Vec<RegisteredWorktree>, GitIntegrationError> {
        let (status, bytes) = self.run_capture_bounded(
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
            return Err(GitIntegrationError::GitCommandFailed);
        }
        parse_worktree_list(&bytes, self.recovery_entry_limit)
    }

    fn validate_owned_recovery_snapshot(
        &self,
        registered: &[RegisteredWorktree],
    ) -> Result<BTreeMap<String, OwnedRecoveryWorktree>, GitIntegrationError> {
        let mut registered_owned = BTreeMap::new();
        for worktree in registered {
            if !worktree.path.is_absolute() {
                return Err(GitIntegrationError::RecoveryWorktreeListMalformed);
            }
            if !path_is_lexically_within(&self.integration_root, &worktree.path) {
                continue;
            }

            let parent = worktree
                .path
                .parent()
                .ok_or(GitIntegrationError::RecoveryOwnedPathMalformed)?;
            if !paths_lexically_equal(parent, &self.integration_root) {
                return Err(GitIntegrationError::RecoveryOwnedPathMalformed);
            }
            let token = worktree
                .path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(GitIntegrationError::RecoveryOwnedPathMalformed)?;
            let action = ActionId::from_token(token)
                .map_err(|_| GitIntegrationError::RecoveryOwnedPathMalformed)?;
            if action.to_token() != token {
                return Err(GitIntegrationError::RecoveryOwnedPathMalformed);
            }
            if !worktree.locked {
                return Err(GitIntegrationError::RecoveryOwnedWorktreeUnlocked);
            }
            if registered_owned.contains_key(token) {
                return Err(GitIntegrationError::RecoveryWorktreeListMalformed);
            }
            if registered_owned.len()
                >= usize::try_from(self.recovery_owned_limit).unwrap_or(usize::MAX)
            {
                return Err(GitIntegrationError::RecoveryOwnedLimitExceeded);
            }

            let metadata =
                fs::symlink_metadata(&worktree.path).map_err(|error| match error.kind() {
                    io::ErrorKind::NotFound => GitIntegrationError::RecoveryOwnedWorktreeMissing,
                    _ => GitIntegrationError::Io(error),
                })?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(GitIntegrationError::RecoveryOwnedPathUnsafe);
            }
            let canonical = fs::canonicalize(&worktree.path)?;
            if !canonical.starts_with(&self.integration_root)
                || canonical.parent() != Some(self.integration_root.as_path())
            {
                return Err(GitIntegrationError::RecoveryOwnedPathUnsafe);
            }

            registered_owned.insert(
                token.to_owned(),
                OwnedRecoveryWorktree {
                    canonical_path: canonical,
                },
            );
        }

        let mut seen_owned = BTreeSet::new();
        let mut scanned = 0_u32;
        for entry in fs::read_dir(&self.integration_root)? {
            scanned = scanned
                .checked_add(1)
                .ok_or(GitIntegrationError::RecoveryEntryLimitExceeded)?;
            if scanned > self.recovery_entry_limit {
                return Err(GitIntegrationError::RecoveryEntryLimitExceeded);
            }
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| GitIntegrationError::RecoveryUnexpectedEntry)?;
            if name == DISABLED_HOOKS_DIRECTORY {
                if entry.path() != self.disabled_hooks_root {
                    return Err(GitIntegrationError::RecoveryUnexpectedEntry);
                }
                continue;
            }

            let action = ActionId::from_token(&name)
                .map_err(|_| GitIntegrationError::RecoveryUnexpectedEntry)?;
            if action.to_token() != name {
                return Err(GitIntegrationError::RecoveryUnexpectedEntry);
            }
            let Some(owned) = registered_owned.get(&name) else {
                return Err(GitIntegrationError::RecoveryUnregisteredOwnedPath);
            };
            let metadata = fs::symlink_metadata(entry.path())?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(GitIntegrationError::RecoveryOwnedPathUnsafe);
            }
            let canonical = fs::canonicalize(entry.path())?;
            if canonical != owned.canonical_path {
                return Err(GitIntegrationError::RecoveryOwnedPathUnsafe);
            }
            seen_owned.insert(name);
        }

        if seen_owned.len() != registered_owned.len() {
            return Err(GitIntegrationError::RecoveryOwnedWorktreeMissing);
        }
        Ok(registered_owned)
    }

    fn revalidate_owned_recovery_path(
        &self,
        token: &str,
        expected_canonical: &Path,
    ) -> Result<(), GitIntegrationError> {
        let action = ActionId::from_token(token)
            .map_err(|_| GitIntegrationError::RecoveryOwnedPathMalformed)?;
        if action.to_token() != token {
            return Err(GitIntegrationError::RecoveryOwnedPathMalformed);
        }
        let path = self.integration_root.join(token);
        let metadata = fs::symlink_metadata(&path).map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => GitIntegrationError::RecoveryOwnedWorktreeMissing,
            _ => GitIntegrationError::Io(error),
        })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(GitIntegrationError::RecoveryOwnedPathUnsafe);
        }
        let canonical = fs::canonicalize(path)?;
        if canonical != expected_canonical
            || !canonical.starts_with(&self.integration_root)
            || canonical.parent() != Some(self.integration_root.as_path())
            || canonical.file_name().and_then(|value| value.to_str()) != Some(token)
        {
            return Err(GitIntegrationError::RecoveryOwnedPathUnsafe);
        }
        Ok(())
    }

    fn validate_target_ref(&self) -> Result<(), GitIntegrationError> {
        if !self.target_ref.starts_with(INTEGRATION_REF_PREFIX)
            || self.target_ref.len() == INTEGRATION_REF_PREFIX.len()
        {
            return Err(GitIntegrationError::TargetRefOutsideOpticNamespace);
        }
        let status = self.run_status(
            &self.repository_root,
            [
                OsString::from("check-ref-format"),
                OsString::from(&self.target_ref),
            ],
        )?;
        if !status.success() {
            return Err(GitIntegrationError::InvalidTargetRef);
        }
        Ok(())
    }

    fn ensure_expected_target(&self, expected: &GitObjectId) -> Result<(), GitIntegrationError> {
        let observed = self.target_head()?;
        if &observed != expected {
            return Err(GitIntegrationError::StaleTarget {
                expected: expected.clone(),
                observed,
            });
        }
        Ok(())
    }

    fn ensure_target_ref_is_direct(&self) -> Result<(), GitIntegrationError> {
        let status = self.run_status(
            &self.repository_root,
            [
                OsString::from("symbolic-ref"),
                OsString::from("-q"),
                OsString::from(&self.target_ref),
            ],
        )?;
        match status.code() {
            Some(1) => Ok(()),
            Some(0) => Err(GitIntegrationError::SymbolicTargetRef),
            _ => Err(GitIntegrationError::GitCommandFailed),
        }
    }

    fn ensure_disabled_hooks_empty(&self) -> Result<(), GitIntegrationError> {
        if fs::read_dir(&self.disabled_hooks_root)?.next().is_some() {
            return Err(GitIntegrationError::DisabledHooksDirectoryNotEmpty);
        }
        Ok(())
    }

    fn require_fast_forward(
        &self,
        expected: &GitObjectId,
        source: &GitObjectId,
    ) -> Result<(), GitIntegrationError> {
        let status = self.run_status(
            &self.repository_root,
            [
                OsString::from("merge-base"),
                OsString::from("--is-ancestor"),
                OsString::from(expected.as_str()),
                OsString::from(source.as_str()),
            ],
        )?;
        match status.code() {
            Some(0) => Ok(()),
            Some(1) => Err(GitIntegrationError::NonFastForward),
            _ => Err(GitIntegrationError::GitCommandFailed),
        }
    }

    fn create_locked_worktree(
        &self,
        worktree_path: &Path,
        expected: &GitObjectId,
    ) -> Result<(), GitIntegrationError> {
        self.ensure_disabled_hooks_empty()?;
        let status = self.run_status(
            &self.repository_root,
            [
                OsString::from("worktree"),
                OsString::from("add"),
                OsString::from("--detach"),
                OsString::from("--no-checkout"),
                OsString::from("--lock"),
                git_path_arg(worktree_path),
                OsString::from(expected.as_str()),
            ],
        )?;
        if status.success() {
            Ok(())
        } else {
            Err(GitIntegrationError::WorktreeCreateFailed)
        }
    }

    fn validate_owned_worktree(
        &self,
        worktree_path: &Path,
        expected: &GitObjectId,
    ) -> Result<(), GitIntegrationError> {
        let canonical = fs::canonicalize(worktree_path)?;
        if !canonical.starts_with(&self.integration_root) {
            return Err(GitIntegrationError::WorktreeEscapedIntegrationRoot);
        }
        let observed = self.resolve_commit(&canonical, "HEAD")?;
        if &observed != expected {
            return Err(GitIntegrationError::WorktreeHeadMismatch);
        }
        Ok(())
    }

    fn remove_owned_worktree(&self, worktree_path: &Path) -> Result<(), GitIntegrationError> {
        // Keep the ownership lock in place until Git removes the worktree. Git
        // requires --force twice for a locked worktree, which avoids an unlock /
        // remove race window.
        let remove = self.run_status(
            &self.repository_root,
            [
                OsString::from("worktree"),
                OsString::from("remove"),
                OsString::from("--force"),
                OsString::from("--force"),
                git_path_arg(worktree_path),
            ],
        )?;
        if !remove.success() || worktree_path.exists() {
            return Err(GitIntegrationError::WorktreeCleanupFailed);
        }
        Ok(())
    }

    fn resolve_commit(
        &self,
        context: &Path,
        object: &str,
    ) -> Result<GitObjectId, GitIntegrationError> {
        let commitish = format!("{object}^{{commit}}");
        let (status, output) = self.run_capture_small(
            context,
            [
                OsString::from("rev-parse"),
                OsString::from("--verify"),
                OsString::from(commitish),
            ],
        )?;
        if !status.success() {
            return Err(GitIntegrationError::ObjectNotFound);
        }
        let value = std::str::from_utf8(&output)
            .map_err(|_| GitIntegrationError::InvalidObjectId)?
            .trim();
        Ok(GitObjectId::parse(value.to_owned())?)
    }

    fn base_command(&self, context: &Path) -> Command {
        let mut command = Command::new(&self.git_executable);
        command
            .arg("--no-pager")
            .arg("--literal-pathspecs")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .arg("-c")
            .arg("core.untrackedCache=false")
            .arg("-c")
            .arg(format!(
                "core.hooksPath={}",
                git_path_arg(&self.disabled_hooks_root).to_string_lossy()
            ))
            .arg("-c")
            .arg("commit.gpgSign=false")
            .arg("-C")
            .arg(git_path_arg(context))
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_PAGER", "cat")
            .env("PAGER", "cat")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", NULL_CONFIG_PATH)
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .stdin(Stdio::null());
        command
    }

    fn run_status<I>(&self, context: &Path, args: I) -> Result<ExitStatus, GitIntegrationError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut command = self.base_command(context);
        command
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = command.spawn()?;
        let deadline = Instant::now() + self.command_timeout;
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitIntegrationError::CommandTimedOut);
            }
            thread::sleep(CHILD_POLL_INTERVAL);
        }
    }

    fn run_capture_small<I>(
        &self,
        context: &Path,
        args: I,
    ) -> Result<(ExitStatus, Vec<u8>), GitIntegrationError>
    where
        I: IntoIterator<Item = OsString>,
    {
        self.run_capture_bounded(context, args, CAPTURE_LIMIT_BYTES)
    }

    fn run_capture_bounded<I>(
        &self,
        context: &Path,
        args: I,
        output_limit: u64,
    ) -> Result<(ExitStatus, Vec<u8>), GitIntegrationError>
    where
        I: IntoIterator<Item = OsString>,
    {
        if output_limit == 0 {
            return Err(GitIntegrationError::CommandOutputTooLarge);
        }
        let mut command = self.base_command(context);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or(GitIntegrationError::MissingChildPipe)?;
        let stderr = child
            .stderr
            .take()
            .ok_or(GitIntegrationError::MissingChildPipe)?;
        let total = Arc::new(AtomicU64::new(0));
        let exceeded = Arc::new(AtomicBool::new(false));
        let stdout_reader = spawn_bounded_reader(
            stdout,
            Arc::clone(&total),
            Arc::clone(&exceeded),
            output_limit,
        );
        let stderr_reader =
            spawn_bounded_reader(stderr, total, Arc::clone(&exceeded), output_limit);

        let started = Instant::now();
        let mut timed_out = false;
        let status = loop {
            if exceeded.load(Ordering::Acquire) {
                let _ = child.kill();
                break child.wait()?;
            }
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() >= self.command_timeout {
                timed_out = true;
                let _ = child.kill();
                break child.wait()?;
            }
            thread::sleep(CHILD_POLL_INTERVAL);
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| GitIntegrationError::ReaderThreadPanicked)??;
        let _stderr = stderr_reader
            .join()
            .map_err(|_| GitIntegrationError::ReaderThreadPanicked)??;
        if timed_out {
            return Err(GitIntegrationError::CommandTimedOut);
        }
        if exceeded.load(Ordering::Acquire) {
            return Err(GitIntegrationError::CommandOutputTooLarge);
        }
        Ok((status, stdout))
    }
}

fn spawn_bounded_reader<R>(
    mut reader: R,
    total: Arc<AtomicU64>,
    exceeded: Arc<AtomicBool>,
    limit: u64,
) -> JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; PIPE_READ_CHUNK_BYTES];
        loop {
            let read = reader.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            let read_u64 = u64::try_from(read).unwrap_or(u64::MAX);
            let previous = total.fetch_add(read_u64, Ordering::AcqRel);
            if previous.saturating_add(read_u64) > limit {
                exceeded.store(true, Ordering::Release);
                break;
            }
            bytes.extend_from_slice(&chunk[..read]);
        }
        Ok(bytes)
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RegisteredWorktree {
    path: PathBuf,
    locked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OwnedRecoveryWorktree {
    canonical_path: PathBuf,
}

fn parse_worktree_list(
    bytes: &[u8],
    entry_limit: u32,
) -> Result<Vec<RegisteredWorktree>, GitIntegrationError> {
    if bytes.is_empty() || !bytes.ends_with(b"\0\0") {
        return Err(GitIntegrationError::RecoveryWorktreeListMalformed);
    }

    let mut records = Vec::new();
    let mut fields: Vec<&[u8]> = Vec::new();
    for field in bytes.split(|byte| *byte == 0) {
        if field.is_empty() {
            if fields.is_empty() {
                continue;
            }
            if records.len() >= usize::try_from(entry_limit).unwrap_or(usize::MAX) {
                return Err(GitIntegrationError::RecoveryEntryLimitExceeded);
            }
            records.push(parse_worktree_record(&fields)?);
            fields.clear();
        } else {
            fields.push(field);
        }
    }
    if !fields.is_empty() || records.is_empty() {
        return Err(GitIntegrationError::RecoveryWorktreeListMalformed);
    }
    Ok(records)
}

fn parse_worktree_record(fields: &[&[u8]]) -> Result<RegisteredWorktree, GitIntegrationError> {
    let first = fields
        .first()
        .ok_or(GitIntegrationError::RecoveryWorktreeListMalformed)?;
    let raw_path = first
        .strip_prefix(b"worktree ")
        .ok_or(GitIntegrationError::RecoveryWorktreeListMalformed)?;
    if raw_path.is_empty()
        || fields[1..]
            .iter()
            .any(|field| field.starts_with(b"worktree "))
    {
        return Err(GitIntegrationError::RecoveryWorktreeListMalformed);
    }
    let path = std::str::from_utf8(raw_path)
        .map_err(|_| GitIntegrationError::RecoveryWorktreeListMalformed)?;
    let locked = fields[1..]
        .iter()
        .any(|field| *field == b"locked" || field.starts_with(b"locked "));
    Ok(RegisteredWorktree {
        path: PathBuf::from(path),
        locked,
    })
}

#[cfg(windows)]
fn normalized_windows_path(path: &Path) -> String {
    let mut value = path.as_os_str().to_string_lossy().replace('/', "\\");
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        value = format!(r"\\{rest}");
    } else if let Some(rest) = value.strip_prefix(r"\\?\") {
        value = rest.to_owned();
    }
    while value.len() > 3 && value.ends_with('\\') {
        value.pop();
    }
    value.to_lowercase()
}

#[cfg(windows)]
fn paths_lexically_equal(left: &Path, right: &Path) -> bool {
    normalized_windows_path(left) == normalized_windows_path(right)
}

#[cfg(not(windows))]
fn paths_lexically_equal(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(windows)]
fn path_is_lexically_within(root: &Path, candidate: &Path) -> bool {
    let root = normalized_windows_path(root);
    let candidate = normalized_windows_path(candidate);
    candidate == root
        || candidate
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

#[cfg(not(windows))]
fn path_is_lexically_within(root: &Path, candidate: &Path) -> bool {
    candidate.starts_with(root)
}

fn git_path_arg(path: &Path) -> OsString {
    #[cfg(windows)]
    {
        let value = path.as_os_str().to_string_lossy();
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            return OsString::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = value.strip_prefix(r"\\?\") {
            return OsString::from(rest);
        }
    }
    path.as_os_str().to_os_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitIntegrationMode {
    FastForward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitIntegrationResult {
    pub action_id: ActionId,
    pub previous_target_head: GitObjectId,
    pub new_target_head: GitObjectId,
    pub mode: GitIntegrationMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GitIntegrationRecoveryReport {
    pub removed_worktrees: u32,
}

#[derive(Debug, Error)]
pub enum GitIntegrationError {
    #[error("Git integration hard limits are invalid")]
    InvalidLimits,
    #[error("Git read boundary validation failed: {0}")]
    ReadBoundary(#[from] GitReadError),
    #[error("integration root path must be absolute")]
    IntegrationRootMustBeAbsolute,
    #[error("canonical integration root is not a directory")]
    IntegrationRootNotDirectory,
    #[error("integration root must not overlap the configured repository")]
    IntegrationRootOverlapsRepository,
    #[error("the disabled-hooks directory must be empty")]
    DisabledHooksDirectoryNotEmpty,
    #[error("target ref must be under refs/optic/integration/")]
    TargetRefOutsideOpticNamespace,
    #[error("target ref is not a valid Git ref name")]
    InvalidTargetRef,
    #[error("the operator-owned integration target ref does not exist")]
    TargetRefMissing,
    #[error("the operator-owned integration target ref must be a direct ref, not a symbolic ref")]
    SymbolicTargetRef,
    #[error("Git object does not resolve to a commit")]
    ObjectNotFound,
    #[error("Git returned an invalid object id")]
    InvalidObjectId,
    #[error("target ref moved away from the exact expected head")]
    StaleTarget {
        expected: GitObjectId,
        observed: GitObjectId,
    },
    #[error("source head is not a fast-forward descendant of the expected target")]
    NonFastForward,
    #[error("operation-owned integration path already exists")]
    OperationPathAlreadyExists,
    #[error("failed to create the isolated Git worktree")]
    WorktreeCreateFailed,
    #[error("isolated worktree escaped the configured integration root")]
    WorktreeEscapedIntegrationRoot,
    #[error("isolated worktree HEAD does not match the exact expected target")]
    WorktreeHeadMismatch,
    #[error("isolated worktree cleanup failed; target ref was not advanced")]
    WorktreeCleanupFailed,
    #[error("Git recovery worktree listing is malformed")]
    RecoveryWorktreeListMalformed,
    #[error("Git recovery entry count exceeded its hard ceiling")]
    RecoveryEntryLimitExceeded,
    #[error("Optic-owned recovery worktree count exceeded concurrent-request ceiling")]
    RecoveryOwnedLimitExceeded,
    #[error("registered Optic worktree path is malformed")]
    RecoveryOwnedPathMalformed,
    #[error("registered Optic worktree is missing from disk")]
    RecoveryOwnedWorktreeMissing,
    #[error("registered Optic worktree is not locked")]
    RecoveryOwnedWorktreeUnlocked,
    #[error("registered Optic worktree path is unsafe or escaped")]
    RecoveryOwnedPathUnsafe,
    #[error("integration root contains an unexpected entry")]
    RecoveryUnexpectedEntry,
    #[error("integration root contains an ActionId path that Git does not register")]
    RecoveryUnregisteredOwnedPath,
    #[error("Git child process did not expose the required output pipe")]
    MissingChildPipe,
    #[error("Git bounded-output reader thread panicked")]
    ReaderThreadPanicked,
    #[error("Git command exceeded the hard request-duration ceiling")]
    CommandTimedOut,
    #[error("bounded Git command output exceeded its hard ceiling")]
    CommandOutputTooLarge,
    #[error("Git command failed")]
    GitCommandFailed,
    #[error("target ref update could not be verified")]
    PostUpdateVerificationFailed,
    #[error("operating-system entropy source is unavailable")]
    EntropyUnavailable,
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl From<GitObjectIdError> for GitIntegrationError {
    fn from(_: GitObjectIdError) -> Self {
        Self::InvalidObjectId
    }
}

impl From<IdError> for GitIntegrationError {
    fn from(_: IdError) -> Self {
        Self::EntropyUnavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, ffi::OsStr, process::Command};

    const TARGET_REF: &str = "refs/optic/integration/default";

    struct RepoFixture {
        base: PathBuf,
        repo: PathBuf,
        integration_root: PathBuf,
        git: PathBuf,
        initial: GitObjectId,
        source: GitObjectId,
    }

    impl RepoFixture {
        fn new(git: PathBuf, label: &str) -> Self {
            let id = ActionId::generate().expect("fixture id").to_token();
            let short_id = &id[..12];
            let base = env::temp_dir().join(format!("ogi-{label}-{short_id}"));
            let repo = base.join("repo");
            let integration_root = base.join("integration");
            fs::create_dir_all(&repo).expect("repo dir");
            fs::create_dir_all(&integration_root).expect("integration dir");
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

            fs::write(repo.join("tracked.txt"), b"alpha\n").expect("initial file");
            commit_all(&git, &repo, "initial");
            let initial = head(&git, &repo);
            run_git_owned(&git, &repo, ["update-ref", TARGET_REF, initial.as_str()]);

            fs::write(repo.join("tracked.txt"), b"beta\n").expect("source file");
            commit_all(&git, &repo, "source");
            let source = head(&git, &repo);

            Self {
                base,
                repo,
                integration_root,
                git,
                initial,
                source,
            }
        }

        fn service(&self) -> GitIntegrationService {
            GitIntegrationService::from_hard_limits(
                &self.repo,
                &self.git,
                &self.integration_root,
                TARGET_REF,
                HardLimits::default(),
            )
            .expect("integration service")
        }
    }

    impl Drop for RepoFixture {
        fn drop(&mut self) {
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
            .expect("run fixture git");
        assert!(status.success(), "fixture git command failed");
    }

    fn run_git_owned<const N: usize>(git: &Path, repo: &Path, args: [&str; N]) {
        let status = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("run fixture git");
        assert!(status.success(), "fixture git command failed");
    }

    fn git_output<const N: usize>(git: &Path, repo: &Path, args: [&str; N]) -> Vec<u8> {
        let output = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("fixture git output");
        assert!(output.status.success(), "fixture git output failed");
        output.stdout
    }

    fn commit_all(git: &Path, repo: &Path, message: &str) {
        run_git(git, Some(repo), [OsStr::new("add"), OsStr::new(".")]);
        run_git(
            git,
            Some(repo),
            [
                OsStr::new("commit"),
                OsStr::new("--quiet"),
                OsStr::new("-m"),
                OsStr::new(message),
            ],
        );
    }

    fn head(git: &Path, repo: &Path) -> GitObjectId {
        let bytes = git_output(git, repo, ["rev-parse", "HEAD"]);
        let value = std::str::from_utf8(&bytes).expect("utf8 head").trim();
        GitObjectId::parse(value.to_owned()).expect("head oid")
    }

    fn ref_head(git: &Path, repo: &Path, target_ref: &str) -> GitObjectId {
        let bytes = git_output(git, repo, ["rev-parse", "--verify", target_ref]);
        let value = std::str::from_utf8(&bytes).expect("utf8 ref").trim();
        GitObjectId::parse(value.to_owned()).expect("ref oid")
    }

    #[test]
    fn fast_forward_updates_only_optic_ref_and_cleans_owned_worktree() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "fast-forward");
        let service = fixture.service();
        let action = ActionId::generate().expect("action");
        let workspace_head_before = head(&fixture.git, &fixture.repo);
        let workspace_bytes_before = fs::read(fixture.repo.join("tracked.txt")).expect("tracked");

        let result = service
            .integrate_fast_forward(&action, &fixture.source, &fixture.initial)
            .expect("fast-forward integration");

        assert_eq!(result.action_id, action);
        assert_eq!(result.previous_target_head, fixture.initial);
        assert_eq!(result.new_target_head, fixture.source);
        assert_eq!(result.mode, GitIntegrationMode::FastForward);
        assert_eq!(
            ref_head(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.source
        );
        assert_eq!(head(&fixture.git, &fixture.repo), workspace_head_before);
        assert_eq!(
            fs::read(fixture.repo.join("tracked.txt")).expect("tracked after"),
            workspace_bytes_before
        );
        assert!(!fixture.integration_root.join(action.to_token()).exists());
        let worktrees = git_output(
            &fixture.git,
            &fixture.repo,
            ["worktree", "list", "--porcelain"],
        );
        assert!(!String::from_utf8_lossy(&worktrees).contains(&action.to_token()));
    }

    fn add_locked_owned_worktree(fixture: &RepoFixture, action: &ActionId) -> PathBuf {
        let path = fixture.integration_root.join(action.to_token());
        let status = Command::new(&fixture.git)
            .arg("-C")
            .arg(&fixture.repo)
            .args(["worktree", "add", "--detach", "--no-checkout", "--lock"])
            .arg(&path)
            .arg(fixture.initial.as_str())
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("add owned worktree");
        assert!(status.success(), "owned worktree creation failed");
        path
    }

    #[test]
    fn recovery_removes_only_registered_locked_action_worktrees() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "recovery-success");
        let service = fixture.service();
        let action = ActionId::generate().expect("action");
        let owned = add_locked_owned_worktree(&fixture, &action);
        let user_worktree = fixture.base.join("user-worktree");
        let status = Command::new(&fixture.git)
            .arg("-C")
            .arg(&fixture.repo)
            .args(["worktree", "add", "--detach", "--no-checkout"])
            .arg(&user_worktree)
            .arg(fixture.initial.as_str())
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("add user worktree");
        assert!(status.success());

        let report = service.recover_owned_worktrees().expect("recovery");
        assert_eq!(report.removed_worktrees, 1);
        assert!(!owned.exists());
        assert!(user_worktree.exists());
        assert_eq!(
            ref_head(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.initial
        );
        let worktrees = git_output(
            &fixture.git,
            &fixture.repo,
            ["worktree", "list", "--porcelain", "-z"],
        );
        assert!(!String::from_utf8_lossy(&worktrees).contains(&action.to_token()));
        assert!(String::from_utf8_lossy(&worktrees).contains("user-worktree"));
    }

    #[test]
    fn recovery_rejects_unregistered_action_directory_without_deleting_it() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "recovery-unregistered");
        let service = fixture.service();
        let action = ActionId::generate().expect("action");
        let path = fixture.integration_root.join(action.to_token());
        fs::create_dir(&path).expect("unregistered action dir");

        let error = service
            .recover_owned_worktrees()
            .expect_err("unregistered path must fail closed");
        assert!(matches!(
            error,
            GitIntegrationError::RecoveryUnregisteredOwnedPath
        ));
        assert!(path.exists());
    }

    #[test]
    fn recovery_rejects_unlocked_registered_owned_worktree() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "recovery-unlocked");
        let service = fixture.service();
        let action = ActionId::generate().expect("action");
        let path = fixture.integration_root.join(action.to_token());
        let status = Command::new(&fixture.git)
            .arg("-C")
            .arg(&fixture.repo)
            .args(["worktree", "add", "--detach", "--no-checkout"])
            .arg(&path)
            .arg(fixture.initial.as_str())
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("add unlocked worktree");
        assert!(status.success());

        let error = service
            .recover_owned_worktrees()
            .expect_err("unlocked owned worktree must fail closed");
        assert!(matches!(
            error,
            GitIntegrationError::RecoveryOwnedWorktreeUnlocked
        ));
        assert!(path.exists());
    }

    #[test]
    fn recovery_rejects_missing_registered_owned_worktree_without_pruning() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "recovery-missing");
        let service = fixture.service();
        let action = ActionId::generate().expect("action");
        let path = add_locked_owned_worktree(&fixture, &action);
        fs::remove_dir_all(&path).expect("simulate missing worktree directory");

        let error = service
            .recover_owned_worktrees()
            .expect_err("missing registered worktree must fail closed");
        assert!(matches!(
            error,
            GitIntegrationError::RecoveryOwnedWorktreeMissing
        ));
        let worktrees = git_output(
            &fixture.git,
            &fixture.repo,
            ["worktree", "list", "--porcelain", "-z"],
        );
        assert!(String::from_utf8_lossy(&worktrees).contains(&action.to_token()));
    }

    #[test]
    fn recovery_directory_scan_is_hard_bounded() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "recovery-limit");
        let limits = HardLimits {
            max_fs_directory_scan_entries: 1,
            ..HardLimits::default()
        };
        let service = GitIntegrationService::from_hard_limits(
            &fixture.repo,
            &fixture.git,
            &fixture.integration_root,
            TARGET_REF,
            limits,
        )
        .expect("integration service");
        let action = ActionId::generate().expect("action");
        let path = add_locked_owned_worktree(&fixture, &action);

        let error = service
            .recover_owned_worktrees()
            .expect_err("scan ceiling must fail closed");
        assert!(matches!(
            error,
            GitIntegrationError::RecoveryEntryLimitExceeded
        ));
        assert!(path.exists());
    }

    #[test]
    fn recovery_owned_worktree_count_is_bounded_before_cleanup() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "recovery-owned-limit");
        let limits = HardLimits {
            max_concurrent_requests: 1,
            ..HardLimits::default()
        };
        let service = GitIntegrationService::from_hard_limits(
            &fixture.repo,
            &fixture.git,
            &fixture.integration_root,
            TARGET_REF,
            limits,
        )
        .expect("integration service");
        let first = ActionId::generate().expect("first action");
        let second = ActionId::generate().expect("second action");
        let first_path = add_locked_owned_worktree(&fixture, &first);
        let second_path = add_locked_owned_worktree(&fixture, &second);

        let error = service
            .recover_owned_worktrees()
            .expect_err("owned recovery ceiling must fail closed");
        assert!(matches!(
            error,
            GitIntegrationError::RecoveryOwnedLimitExceeded
        ));
        assert!(first_path.exists());
        assert!(second_path.exists());
    }

    #[test]
    fn worktree_porcelain_parser_requires_bounded_complete_records() {
        let raw = b"worktree /repo\0HEAD 1111111111111111111111111111111111111111\0\0worktree /owned\0HEAD 2222222222222222222222222222222222222222\0detached\0locked optic\0\0";
        let parsed = parse_worktree_list(raw, 2).expect("parse");
        assert_eq!(parsed.len(), 2);
        assert!(!parsed[0].locked);
        assert!(parsed[1].locked);
        assert!(matches!(
            parse_worktree_list(raw, 1),
            Err(GitIntegrationError::RecoveryEntryLimitExceeded)
        ));
        assert!(matches!(
            parse_worktree_list(&raw[..raw.len() - 1], 2),
            Err(GitIntegrationError::RecoveryWorktreeListMalformed)
        ));
    }

    #[test]
    fn stale_target_is_rejected_before_integration() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "stale");
        let service = fixture.service();
        run_git_owned(
            &fixture.git,
            &fixture.repo,
            ["update-ref", TARGET_REF, fixture.source.as_str()],
        );
        let action = ActionId::generate().expect("action");

        let error = service
            .integrate_fast_forward(&action, &fixture.source, &fixture.initial)
            .expect_err("stale target must fail");
        assert!(matches!(error, GitIntegrationError::StaleTarget { .. }));
        assert!(!fixture.integration_root.join(action.to_token()).exists());
    }

    #[test]
    fn divergent_source_is_rejected_as_non_fast_forward() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "diverged");
        run_git_owned(
            &fixture.git,
            &fixture.repo,
            ["checkout", "--quiet", "--detach", fixture.initial.as_str()],
        );
        fs::write(fixture.repo.join("other.txt"), b"other\n").expect("other file");
        commit_all(&fixture.git, &fixture.repo, "divergent");
        let divergent = head(&fixture.git, &fixture.repo);
        run_git_owned(
            &fixture.git,
            &fixture.repo,
            ["update-ref", TARGET_REF, fixture.source.as_str()],
        );
        let service = fixture.service();
        let action = ActionId::generate().expect("action");

        let error = service
            .integrate_fast_forward(&action, &divergent, &fixture.source)
            .expect_err("divergent integration must fail");
        assert!(matches!(error, GitIntegrationError::NonFastForward));
        assert_eq!(
            ref_head(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.source
        );
        assert!(!fixture.integration_root.join(action.to_token()).exists());
    }

    #[test]
    fn non_commit_source_is_rejected() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "blob");
        let blob_path = fixture.base.join("blob.txt");
        fs::write(&blob_path, b"blob\n").expect("blob bytes");
        let output = Command::new(&fixture.git)
            .arg("-C")
            .arg(&fixture.repo)
            .args(["hash-object", "-w"])
            .arg(&blob_path)
            .output()
            .expect("hash blob");
        assert!(output.status.success());
        let blob = GitObjectId::parse(
            std::str::from_utf8(&output.stdout)
                .expect("blob utf8")
                .trim()
                .to_owned(),
        )
        .expect("blob oid");
        let service = fixture.service();

        let error = service
            .integrate_fast_forward(
                &ActionId::generate().expect("action"),
                &blob,
                &fixture.initial,
            )
            .expect_err("blob must not integrate");
        assert!(matches!(error, GitIntegrationError::ObjectNotFound));
        assert_eq!(
            ref_head(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.initial
        );
    }

    #[test]
    fn operation_path_collision_fails_closed_without_ref_update() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "collision");
        let service = fixture.service();
        let action = ActionId::generate().expect("action");
        fs::create_dir(fixture.integration_root.join(action.to_token())).expect("collision dir");

        let error = service
            .integrate_fast_forward(&action, &fixture.source, &fixture.initial)
            .expect_err("collision must fail");
        assert!(matches!(
            error,
            GitIntegrationError::OperationPathAlreadyExists
        ));
        assert_eq!(
            ref_head(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.initial
        );
    }

    #[test]
    fn integration_root_must_not_overlap_repository() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "overlap");
        let overlapping = fixture.repo.join(".optic-integration");
        let error = GitIntegrationService::from_hard_limits(
            &fixture.repo,
            &fixture.git,
            &overlapping,
            TARGET_REF,
            HardLimits::default(),
        )
        .expect_err("overlap must fail");
        assert!(matches!(
            error,
            GitIntegrationError::IntegrationRootOverlapsRepository
        ));
    }

    #[test]
    fn target_ref_is_confined_to_optic_namespace() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "namespace");
        let error = GitIntegrationService::from_hard_limits(
            &fixture.repo,
            &fixture.git,
            &fixture.integration_root,
            "refs/heads/main",
            HardLimits::default(),
        )
        .expect_err("ordinary branch must be rejected");
        assert!(matches!(
            error,
            GitIntegrationError::TargetRefOutsideOpticNamespace
        ));
    }

    const RECOVERY_CHILD_ENV: &str = "OPTIC_GIT_RECOVERY_CHILD";
    const RECOVERY_CHILD_REPO_ENV: &str = "OPTIC_GIT_RECOVERY_REPO";
    const RECOVERY_CHILD_GIT_ENV: &str = "OPTIC_GIT_RECOVERY_GIT";
    const RECOVERY_CHILD_ROOT_ENV: &str = "OPTIC_GIT_RECOVERY_ROOT";
    const RECOVERY_CHILD_TARGET_ENV: &str = "OPTIC_GIT_RECOVERY_TARGET";
    const RECOVERY_CHILD_EXPECTED_ENV: &str = "OPTIC_GIT_RECOVERY_EXPECTED";
    const RECOVERY_CHILD_ACTION_ENV: &str = "OPTIC_GIT_RECOVERY_ACTION";

    #[test]
    fn forced_git_recovery_child_entrypoint() {
        if env::var_os(RECOVERY_CHILD_ENV).is_none() {
            return;
        }
        let repo = PathBuf::from(env::var_os(RECOVERY_CHILD_REPO_ENV).expect("child repo"));
        let git = PathBuf::from(env::var_os(RECOVERY_CHILD_GIT_ENV).expect("child git"));
        let root = PathBuf::from(env::var_os(RECOVERY_CHILD_ROOT_ENV).expect("child root"));
        let target = env::var(RECOVERY_CHILD_TARGET_ENV).expect("child target");
        let expected =
            GitObjectId::parse(env::var(RECOVERY_CHILD_EXPECTED_ENV).expect("child expected"))
                .expect("expected oid");
        let action =
            ActionId::from_token(&env::var(RECOVERY_CHILD_ACTION_ENV).expect("child action"))
                .expect("action id");
        let service =
            GitIntegrationService::from_hard_limits(repo, git, root, target, HardLimits::default())
                .expect("child service");
        service
            .create_locked_worktree(&service.integration_root.join(action.to_token()), &expected)
            .expect("child worktree");
        std::process::exit(91);
    }

    #[test]
    fn process_termination_orphan_is_recovered_without_target_movement() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "recovery-process-crash");
        let action = ActionId::generate().expect("action");
        let current_exe = env::current_exe().expect("current test executable");
        let status = Command::new(current_exe)
            .args([
                "--exact",
                "git_integrate::tests::forced_git_recovery_child_entrypoint",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(RECOVERY_CHILD_ENV, "1")
            .env(RECOVERY_CHILD_REPO_ENV, &fixture.repo)
            .env(RECOVERY_CHILD_GIT_ENV, &fixture.git)
            .env(RECOVERY_CHILD_ROOT_ENV, &fixture.integration_root)
            .env(RECOVERY_CHILD_TARGET_ENV, TARGET_REF)
            .env(RECOVERY_CHILD_EXPECTED_ENV, fixture.initial.as_str())
            .env(RECOVERY_CHILD_ACTION_ENV, action.to_token())
            .status()
            .expect("spawn recovery child");
        assert_eq!(status.code(), Some(91));
        let orphan = fixture.integration_root.join(action.to_token());
        assert!(orphan.exists());
        assert_eq!(
            ref_head(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.initial
        );

        let report = fixture
            .service()
            .recover_owned_worktrees()
            .expect("recover orphan");
        assert_eq!(report.removed_worktrees, 1);
        assert!(!orphan.exists());
        assert_eq!(
            ref_head(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.initial
        );
    }

    #[test]
    fn missing_target_ref_fails_closed() {
        let Some(git) = find_git_executable() else {
            return;
        };
        let fixture = RepoFixture::new(git, "missing-target");
        run_git_owned(
            &fixture.git,
            &fixture.repo,
            ["update-ref", "-d", TARGET_REF],
        );
        let error = GitIntegrationService::from_hard_limits(
            &fixture.repo,
            &fixture.git,
            &fixture.integration_root,
            TARGET_REF,
            HardLimits::default(),
        )
        .expect_err("missing target must fail");
        assert!(matches!(error, GitIntegrationError::TargetRefMissing));
    }
}

// Security regressions added after the initial runtime slice: symbolic refs must
// never let an Optic-owned ref dereference into a user branch, and the disabled
// hook directory is revalidated immediately before mutation-capable Git calls.
#[cfg(test)]
mod hardening_tests {
    use super::*;
    use std::{env, process::Command};

    const TARGET_REF: &str = "refs/optic/integration/default";

    fn git() -> Option<PathBuf> {
        let path = env::var_os("PATH")?;
        env::split_paths(&path).find_map(|directory| {
            #[cfg(windows)]
            let candidate = directory.join("git.exe");
            #[cfg(not(windows))]
            let candidate = directory.join("git");
            candidate
                .is_file()
                .then(|| fs::canonicalize(candidate).ok())
                .flatten()
        })
    }

    fn run(git: &Path, repo: &Path, args: &[&str]) -> Vec<u8> {
        let output = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("git fixture command");
        assert!(output.status.success(), "git fixture command failed");
        output.stdout
    }

    fn fixture(
        label: &str,
    ) -> Option<(PathBuf, PathBuf, PathBuf, PathBuf, GitObjectId, GitObjectId)> {
        let git = git()?;
        let token = ActionId::generate().ok()?.to_token();
        let base = env::temp_dir().join(format!("ogi-hard-{label}-{}", &token[..10]));
        let repo = base.join("repo");
        let integration = base.join("integration");
        fs::create_dir_all(&repo).ok()?;
        fs::create_dir_all(&integration).ok()?;
        run(&git, &repo, &["init", "--quiet"]);
        run(
            &git,
            &repo,
            &["config", "user.email", "optic@example.invalid"],
        );
        run(&git, &repo, &["config", "user.name", "Optic Test"]);
        fs::write(repo.join("tracked.txt"), b"one\n").ok()?;
        run(&git, &repo, &["add", "."]);
        run(&git, &repo, &["commit", "--quiet", "-m", "one"]);
        let first = GitObjectId::parse(
            String::from_utf8(run(&git, &repo, &["rev-parse", "HEAD"]))
                .ok()?
                .trim()
                .to_owned(),
        )
        .ok()?;
        run(&git, &repo, &["update-ref", TARGET_REF, first.as_str()]);
        fs::write(repo.join("tracked.txt"), b"two\n").ok()?;
        run(&git, &repo, &["add", "."]);
        run(&git, &repo, &["commit", "--quiet", "-m", "two"]);
        let second = GitObjectId::parse(
            String::from_utf8(run(&git, &repo, &["rev-parse", "HEAD"]))
                .ok()?
                .trim()
                .to_owned(),
        )
        .ok()?;
        Some((base, repo, integration, git, first, second))
    }

    #[test]
    fn symbolic_target_ref_is_rejected_before_service_creation() {
        let Some((base, repo, integration, git, _, _)) = fixture("symbolic") else {
            return;
        };
        let branch = String::from_utf8(run(&git, &repo, &["symbolic-ref", "HEAD"]))
            .expect("branch utf8")
            .trim()
            .to_owned();
        run(&git, &repo, &["symbolic-ref", TARGET_REF, &branch]);

        let error = GitIntegrationService::from_hard_limits(
            &repo,
            &git,
            &integration,
            TARGET_REF,
            HardLimits::default(),
        )
        .expect_err("symbolic target must fail closed");
        assert!(matches!(error, GitIntegrationError::SymbolicTargetRef));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn hook_directory_is_rechecked_before_integration() {
        let Some((base, repo, integration, git, first, second)) = fixture("hooks") else {
            return;
        };
        let service = GitIntegrationService::from_hard_limits(
            &repo,
            &git,
            &integration,
            TARGET_REF,
            HardLimits::default(),
        )
        .expect("service");
        fs::write(
            integration
                .join("hooks-disabled")
                .join("reference-transaction"),
            b"x",
        )
        .expect("inject hook fixture");

        let error = service
            .integrate_fast_forward(&ActionId::generate().expect("action"), &second, &first)
            .expect_err("non-empty hooks directory must fail closed");
        assert!(matches!(
            error,
            GitIntegrationError::DisabledHooksDirectoryNotEmpty
        ));
        let observed = GitObjectId::parse(
            String::from_utf8(run(&git, &repo, &["rev-parse", "--verify", TARGET_REF]))
                .expect("target utf8")
                .trim()
                .to_owned(),
        )
        .expect("target oid");
        assert_eq!(observed, first);
        let _ = fs::remove_dir_all(base);
    }
}
