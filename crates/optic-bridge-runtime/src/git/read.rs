use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::ExitStatus,
    time::Duration,
};

use optic_bridge_core::{
    GitObjectId, GitObjectIdError, HardLimits, WorkspacePath, requires_sensitive_authority,
};
use thiserror::Error;

use crate::{
    HardenedCommandError, HardenedCommandOutput, HardenedCommandRunner, HardenedCommandSpec,
};

const REPOSITORY_PROBE_BYTES: u64 = 16 * 1024;
#[cfg(windows)]
const NULL_CONFIG_PATH: &str = "NUL";
#[cfg(not(windows))]
const NULL_CONFIG_PATH: &str = "/dev/null";

#[derive(Debug)]
pub struct GitReadService {
    repository_root: PathBuf,
    git_executable: PathBuf,
    max_read_bytes: u64,
    max_log_entries: u32,
    runner: HardenedCommandRunner,
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
            runner: HardenedCommandRunner::new(Duration::from_millis(
                limits.max_request_duration_ms,
            ))
            .map_err(|_| GitReadError::InvalidLimits)?,
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
        self.status_with_max_bytes(self.max_read_bytes)
    }

    pub fn status_with_max_bytes(&self, max_bytes: u64) -> Result<GitStatusSnapshot, GitReadError> {
        let limit = self.requested_byte_limit(Some(max_bytes))?;
        let args = os_args([
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
            "--ignore-submodules=all",
        ]);
        let output = self.run_success(&args, limit)?;
        let head = parse_status_head(&output.stdout)?;
        // Validate the complete NUL-delimited record stream before any MCP
        // response may expose it. Unknown/unrepresentable records fail closed.
        let sensitive_paths = parse_sensitive_status_paths(&output.stdout)?;
        Ok(GitStatusSnapshot {
            head,
            porcelain_v2: output.stdout,
            sensitive_paths,
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
            "--ignore-submodules=all",
            "--raw",
            "-z",
            "--patch",
        ]);
        if staged {
            args.push(OsString::from("--cached"));
        }
        if let Some(path) = path {
            args.push(OsString::from("--"));
            args.push(OsString::from(path.as_str()));
        }
        let output = self.run_success(&args, limit)?;
        parse_verified_diff(output.stdout)
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

        let current_head = self.resolve_head()?;
        let (snapshot_head, offset) = match cursor {
            Some(cursor) => {
                let current_head = current_head
                    .as_ref()
                    .ok_or(GitReadError::LogCursorNotReachable)?;
                self.ensure_log_cursor_reachable(&cursor.head, current_head)?;
                (Some(cursor.head.clone()), cursor.offset)
            }
            None => (current_head, 0),
        };
        let Some(snapshot_head) = snapshot_head else {
            return Ok(GitLogPage {
                snapshot_head: None,
                entries: Vec::new(),
                next_cursor: None,
            });
        };

        let requested_count = limit.checked_add(1).ok_or(GitReadError::InvalidLogRecord)?;
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

    fn ensure_log_cursor_reachable(
        &self,
        cursor_head: &GitObjectId,
        current_head: &GitObjectId,
    ) -> Result<(), GitReadError> {
        if cursor_head == current_head {
            return Ok(());
        }
        let args = vec![
            OsString::from("merge-base"),
            OsString::from("--is-ancestor"),
            OsString::from(cursor_head.as_str()),
            OsString::from(current_head.as_str()),
        ];
        let output = self.run_command(&args, self.max_read_bytes.min(REPOSITORY_PROBE_BYTES))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(GitReadError::LogCursorNotReachable)
        }
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
    ) -> Result<HardenedCommandOutput, GitReadError> {
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
    ) -> Result<HardenedCommandOutput, GitReadError> {
        if output_limit == 0 {
            return Err(GitReadError::ZeroRequestLimit);
        }

        let mut command_args = os_args([
            "--no-pager",
            "--literal-pathspecs",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
        ]);
        command_args.extend(args.iter().cloned());
        let spec = HardenedCommandSpec {
            executable: self.git_executable.clone(),
            cwd: self.repository_root.clone(),
            args: command_args,
            env: git_read_environment(),
            output_limit,
        };
        self.runner.run(&spec).map_err(map_runner_error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitStatusSnapshot {
    pub head: Option<GitObjectId>,
    pub porcelain_v2: Vec<u8>,
    /// Canonical-grammar Git paths that need an additional sensitive lease.
    /// A malformed or unrepresentable status is never returned as a snapshot.
    pub sensitive_paths: Vec<WorkspacePath>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitDiffSnapshot {
    /// Original Git patch bytes, excluding the private raw-path prelude.
    pub bytes: Vec<u8>,
    /// Changed paths requiring the additional sensitive read authority.
    pub sensitive_paths: Vec<WorkspacePath>,
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
    #[error("Git log cursor does not point to the current HEAD or a reachable ancestor")]
    LogCursorNotReachable,
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
    #[error("Git status output cannot be verified against sensitive-path policy")]
    UnverifiableStatus,
    #[error("Git diff output cannot be verified against sensitive-path policy")]
    UnverifiableDiff,
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

/// Git porcelain-v2 -z is NUL-delimited, with an extra NUL pathname
/// after record type '2' (rename/copy). Pathnames may contain spaces and
/// newlines; never inspect status as unstructured lines or search raw text.
fn parse_sensitive_status_paths(bytes: &[u8]) -> Result<Vec<WorkspacePath>, GitReadError> {
    let body = bytes
        .strip_suffix(&[0])
        .ok_or(GitReadError::UnverifiableStatus)?;
    let mut records = body.split(|byte| *byte == 0);
    let mut sensitive_paths = Vec::new();
    let mut seen_head = false;
    while let Some(record) = records.next() {
        if record.is_empty() {
            return Err(GitReadError::UnverifiableStatus);
        }
        if record.starts_with(b"# ") {
            // --branch emits only these known headers; refuse new/unknown
            // records instead of treating their unexamined data as harmless.
            if record.starts_with(b"# branch.oid ") {
                if seen_head {
                    return Err(GitReadError::UnverifiableStatus);
                }
                seen_head = true;
            } else if ![
                b"# branch.head ".as_slice(),
                b"# branch.upstream ".as_slice(),
                b"# branch.ab ".as_slice(),
            ]
            .iter()
            .any(|prefix| record.starts_with(prefix))
            {
                return Err(GitReadError::UnverifiableStatus);
            }
            continue;
        }
        let (path, renamed) = match record.first() {
            Some(b'1') if record.starts_with(b"1 ") => (status_path_field(record, 8)?, false),
            Some(b'2') if record.starts_with(b"2 ") => (status_path_field(record, 9)?, true),
            Some(b'u') if record.starts_with(b"u ") => (status_path_field(record, 10)?, false),
            Some(b'?') if record.starts_with(b"? ") => (&record[2..], false),
            Some(b'!') if record.starts_with(b"! ") => (&record[2..], false),
            _ => return Err(GitReadError::UnverifiableStatus),
        };
        classify_status_path(path, &mut sensitive_paths)?;
        if renamed {
            classify_status_path(
                records.next().ok_or(GitReadError::UnverifiableStatus)?,
                &mut sensitive_paths,
            )?;
        }
    }
    if !seen_head {
        return Err(GitReadError::UnverifiableStatus);
    }
    Ok(sensitive_paths)
}

fn status_path_field(record: &[u8], preceding_fields: usize) -> Result<&[u8], GitReadError> {
    let mut tail = record;
    for _ in 0..preceding_fields {
        let separator = tail
            .iter()
            .position(|byte| *byte == b' ')
            .ok_or(GitReadError::UnverifiableStatus)?;
        if separator == 0 {
            return Err(GitReadError::UnverifiableStatus);
        }
        tail = &tail[separator + 1..];
    }
    if tail.is_empty() {
        return Err(GitReadError::UnverifiableStatus);
    }
    Ok(tail)
}

fn classify_status_path(
    path: &[u8],
    sensitive_paths: &mut Vec<WorkspacePath>,
) -> Result<(), GitReadError> {
    let path = std::str::from_utf8(path).map_err(|_| GitReadError::UnverifiableStatus)?;
    // Git emits slash-separated repository-relative names under -z. In
    // particular a Unix literal backslash cannot be normalized for authority,
    // and on Windows Git is not allowed to invent a different separator here.
    if path.contains('\\') {
        return Err(GitReadError::UnverifiableStatus);
    }
    let path = WorkspacePath::from_native_relative(Path::new(path))
        .map_err(|_| GitReadError::UnverifiableStatus)?;
    if requires_sensitive_authority(&path) {
        sensitive_paths.push(path);
    }
    Ok(())
}

/// Both path identities and patch bytes originate from ONE Git diff invocation.
/// A separate --name-only preflight would race the working tree/index. The raw
/// -z prelude is machine-parseable; only the patch suffix is returned to MCP.
/// No unverified byte is published on unexpected/native-unrepresentable output.
fn parse_verified_diff(output: Vec<u8>) -> Result<GitDiffSnapshot, GitReadError> {
    if output.is_empty() {
        return Ok(GitDiffSnapshot {
            bytes: Vec::new(),
            sensitive_paths: Vec::new(),
        });
    }
    let mut offset = 0usize;
    let mut count = 0usize;
    let mut sensitive_paths = Vec::new();
    while output.get(offset) == Some(&b':') {
        let header_end = diff_nul_end(&output, offset)?;
        let status = verify_raw_diff_header(&output[offset..header_end])?;
        offset = header_end + 1;
        let path_end = diff_nul_end(&output, offset)?;
        classify_status_path(&output[offset..path_end], &mut sensitive_paths)
            .map_err(|_| GitReadError::UnverifiableDiff)?;
        offset = path_end + 1;
        if matches!(status, b'R' | b'C') {
            let other_end = diff_nul_end(&output, offset)?;
            classify_status_path(&output[offset..other_end], &mut sensitive_paths)
                .map_err(|_| GitReadError::UnverifiableDiff)?;
            offset = other_end + 1;
        }
        count += 1;
    }
    if count == 0 {
        return Err(GitReadError::UnverifiableDiff);
    }
    // Git emits exactly one additional NUL between its -z raw record
    // stream and patch text. Reject any other delimiter rather than trying
    // to guess where confidential patch bytes begin.
    let patch = output[offset..]
        .strip_prefix(b"\0")
        .ok_or(GitReadError::UnverifiableDiff)?;
    if !patch.starts_with(b"diff --git ") {
        return Err(GitReadError::UnverifiableDiff);
    }
    Ok(GitDiffSnapshot {
        bytes: patch.to_vec(),
        sensitive_paths,
    })
}

fn diff_nul_end(output: &[u8], offset: usize) -> Result<usize, GitReadError> {
    output
        .get(offset..)
        .and_then(|tail| tail.iter().position(|byte| *byte == 0))
        .map(|relative| offset + relative)
        .ok_or(GitReadError::UnverifiableDiff)
}

fn verify_raw_diff_header(header: &[u8]) -> Result<u8, GitReadError> {
    let invalid = || GitReadError::UnverifiableDiff;
    let mut fields = header.split(|byte| *byte == b' ');
    let old_mode = fields.next().ok_or_else(invalid)?;
    let new_mode = fields.next().ok_or_else(invalid)?;
    let old_oid = fields.next().ok_or_else(invalid)?;
    let new_oid = fields.next().ok_or_else(invalid)?;
    let status = fields.next().ok_or_else(invalid)?;
    if fields.next().is_some()
        || old_mode.len() != 7
        || old_mode[0] != b':'
        || !old_mode[1..].iter().all(u8::is_ascii_digit)
        || new_mode.len() != 6
        || !new_mode.iter().all(u8::is_ascii_digit)
        || ![old_oid, new_oid]
            .iter()
            .all(|oid| (7..=64).contains(&oid.len()) && oid.iter().all(u8::is_ascii_hexdigit))
    {
        return Err(invalid());
    }
    let (kind, detail) = status.split_first().ok_or_else(invalid)?;
    if !matches!(kind, b'A' | b'D' | b'M' | b'T' | b'U' | b'R' | b'C')
        || (!detail.is_empty()
            && (!matches!(kind, b'M' | b'R' | b'C') || !detail.iter().all(u8::is_ascii_digit)))
    {
        return Err(invalid());
    }
    Ok(*kind)
}

fn parse_log_entries(bytes: &[u8]) -> Result<Vec<GitLogEntry>, GitReadError> {
    let text = std::str::from_utf8(bytes).map_err(|_| GitReadError::InvalidLogRecord)?;
    let mut entries = Vec::new();
    for line in text.lines() {
        // Empty rows are not commit records. Silently skipping them can
        // make offset-based pagination disagree with the emitted Git stream.
        if line.is_empty() {
            return Err(GitReadError::InvalidLogRecord);
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

fn git_read_environment() -> BTreeMap<OsString, OsString> {
    [
        ("GIT_OPTIONAL_LOCKS", "0"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_PAGER", "cat"),
        ("PAGER", "cat"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", NULL_CONFIG_PATH),
        ("GIT_NO_REPLACE_OBJECTS", "1"),
        ("GIT_ATTR_NOSYSTEM", "1"),
        ("LC_ALL", "C"),
    ]
    .into_iter()
    .map(|(key, value)| (OsString::from(key), OsString::from(value)))
    .collect()
}

fn map_runner_error(error: HardenedCommandError) -> GitReadError {
    match error {
        HardenedCommandError::TimedOut => GitReadError::CommandTimedOut,
        HardenedCommandError::OutputLimitExceeded { limit } => {
            GitReadError::OutputLimitExceeded { limit }
        }
        HardenedCommandError::MissingChildPipe => GitReadError::MissingChildPipe,
        HardenedCommandError::ReaderThreadPanicked | HardenedCommandError::WriterThreadPanicked => {
            GitReadError::ReaderThreadPanicked
        }
        HardenedCommandError::Io(error) => GitReadError::Io(error),
        HardenedCommandError::ExecutableMustBeAbsolute
        | HardenedCommandError::WorkingDirectoryMustBeAbsolute
        | HardenedCommandError::ZeroOutputLimit
        | HardenedCommandError::ZeroTimeout
        | HardenedCommandError::InputLimitExceeded { .. } => GitReadError::InvalidLimits,
    }
}

fn command_failed(status: ExitStatus, stderr: &[u8]) -> GitReadError {
    GitReadError::CommandFailed {
        code: status.code(),
        stderr: String::from_utf8_lossy(stderr).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::{env, ffi::OsStr, process::Command, sync::Arc, thread};

    use optic_bridge_core::ActionId;

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
            run_git(
                &git,
                None,
                [OsStr::new("init"), OsStr::new("--quiet"), repo.as_os_str()],
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

    fn run_git<const N: usize>(git: &Path, repo: Option<&Path>, args: [&OsStr; N]) {
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
            [
                OsStr::new("commit"),
                OsStr::new("--quiet"),
                OsStr::new("-m"),
                OsStr::new(message),
            ],
        );
    }

    fn path(value: &str) -> WorkspacePath {
        WorkspacePath::parse(value).expect("workspace path")
    }

    #[test]
    fn git_read_environment_is_explicit_and_excludes_user_search_paths() {
        let env = git_read_environment();
        assert_eq!(
            env.get(&OsString::from("GIT_CONFIG_NOSYSTEM")),
            Some(&OsString::from("1"))
        );
        assert_eq!(
            env.get(&OsString::from("GIT_CONFIG_GLOBAL")),
            Some(&OsString::from(NULL_CONFIG_PATH))
        );
        assert!(!env.contains_key(&OsString::from("PATH")));
        assert!(!env.contains_key(&OsString::from("HOME")));
        assert!(!env.contains_key(&OsString::from("HTTP_PROXY")));
        assert!(!env.contains_key(&OsString::from("HTTPS_PROXY")));
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
        assert!(
            status
                .porcelain_v2
                .windows(b"a.txt".len())
                .any(|v| v == b"a.txt")
        );
        assert!(
            status
                .porcelain_v2
                .windows(b"new.txt".len())
                .any(|v| v == b"new.txt")
        );
    }

    #[test]
    fn status_classifies_sensitive_staged_and_untracked_names() {
        let fixture = RepoFixture::new("sensitive-status");
        fs::write(fixture.repo.join(".ENV.production"), b"fixture only").expect("fixture");
        fs::write(fixture.repo.join("ordinary notes.txt"), b"ordinary").expect("fixture");
        run_git(
            &fixture.git,
            Some(&fixture.repo),
            [OsStr::new("add"), OsStr::new(".ENV.production")],
        );
        fs::create_dir_all(fixture.repo.join(".aws")).expect("fixture directory");
        fs::write(
            fixture.repo.join(".aws").join("credentials"),
            b"fixture only",
        )
        .expect("fixture");
        let snapshot = fixture.service().status().expect("status parsed");
        let paths: Vec<_> = snapshot
            .sensitive_paths
            .iter()
            .map(WorkspacePath::as_str)
            .collect();
        assert!(paths.contains(&".ENV.production"));
        assert!(paths.contains(&".aws/credentials"));
        assert!(!paths.contains(&"ordinary notes.txt"));
    }

    #[test]
    fn porcelain_v2_parser_checks_both_rename_names_and_all_record_shapes() {
        // Synthetic protocol records exercise cases even if a particular Git
        // version does not emit a rename or unmerged entry in a local fixture.
        let mut raw = b"# branch.oid (initial)\0".to_vec();
        raw.extend_from_slice(b"1 M. N... 100644 100644 100644 aa bb src/main.rs\0");
        raw.extend_from_slice(
            b"2 R. N... 100644 100644 100644 aa bb R100 normal.txt\0nested/.env\0",
        );
        raw.extend_from_slice(b"u UU N... 100644 100644 100644 100644 a b c auth/secret.key\0");
        raw.extend_from_slice(b"? new-token.txt\0");
        raw.extend_from_slice(b"! ignored.txt\0");
        let paths = parse_sensitive_status_paths(&raw).expect("all record types parsed");
        let names: Vec<_> = paths.iter().map(WorkspacePath::as_str).collect();
        assert_eq!(names, ["nested/.env", "auth/secret.key", "new-token.txt"]);
    }

    #[test]
    fn porcelain_v2_parser_fails_closed_for_unknown_truncated_and_native_alias_records() {
        for raw in [
            b"# branch.oid (initial)\0x unexpected\0".as_slice(),
            b"# branch.oid (initial)\0? unclosed.txt".as_slice(),
            b"# branch.oid (initial)\0? ..\\.env\0".as_slice(),
            b"# branch.oid (initial)\0? ../.env\0".as_slice(),
            b"# branch.oid (initial)\0? \xff\0".as_slice(),
            b"# branch.oid (initial)\x002 R. N... 100644 100644 100644 aa bb R100 path.txt\0"
                .as_slice(),
            b"# branch.stash 1\0".as_slice(),
        ] {
            assert!(
                matches!(
                    parse_sensitive_status_paths(raw),
                    Err(GitReadError::UnverifiableStatus)
                ),
                "unverifiable status must be denied"
            );
        }
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
    fn diff_treats_workspace_path_as_literal_pathspec() {
        let fixture = RepoFixture::new("diff-literal");
        let literal = "literal[1].txt";
        fs::write(fixture.repo.join(literal), b"one\n").expect("literal fixture");
        commit_all(&fixture.git, &fixture.repo, "literal path");
        fs::write(fixture.repo.join(literal), b"two\n").expect("modify literal");

        let diff = fixture
            .service()
            .diff(Some(&path(literal)), false, None)
            .expect("literal diff");
        let text = String::from_utf8(diff.bytes).expect("git diff utf8 fixture");
        assert!(text.contains(literal));
    }

    #[test]
    fn git_raw_and_patch_framing_from_real_hardened_command_is_verifiable() {
        let fixture = RepoFixture::new("diff-framing");
        fs::write(fixture.repo.join("a.txt"), b"modified synthetic content\n").expect("fixture");
        let output = fixture
            .service()
            .run_success(
                &os_args([
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    "--no-renames",
                    "--ignore-submodules=all",
                    "--raw",
                    "-z",
                    "--patch",
                ]),
                64 * 1024,
            )
            .expect("hardened Git raw + patch");
        assert!(
            parse_verified_diff(output.stdout.clone()).is_ok(),
            "real Git format is unverified (first 200 synthetic bytes): {:?}",
            &output.stdout[..output.stdout.len().min(200)]
        );
    }

    #[test]
    fn diff_single_invocation_classifies_worktree_and_staged_sensitive_paths() {
        let fixture = RepoFixture::new("diff-sensitive");
        fs::write(fixture.repo.join(".env"), b"first fixture only\n").expect("fixture");
        fs::write(fixture.repo.join("a.txt"), b"ordinary changed\n").expect("fixture");
        commit_all(&fixture.git, &fixture.repo, "fixture secrets");
        fs::write(fixture.repo.join(".env"), b"second fixture only\n").expect("change");
        fs::write(fixture.repo.join("a.txt"), b"ordinary again\n").expect("change");
        let service = fixture.service();
        let all = service.diff(None, false, None).expect("whole-repo diff");
        assert!(all.sensitive_paths.iter().any(|p| p.as_str() == ".env"));
        assert!(
            String::from_utf8_lossy(&all.bytes).contains("second fixture only"),
            "existing raw patch semantics preserved"
        );
        let ordinary = service
            .diff(Some(&path("a.txt")), false, None)
            .expect("literal ordinary diff");
        assert!(ordinary.sensitive_paths.is_empty());
        assert!(String::from_utf8_lossy(&ordinary.bytes).contains("ordinary again"));
        let protected = service
            .diff(Some(&path(".env")), false, None)
            .expect("literal protected diff");
        assert_eq!(
            protected
                .sensitive_paths
                .iter()
                .map(WorkspacePath::as_str)
                .collect::<Vec<_>>(),
            [".env"]
        );

        // A staged newly added protected file has no committed worktree
        // predecessor, but its contents must still require sensitive access.
        fs::write(fixture.repo.join("nested.key"), b"new key fixture only\n").expect("fixture");
        run_git(
            &fixture.git,
            Some(&fixture.repo),
            [OsStr::new("add"), OsStr::new("nested.key")],
        );
        let staged = service.diff(None, true, None).expect("staged diff");
        assert!(
            staged
                .sensitive_paths
                .iter()
                .any(|p| p.as_str() == "nested.key")
        );
        assert!(String::from_utf8_lossy(&staged.bytes).contains("new key fixture only"));
    }

    #[test]
    fn diff_single_invocation_checks_directory_pathspec_children() {
        let fixture = RepoFixture::new("diff-directory");
        fs::create_dir_all(fixture.repo.join("src")).expect("directory");
        fs::write(fixture.repo.join("src").join(".env.local"), b"first\n").expect("fixture");
        fs::write(fixture.repo.join("src").join("main.rs"), b"first\n").expect("fixture");
        commit_all(&fixture.git, &fixture.repo, "nested paths");
        fs::write(
            fixture.repo.join("src").join(".env.local"),
            b"changed secret\n",
        )
        .expect("change");
        fs::write(
            fixture.repo.join("src").join("main.rs"),
            b"changed source\n",
        )
        .expect("change");
        let selected = fixture
            .service()
            .diff(Some(&path("src")), false, None)
            .expect("directory diff");
        assert!(
            selected
                .sensitive_paths
                .iter()
                .any(|p| p.as_str() == "src/.env.local")
        );
        assert!(String::from_utf8_lossy(&selected.bytes).contains("changed source"));
    }

    #[test]
    fn malformed_raw_diff_stream_refuses_any_patch_before_release() {
        let good = b":100644 100644 abcdef0 1234567 M\0src/main.rs\0\0diff --git a/src/main.rs b/src/main.rs\n";
        let verified = parse_verified_diff(good.to_vec()).expect("verified diff");
        assert_eq!(verified.bytes, b"diff --git a/src/main.rs b/src/main.rs\n");
        assert!(verified.sensitive_paths.is_empty());
        let protected =
            b":100644 100644 abcdef0 1234567 M\0src/.env\0\0diff --git a/src/.env b/src/.env\n";
        assert_eq!(
            parse_verified_diff(protected.to_vec())
                .expect("protected path parsed")
                .sensitive_paths[0]
                .as_str(),
            "src/.env"
        );
        for invalid in [
            b"diff --git a/.env b/.env\n".as_slice(),
            b":100644 100644 abcdef0 1234567 M\0src/.env".as_slice(),
            b":100644 100644 abcdef0 1234567 X\0src/.env\0\ndiff --git ".as_slice(),
            b":100644 100644 abcdef0 1234567 M\0../.env\0\ndiff --git ".as_slice(),
            b":100644 100644 abcdef0 1234567 M\0src\\.env\0\ndiff --git ".as_slice(),
            b":100644 100644 abcdef0 1234567 M\0\xff\0\ndiff --git ".as_slice(),
            b":100644 100644 abcdef0 1234567 M\0src/.env\0\ncorrupted".as_slice(),
            b":100644 100644 abcdef0 1234567 M\0src/.env\0".as_slice(),
            b":100644 100644 abcdef0 1234567 M\0src/.env\0diff --git a/src/.env b/src/.env\n"
                .as_slice(),
            b":100644 100644 abcdef0 1234567 M\0src/.env\0\ndiff --git a/src/.env b/src/.env\n"
                .as_slice(),
        ] {
            assert!(
                matches!(
                    parse_verified_diff(invalid.to_vec()),
                    Err(GitReadError::UnverifiableDiff)
                ),
                "malformed/unverifiable diff must be rejected"
            );
        }
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
    fn git_log_parser_rejects_blank_rows_instead_of_skipping_commits() {
        let id = "0123456789012345678901234567890123456789";
        let one = format!("{id}\t\t42\n");
        let two = format!("{id}\t\t43\n");
        assert!(parse_log_entries(b"").expect("empty result").is_empty());
        let valid = parse_log_entries(format!("{one}{two}").as_bytes())
            .expect("consecutive commit records");
        assert_eq!(valid.len(), 2);
        assert_eq!(valid[0].commit_time_unix_seconds, 42);
        assert_eq!(valid[1].commit_time_unix_seconds, 43);

        for malformed in [
            format!("\n{one}"),
            format!("{one}\n{two}"),
            format!("{one}\n"),
            "\n".to_owned(),
        ] {
            assert!(
                matches!(
                    parse_log_entries(malformed.as_bytes()),
                    Err(GitReadError::InvalidLogRecord)
                ),
                "a blank Git record must never silently advance or shorten a cursor page"
            );
        }
    }

    #[test]
    fn log_rejects_unreachable_cursor_head() {
        let fixture = RepoFixture::new("log-unreachable");
        let service = fixture.service();
        let cursor = GitLogCursor {
            head: GitObjectId::parse("0000000000000000000000000000000000000000".to_owned())
                .expect("syntactically valid object id"),
            offset: 0,
        };
        assert!(matches!(
            service.log(Some(&cursor), 1),
            Err(GitReadError::LogCursorNotReachable)
        ));
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
