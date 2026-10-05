use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use optic_bridge_core::{
    HardLimits, JobId, LimitError, ResourceBudget, SessionHandle, WorkspacePath,
};
#[cfg(windows)]
use optic_bridge_windows::LimitedJobObject;
use process_wrap::tokio::{ChildWrapper, CommandWrap};
#[cfg(unix)]
use process_wrap::tokio::{KillOnDrop, ProcessGroup};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    runtime::Handle,
    time::Instant,
};

const MONITOR_POLL_INTERVAL: Duration = Duration::from_millis(10);
const DRAIN_CHUNK_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug)]
pub struct ProcessStartSpec {
    pub session: SessionHandle,
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: Option<WorkspacePath>,
    pub env_allowlist: Vec<String>,
    pub resources: ResourceBudget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessStatus {
    Running,
    Exited,
    Stopped,
    TimedOut,
    OutputLimitExceeded,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessReadChunk {
    pub bytes: Vec<u8>,
    pub offset: u64,
    pub next_offset: u64,
    pub eof: bool,
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessResult {
    pub status: ProcessStatus,
    pub exit_code: Option<i32>,
    pub output_truncated: bool,
}

#[derive(Debug)]
pub struct ProcessManager {
    root: PathBuf,
    limits: HardLimits,
    allowed_env_vars: BTreeSet<String>,
    jobs: Arc<Mutex<JobStore>>,
    sequence: AtomicU64,
}

#[derive(Debug, Default)]
struct JobStore {
    jobs: HashMap<JobId, Arc<JobRecord>>,
}

#[derive(Debug)]
struct JobRecord {
    owner: SessionHandle,
    sequence: u64,
    reserved_output_bytes: u64,
    output: Mutex<OutputState>,
    state: Mutex<JobState>,
    stop_requested: AtomicBool,
    output_overflow: AtomicBool,
}

#[derive(Debug)]
struct OutputState {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_closed: bool,
    stderr_closed: bool,
}

#[derive(Clone, Copy, Debug)]
struct JobState {
    status: ProcessStatus,
    exit_code: Option<i32>,
}

impl ProcessManager {
    pub fn new(
        root: impl AsRef<Path>,
        limits: HardLimits,
        allowed_env_vars: impl IntoIterator<Item = String>,
    ) -> Result<Self, ProcessError> {
        let limits = limits.validate_nonzero()?;
        let root = fs::canonicalize(root).map_err(ProcessError::Io)?;
        if !root.is_dir() {
            return Err(ProcessError::RootNotDirectory);
        }
        let allowed_env_vars = allowed_env_vars
            .into_iter()
            .map(|name| normalize_env_name(&name))
            .collect::<Result<BTreeSet<_>, _>>()?;

        Ok(Self {
            root,
            limits,
            allowed_env_vars,
            jobs: Arc::new(Mutex::new(JobStore::default())),
            sequence: AtomicU64::new(0),
        })
    }

    pub fn canonicalize_executable(&self, executable: &str) -> Result<String, ProcessError> {
        let path = Path::new(executable);
        if !path.is_absolute() {
            return Err(ProcessError::ExecutableMustBeAbsolute);
        }
        let canonical = fs::canonicalize(path).map_err(ProcessError::Io)?;
        if !canonical.is_file() {
            return Err(ProcessError::ExecutableNotFile);
        }
        canonical
            .to_str()
            .map(str::to_owned)
            .ok_or(ProcessError::NonUtf8Executable)
    }

    pub fn start(&self, spec: ProcessStartSpec) -> Result<JobId, ProcessError> {
        let resources = spec.resources.validate_nonzero()?;
        if !resources.fits_within(self.limits.max_process_budget) {
            return Err(ProcessError::ResourceBudgetExceeded);
        }
        if resources.output_bytes > self.limits.max_active_output_ram_bytes {
            return Err(ProcessError::OutputMemoryLimitExceeded);
        }
        if resources.output_bytes > self.limits.max_active_output_ram_bytes_per_session {
            return Err(ProcessError::OutputMemoryLimitExceededForSession);
        }
        self.validate_request_shape(&spec)?;
        let executable = self.canonicalize_executable(&spec.executable)?;
        let cwd = self.resolve_cwd(spec.cwd.as_ref())?;
        let environment = self.resolve_environment(&spec.env_allowlist)?;
        let runtime = Handle::try_current().map_err(|_| ProcessError::RuntimeUnavailable)?;

        let job_id = JobId::generate().map_err(|_| ProcessError::JobIdUnavailable)?;
        let sequence = self.sequence.fetch_add(1, Ordering::AcqRel);
        let owner = spec.session.clone();
        let record = Arc::new(JobRecord {
            owner: spec.session,
            sequence,
            reserved_output_bytes: resources.output_bytes,
            output: Mutex::new(OutputState {
                stdout: Vec::new(),
                stderr: Vec::new(),
                stdout_closed: false,
                stderr_closed: false,
            }),
            state: Mutex::new(JobState {
                status: ProcessStatus::Running,
                exit_code: None,
            }),
            stop_requested: AtomicBool::new(false),
            output_overflow: AtomicBool::new(false),
        });

        {
            let mut store = self
                .jobs
                .lock()
                .map_err(|_| ProcessError::StateUnavailable)?;
            self.prepare_store_for_start(&mut store, &owner, resources.output_bytes)?;
            store.jobs.insert(job_id.clone(), Arc::clone(&record));
        }

        let mut command = Command::new(&executable);
        command
            .args(&spec.args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();
        for (name, value) in environment {
            command.env(name, value);
        }

        let mut command = CommandWrap::from(command);
        #[cfg(windows)]
        command.wrap(
            LimitedJobObject::new(resources.process_count, resources.memory_bytes)
                .map_err(ProcessError::Io)?,
        );
        #[cfg(unix)]
        {
            command.wrap(KillOnDrop);
            command.wrap(ProcessGroup::leader());
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                self.remove_job(&job_id);
                return Err(ProcessError::Io(error));
            }
        };
        let stdout = child.stdout().take();
        let stderr = child.stderr().take();

        let stdout_record = Arc::clone(&record);
        let stdout_task = runtime.spawn(async move {
            if let Some(stdout) = stdout {
                drain_output(stdout, ProcessStream::Stdout, stdout_record).await;
            } else {
                mark_stream_closed(&stdout_record, ProcessStream::Stdout);
            }
        });
        let stderr_record = Arc::clone(&record);
        let stderr_task = runtime.spawn(async move {
            if let Some(stderr) = stderr {
                drain_output(stderr, ProcessStream::Stderr, stderr_record).await;
            } else {
                mark_stream_closed(&stderr_record, ProcessStream::Stderr);
            }
        });
        let monitor_record = Arc::clone(&record);
        runtime.spawn(async move {
            monitor_child(
                &mut *child,
                monitor_record,
                resources.timeout_ms,
                stdout_task,
                stderr_task,
            )
            .await;
        });

        Ok(job_id)
    }

    pub fn read(
        &self,
        session: &SessionHandle,
        job_id: &JobId,
        stream: ProcessStream,
        cursor: u64,
        max_bytes: u64,
    ) -> Result<ProcessReadChunk, ProcessError> {
        if max_bytes == 0 || max_bytes > self.limits.max_process_read_bytes {
            return Err(ProcessError::ReadLimitExceeded);
        }
        let record = self.owned_job(session, job_id)?;
        let output = record
            .output
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?;
        let (bytes, closed) = match stream {
            ProcessStream::Stdout => (&output.stdout, output.stdout_closed),
            ProcessStream::Stderr => (&output.stderr, output.stderr_closed),
        };
        let start = usize::try_from(cursor).map_err(|_| ProcessError::CursorOutOfRange)?;
        if start > bytes.len() {
            return Err(ProcessError::CursorOutOfRange);
        }
        let take = usize::try_from(max_bytes).map_err(|_| ProcessError::ReadLimitExceeded)?;
        let end = start.saturating_add(take).min(bytes.len());
        let next_offset = u64::try_from(end).map_err(|_| ProcessError::CursorOutOfRange)?;
        Ok(ProcessReadChunk {
            bytes: bytes[start..end].to_vec(),
            offset: cursor,
            next_offset,
            eof: closed && end == bytes.len(),
            truncated: record.output_overflow.load(Ordering::Acquire),
        })
    }

    pub fn stop(&self, session: &SessionHandle, job_id: &JobId) -> Result<bool, ProcessError> {
        let record = self.owned_job(session, job_id)?;
        let running = record
            .state
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?
            .status
            == ProcessStatus::Running;
        if running {
            record.stop_requested.store(true, Ordering::Release);
        }
        Ok(running)
    }

    pub fn result(
        &self,
        session: &SessionHandle,
        job_id: &JobId,
    ) -> Result<ProcessResult, ProcessError> {
        let record = self.owned_job(session, job_id)?;
        let state = *record
            .state
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?;
        Ok(ProcessResult {
            status: state.status,
            exit_code: state.exit_code,
            output_truncated: record.output_overflow.load(Ordering::Acquire),
        })
    }

    pub fn cancel_session(&self, session: &SessionHandle) -> Result<usize, ProcessError> {
        let store = self
            .jobs
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?;
        let mut changed = 0;
        for record in store.jobs.values() {
            if &record.owner == session {
                let running = record
                    .state
                    .lock()
                    .map_err(|_| ProcessError::StateUnavailable)?
                    .status
                    == ProcessStatus::Running;
                if running && !record.stop_requested.swap(true, Ordering::AcqRel) {
                    changed += 1;
                }
            }
        }
        Ok(changed)
    }

    pub(crate) fn active_session_job_count(
        &self,
        session: &SessionHandle,
    ) -> Result<u32, ProcessError> {
        let store = self
            .jobs
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?;
        active_job_count_for_session(&store, session)
    }

    pub(crate) fn remove_terminal_session_records(
        &self,
        session: &SessionHandle,
    ) -> Result<usize, ProcessError> {
        let mut store = self
            .jobs
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?;
        let mut terminal = Vec::new();
        for (job_id, record) in &store.jobs {
            if &record.owner != session {
                continue;
            }
            let status = record
                .state
                .lock()
                .map_err(|_| ProcessError::StateUnavailable)?
                .status;
            if status != ProcessStatus::Running {
                terminal.push(job_id.clone());
            }
        }
        let removed = terminal.len();
        for job_id in terminal {
            store.jobs.remove(&job_id);
        }
        Ok(removed)
    }

    #[must_use]
    pub const fn limits(&self) -> HardLimits {
        self.limits
    }

    fn validate_request_shape(&self, spec: &ProcessStartSpec) -> Result<(), ProcessError> {
        let mut bytes = u64::try_from(spec.executable.len()).unwrap_or(u64::MAX);
        for arg in &spec.args {
            bytes = bytes.saturating_add(u64::try_from(arg.len()).unwrap_or(u64::MAX));
        }
        for name in &spec.env_allowlist {
            bytes = bytes.saturating_add(u64::try_from(name.len()).unwrap_or(u64::MAX));
        }
        if let Some(cwd) = &spec.cwd {
            bytes = bytes.saturating_add(u64::try_from(cwd.as_str().len()).unwrap_or(u64::MAX));
        }
        if bytes > self.limits.max_request_bytes {
            return Err(ProcessError::RequestShapeTooLarge);
        }
        Ok(())
    }

    fn resolve_environment(
        &self,
        requested: &[String],
    ) -> Result<Vec<(String, std::ffi::OsString)>, ProcessError> {
        let mut seen = BTreeSet::new();
        let mut output = Vec::new();
        for raw in requested {
            let normalized = normalize_env_name(raw)?;
            if !self.allowed_env_vars.contains(&normalized) {
                return Err(ProcessError::EnvironmentNotAllowed);
            }
            if seen.insert(normalized.clone())
                && let Some(value) = std::env::var_os(&normalized)
            {
                output.push((normalized, value));
            }
        }
        Ok(output)
    }

    fn resolve_cwd(&self, cwd: Option<&WorkspacePath>) -> Result<PathBuf, ProcessError> {
        let Some(cwd) = cwd else {
            return Ok(self.root.clone());
        };
        let mut candidate = self.root.clone();
        for segment in cwd.as_str().split('/') {
            candidate.push(segment);
        }
        let canonical = fs::canonicalize(candidate).map_err(ProcessError::Io)?;
        if !canonical.starts_with(&self.root) {
            return Err(ProcessError::CwdOutsideWorkspace);
        }
        if !canonical.is_dir() {
            return Err(ProcessError::CwdNotDirectory);
        }
        Ok(canonical)
    }

    fn owned_job(
        &self,
        session: &SessionHandle,
        job_id: &JobId,
    ) -> Result<Arc<JobRecord>, ProcessError> {
        let store = self
            .jobs
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?;
        let record = store.jobs.get(job_id).ok_or(ProcessError::UnknownJob)?;
        if &record.owner != session {
            return Err(ProcessError::UnknownJob);
        }
        Ok(Arc::clone(record))
    }

    fn prepare_store_for_start(
        &self,
        store: &mut JobStore,
        session: &SessionHandle,
        requested_output_bytes: u64,
    ) -> Result<(), ProcessError> {
        loop {
            let active = active_job_count(store)?;
            if active >= self.limits.max_active_process_jobs {
                return Err(ProcessError::TooManyActiveJobs);
            }
            let session_active = active_job_count_for_session(store, session)?;
            if session_active >= self.limits.max_active_process_jobs_per_session {
                return Err(ProcessError::TooManyActiveJobsForSession);
            }

            let reserved = reserved_output_bytes(store);
            let session_reserved = reserved_output_bytes_for_session(store, session);
            let global_record_limit_reached = store.jobs.len()
                >= usize::try_from(self.limits.max_process_records)
                    .map_err(|_| ProcessError::InvalidLimits(LimitError::InvalidRelationship))?;
            let session_record_limit_reached = job_count_for_session(store, session)
                >= usize::try_from(self.limits.max_process_records_per_session)
                    .map_err(|_| ProcessError::InvalidLimits(LimitError::InvalidRelationship))?;
            let global_output_limit_reached = reserved.saturating_add(requested_output_bytes)
                > self.limits.max_active_output_ram_bytes;
            let session_output_limit_reached = session_reserved
                .saturating_add(requested_output_bytes)
                > self.limits.max_active_output_ram_bytes_per_session;

            if !global_record_limit_reached
                && !session_record_limit_reached
                && !global_output_limit_reached
                && !session_output_limit_reached
            {
                return Ok(());
            }

            // A start request may only retire history owned by the same session.
            // Cross-session eviction would let one session destroy another session's
            // observable process result even though JobId access itself is owner-bound.
            let Some(oldest_terminal) = oldest_terminal_job_for_session(store, session)? else {
                return Err(if session_output_limit_reached {
                    ProcessError::OutputMemoryLimitExceededForSession
                } else if global_output_limit_reached {
                    ProcessError::OutputMemoryLimitExceeded
                } else if session_record_limit_reached {
                    ProcessError::ProcessRecordLimitExceededForSession
                } else {
                    ProcessError::ProcessRecordLimitExceeded
                });
            };
            store.jobs.remove(&oldest_terminal);
        }
    }

    fn remove_job(&self, job_id: &JobId) {
        if let Ok(mut store) = self.jobs.lock() {
            store.jobs.remove(job_id);
        }
    }
}

fn normalize_env_name(name: &str) -> Result<String, ProcessError> {
    if name.is_empty() || name.len() > 128 || name.contains('=') || name.contains('\0') {
        return Err(ProcessError::InvalidEnvironmentName);
    }
    if cfg!(windows) {
        Ok(name.to_ascii_uppercase())
    } else {
        Ok(name.to_owned())
    }
}

fn active_job_count(store: &JobStore) -> Result<u32, ProcessError> {
    active_job_count_matching(store, |_| true)
}

fn active_job_count_for_session(
    store: &JobStore,
    session: &SessionHandle,
) -> Result<u32, ProcessError> {
    active_job_count_matching(store, |record| &record.owner == session)
}

fn active_job_count_matching(
    store: &JobStore,
    matches: impl Fn(&JobRecord) -> bool,
) -> Result<u32, ProcessError> {
    let mut active = 0_u32;
    for record in store.jobs.values() {
        if matches(record)
            && record
                .state
                .lock()
                .map_err(|_| ProcessError::StateUnavailable)?
                .status
                == ProcessStatus::Running
        {
            active = active.saturating_add(1);
        }
    }
    Ok(active)
}

fn reserved_output_bytes(store: &JobStore) -> u64 {
    store.jobs.values().fold(0_u64, |sum, record| {
        sum.saturating_add(record.reserved_output_bytes)
    })
}

fn reserved_output_bytes_for_session(store: &JobStore, session: &SessionHandle) -> u64 {
    store.jobs.values().fold(0_u64, |sum, record| {
        if &record.owner == session {
            sum.saturating_add(record.reserved_output_bytes)
        } else {
            sum
        }
    })
}

fn job_count_for_session(store: &JobStore, session: &SessionHandle) -> usize {
    store
        .jobs
        .values()
        .filter(|record| &record.owner == session)
        .count()
}

fn oldest_terminal_job_for_session(
    store: &JobStore,
    session: &SessionHandle,
) -> Result<Option<JobId>, ProcessError> {
    let mut oldest: Option<(&JobId, u64)> = None;
    for (job_id, record) in &store.jobs {
        if &record.owner != session {
            continue;
        }
        let status = record
            .state
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?
            .status;
        if status == ProcessStatus::Running {
            continue;
        }
        if oldest.is_none_or(|(_, sequence)| record.sequence < sequence) {
            oldest = Some((job_id, record.sequence));
        }
    }
    Ok(oldest.map(|(job_id, _)| job_id.clone()))
}

async fn drain_output<R>(mut reader: R, stream: ProcessStream, record: Arc<JobRecord>)
where
    R: AsyncRead + Unpin,
{
    let mut chunk = [0_u8; DRAIN_CHUNK_BYTES];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) => break,
            Ok(read) => append_output(&record, stream, &chunk[..read]),
            Err(_) => break,
        }
    }
    mark_stream_closed(&record, stream);
}

