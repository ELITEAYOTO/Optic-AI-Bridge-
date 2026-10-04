use std::{
    ffi::OsString,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use optic_bridge_core::{ActionId, GitObjectId, GitObjectIdError, HardLimits, WorkspacePath};
use thiserror::Error;

const REPOSITORY_PROBE_BYTES: u64 = 16 * 1024;
const PIPE_READ_CHUNK_BYTES: usize = 8 * 1024;
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Debug)]
pub struct GitReadService {
    repository_root: PathBuf,
    git_executable: PathBuf,
    max_read_bytes: u64,
    max_log_entries: u32,
    command_timeout: Duration,
}

impl GitReadService {
    pub fn from_hard_limits(
        repository_root: impl AsRef<Path>,
        git_executable: impl AsRef<Path>,
        limits: HardLimits,
    ) -> Result<Self, GitReadError> {
        limits
            .validate_nonzero()
            .map_err(|_| GitReadError::InvalidLimits)?;

        if !git_executable.as_ref().is_absolute() {
            return Err(GitReadError::GitExecutableMustBeAbsolute);
        }
        let git_executable = fs::canonicalize(git_executable.as_ref())?;
        if !fs::metadata(&git_executable)?.is_file() {
            return Err(GitReadError::GitExecutableNotFile);
        }

        let repository_root = fs::canonicalize(repository_root.as_ref())?;
        if !fs::metadata(&repository_root)?.is_dir() {
            return Err(GitReadError::RepositoryRootNotDirectory);
        }

        let service = Self {
            repository_root,
            git_executable,
            max_read_bytes: limits.max_git_read_bytes,
            max_log_entries: limits.max_git_log_entries,
            command_timeout: Duration::from_millis(limits.max_request_duration_ms),
        };
        service.validate_repository_root()?;
        Ok(service)
    }

    #[must_use]
    pub fn repository_root(&self) -> &Path {
        &self.repository_root
    }

    #[must_use]
    pub fn git_executable(&self) -> &Path {
        &self.git_executable
    }

    pub fn status(&self) -> Result<GitStatusSnapshot, GitReadError> {
        let args = os_args([
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
        ]);
        let output = self.run_success(&args, self.max_read_bytes)?;
        let head = parse_status_head(&output.stdout)?;
        Ok(GitStatusSnapshot {
            head,
            porcelain_v2: output.stdout,
        })
    }

    pub fn diff(
        &self,
        path: Option<&WorkspacePath>,
        staged: bool,
        max_bytes: Option<u64>,
    ) -> Result<GitDiffSnapshot, GitReadError> {
        let limit = self.requested_byte_limit(max_bytes)?;
        let mut args = os_args([
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--no-renames",
        ]);
        if staged {
            args.push(OsString::from("--cached"));
        }
        if let Some(path) = path {
            args.push(OsString::from("--"));
            args.push(OsString::from(path.as_str()));
        }
        let output = self.run_success(&args, limit)?;
        Ok(GitDiffSnapshot {
            bytes: output.stdout,
        })
    }

    pub fn log(
        &self,
        cursor: Option<&GitLogCursor>,
        limit: u32,
    ) -> Result<GitLogPage, GitReadError> {
        if limit == 0 {
            return Err(GitReadError::ZeroRequestLimit);
        }
        if limit > self.max_log_entries {
            return Err(GitReadError::LogEntryLimitExceeded {
                requested: limit,
                limit: self.max_log_entries,
            });
        }

        let (snapshot_head, offset) = match cursor {
            Some(cursor) => (Some(cursor.head.clone()), cursor.offset),
            None => (self.resolve_head()?, 0),
        };
        let Some(snapshot_head) = snapshot_head else {
            return Ok(GitLogPage {
                snapshot_head: None,
                entries: Vec::new(),
                next_cursor: None,
            });
        };

        let requested_count = limit
            .checked_add(1)
            .ok_or(GitReadError::InvalidLogRecord)?;
        let args = vec![
            OsString::from("log"),
            OsString::from(snapshot_head.as_str()),
            OsString::from("--topo-order"),
            OsString::from("--no-decorate"),
            OsString::from(format!("--max-count={requested_count}")),
            OsString::from(format!("--skip={offset}")),
            OsString::from("--format=%H%x09%P%x09%ct"),
        ];
        let output = self.run_success(&args, self.max_read_bytes)?;
        let mut entries = parse_log_entries(&output.stdout)?;
        let has_more = entries.len() > limit as usize;
        if has_more {
            entries.truncate(limit as usize);
        }
        let next_cursor = if has_more {
            Some(GitLogCursor {
                head: snapshot_head.clone(),
                offset: offset
                    .checked_add(u64::from(limit))
                    .ok_or(GitReadError::InvalidLogRecord)?,
            })
        } else {
            None
        };

        Ok(GitLogPage {
            snapshot_head: Some(snapshot_head),
            entries,
            next_cursor,
        })
    }

