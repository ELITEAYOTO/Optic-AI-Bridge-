mod output;
#[cfg(windows)]
use output::append_output;
use output::{OutputState, drain_output, mark_stream_closed};

#[cfg(windows)]
use std::ffi::OsString;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    future::Future,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use optic_bridge_core::{
    HardLimits, JobId, LimitError, NetworkAccess, ProcessExecutionClass, ResourceBudget,
    SessionHandle, WorkloadClass, WorkspacePath,
};
#[cfg(windows)]
use optic_bridge_windows::{
    HostMemorySnapshot, LimitedJobObject, PinnedDirectoryChain, PinnedDirectoryError,
    PinnedExecutableFile, open_pinned_executable, pin_directory_chain, query_host_memory_snapshot,
};
use process_wrap::tokio::{ChildWrapper, CommandWrap};
#[cfg(unix)]
use process_wrap::tokio::{KillOnDrop, ProcessGroup};
#[cfg(windows)]
use serde::Serialize;
use thiserror::Error;
#[cfg(windows)]
use tokio::io::AsyncWriteExt;
use tokio::{process::Command, runtime::Handle, time::Instant};

const MONITOR_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TERMINATION_CONFIRM_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(windows)]
const ISOLATION_LAUNCHER_PROTOCOL_VERSION: u32 = 2;
#[cfg(windows)]
const ISOLATION_LAUNCHER_REQUEST_LIMIT_BYTES: usize = 64 * 1024;
#[cfg(windows)]
const ISOLATION_LAUNCHER_MAX_ARGS: usize = 128;
#[cfg(windows)]
const ISOLATION_LAUNCHER_MAX_READ_FILES: usize = 32;
#[cfg(windows)]
const ISOLATION_LAUNCHER_FAILURE_EXIT: i32 = 126;
#[cfg(windows)]
fn isolation_launcher_environment_from<F>(
    mut lookup: F,
) -> Result<Vec<(&'static str, OsString)>, ProcessError>
where
    F: FnMut(&str) -> Option<OsString>,
{
    let system_root =
        lookup("SystemRoot").ok_or(ProcessError::IsolationEnvironmentUnavailable("SystemRoot"))?;
    let local_appdata = lookup("LOCALAPPDATA").ok_or(
        ProcessError::IsolationEnvironmentUnavailable("LOCALAPPDATA"),
    )?;
    let temp = lookup("TEMP");
    let tmp = lookup("TMP");
    let fallback = temp
        .clone()
        .or_else(|| tmp.clone())
        .ok_or(ProcessError::IsolationEnvironmentUnavailable("TEMP/TMP"))?;
    let temp = temp.unwrap_or_else(|| fallback.clone());
    let tmp = tmp.unwrap_or(fallback);

    Ok(vec![
        ("SystemRoot", system_root),
        ("LOCALAPPDATA", local_appdata),
        ("TEMP", temp),
        ("TMP", tmp),
    ])
}

#[cfg(windows)]
#[derive(Debug)]
struct IsolationLauncher {
    path: PathBuf,
    _pin: PinnedExecutableFile,
}

#[cfg(windows)]
trait HostMemoryProvider: std::fmt::Debug + Send + Sync {
    fn snapshot(&self) -> std::io::Result<HostMemorySnapshot>;
}

#[cfg(windows)]
#[derive(Debug)]
struct WindowsHostMemoryProvider;

#[cfg(windows)]
impl HostMemoryProvider for WindowsHostMemoryProvider {
    fn snapshot(&self) -> std::io::Result<HostMemorySnapshot> {
        query_host_memory_snapshot()
    }
}

#[cfg(windows)]
#[derive(Serialize)]
struct IsolationLauncherRequest<'a> {
    version: u32,
    executable: &'a str,
    args: &'a [String],
    cwd: &'a str,
    timeout_ms: u64,
    workspace_root: &'a str,
    workspace_read_files: &'a [String],
}