fn append_output(record: &JobRecord, stream: ProcessStream, bytes: &[u8]) {
    let Ok(mut output) = record.output.lock() else {
        record.output_overflow.store(true, Ordering::Release);
        return;
    };
    let used = output.stdout.len().saturating_add(output.stderr.len());
    let budget = usize::try_from(record.reserved_output_bytes).unwrap_or(usize::MAX);
    let remaining = budget.saturating_sub(used);
    let keep = remaining.min(bytes.len());
    let target = match stream {
        ProcessStream::Stdout => &mut output.stdout,
        ProcessStream::Stderr => &mut output.stderr,
    };
    target.extend_from_slice(&bytes[..keep]);
    if keep < bytes.len() {
        record.output_overflow.store(true, Ordering::Release);
    }
}

fn mark_stream_closed(record: &JobRecord, stream: ProcessStream) {
    if let Ok(mut output) = record.output.lock() {
        match stream {
            ProcessStream::Stdout => output.stdout_closed = true,
            ProcessStream::Stderr => output.stderr_closed = true,
        }
    }
}

async fn monitor_child(
    child: &mut dyn ChildWrapper,
    record: Arc<JobRecord>,
    timeout_ms: u64,
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
) {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let (status, exit_code) = loop {
        match child.try_wait() {
            Ok(Some(exit)) => break (ProcessStatus::Exited, exit.code()),
            Err(_) => break (ProcessStatus::Failed, None),
            Ok(None) => {}
        }

        let reason = if record.stop_requested.load(Ordering::Acquire) {
            Some(ProcessStatus::Stopped)
        } else if record.output_overflow.load(Ordering::Acquire) {
            Some(ProcessStatus::OutputLimitExceeded)
        } else if Instant::now() >= deadline {
            Some(ProcessStatus::TimedOut)
        } else {
            None
        };

        if let Some(reason) = reason {
            let _ = child.start_kill();
            let exit_code = child.wait().await.ok().and_then(|status| status.code());
            break (reason, exit_code);
        }
        tokio::time::sleep(MONITOR_POLL_INTERVAL).await;
    };

    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let status =
        if status == ProcessStatus::Exited && record.output_overflow.load(Ordering::Acquire) {
            ProcessStatus::OutputLimitExceeded
        } else {
            status
        };
    if let Ok(mut state) = record.state.lock() {
        *state = JobState { status, exit_code };
    }
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("invalid process hard limits: {0}")]
    InvalidLimits(#[from] LimitError),
    #[error("workspace root must be a directory")]
    RootNotDirectory,
    #[error("process executable must be an absolute path")]
    ExecutableMustBeAbsolute,
    #[error("process executable must resolve to a regular file")]
    ExecutableNotFile,
    #[error("canonical executable path cannot be represented as UTF-8")]
    NonUtf8Executable,
    #[error("process working directory resolves outside the workspace")]
    CwdOutsideWorkspace,
    #[error("process working directory must resolve to a directory")]
    CwdNotDirectory,
    #[error("process environment variable name is invalid")]
    InvalidEnvironmentName,
    #[error("process environment variable is not operator-allowlisted")]
    EnvironmentNotAllowed,
    #[error("requested process resources exceed the hard ceiling")]
    ResourceBudgetExceeded,
    #[error("process request shape exceeds the hard request byte ceiling")]
    RequestShapeTooLarge,
    #[error("too many process jobs are already active")]
    TooManyActiveJobs,
    #[error("this session already has too many active process jobs")]
    TooManyActiveJobsForSession,
    #[error("bounded process record history is full")]
    ProcessRecordLimitExceeded,
    #[error("this session's bounded process record history is full")]
    ProcessRecordLimitExceededForSession,
    #[error("reserved process output would exceed the hard in-memory ceiling")]
    OutputMemoryLimitExceeded,
    #[error("this session's reserved process output would exceed its hard in-memory ceiling")]
    OutputMemoryLimitExceededForSession,
    #[error("process output read exceeds the hard per-call byte ceiling")]
    ReadLimitExceeded,
    #[error("process output cursor is out of range")]
    CursorOutOfRange,
    #[error("process job is unknown to this session")]
    UnknownJob,
    #[error("Tokio runtime is unavailable")]
    RuntimeUnavailable,
    #[error("operating-system entropy unavailable for process job id")]
    JobIdUnavailable,
    #[error("process runtime state is unavailable")]
    StateUnavailable,
    #[error("process operating-system operation failed: {0}")]
    Io(std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::{env, fs, thread};

    use optic_bridge_core::ActionId;

    use super::*;

    fn workspace(label: &str) -> PathBuf {
        let token = ActionId::generate().expect("test entropy").to_token();
        let root = env::temp_dir().join(format!("optic-process-{label}-{token}"));
        fs::create_dir_all(&root).expect("create temp workspace");
        root
    }

    fn spec(root: &Path, session: SessionHandle, resources: ResourceBudget) -> ProcessStartSpec {
        let executable = env::current_exe()
            .expect("current test executable")
            .canonicalize()
            .expect("canonical test executable")
            .to_string_lossy()
            .into_owned();
        let _ = root;
        ProcessStartSpec {
            session,
            executable,
            args: vec![
                "--exact".to_owned(),
                "process::tests::process_fixture_child".to_owned(),
                "--nocapture".to_owned(),
            ],
            cwd: None,
            env_allowlist: Vec::new(),
            resources,
        }
    }

    fn budget(timeout_ms: u64, output_bytes: u64) -> ResourceBudget {
        ResourceBudget {
            timeout_ms,
            output_bytes,
            memory_bytes: 64 * 1024 * 1024,
            process_count: 4,
        }
    }

    async fn await_terminal(
        manager: &ProcessManager,
        session: &SessionHandle,
        job: &JobId,
    ) -> ProcessResult {
        for _ in 0..300 {
            let result = manager.result(session, job).expect("read process result");
            if result.status != ProcessStatus::Running {
                return result;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("process did not reach a terminal state");
    }

    #[test]
    fn process_fixture_child() {
        let cwd = env::current_dir().expect("fixture cwd");
        if cwd.join("fixture-output").exists() {
            print!("fixture-stdout");
            eprint!("fixture-stderr");
        } else if cwd.join("fixture-sleep").exists() {
            thread::sleep(Duration::from_secs(2));
        } else if cwd.join("fixture-flood").exists() {
            print!("{}", "x".repeat(128 * 1024));
        }
    }

    #[tokio::test]
    async fn process_output_is_cursor_readable_and_owned() {
        let root = workspace("output");
        fs::write(root.join("fixture-output"), b"1").expect("write fixture mode");
        let manager =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let owner = SessionHandle::generate().expect("owner session");
        let other = SessionHandle::generate().expect("other session");
        let job = manager
            .start(spec(&root, owner.clone(), budget(2000, 64 * 1024)))
            .expect("start process");
        let result = await_terminal(&manager, &owner, &job).await;
        assert_eq!(result.status, ProcessStatus::Exited);
        assert_eq!(
            manager
                .result(&other, &job)
                .expect_err("cross-session result must fail")
                .to_string(),
            ProcessError::UnknownJob.to_string()
        );
        let stdout = manager
            .read(&owner, &job, ProcessStream::Stdout, 0, 64 * 1024)
            .expect("read stdout");
        assert!(String::from_utf8_lossy(&stdout.bytes).contains("fixture-stdout"));
        assert!(stdout.eof);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn timeout_terminates_owned_process() {
        let root = workspace("timeout");
        fs::write(root.join("fixture-sleep"), b"1").expect("write fixture mode");
        let manager =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let owner = SessionHandle::generate().expect("owner session");
        let job = manager
            .start(spec(&root, owner.clone(), budget(100, 64 * 1024)))
            .expect("start process");
        let result = await_terminal(&manager, &owner, &job).await;
        assert_eq!(result.status, ProcessStatus::TimedOut);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn explicit_stop_terminates_owned_process() {
        let root = workspace("stop");
        fs::write(root.join("fixture-sleep"), b"1").expect("write fixture mode");
        let manager =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let owner = SessionHandle::generate().expect("owner session");
        let job = manager
            .start(spec(&root, owner.clone(), budget(5000, 64 * 1024)))
            .expect("start process");
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(manager.stop(&owner, &job).expect("stop process"));
        let result = await_terminal(&manager, &owner, &job).await;
        assert_eq!(result.status, ProcessStatus::Stopped);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn output_overflow_is_bounded_and_terminates_process() {
        let root = workspace("overflow");
        fs::write(root.join("fixture-flood"), b"1").expect("write fixture mode");
        let manager =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let owner = SessionHandle::generate().expect("owner session");
        let job = manager
            .start(spec(&root, owner.clone(), budget(5000, 1024)))
            .expect("start process");
        let result = await_terminal(&manager, &owner, &job).await;
        assert_eq!(result.status, ProcessStatus::OutputLimitExceeded);
        assert!(result.output_truncated);
        let stdout = manager
            .read(&owner, &job, ProcessStream::Stdout, 0, 1024)
            .expect("read bounded stdout");
        assert!(stdout.bytes.len() <= 1024);
        assert!(stdout.truncated);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn terminal_session_record_reap_is_owner_scoped_and_preserves_running_jobs() {
        let root = workspace("session-lifecycle-reap");
        fs::write(root.join("fixture-sleep"), b"1").expect("write fixture mode");
        let manager =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");

        let a_running = manager
            .start(spec(&root, session_a.clone(), budget(5000, 1024)))
            .expect("start A running");
        let b_running = manager
            .start(spec(&root, session_b.clone(), budget(5000, 1024)))
            .expect("start B running");

        assert_eq!(
            manager
                .active_session_job_count(&session_a)
                .expect("A count"),
            1
        );
        assert_eq!(
            manager
                .remove_terminal_session_records(&session_a)
                .expect("reap A"),
            0
        );
        manager
            .result(&session_a, &a_running)
            .expect("A running job must remain");
        manager
            .result(&session_b, &b_running)
            .expect("B running job must remain");

        assert!(manager.stop(&session_a, &a_running).expect("stop A"));
        let _ = await_terminal(&manager, &session_a, &a_running).await;
        assert_eq!(
            manager
                .active_session_job_count(&session_a)
                .expect("A count"),
            0
        );
        assert_eq!(
            manager
                .remove_terminal_session_records(&session_a)
                .expect("reap A"),
            1
        );
        assert!(matches!(
            manager.result(&session_a, &a_running),
            Err(ProcessError::UnknownJob)
        ));
        manager
            .result(&session_b, &b_running)
            .expect("B job must remain after A reap");

        assert!(manager.stop(&session_b, &b_running).expect("stop B"));
        let _ = await_terminal(&manager, &session_b, &b_running).await;
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn per_session_active_job_limit_does_not_block_other_session() {
        let root = workspace("session-active-limit");
        fs::write(root.join("fixture-sleep"), b"1").expect("write fixture mode");
        let limits = HardLimits {
            max_active_process_jobs: 2,
            max_active_process_jobs_per_session: 1,
            ..HardLimits::default()
        };
        let manager = ProcessManager::new(&root, limits, Vec::new()).expect("process manager");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");

        let a1 = manager
            .start(spec(&root, session_a.clone(), budget(5000, 1024)))
            .expect("start A1");
        assert!(matches!(
            manager.start(spec(&root, session_a.clone(), budget(5000, 1024))),
            Err(ProcessError::TooManyActiveJobsForSession)
        ));
        let b1 = manager
            .start(spec(&root, session_b.clone(), budget(5000, 1024)))
            .expect("B must retain its own active slot");

        assert!(manager.stop(&session_a, &a1).expect("stop A1"));
        assert!(manager.stop(&session_b, &b1).expect("stop B1"));
        assert_eq!(
            await_terminal(&manager, &session_a, &a1).await.status,
            ProcessStatus::Stopped
        );
        assert_eq!(
            await_terminal(&manager, &session_b, &b1).await.status,
            ProcessStatus::Stopped
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn per_session_output_reservation_does_not_block_other_session() {
        let root = workspace("session-output-limit");
        fs::write(root.join("fixture-sleep"), b"1").expect("write fixture mode");
        let limits = HardLimits {
            max_active_output_ram_bytes: 2048,
            max_active_output_ram_bytes_per_session: 1024,
            max_process_budget: ResourceBudget {
                output_bytes: 1024,
                ..HardLimits::default().max_process_budget
            },
            ..HardLimits::default()
        };
        let manager = ProcessManager::new(&root, limits, Vec::new()).expect("process manager");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");

        let a1 = manager
            .start(spec(&root, session_a.clone(), budget(5000, 1024)))
            .expect("start A1");
        assert!(matches!(
            manager.start(spec(&root, session_a.clone(), budget(5000, 1024))),
            Err(ProcessError::OutputMemoryLimitExceededForSession)
        ));
        let b1 = manager
            .start(spec(&root, session_b.clone(), budget(5000, 1024)))
            .expect("B must retain its own output reservation");

        assert!(manager.stop(&session_a, &a1).expect("stop A1"));
        assert!(manager.stop(&session_b, &b1).expect("stop B1"));
        let _ = await_terminal(&manager, &session_a, &a1).await;
        let _ = await_terminal(&manager, &session_b, &b1).await;
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn record_pressure_never_evicts_another_sessions_terminal_job() {
        let root = workspace("session-record-isolation");
        fs::write(root.join("fixture-output"), b"1").expect("write fixture mode");
        let limits = HardLimits {
            max_active_process_jobs: 2,
            max_active_process_jobs_per_session: 1,
            max_process_records: 2,
            max_process_records_per_session: 1,
            ..HardLimits::default()
        };
        let manager = ProcessManager::new(&root, limits, Vec::new()).expect("process manager");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");

        let b1 = manager
            .start(spec(&root, session_b.clone(), budget(2000, 1024)))
            .expect("start B1");
        assert_eq!(
            await_terminal(&manager, &session_b, &b1).await.status,
            ProcessStatus::Exited
        );
        let a1 = manager
            .start(spec(&root, session_a.clone(), budget(2000, 1024)))
            .expect("start A1");
        assert_eq!(
            await_terminal(&manager, &session_a, &a1).await.status,
            ProcessStatus::Exited
        );

        let a2 = manager
            .start(spec(&root, session_a.clone(), budget(2000, 1024)))
            .expect("A may retire only its own terminal history");
        assert!(matches!(
            manager.result(&session_a, &a1),
            Err(ProcessError::UnknownJob)
        ));
        assert_eq!(
            manager
                .result(&session_b, &b1)
                .expect("B1 must remain observable")
                .status,
            ProcessStatus::Exited
        );
        let _ = await_terminal(&manager, &session_a, &a2).await;
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