    fn validate_repository_root(&self) -> Result<(), GitReadError> {
        let args = os_args(["rev-parse", "--show-toplevel"]);
        let limit = self.max_read_bytes.min(REPOSITORY_PROBE_BYTES);
        let output = self.run_success(&args, limit)?;
        let printed = std::str::from_utf8(&output.stdout)
            .map_err(|_| GitReadError::NonUtf8RepositoryRoot)?
            .trim();
        if printed.is_empty() {
            return Err(GitReadError::RepositoryRootMismatch);
        }
        let observed = fs::canonicalize(Path::new(printed))?;
        if observed != self.repository_root {
            return Err(GitReadError::RepositoryRootMismatch);
        }
        Ok(())
    }

    fn resolve_head(&self) -> Result<Option<GitObjectId>, GitReadError> {
        let args = os_args(["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]);
        let output = self.run_command(&args, self.max_read_bytes.min(REPOSITORY_PROBE_BYTES))?;
        if output.status.success() {
            let value = std::str::from_utf8(&output.stdout)
                .map_err(|_| GitReadError::InvalidObjectId)?
                .trim();
            return Ok(Some(GitObjectId::parse(value.to_owned())?));
        }
        if output.status.code() == Some(1) && output.stdout.is_empty() && output.stderr.is_empty() {
            return Ok(None);
        }
        Err(command_failed(output.status, &output.stderr))
    }

    fn requested_byte_limit(&self, requested: Option<u64>) -> Result<u64, GitReadError> {
        let requested = requested.unwrap_or(self.max_read_bytes);
        if requested == 0 {
            return Err(GitReadError::ZeroRequestLimit);
        }
        if requested > self.max_read_bytes {
            return Err(GitReadError::ReadByteLimitExceeded {
                requested,
                limit: self.max_read_bytes,
            });
        }
        Ok(requested)
    }

    fn run_success(
        &self,
        args: &[OsString],
        output_limit: u64,
    ) -> Result<BoundedGitOutput, GitReadError> {
        let output = self.run_command(args, output_limit)?;
        if !output.status.success() {
            return Err(command_failed(output.status, &output.stderr));
        }
        Ok(output)
    }

    fn run_command(
        &self,
        args: &[OsString],
        output_limit: u64,
    ) -> Result<BoundedGitOutput, GitReadError> {
        if output_limit == 0 {
            return Err(GitReadError::ZeroRequestLimit);
        }

        let mut command = Command::new(&self.git_executable);
        command
            .arg("--no-pager")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .arg("-c")
            .arg("core.untrackedCache=false")
            .arg("-C")
            .arg(&self.repository_root)
            .args(args)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_PAGER", "cat")
            .env("PAGER", "cat")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn()?;
        let stdout = child.stdout.take().ok_or(GitReadError::MissingChildPipe)?;
        let stderr = child.stderr.take().ok_or(GitReadError::MissingChildPipe)?;
        let total = Arc::new(AtomicU64::new(0));
        let exceeded = Arc::new(AtomicBool::new(false));
        let stdout_reader = spawn_bounded_reader(stdout, Arc::clone(&total), Arc::clone(&exceeded), output_limit);
        let stderr_reader = spawn_bounded_reader(stderr, total, Arc::clone(&exceeded), output_limit);

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
            .map_err(|_| GitReadError::ReaderThreadPanicked)??;
        let stderr = stderr_reader
            .join()
            .map_err(|_| GitReadError::ReaderThreadPanicked)??;

        if timed_out {
            return Err(GitReadError::CommandTimedOut);
        }
        if exceeded.load(Ordering::Acquire) {
            return Err(GitReadError::OutputLimitExceeded {
                limit: output_limit,
            });
        }

        Ok(BoundedGitOutput {
            status,
            stdout,
            stderr,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitStatusSnapshot {
    pub head: Option<GitObjectId>,
    pub porcelain_v2: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitDiffSnapshot {
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitLogCursor {
    pub head: GitObjectId,
    pub offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitLogEntry {
    pub id: GitObjectId,
    pub parents: Vec<GitObjectId>,
    pub commit_time_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitLogPage {
    pub snapshot_head: Option<GitObjectId>,
    pub entries: Vec<GitLogEntry>,
    pub next_cursor: Option<GitLogCursor>,
}

struct BoundedGitOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum GitReadError {
    #[error("git read hard limits are invalid")]
    InvalidLimits,
    #[error("git executable path must be absolute")]
    GitExecutableMustBeAbsolute,
    #[error("canonical git executable is not a regular file")]
    GitExecutableNotFile,
    #[error("canonical repository root is not a directory")]
    RepositoryRootNotDirectory,
    #[error("configured repository root is not the exact Git worktree top level")]
    RepositoryRootMismatch,
    #[error("Git reported a non-UTF-8 repository root")]
    NonUtf8RepositoryRoot,
    #[error("requested Git read byte limit must be non-zero")]
    ZeroRequestLimit,
    #[error("requested Git read byte limit {requested} exceeds hard ceiling {limit}")]
    ReadByteLimitExceeded { requested: u64, limit: u64 },
    #[error("requested Git log entry count {requested} exceeds hard ceiling {limit}")]
    LogEntryLimitExceeded { requested: u32, limit: u32 },
    #[error("Git command output exceeded byte ceiling {limit}")]
    OutputLimitExceeded { limit: u64 },
    #[error("Git command exceeded its hard deadline")]
    CommandTimedOut,
    #[error("Git command failed with exit code {code:?}: {stderr}")]
    CommandFailed { code: Option<i32>, stderr: String },
    #[error("Git command child pipe was unavailable")]
    MissingChildPipe,
    #[error("Git output reader thread panicked")]
    ReaderThreadPanicked,
    #[error("Git status/log contained an invalid object id")]
    InvalidObjectId,
    #[error("Git log output was malformed")]
    InvalidLogRecord,
    #[error("filesystem/process operation failed: {0}")]
    Io(#[from] io::Error),
    #[error("Git object id is invalid: {0}")]
    ObjectId(#[from] GitObjectIdError),
}

fn os_args<const N: usize>(values: [&str; N]) -> Vec<OsString> {
    values.into_iter().map(OsString::from).collect()
}

fn parse_status_head(bytes: &[u8]) -> Result<Option<GitObjectId>, GitReadError> {
    for record in bytes.split(|byte| *byte == 0) {
        let Some(value) = record.strip_prefix(b"# branch.oid ") else {
            continue;
        };
        let value = std::str::from_utf8(value).map_err(|_| GitReadError::InvalidObjectId)?;
        if value.starts_with('(') {
            return Ok(None);
        }
        return Ok(Some(GitObjectId::parse(value.to_owned())?));
    }
    Ok(None)
}

fn parse_log_entries(bytes: &[u8]) -> Result<Vec<GitLogEntry>, GitReadError> {
    let text = std::str::from_utf8(bytes).map_err(|_| GitReadError::InvalidLogRecord)?;
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let id = fields.next().ok_or(GitReadError::InvalidLogRecord)?;
        let parents = fields.next().ok_or(GitReadError::InvalidLogRecord)?;
        let commit_time = fields.next().ok_or(GitReadError::InvalidLogRecord)?;
        if fields.next().is_some() {
            return Err(GitReadError::InvalidLogRecord);
        }
        let parents = if parents.is_empty() {
            Vec::new()
        } else {
            parents
                .split(' ')
                .map(|value| GitObjectId::parse(value.to_owned()).map_err(GitReadError::from))
                .collect::<Result<Vec<_>, _>>()?
        };
        let commit_time_unix_seconds = commit_time
            .parse::<u64>()
            .map_err(|_| GitReadError::InvalidLogRecord)?;
        entries.push(GitLogEntry {
            id: GitObjectId::parse(id.to_owned())?,
            parents,
            commit_time_unix_seconds,
        });
    }
    Ok(entries)
}

fn spawn_bounded_reader<R: Read + Send + 'static>(
    mut reader: R,
    total: Arc<AtomicU64>,
    exceeded: Arc<AtomicBool>,
    limit: u64,
) -> thread::JoinHandle<Result<Vec<u8>, io::Error>> {
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0_u8; PIPE_READ_CHUNK_BYTES];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let read_u64 = read as u64;
            let previous = total.fetch_add(read_u64, Ordering::AcqRel);
            let remaining = limit.saturating_sub(previous);
            let accepted = remaining.min(read_u64) as usize;
            captured.extend_from_slice(&buffer[..accepted]);
            if read_u64 > remaining {
                exceeded.store(true, Ordering::Release);
            }
        }
        Ok(captured)
    })
}

fn command_failed(status: ExitStatus, stderr: &[u8]) -> GitReadError {
    GitReadError::CommandFailed {
        code: status.code(),
        stderr: String::from_utf8_lossy(stderr).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::{env, ffi::OsStr, sync::Arc};

    use super::*;

    struct RepoFixture {
        base: PathBuf,
        repo: PathBuf,
        git: PathBuf,
    }

    impl RepoFixture {
        fn new(label: &str) -> Self {
            let git = find_git_executable().expect("git on CI PATH");
            let token = ActionId::generate().expect("test entropy").to_token();
            let base = env::temp_dir().join(format!("optic-git-read-{label}-{token}"));
            let repo = base.join("repo");
            fs::create_dir_all(&repo).expect("repo dir");
            run_git(&git, None, [OsStr::new("init"), OsStr::new("--quiet"), repo.as_os_str()]);
            run_git(
                &git,
                Some(&repo),
                [OsStr::new("config"), OsStr::new("user.email"), OsStr::new("optic@example.invalid")],
            );
            run_git(
                &git,
                Some(&repo),
                [OsStr::new("config"), OsStr::new("user.name"), OsStr::new("Optic Test")],
            );
            fs::write(repo.join("a.txt"), b"alpha\n").expect("a fixture");
            fs::write(repo.join("b.txt"), b"bravo\n").expect("b fixture");
            commit_all(&git, &repo, "initial");
            Self { base, repo, git }
        }

        fn service(&self) -> GitReadService {
            GitReadService::from_hard_limits(&self.repo, &self.git, HardLimits::default())
                .expect("git service")
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

    fn run_git<'a, const N: usize>(
        git: &Path,
        repo: Option<&Path>,
        args: [&'a OsStr; N],
    ) {
        let mut command = Command::new(git);
        if let Some(repo) = repo {
            command.arg("-C").arg(repo);
        }
        let status = command.args(args).status().expect("run fixture git");
        assert!(status.success(), "fixture git command failed");
    }

    fn commit_all(git: &Path, repo: &Path, message: &str) {
        run_git(git, Some(repo), [OsStr::new("add"), OsStr::new(".")]);
        run_git(
            git,
            Some(repo),
            [OsStr::new("commit"), OsStr::new("--quiet"), OsStr::new("-m"), OsStr::new(message)],
        );
    }

    fn path(value: &str) -> WorkspacePath {
        WorkspacePath::parse(value).expect("workspace path")
    }

    #[test]
    fn configured_root_must_be_exact_worktree_top_level() {
        let fixture = RepoFixture::new("root");
        let nested = fixture.repo.join("nested");
        fs::create_dir_all(&nested).expect("nested");
        assert!(matches!(
            GitReadService::from_hard_limits(&nested, &fixture.git, HardLimits::default()),
            Err(GitReadError::RepositoryRootMismatch)
        ));
    }

    #[test]
    fn status_is_porcelain_v2_and_reports_head() {
        let fixture = RepoFixture::new("status");
        fs::write(fixture.repo.join("a.txt"), b"changed\n").expect("modify a");
        fs::write(fixture.repo.join("new.txt"), b"new\n").expect("new file");
        let status = fixture.service().status().expect("status");
        assert!(status.head.is_some());
        assert!(status.porcelain_v2.windows(b"a.txt".len()).any(|v| v == b"a.txt"));
        assert!(status.porcelain_v2.windows(b"new.txt".len()).any(|v| v == b"new.txt"));
    }

    #[test]
    fn diff_is_path_scoped() {
        let fixture = RepoFixture::new("diff-path");
        fs::write(fixture.repo.join("a.txt"), b"alpha changed\n").expect("modify a");
        fs::write(fixture.repo.join("b.txt"), b"bravo changed\n").expect("modify b");
        let diff = fixture
            .service()
            .diff(Some(&path("a.txt")), false, None)
            .expect("diff");
        let text = String::from_utf8(diff.bytes).expect("git diff utf8 fixture");
        assert!(text.contains("a.txt"));
        assert!(!text.contains("b.txt"));
    }

    #[test]
    fn diff_output_is_hard_bounded() {
        let fixture = RepoFixture::new("diff-limit");
        fs::write(fixture.repo.join("a.txt"), vec![b'x'; 4096]).expect("large diff");
        assert!(matches!(
            fixture.service().diff(None, false, Some(64)),
            Err(GitReadError::OutputLimitExceeded { limit: 64 })
        ));
    }

    #[test]
    fn log_cursor_stays_pinned_to_original_head() {
        let fixture = RepoFixture::new("log-cursor");
        fs::write(fixture.repo.join("a.txt"), b"second\n").expect("second");
        commit_all(&fixture.git, &fixture.repo, "second");
        fs::write(fixture.repo.join("a.txt"), b"third\n").expect("third");
        commit_all(&fixture.git, &fixture.repo, "third");

        let service = fixture.service();
        let first = service.log(None, 2).expect("first page");
        assert_eq!(first.entries.len(), 2);
        let cursor = first.next_cursor.clone().expect("next cursor");
        let pinned_head = first.snapshot_head.clone().expect("snapshot head");

        fs::write(fixture.repo.join("a.txt"), b"fourth\n").expect("fourth");
        commit_all(&fixture.git, &fixture.repo, "fourth");
        let current = service.log(None, 1).expect("current head");
        assert_ne!(current.snapshot_head, Some(pinned_head.clone()));

        let second = service.log(Some(&cursor), 2).expect("pinned second page");
        assert_eq!(second.snapshot_head, Some(pinned_head));
        assert_eq!(second.entries.len(), 1);
        assert!(second.next_cursor.is_none());
    }

    #[test]
    fn concurrent_readers_do_not_share_mutable_runtime_state() {
        let fixture = RepoFixture::new("concurrent");
        fs::write(fixture.repo.join("a.txt"), b"changed\n").expect("modify");
        let service = Arc::new(fixture.service());
        let mut readers = Vec::new();
        for _ in 0..4 {
            let service = Arc::clone(&service);
            readers.push(thread::spawn(move || {
                let status = service.status().expect("concurrent status");
                let log = service.log(None, 1).expect("concurrent log");
                (status.head, log.entries.len())
            }));
        }
        for reader in readers {
            let (head, count) = reader.join().expect("reader join");
            assert!(head.is_some());
            assert_eq!(count, 1);
        }
    }
}