#[derive(Clone, Debug)]
pub struct ProcessStartSpec {
    pub session: SessionHandle,
    pub class: ProcessExecutionClass,
    pub workload_class: WorkloadClass,
    pub network: NetworkAccess,
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: Option<WorkspacePath>,
    pub workspace_read_files: Vec<WorkspacePath>,
    pub env_allowlist: Vec<String>,
    pub resources: ResourceBudget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvironmentVariableClass {
    Benign,
    Sensitive,
    Forbidden,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentGrant {
    pub name: String,
    pub class: EnvironmentVariableClass,
}

impl EnvironmentGrant {
    #[must_use]
    pub fn benign(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            class: EnvironmentVariableClass::Benign,
        }
    }
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
    TerminationUncertain,
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
    environment_grants: BTreeMap<String, EnvironmentVariableClass>,
    #[cfg(windows)]
    isolation_launcher: Option<IsolationLauncher>,
    #[cfg(windows)]
    host_memory_provider: Arc<dyn HostMemoryProvider>,
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
    private_action: Option<optic_bridge_core::ActionId>,
    sequence: u64,
    workload_class: WorkloadClass,
    reserved_output_bytes: u64,
    reserved_memory_bytes: u64,
    reserved_cpu_percent: u32,
    #[cfg(windows)]
    cwd_pin: Mutex<Option<PinnedDirectoryChain>>,
    output: Mutex<OutputState>,
    state: Mutex<JobState>,
    stop_requested: Arc<AtomicBool>,
    output_overflow: AtomicBool,
    output_read_failed: AtomicBool,
}

#[derive(Clone, Copy, Debug)]
struct JobState {
    status: ProcessStatus,
    exit_code: Option<i32>,
}

#[cfg(windows)]
fn effective_host_memory_headroom_bytes(limits: &HardLimits, total_physical_bytes: u64) -> u64 {
    let proportional = (u128::from(total_physical_bytes)
        * u128::from(limits.min_host_memory_headroom_percent))
    .div_ceil(100);
    let proportional = u64::try_from(proportional).unwrap_or(u64::MAX);
    limits.min_host_memory_headroom_bytes.max(proportional)
}

impl ProcessManager {
    pub fn new(
        root: impl AsRef<Path>,
        limits: HardLimits,
        allowed_env_vars: impl IntoIterator<Item = String>,
    ) -> Result<Self, ProcessError> {
        Self::new_with_environment_grants(
            root,
            limits,
            allowed_env_vars.into_iter().map(EnvironmentGrant::benign),
        )
    }

    pub fn new_with_environment_grants(
        root: impl AsRef<Path>,
        limits: HardLimits,
        grants: impl IntoIterator<Item = EnvironmentGrant>,
    ) -> Result<Self, ProcessError> {
        let limits = limits.validate_nonzero()?;
        let root = fs::canonicalize(root).map_err(ProcessError::Io)?;
        if !root.is_dir() {
            return Err(ProcessError::RootNotDirectory);
        }
        let mut environment_grants = BTreeMap::new();
        for grant in grants {
            let normalized = normalize_env_name(&grant.name)?;
            match environment_grants.insert(normalized, grant.class) {
                Some(existing) if existing != grant.class => {
                    return Err(ProcessError::ConflictingEnvironmentGrant);
                }
                _ => {}
            }
        }

        Ok(Self {
            root,
            limits,
            environment_grants,
            #[cfg(windows)]
            isolation_launcher: None,
            #[cfg(windows)]
            host_memory_provider: Arc::new(WindowsHostMemoryProvider),
            jobs: Arc::new(Mutex::new(JobStore::default())),
            sequence: AtomicU64::new(0),
        })
    }

    #[cfg(windows)]
    pub fn new_with_isolation_launcher(
        root: impl AsRef<Path>,
        limits: HardLimits,
        allowed_env_vars: impl IntoIterator<Item = String>,
        isolation_launcher: impl AsRef<Path>,
    ) -> Result<Self, ProcessError> {
        Self::new_with_isolation_launcher_and_environment_grants(
            root,
            limits,
            allowed_env_vars.into_iter().map(EnvironmentGrant::benign),
            isolation_launcher,
        )
    }

    #[cfg(windows)]
    pub fn new_with_isolation_launcher_and_environment_grants(
        root: impl AsRef<Path>,
        limits: HardLimits,
        grants: impl IntoIterator<Item = EnvironmentGrant>,
        isolation_launcher: impl AsRef<Path>,
    ) -> Result<Self, ProcessError> {
        let mut manager = Self::new_with_environment_grants(root, limits, grants)?;
        let launcher = isolation_launcher.as_ref();
        if !launcher.is_absolute() {
            return Err(ProcessError::IsolationLauncherMustBeAbsolute);
        }
        let canonical = fs::canonicalize(launcher).map_err(ProcessError::Io)?;
        if !canonical.is_file() {
            return Err(ProcessError::IsolationLauncherNotFile);
        }
        let pin = open_pinned_executable(&canonical).map_err(ProcessError::Io)?;
        manager.isolation_launcher = Some(IsolationLauncher {
            path: canonical,
            _pin: pin,
        });
        Ok(manager)
    }

