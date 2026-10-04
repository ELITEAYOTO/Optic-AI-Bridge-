use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use optic_bridge_core::{ActionId, GitObjectId, GitObjectIdError, HardLimits, IdError};
use thiserror::Error;

use crate::{GitReadError, GitReadService};

const INTEGRATION_REF_PREFIX: &str = "refs/optic/integration/";
const CAPTURE_LIMIT_BYTES: u64 = 4 * 1024;
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(5);

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

        let disabled_hooks_root = integration_root.join("hooks-disabled");
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
        let unlock = self.run_status(
            &self.repository_root,
            [
                OsString::from("worktree"),
                OsString::from("unlock"),
                git_path_arg(worktree_path),
            ],
        )?;
        if !unlock.success() {
            return Err(GitIntegrationError::WorktreeCleanupFailed);
        }
        let remove = self.run_status(
            &self.repository_root,
            [
                OsString::from("worktree"),
                OsString::from("remove"),
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
        let capture_id = ActionId::generate()?;
        let capture_path = self
            .integration_root
            .join(format!("capture-{}.out", capture_id.to_token()));
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&capture_path)?;
        let stdout = output.try_clone()?;

        let mut command = self.base_command(context);
        command
            .args(args)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::null());
        let mut child = command.spawn()?;
        let deadline = Instant::now() + self.command_timeout;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                drop(output);
                let _ = fs::remove_file(&capture_path);
                return Err(GitIntegrationError::CommandTimedOut);
            }
            thread::sleep(CHILD_POLL_INTERVAL);
        };
        drop(output);

        let result = (|| {
            let metadata = fs::metadata(&capture_path)?;
            if metadata.len() > CAPTURE_LIMIT_BYTES {
                return Err(GitIntegrationError::CommandOutputTooLarge);
            }
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            File::open(&capture_path)?.read_to_end(&mut bytes)?;
            Ok((status, bytes))
        })();
        let _ = fs::remove_file(&capture_path);
        result
    }
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