    #[cfg(windows)]
    fn ensure_host_memory_headroom(
        &self,
        reserved_memory_bytes: u64,
        requested_memory_bytes: u64,
    ) -> Result<(), ProcessError> {
        let snapshot = self
            .host_memory_provider
            .snapshot()
            .map_err(|_| ProcessError::HostMemoryUnavailable)?;
        if snapshot.total_physical_bytes == 0
            || snapshot.available_physical_bytes > snapshot.total_physical_bytes
        {
            return Err(ProcessError::HostMemoryUnavailable);
        }

        let reserve =
            effective_host_memory_headroom_bytes(&self.limits, snapshot.total_physical_bytes);
        let required = reserve
            .checked_add(reserved_memory_bytes)
            .and_then(|value| value.checked_add(requested_memory_bytes))
            .ok_or(ProcessError::HostMemoryHeadroomExceeded)?;
        if snapshot.available_physical_bytes < required {
            return Err(ProcessError::HostMemoryHeadroomExceeded);
        }
        Ok(())
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

    #[cfg(windows)]
    pub fn validate_workspace_read_files(
        &self,
        paths: &[WorkspacePath],
    ) -> Result<(), ProcessError> {
        self.resolve_workspace_read_files(paths).map(|_| ())
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
        #[cfg(windows)]
        let cwd_pin = pin_directory_chain(
            &self.root,
            spec.cwd.as_ref().map(|cwd| Path::new(cwd.as_str())),
        )?;
        #[cfg(windows)]
        let cwd = cwd_pin.final_path().to_path_buf();
        #[cfg(not(windows))]
        let cwd = self.resolve_cwd(spec.cwd.as_ref())?;
        let environment = self.resolve_environment(&spec.env_allowlist)?;
        let requires_isolation = spec.network == NetworkAccess::Denied
            || matches!(
                spec.class,
                ProcessExecutionClass::Interpreter | ProcessExecutionClass::RepositoryCode
            );
        if !requires_isolation && !spec.workspace_read_files.is_empty() {
            return Err(ProcessError::WorkspaceReadGrantsRequireIsolation);
        }
        #[cfg(not(windows))]
        if requires_isolation {
            return Err(ProcessError::IsolationUnavailable);
        }
        #[cfg(windows)]
        let isolation_environment = if requires_isolation {
            Some(self.resolve_isolation_launcher_environment()?)
        } else {
            None
        };
        #[cfg(windows)]
        let workspace_read_files = if requires_isolation {
            self.resolve_workspace_read_files(&spec.workspace_read_files)?
        } else {
            Vec::new()
        };
        #[cfg(windows)]
        let isolation_request = if requires_isolation {
            let launcher = self
                .isolation_launcher
                .as_ref()
                .ok_or(ProcessError::IsolationUnavailable)?;
            if spec.args.len() > ISOLATION_LAUNCHER_MAX_ARGS {
                return Err(ProcessError::TooManyProcessArguments);
            }
            let cwd = cwd.to_str().ok_or(ProcessError::NonUtf8WorkingDirectory)?;
            let workspace_root = self
                .root
                .to_str()
                .ok_or(ProcessError::NonUtf8WorkspaceRoot)?;
            let request = IsolationLauncherRequest {
                version: ISOLATION_LAUNCHER_PROTOCOL_VERSION,
                executable: &executable,
                args: &spec.args,
                cwd,
                timeout_ms: resources.timeout_ms,
                workspace_root,
                workspace_read_files: &workspace_read_files,
            };
            let payload =
                serde_json::to_vec(&request).map_err(ProcessError::IsolationLauncherProtocol)?;
            if payload.len() > ISOLATION_LAUNCHER_REQUEST_LIMIT_BYTES {
                return Err(ProcessError::RequestShapeTooLarge);
            }
            Some((launcher.path.clone(), payload))
        } else {
            None
        };
        #[cfg(windows)]
        let kernel_process_count = if isolation_request.is_some() {
            resources
                .process_count
                .checked_add(1)
                .ok_or(ProcessError::ResourceBudgetExceeded)?
        } else {
            resources.process_count
        };
        let runtime = Handle::try_current().map_err(|_| ProcessError::RuntimeUnavailable)?;

        let job_id = JobId::generate().map_err(|_| ProcessError::JobIdUnavailable)?;
        let sequence = self.sequence.fetch_add(1, Ordering::AcqRel);
        let owner = spec.session.clone();
        let workload_class = spec.workload_class;
        let reserved_cpu_percent = self.limits.max_process_cpu_percent_per_job;
        let record = Arc::new(JobRecord {
            owner: spec.session,
            private_action: None,
            sequence,
            workload_class,
            reserved_output_bytes: resources.output_bytes,
            reserved_memory_bytes: resources.memory_bytes,
            reserved_cpu_percent,
            #[cfg(windows)]
            cwd_pin: Mutex::new(Some(cwd_pin)),
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
            stop_requested: Arc::new(AtomicBool::new(false)),
            output_overflow: AtomicBool::new(false),
            output_read_failed: AtomicBool::new(false),
        });

        #[cfg(windows)]
        let limited_job;
        {
            let mut store = self
                .jobs
                .lock()
                .map_err(|_| ProcessError::StateUnavailable)?;
            self.prepare_store_for_start_with_workload(
                &mut store,
                &owner,
                workload_class,
                resources.output_bytes,
                resources.memory_bytes,
                reserved_cpu_percent,
            )?;
            // Establish the Windows kernel limits before publishing the JobRecord. If
            // Job Object creation/configuration fails, no output/CPU/memory reservation
            // is visible in the store and there is nothing to roll back.
            #[cfg(windows)]
            {
                limited_job = LimitedJobObject::new(
                    kernel_process_count,
                    resources.memory_bytes,
                    reserved_cpu_percent,
                )
                .map_err(ProcessError::Io)?;
            }
            store.jobs.insert(job_id.clone(), Arc::clone(&record));
        }

        #[cfg(windows)]
        let (spawn_executable, launcher_payload) = match isolation_request {
            Some((launcher, payload)) => (launcher, Some(payload)),
            None => (PathBuf::from(&executable), None),
        };
        #[cfg(not(windows))]
        let spawn_executable = PathBuf::from(&executable);

        let mut command = Command::new(&spawn_executable);
        #[cfg(windows)]
        if requires_isolation {
            command.stdin(Stdio::piped());
        } else {
            command.args(&spec.args).stdin(Stdio::null());
        }
        #[cfg(not(windows))]
        command.args(&spec.args).stdin(Stdio::null());
        command
            .current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();
        #[cfg(windows)]
        if let Some(isolation_environment) = isolation_environment {
            for (name, value) in isolation_environment {
                command.env(name, value);
            }
        }
        for (name, value) in environment {
            command.env(name, value);
        }

        let mut command = CommandWrap::from(command);
        #[cfg(windows)]
        command.wrap(limited_job);
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
        #[cfg(windows)]
        let launcher_stdin = launcher_payload.and_then(|payload| {
            child
                .inner_mut()
                .stdin()
                .take()
                .map(|stdin| (stdin, payload))
        });
        let stdout = child.stdout().take();
        let stderr = child.stderr().take();

        #[cfg(windows)]
        if let Some((mut stdin, payload)) = launcher_stdin {
            std::mem::drop(runtime.spawn(async move {
                let _ = stdin.write_all(&payload).await;
                let _ = stdin.shutdown().await;
            }));
        }

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
        #[cfg(windows)]
        let internal_failure_exit = requires_isolation.then_some(ISOLATION_LAUNCHER_FAILURE_EXIT);
        #[cfg(not(windows))]
        let internal_failure_exit: Option<i32> = None;
        let monitor_record = Arc::clone(&record);
        runtime.spawn(async move {
            monitor_child(
                &mut *child,
                monitor_record,
                resources.timeout_ms,
                internal_failure_exit,
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
            truncated: record.output_overflow.load(Ordering::Acquire)
                || record.output_read_failed.load(Ordering::Acquire),
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
            output_truncated: record.output_overflow.load(Ordering::Acquire)
                || record.output_read_failed.load(Ordering::Acquire),
        })
    }

    /// Reconcile a lost private-task start reply without launching it again.
    /// This scans bounded retained records, is owner-scoped, and is not a
    /// durable restart record or a promise after terminal-history eviction.
    pub fn find_recorded_private_action(
        &self,
        session: &SessionHandle,
        action: &optic_bridge_core::ActionId,
    ) -> Result<Option<JobId>, ProcessError> {
        let store = self
            .jobs
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?;
        Ok(store.jobs.iter().find_map(|(id, record)| {
            (&record.owner == session && record.private_action.as_ref() == Some(action))
                .then(|| id.clone())
        }))
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
            if !status_holds_process_ownership(status) {
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
        for path in &spec.workspace_read_files {
            bytes = bytes.saturating_add(u64::try_from(path.as_str().len()).unwrap_or(u64::MAX));
        }
        if let Some(cwd) = &spec.cwd {
            bytes = bytes.saturating_add(u64::try_from(cwd.as_str().len()).unwrap_or(u64::MAX));
        }
        if bytes > self.limits.max_request_bytes {
            return Err(ProcessError::RequestShapeTooLarge);
        }
        Ok(())
    }

    pub fn normalize_environment_allowlist(
        &self,
        requested: &[String],
    ) -> Result<BTreeSet<String>, ProcessError> {
        let mut normalized_names = BTreeSet::new();
        for raw in requested {
            let normalized = normalize_env_name(raw)?;
            if self.environment_grants.get(&normalized) != Some(&EnvironmentVariableClass::Benign) {
                return Err(ProcessError::EnvironmentNotAllowed);
            }
            normalized_names.insert(normalized);
        }
        Ok(normalized_names)
    }

    fn resolve_environment(
        &self,
        requested: &[String],
    ) -> Result<Vec<(String, std::ffi::OsString)>, ProcessError> {
        let mut seen = BTreeSet::new();
        let mut output = Vec::new();
        for raw in requested {
            let normalized = normalize_env_name(raw)?;
            if self.environment_grants.get(&normalized) != Some(&EnvironmentVariableClass::Benign) {
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

    #[cfg(windows)]
    fn resolve_isolation_launcher_environment(
        &self,
    ) -> Result<Vec<(&'static str, OsString)>, ProcessError> {
        isolation_launcher_environment_from(|name| std::env::var_os(name))
    }

    #[cfg(windows)]
    fn resolve_workspace_read_files(
        &self,
        paths: &[WorkspacePath],
    ) -> Result<Vec<String>, ProcessError> {
        if paths.len() > ISOLATION_LAUNCHER_MAX_READ_FILES {
            return Err(ProcessError::TooManyWorkspaceReadGrants);
        }
        let mut seen = BTreeSet::new();
        let mut resolved = Vec::with_capacity(paths.len());
        for path in paths {
            let mut candidate = self.root.clone();
            for segment in path.as_str().split('/') {
                candidate.push(segment);
            }
            let file_name = candidate
                .file_name()
                .ok_or(ProcessError::WorkspaceReadGrantNotFile)?
                .to_owned();
            let parent = candidate
                .parent()
                .ok_or(ProcessError::WorkspaceReadGrantOutsideWorkspace)?;
            let canonical_parent = fs::canonicalize(parent).map_err(ProcessError::Io)?;
            if !canonical_parent.starts_with(&self.root) {
                return Err(ProcessError::WorkspaceReadGrantOutsideWorkspace);
            }
            let exact = canonical_parent.join(file_name);
            let metadata = fs::symlink_metadata(&exact).map_err(ProcessError::Io)?;
            if !metadata.file_type().is_file() {
                return Err(ProcessError::WorkspaceReadGrantNotFile);
            }
            if !seen.insert(exact.clone()) {
                return Err(ProcessError::DuplicateWorkspaceReadGrant);
            }
            let value = exact
                .to_str()
                .ok_or(ProcessError::NonUtf8WorkspaceReadGrant)?
                .to_owned();
            resolved.push(value);
        }
        Ok(resolved)
    }

    #[cfg(not(windows))]
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

    fn prepare_store_for_start_with_workload(
        &self,
        store: &mut JobStore,
        session: &SessionHandle,
        workload_class: WorkloadClass,
        requested_output_bytes: u64,
        requested_memory_bytes: u64,
        requested_cpu_percent: u32,
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

            if workload_class == WorkloadClass::Heavy {
                let session_heavy = active_heavy_job_count_for_session(store, session)?;
                if session_heavy >= self.limits.max_active_heavy_process_jobs_per_session {
                    return Err(ProcessError::HeavyWorkloadCapacityExceededForSession);
                }
                let heavy = active_heavy_job_count(store)?;
                if heavy >= self.limits.max_active_heavy_process_jobs {
                    return Err(ProcessError::HeavyWorkloadCapacityExceeded);
                }
            }

            let reserved = reserved_output_bytes(store);
            let session_reserved = reserved_output_bytes_for_session(store, session);
            let reserved_memory = reserved_process_memory_bytes(store)?;
            let session_reserved_memory =
                reserved_process_memory_bytes_for_session(store, session)?;
            let reserved_cpu = reserved_cpu_percent(store)?;
            let session_reserved_cpu = reserved_cpu_percent_for_session(store, session)?;
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
            let global_memory_limit_reached = reserved_memory
                .saturating_add(requested_memory_bytes)
                > self.limits.max_active_process_memory_bytes;
            let session_memory_limit_reached = session_reserved_memory
                .saturating_add(requested_memory_bytes)
                > self.limits.max_active_process_memory_bytes_per_session;
            let global_cpu_limit_reached = reserved_cpu.saturating_add(requested_cpu_percent)
                > self.limits.max_active_process_cpu_percent;
            let session_cpu_limit_reached = session_reserved_cpu
                .saturating_add(requested_cpu_percent)
                > self.limits.max_active_process_cpu_percent_per_session;

            if session_cpu_limit_reached {
                return Err(ProcessError::CpuCapacityExceededForSession);
            }
            if global_cpu_limit_reached {
                return Err(ProcessError::CpuCapacityExceeded);
            }
            if session_memory_limit_reached {
                return Err(ProcessError::ProcessMemoryCapacityExceededForSession);
            }
            if global_memory_limit_reached {
                return Err(ProcessError::ProcessMemoryCapacityExceeded);
            }

            #[cfg(windows)]
            self.ensure_host_memory_headroom(reserved_memory, requested_memory_bytes)?;

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

    #[cfg(test)]
    fn prepare_store_for_start(
        &self,
        store: &mut JobStore,
        session: &SessionHandle,
        requested_output_bytes: u64,
        requested_memory_bytes: u64,
        requested_cpu_percent: u32,
    ) -> Result<(), ProcessError> {
        self.prepare_store_for_start_with_workload(
            store,
            session,
            WorkloadClass::Standard,
            requested_output_bytes,
            requested_memory_bytes,
            requested_cpu_percent,
        )
    }

    fn remove_job(&self, job_id: &JobId) {
        if let Ok(mut store) = self.jobs.lock() {
            store.jobs.remove(job_id);
        }
    }
}

/// Internal producer for a native private-task worker. It shares the exact
/// process store/admission and output buffers used by ordinary managed jobs.
#[cfg(windows)]
pub(crate) struct NativeProcessReservation {
    id: JobId,
    record: Arc<JobRecord>,
    pub(crate) limits: LimitedJobObject,
    may_have_run: bool,
    finished: bool,
}

#[cfg(windows)]
impl ProcessManager {
    pub(crate) fn reserve_native_private(
        &self,
        spec: &ProcessStartSpec,
        action: &optic_bridge_core::ActionId,
    ) -> Result<NativeProcessReservation, ProcessError> {
        let resources = spec.resources.validate_nonzero()?;
        if !resources.fits_within(self.limits.max_process_budget)
            || resources.timeout_ms > u64::from(u32::MAX)
            || spec.network != NetworkAccess::Denied
            || !spec.workspace_read_files.is_empty()
            || !spec.env_allowlist.is_empty()
            || spec.cwd.is_some()
        {
            return Err(ProcessError::ResourceBudgetExceeded);
        }
        if resources.output_bytes > self.limits.max_active_output_ram_bytes {
            return Err(ProcessError::OutputMemoryLimitExceeded);
        }
        if resources.output_bytes > self.limits.max_active_output_ram_bytes_per_session {
            return Err(ProcessError::OutputMemoryLimitExceededForSession);
        }
        self.validate_request_shape(spec)?;
        let canonical = self.canonicalize_executable(&spec.executable)?;
        if canonical != spec.executable {
            return Err(ProcessError::ExecutableMustBeAbsolute);
        }
        let id = JobId::generate().map_err(|_| ProcessError::JobIdUnavailable)?;
        let record = Arc::new(JobRecord {
            owner: spec.session.clone(),
            private_action: Some(action.clone()),
            sequence: self.sequence.fetch_add(1, Ordering::AcqRel),
            workload_class: spec.workload_class,
            reserved_output_bytes: resources.output_bytes,
            reserved_memory_bytes: resources.memory_bytes,
            reserved_cpu_percent: self.limits.max_process_cpu_percent_per_job,
            cwd_pin: Mutex::new(None),
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
            stop_requested: Arc::new(AtomicBool::new(false)),
            output_overflow: AtomicBool::new(false),
            output_read_failed: AtomicBool::new(false),
        });
        let mut store = self
            .jobs
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?;
        if store.jobs.values().any(|record| {
            record.owner == spec.session && record.private_action.as_ref() == Some(action)
        }) {
            return Err(ProcessError::PrivateActionAlreadyRecorded);
        }
        self.prepare_store_for_start_with_workload(
            &mut store,
            &spec.session,
            spec.workload_class,
            resources.output_bytes,
            resources.memory_bytes,
            self.limits.max_process_cpu_percent_per_job,
        )?;
        let limits = LimitedJobObject::new(
            resources.process_count,
            resources.memory_bytes,
            self.limits.max_process_cpu_percent_per_job,
        )
        .map_err(ProcessError::Io)?;
        store.jobs.insert(id.clone(), record.clone());
        Ok(NativeProcessReservation {
            id,
            record,
            limits,
            may_have_run: false,
            finished: false,
        })
    }
}

#[cfg(windows)]
impl NativeProcessReservation {
    pub(crate) fn id(&self) -> JobId {
        self.id.clone()
    }
    pub(crate) fn mark_native_created(&mut self) {
        self.may_have_run = true;
    }
    pub(crate) fn stop_requested(&self) -> bool {
        self.record.stop_requested.load(Ordering::Acquire)
    }
    pub(crate) fn stop_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.record.stop_requested)
    }
    pub(crate) fn overflowed(&self) -> bool {
        self.record.output_overflow.load(Ordering::Acquire)
    }
    pub(crate) fn truncate_output(&self) {
        self.record.output_overflow.store(true, Ordering::Release);
    }
    pub(crate) fn append(&self, stream: ProcessStream, bytes: &[u8]) {
        append_output(&self.record, stream, bytes);
    }
    pub(crate) fn finish(&mut self, status: ProcessStatus, code: Option<i32>) {
        mark_stream_closed(&self.record, ProcessStream::Stdout);
        mark_stream_closed(&self.record, ProcessStream::Stderr);
        if let Ok(mut state) = self.record.state.lock() {
            *state = JobState {
                status: if status == ProcessStatus::Running {
                    ProcessStatus::TerminationUncertain
                } else {
                    status
                },
                exit_code: code,
            };
            self.finished = true;
        }
    }
}

#[cfg(windows)]
impl Drop for NativeProcessReservation {
    fn drop(&mut self) {
        if !self.finished {
            self.record.output_overflow.store(true, Ordering::Release);
            self.finish(
                if self.may_have_run {
                    ProcessStatus::TerminationUncertain
                } else {
                    ProcessStatus::Failed
                },
                None,
            );
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

fn active_heavy_job_count(store: &JobStore) -> Result<u32, ProcessError> {
    active_job_count_matching(store, |record| {
        record.workload_class == WorkloadClass::Heavy
    })
}

fn active_heavy_job_count_for_session(
    store: &JobStore,
    session: &SessionHandle,
) -> Result<u32, ProcessError> {
    active_job_count_matching(store, |record| {
        &record.owner == session && record.workload_class == WorkloadClass::Heavy
    })
}

fn active_job_count_matching(
    store: &JobStore,
    matches: impl Fn(&JobRecord) -> bool,
) -> Result<u32, ProcessError> {
    let mut active = 0_u32;
    for record in store.jobs.values() {
        let status = record
            .state
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?
            .status;
        if matches(record) && status_holds_process_ownership(status) {
            active = active.saturating_add(1);
        }
    }
    Ok(active)
}

const fn status_holds_process_ownership(status: ProcessStatus) -> bool {
    matches!(
        status,
        ProcessStatus::Running | ProcessStatus::TerminationUncertain
    )
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

fn reserved_process_memory_bytes(store: &JobStore) -> Result<u64, ProcessError> {
    reserved_process_memory_bytes_matching(store, |_| true)
}

fn reserved_process_memory_bytes_for_session(
    store: &JobStore,
    session: &SessionHandle,
) -> Result<u64, ProcessError> {
    reserved_process_memory_bytes_matching(store, |record| &record.owner == session)
}

fn reserved_process_memory_bytes_matching(
    store: &JobStore,
    matches: impl Fn(&JobRecord) -> bool,
) -> Result<u64, ProcessError> {
    let mut reserved = 0_u64;
    for record in store.jobs.values() {
        let status = record
            .state
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?
            .status;
        if matches(record) && status_holds_process_ownership(status) {
            reserved = reserved.saturating_add(record.reserved_memory_bytes);
        }
    }
    Ok(reserved)
}

fn reserved_cpu_percent(store: &JobStore) -> Result<u32, ProcessError> {
    reserved_cpu_percent_matching(store, |_| true)
}

fn reserved_cpu_percent_for_session(
    store: &JobStore,
    session: &SessionHandle,
) -> Result<u32, ProcessError> {
    reserved_cpu_percent_matching(store, |record| &record.owner == session)
}

fn reserved_cpu_percent_matching(
    store: &JobStore,
    matches: impl Fn(&JobRecord) -> bool,
) -> Result<u32, ProcessError> {
    let mut reserved = 0_u32;
    for record in store.jobs.values() {
        let status = record
            .state
            .lock()
            .map_err(|_| ProcessError::StateUnavailable)?
            .status;
        if matches(record) && status_holds_process_ownership(status) {
            reserved = reserved.saturating_add(record.reserved_cpu_percent);
        }
    }
    Ok(reserved)
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
        if status_holds_process_ownership(status) {
            continue;
        }
        if oldest.is_none_or(|(_, sequence)| record.sequence < sequence) {
            oldest = Some((job_id, record.sequence));
        }
    }
    Ok(oldest.map(|(job_id, _)| job_id.clone()))
}

async fn monitor_child(
    child: &mut dyn ChildWrapper,
    record: Arc<JobRecord>,
    timeout_ms: u64,
    internal_failure_exit: Option<i32>,
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
) {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let (status, exit_code) = loop {
        let observation_failure = match child.try_wait() {
            Ok(Some(exit)) => {
                let code = exit.code();
                let status = if internal_failure_exit.is_some_and(|failure| code == Some(failure)) {
                    ProcessStatus::Failed
                } else {
                    ProcessStatus::Exited
                };
                break (status, code);
            }
            Err(_) => true,
            Ok(None) => false,
        };

        let reason = if observation_failure {
            Some(ProcessStatus::Failed)
        } else if record.stop_requested.load(Ordering::Acquire) {
            Some(ProcessStatus::Stopped)
        } else if record.output_read_failed.load(Ordering::Acquire) {
            Some(ProcessStatus::Failed)
        } else if record.output_overflow.load(Ordering::Acquire) {
            Some(ProcessStatus::OutputLimitExceeded)
        } else if Instant::now() >= deadline {
            Some(ProcessStatus::TimedOut)
        } else {
            None
        };

        if let Some(reason) = reason {
            // A failed kill request is not sufficient evidence that the process is still alive,
            // and a successful request is not sufficient evidence that it is dead. In both cases
            // require bounded OS confirmation before releasing process ownership.
            let kill_request = child.start_kill();
            let confirmation =
                wait_for_termination_confirmation(child.wait(), TERMINATION_CONFIRM_TIMEOUT).await;
            match (kill_request, confirmation) {
                (_, Some(exit)) => break (reason, exit.code()),
                (Ok(()), None) | (Err(_), None) => {
                    break (ProcessStatus::TerminationUncertain, None);
                }
            }
        }
        tokio::time::sleep(MONITOR_POLL_INTERVAL).await;
    };

    if status == ProcessStatus::TerminationUncertain {
        // Do not let inherited/hostile pipe handles turn a bounded termination failure back into an
        // unbounded monitor task. Aborting the drains deliberately reports truncated output.
        record.output_overflow.store(true, Ordering::Release);
        stdout_task.abort();
        stderr_task.abort();
    }
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    if status == ProcessStatus::TerminationUncertain {
        mark_stream_closed(&record, ProcessStream::Stdout);
        mark_stream_closed(&record, ProcessStream::Stderr);
    }
    let status = reconcile_drained_output_status(status, &record);
    if status != ProcessStatus::TerminationUncertain {
        #[cfg(windows)]
        if let Ok(mut cwd_pin) = record.cwd_pin.lock() {
            cwd_pin.take();
        }
    }
    if let Ok(mut state) = record.state.lock() {
        *state = JobState { status, exit_code };
    }
}

// Drain tasks are complete before this check. A failed pipe read must never
// turn an incomplete diagnostic stream into an apparently successful exit.
// A separate flag preserves the distinction from an output-budget overflow.
fn reconcile_drained_output_status(status: ProcessStatus, record: &JobRecord) -> ProcessStatus {
    if status == ProcessStatus::Exited {
        if record.output_read_failed.load(Ordering::Acquire) {
            ProcessStatus::Failed
        } else if record.output_overflow.load(Ordering::Acquire) {
            ProcessStatus::OutputLimitExceeded
        } else {
            status
        }
    } else {
        status
    }
}

async fn wait_for_termination_confirmation<F>(wait: F, timeout: Duration) -> Option<ExitStatus>
where
    F: Future<Output = std::io::Result<ExitStatus>>,
{
    match tokio::time::timeout(timeout, wait).await {
        Ok(Ok(status)) => Some(status),
        Ok(Err(_)) | Err(_) => None,
    }
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("private start action already has a retained process record; reconcile it")]
    PrivateActionAlreadyRecorded,
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
    #[error("canonical process working directory cannot be represented as UTF-8")]
    NonUtf8WorkingDirectory,
    #[error("process working directory resolves outside the workspace")]
    CwdOutsideWorkspace,
    #[error("process working directory must resolve to a directory")]
    CwdNotDirectory,
    #[cfg(windows)]
    #[error("process working directory could not be pinned safely: {0}")]
    PinnedWorkingDirectory(#[from] PinnedDirectoryError),
    #[error("process environment variable name is invalid")]
    InvalidEnvironmentName,
    #[error("process environment variable is not operator-authorized for MCP export")]
    EnvironmentNotAllowed,
    #[error("environment grants conflict after platform name normalization")]
    ConflictingEnvironmentGrant,
    #[error("requested process resources exceed the hard ceiling")]
    ResourceBudgetExceeded,
    #[error("process request shape exceeds the hard request byte ceiling")]
    RequestShapeTooLarge,
    #[error("too many process arguments for the isolation launcher protocol")]
    TooManyProcessArguments,
    #[error("too many process jobs are already active")]
    TooManyActiveJobs,
    #[error("this session already has too many active process jobs")]
    TooManyActiveJobsForSession,
    #[error("all bounded heavy process workload slots are already active")]
    HeavyWorkloadCapacityExceeded,
    #[error("this session already holds its bounded heavy process workload slot")]
    HeavyWorkloadCapacityExceededForSession,
    #[error("bounded process record history is full")]
    ProcessRecordLimitExceeded,
    #[error("this session's bounded process record history is full")]
    ProcessRecordLimitExceededForSession,
    #[error("reserved process output would exceed the hard in-memory ceiling")]
    OutputMemoryLimitExceeded,
    #[error("this session's reserved process output would exceed its hard in-memory ceiling")]
    OutputMemoryLimitExceededForSession,
    #[error("reserved process memory would exceed the hard aggregate Optic memory ceiling")]
    ProcessMemoryCapacityExceeded,
    #[error("this session's reserved process memory would exceed its hard memory ceiling")]
    ProcessMemoryCapacityExceededForSession,
    #[error("Windows host-memory telemetry is unavailable")]
    HostMemoryUnavailable,
    #[error("starting this process would violate the host-memory emergency headroom")]
    HostMemoryHeadroomExceeded,
    #[error("reserved process CPU would exceed the hard aggregate Optic CPU ceiling")]
    CpuCapacityExceeded,
    #[error("this session's reserved process CPU would exceed its hard CPU ceiling")]
    CpuCapacityExceededForSession,
    #[error("process output read exceeds the hard per-call byte ceiling")]
    ReadLimitExceeded,
    #[error("process output cursor is out of range")]
    CursorOutOfRange,
    #[error("process job is unknown to this session")]
    UnknownJob,
    #[error("workspace read grants require the isolated process path")]
    WorkspaceReadGrantsRequireIsolation,
    #[error("too many exact-file workspace read grants")]
    TooManyWorkspaceReadGrants,
    #[error("workspace read grant resolves outside the configured workspace")]
    WorkspaceReadGrantOutsideWorkspace,
    #[error("workspace read grant must resolve to one exact regular file")]
    WorkspaceReadGrantNotFile,
    #[error("workspace root is not valid UTF-8 for the internal isolation protocol")]
    NonUtf8WorkspaceRoot,
    #[error("workspace read grant path is not valid UTF-8")]
    NonUtf8WorkspaceReadGrant,
    #[error("workspace read grant aliases another requested exact file")]
    DuplicateWorkspaceReadGrant,
    #[error("process isolation launcher must be an absolute path")]
    IsolationLauncherMustBeAbsolute,
    #[error("process isolation launcher must resolve to a regular file")]
    IsolationLauncherNotFile,
    #[error("process isolation launcher protocol serialization failed: {0}")]
    IsolationLauncherProtocol(serde_json::Error),
    #[error("required Windows process isolation environment variable is unavailable: {0}")]
    IsolationEnvironmentUnavailable(&'static str),
    #[error("required process isolation profile is unavailable")]
    IsolationUnavailable,
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
mod tests;
