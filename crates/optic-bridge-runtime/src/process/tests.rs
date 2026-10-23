use std::{env, fs, thread};

use optic_bridge_core::ActionId;

use super::*;

#[cfg(windows)]
#[derive(Clone, Copy, Debug)]
enum FakeHostMemoryObservation {
    Snapshot(HostMemorySnapshot),
    Error,
}

#[cfg(windows)]
#[derive(Debug)]
struct FakeHostMemoryProvider {
    observation: FakeHostMemoryObservation,
}

#[cfg(windows)]
impl HostMemoryProvider for FakeHostMemoryProvider {
    fn snapshot(&self) -> std::io::Result<HostMemorySnapshot> {
        match self.observation {
            FakeHostMemoryObservation::Snapshot(snapshot) => Ok(snapshot),
            FakeHostMemoryObservation::Error => {
                Err(std::io::Error::other("fake host-memory failure"))
            }
        }
    }
}

#[cfg(windows)]
fn manager_with_host_memory(
    root: &Path,
    limits: HardLimits,
    observation: FakeHostMemoryObservation,
) -> ProcessManager {
    let mut manager = ProcessManager::new(root, limits, Vec::new()).expect("process manager");
    manager.host_memory_provider = std::sync::Arc::new(FakeHostMemoryProvider { observation });
    manager
}

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
        class: ProcessExecutionClass::FixedTool,
        workload_class: optic_bridge_core::WorkloadClass::Standard,
        network: NetworkAccess::Allowed,
        executable,
        args: vec![
            "--exact".to_owned(),
            "process::tests::process_fixture_child".to_owned(),
            "--nocapture".to_owned(),
        ],
        cwd: None,
        workspace_read_files: Vec::new(),
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

fn synthetic_record(
    owner: SessionHandle,
    sequence: u64,
    status: ProcessStatus,
    reserved_memory_bytes: u64,
    reserved_cpu_percent: u32,
) -> Arc<JobRecord> {
    synthetic_record_with_workload(
        owner,
        sequence,
        status,
        reserved_memory_bytes,
        reserved_cpu_percent,
        WorkloadClass::Standard,
    )
}

fn synthetic_record_with_workload(
    owner: SessionHandle,
    sequence: u64,
    status: ProcessStatus,
    reserved_memory_bytes: u64,
    reserved_cpu_percent: u32,
    workload_class: WorkloadClass,
) -> Arc<JobRecord> {
    Arc::new(JobRecord {
        owner,
        private_action: None,
        sequence,
        workload_class,
        reserved_output_bytes: 1,
        reserved_memory_bytes,
        reserved_cpu_percent,
        #[cfg(windows)]
        cwd_pin: Mutex::new(None),
        output: Mutex::new(OutputState {
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_closed: true,
            stderr_closed: true,
        }),
        state: Mutex::new(JobState {
            status,
            exit_code: None,
        }),
        stop_requested: Arc::new(AtomicBool::new(false)),
        output_overflow: AtomicBool::new(false),
        output_read_failed: AtomicBool::new(false),
    })
}

struct BrokenPipeReader;

impl tokio::io::AsyncRead for BrokenPipeReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Err(std::io::Error::other("synthetic pipe read failure")))
    }
}

#[tokio::test]
async fn pipe_read_failure_is_not_silent_clean_eof_or_success() {
    let root = workspace("failed-pipe-read");
    let manager = ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap();
    let owner = SessionHandle::generate().unwrap();
    let record = synthetic_record(
        owner.clone(),
        0,
        ProcessStatus::Running,
        64 * 1024 * 1024,
        25,
    );
    let job = JobId::generate().unwrap();
    manager
        .jobs
        .lock()
        .unwrap()
        .jobs
        .insert(job.clone(), record.clone());

    drain_output(BrokenPipeReader, ProcessStream::Stdout, record.clone()).await;
    assert!(record.output_read_failed.load(Ordering::Acquire));
    assert!(!record.output_overflow.load(Ordering::Acquire));
    let chunk = manager
        .read(&owner, &job, ProcessStream::Stdout, 0, 1024)
        .unwrap();
    assert!(chunk.eof);
    assert!(
        chunk.truncated,
        "read error cannot masquerade as complete output"
    );

    let final_status = reconcile_drained_output_status(ProcessStatus::Exited, &record);
    assert_eq!(final_status, ProcessStatus::Failed);
    assert_eq!(
        reconcile_drained_output_status(ProcessStatus::TimedOut, &record),
        ProcessStatus::TimedOut,
        "a bounded timeout must retain its original reason"
    );
    record.state.lock().unwrap().status = final_status;
    let result = manager.result(&owner, &job).unwrap();
    assert_eq!(result.status, ProcessStatus::Failed);
    assert!(result.output_truncated);
    drop(manager);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn clean_output_eof_retains_success_status() {
    let record = synthetic_record(
        SessionHandle::generate().unwrap(),
        0,
        ProcessStatus::Running,
        64 * 1024 * 1024,
        25,
    );
    drain_output(tokio::io::empty(), ProcessStream::Stdout, record.clone()).await;
    assert!(!record.output_read_failed.load(Ordering::Acquire));
    assert!(!record.output_overflow.load(Ordering::Acquire));
    assert!(record.output.lock().unwrap().stdout_closed);
    assert_eq!(
        reconcile_drained_output_status(ProcessStatus::Exited, &record),
        ProcessStatus::Exited
    );
}

#[tokio::test]
async fn failed_output_drain_join_does_not_masquerade_as_normal_exit() {
    let record = synthetic_record(
        SessionHandle::generate().unwrap(),
        0,
        ProcessStatus::Running,
        64 * 1024 * 1024,
        25,
    );
    record.output.lock().unwrap().stdout_closed = false;
    let crashed = tokio::spawn(async {
        panic!("synthetic output drain panic");
    });
    join_output_drain(&record, ProcessStream::Stdout, crashed, false).await;
    assert!(record.output_read_failed.load(Ordering::Acquire));
    assert!(record.output.lock().unwrap().stdout_closed);
    assert_eq!(
        reconcile_drained_output_status(ProcessStatus::Exited, &record),
        ProcessStatus::Failed
    );
    assert_eq!(
        reconcile_drained_output_status(ProcessStatus::Stopped, &record),
        ProcessStatus::Stopped
    );
}

#[tokio::test]
async fn intentional_drain_abort_retains_termination_uncertainty_without_false_read_error() {
    let record = synthetic_record(
        SessionHandle::generate().unwrap(),
        0,
        ProcessStatus::TerminationUncertain,
        64 * 1024 * 1024,
        25,
    );
    let aborted = tokio::spawn(std::future::pending::<()>());
    aborted.abort();
    join_output_drain(&record, ProcessStream::Stderr, aborted, true).await;
    assert!(!record.output_read_failed.load(Ordering::Acquire));
    assert_eq!(
        reconcile_drained_output_status(ProcessStatus::TerminationUncertain, &record),
        ProcessStatus::TerminationUncertain
    );
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
fn classified_environment_grants_export_only_benign_names() {
    let root = workspace("env-grants");
    let manager = ProcessManager::new_with_environment_grants(
        &root,
        HardLimits::default(),
        [
            EnvironmentGrant {
                name: "OPTIC_BENIGN".to_owned(),
                class: EnvironmentVariableClass::Benign,
            },
            EnvironmentGrant {
                name: "OPTIC_SECRET".to_owned(),
                class: EnvironmentVariableClass::Sensitive,
            },
            EnvironmentGrant {
                name: "OPTIC_FORBIDDEN".to_owned(),
                class: EnvironmentVariableClass::Forbidden,
            },
        ],
    )
    .expect("classified manager");

    assert_eq!(
        manager
            .normalize_environment_allowlist(&["OPTIC_BENIGN".to_owned()])
            .expect("benign export"),
        BTreeSet::from([normalize_env_name("OPTIC_BENIGN").expect("name")])
    );
    assert!(matches!(
        manager.normalize_environment_allowlist(&["OPTIC_SECRET".to_owned()]),
        Err(ProcessError::EnvironmentNotAllowed)
    ));
    assert!(matches!(
        manager.normalize_environment_allowlist(&["OPTIC_FORBIDDEN".to_owned()]),
        Err(ProcessError::EnvironmentNotAllowed)
    ));
    fs::remove_dir_all(root).expect("remove workspace");
}

#[test]
fn conflicting_environment_grants_fail_closed_after_normalization() {
    let root = workspace("env-conflict");
    let first = EnvironmentGrant {
        name: "OPTIC_CASE".to_owned(),
        class: EnvironmentVariableClass::Benign,
    };
    #[cfg(windows)]
    let second_name = "optic_case";
    #[cfg(not(windows))]
    let second_name = "OPTIC_CASE";
    let result = ProcessManager::new_with_environment_grants(
        &root,
        HardLimits::default(),
        [
            first,
            EnvironmentGrant {
                name: second_name.to_owned(),
                class: EnvironmentVariableClass::Sensitive,
            },
        ],
    );
    assert!(matches!(
        result,
        Err(ProcessError::ConflictingEnvironmentGrant)
    ));
    fs::remove_dir_all(root).expect("remove workspace");
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

#[test]
fn high_risk_execution_classes_fail_closed_in_runtime_before_spawn() {
    let root = workspace("isolation-class");
    let manager =
        ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
    let session = SessionHandle::generate().expect("session");

    for class in [
        ProcessExecutionClass::Interpreter,
        ProcessExecutionClass::RepositoryCode,
    ] {
        let mut request = spec(&root, session.clone(), budget(2_000, 64 * 1024));
        request.class = class;
        assert!(matches!(
            manager.start(request),
            Err(ProcessError::IsolationUnavailable)
        ));
    }
}

#[cfg(windows)]
#[test]
fn isolation_launcher_environment_accepts_temp_without_tmp() {
    let environment = isolation_launcher_environment_from(|name| match name {
        "SystemRoot" => Some(OsString::from(r"C:\Windows")),
        "LOCALAPPDATA" => Some(OsString::from(r"C:\Users\optic\AppData\Local")),
        "TEMP" => Some(OsString::from(r"C:\Users\optic\AppData\Local\Temp")),
        "TMP" => None,
        _ => None,
    })
    .expect("TEMP should satisfy the temp alias requirement");

    assert_eq!(environment[2].1, environment[3].1);
}

#[cfg(windows)]
#[test]
fn isolation_launcher_environment_accepts_tmp_without_temp() {
    let environment = isolation_launcher_environment_from(|name| match name {
        "SystemRoot" => Some(OsString::from(r"C:\Windows")),
        "LOCALAPPDATA" => Some(OsString::from(r"C:\Users\optic\AppData\Local")),
        "TEMP" => None,
        "TMP" => Some(OsString::from(r"C:\Temp")),
        _ => None,
    })
    .expect("TMP should satisfy the temp alias requirement");

    assert_eq!(environment[2].1, environment[3].1);
}

#[cfg(windows)]
#[test]
fn isolation_launcher_phase_diagnostics_only_forward_exact_one() {
    for value in [None, Some("0"), Some("untrusted"), Some("1")] {
        let environment = isolation_launcher_environment_from(|name| match name {
            "SystemRoot" => Some(OsString::from(r"C:\Windows")),
            "LOCALAPPDATA" => Some(OsString::from(r"C:\Users\optic\AppData\Local")),
            "TEMP" | "TMP" => Some(OsString::from(r"C:\Temp")),
            "OPTIC_LAUNCHER_PHASES" => value.map(OsString::from),
            _ => None,
        })
        .expect("launcher environment");

        let diagnostic = environment
            .iter()
            .find(|(name, _)| *name == "OPTIC_LAUNCHER_PHASES");
        assert_eq!(diagnostic.is_some(), value == Some("1"));
        if let Some((_, forwarded)) = diagnostic {
            assert_eq!(forwarded, &OsString::from("1"));
        }
    }
}

#[cfg(windows)]
#[test]
fn isolation_launcher_environment_requires_one_temp_alias() {
    let error = isolation_launcher_environment_from(|name| match name {
        "SystemRoot" => Some(OsString::from(r"C:\Windows")),
        "LOCALAPPDATA" => Some(OsString::from(r"C:\Users\optic\AppData\Local")),
        "TEMP" | "TMP" => None,
        _ => None,
    })
    .expect_err("missing TEMP and TMP must fail closed");

    assert!(matches!(
        error,
        ProcessError::IsolationEnvironmentUnavailable("TEMP/TMP")
    ));
}

#[test]
fn workspace_read_grants_require_isolated_process_path() {
    let root = workspace("grant-fixed-tool");
    let manager =
        ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
    let session = SessionHandle::generate().expect("session");
    let mut request = spec(&root, session, budget(2_000, 64 * 1024));
    request.workspace_read_files =
        vec![WorkspacePath::parse("sentinel.txt").expect("workspace read path")];
    assert!(matches!(
        manager.start(request),
        Err(ProcessError::WorkspaceReadGrantsRequireIsolation)
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(windows)]
#[test]
fn workspace_read_grants_resolve_only_exact_regular_files() {
    let root = workspace("grant-resolution");
    fs::write(root.join("sentinel.txt"), b"sentinel").expect("write sentinel");
    fs::create_dir(root.join("directory")).expect("create directory");
    let manager =
        ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
    let sentinel = WorkspacePath::parse("sentinel.txt").expect("sentinel path");

    let resolved = manager
        .resolve_workspace_read_files(std::slice::from_ref(&sentinel))
        .expect("resolve exact file");
    assert_eq!(resolved.len(), 1);
    assert_eq!(
        PathBuf::from(&resolved[0]),
        root.canonicalize()
            .expect("canonical root")
            .join("sentinel.txt")
    );

    assert!(matches!(
        manager.resolve_workspace_read_files(&[sentinel.clone(), sentinel]),
        Err(ProcessError::DuplicateWorkspaceReadGrant)
    ));
    assert!(matches!(
        manager.resolve_workspace_read_files(&[
            WorkspacePath::parse("directory").expect("directory path")
        ]),
        Err(ProcessError::WorkspaceReadGrantNotFile)
    ));
    let too_many = (0..=ISOLATION_LAUNCHER_MAX_READ_FILES)
        .map(|index| WorkspacePath::parse(&format!("file-{index}.txt")).expect("bounded path"))
        .collect::<Vec<_>>();
    assert!(matches!(
        manager.resolve_workspace_read_files(&too_many),
        Err(ProcessError::TooManyWorkspaceReadGrants)
    ));
    fs::remove_dir_all(root).expect("remove fixture");
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

#[cfg(windows)]
#[test]
fn host_memory_headroom_uses_absolute_or_percent_floor() {
    let limits = HardLimits::default();
    let gib = 1024_u64 * 1024 * 1024;
    assert_eq!(effective_host_memory_headroom_bytes(&limits, 8 * gib), gib);
    assert_eq!(
        effective_host_memory_headroom_bytes(&limits, 20 * gib),
        2 * gib
    );
}

#[cfg(windows)]
#[test]
fn host_memory_headroom_admission_is_bounded_and_fail_closed() {
    let root = workspace("host-memory-headroom");
    let gib = 1024_u64 * 1024 * 1024;
    let limits = HardLimits::default();
    let session = SessionHandle::generate().expect("session");

    let manager = manager_with_host_memory(
        &root,
        limits,
        FakeHostMemoryObservation::Snapshot(HostMemorySnapshot {
            total_physical_bytes: 8 * gib,
            available_physical_bytes: 3 * gib,
        }),
    );
    let mut store = manager.jobs.lock().expect("job store");
    assert!(
        manager
            .prepare_store_for_start(&mut store, &session, 1, 2 * gib, 1)
            .is_ok()
    );
    drop(store);

    let manager = manager_with_host_memory(
        &root,
        limits,
        FakeHostMemoryObservation::Snapshot(HostMemorySnapshot {
            total_physical_bytes: 8 * gib,
            available_physical_bytes: 3 * gib - 1,
        }),
    );
    let mut store = manager.jobs.lock().expect("job store");
    assert!(matches!(
        manager.prepare_store_for_start(&mut store, &session, 1, 2 * gib, 1),
        Err(ProcessError::HostMemoryHeadroomExceeded)
    ));
    drop(store);

    for observation in [
        FakeHostMemoryObservation::Error,
        FakeHostMemoryObservation::Snapshot(HostMemorySnapshot {
            total_physical_bytes: 0,
            available_physical_bytes: 0,
        }),
        FakeHostMemoryObservation::Snapshot(HostMemorySnapshot {
            total_physical_bytes: gib,
            available_physical_bytes: 2 * gib,
        }),
    ] {
        let manager = manager_with_host_memory(&root, limits, observation);
        let mut store = manager.jobs.lock().expect("job store");
        assert!(matches!(
            manager.prepare_store_for_start(&mut store, &session, 1, 1, 1),
            Err(ProcessError::HostMemoryUnavailable)
        ));
    }

    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(windows)]
#[test]
fn host_memory_headroom_counts_existing_declared_reservations() {
    let root = workspace("host-memory-reservations");
    let gib = 1024_u64 * 1024 * 1024;
    let limits = HardLimits::default();
    let owner = SessionHandle::generate().expect("owner");
    let other = SessionHandle::generate().expect("other");
    let existing_job = JobId::generate().expect("job id");

    for (available, expected_ok) in [(3 * gib, true), (3 * gib - 1, false)] {
        let manager = manager_with_host_memory(
            &root,
            limits,
            FakeHostMemoryObservation::Snapshot(HostMemorySnapshot {
                total_physical_bytes: 8 * gib,
                available_physical_bytes: available,
            }),
        );
        let mut store = manager.jobs.lock().expect("job store");
        store.jobs.insert(
            existing_job.clone(),
            synthetic_record(owner.clone(), 0, ProcessStatus::Running, gib, 1),
        );
        let result = manager.prepare_store_for_start(&mut store, &other, 1, gib, 1);
        if expected_ok {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(ProcessError::HostMemoryHeadroomExceeded)
            ));
        }
    }

    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(windows)]
#[test]
fn host_memory_failure_does_not_evict_terminal_history() {
    let root = workspace("host-memory-no-eviction");
    let defaults = HardLimits::default();
    let limits = HardLimits {
        max_active_process_jobs: 1,
        max_active_process_jobs_per_session: 1,
        max_active_heavy_process_jobs: 1,
        max_active_heavy_process_jobs_per_session: 1,
        max_process_records: 1,
        max_process_records_per_session: 1,
        ..defaults
    };
    let manager = manager_with_host_memory(&root, limits, FakeHostMemoryObservation::Error);
    let owner = SessionHandle::generate().expect("owner");
    let terminal_job = JobId::generate().expect("terminal job id");
    let mut store = manager.jobs.lock().expect("job store");
    store.jobs.insert(
        terminal_job.clone(),
        synthetic_record(owner.clone(), 0, ProcessStatus::Exited, 0, 0),
    );

    assert!(matches!(
        manager.prepare_store_for_start(&mut store, &owner, 1, 1, 1),
        Err(ProcessError::HostMemoryUnavailable)
    ));
    assert!(store.jobs.contains_key(&terminal_job));
    drop(store);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(windows)]
#[test]
fn declared_memory_capacity_errors_precede_host_telemetry() {
    let root = workspace("host-memory-precedence");
    let memory = 64 * 1024 * 1024;
    let defaults = HardLimits::default();
    let limits = HardLimits {
        max_active_process_jobs: 8,
        max_active_process_jobs_per_session: 8,
        max_active_process_memory_bytes: memory,
        max_active_process_memory_bytes_per_session: memory,
        max_process_budget: ResourceBudget {
            memory_bytes: memory,
            ..defaults.max_process_budget
        },
        ..defaults
    };
    let manager = manager_with_host_memory(&root, limits, FakeHostMemoryObservation::Error);
    let owner = SessionHandle::generate().expect("owner");
    let other = SessionHandle::generate().expect("other");
    let mut store = manager.jobs.lock().expect("job store");
    store.jobs.insert(
        JobId::generate().expect("job id"),
        synthetic_record(owner, 0, ProcessStatus::Running, memory, 1),
    );
    assert!(matches!(
        manager.prepare_store_for_start(&mut store, &other, 1, 1, 1),
        Err(ProcessError::ProcessMemoryCapacityExceeded)
    ));
    drop(store);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn memory_reservations_are_global_session_scoped_and_uncertainty_holds_capacity() {
    let root = workspace("memory-governor");
    let memory = 64 * 1024 * 1024;
    let defaults = HardLimits::default();
    let limits = HardLimits {
        max_active_process_jobs: 8,
        max_active_process_jobs_per_session: 8,
        max_active_process_memory_bytes: memory * 2,
        max_active_process_memory_bytes_per_session: memory,
        max_process_budget: ResourceBudget {
            memory_bytes: memory,
            ..defaults.max_process_budget
        },
        ..defaults
    };
    let manager = ProcessManager::new(&root, limits, Vec::new()).expect("process manager");
    let owner = SessionHandle::generate().expect("owner session");
    let other = SessionHandle::generate().expect("other session");
    let third = SessionHandle::generate().expect("third session");
    let owner_job = JobId::generate().expect("owner job id");
    let other_job = JobId::generate().expect("other job id");

    let mut store = manager.jobs.lock().expect("job store");
    store.jobs.insert(
        owner_job.clone(),
        synthetic_record(
            owner.clone(),
            0,
            ProcessStatus::TerminationUncertain,
            memory,
            limits.max_process_cpu_percent_per_job,
        ),
    );
    assert!(matches!(
        manager.prepare_store_for_start(&mut store, &owner, 1, 1, 1),
        Err(ProcessError::ProcessMemoryCapacityExceededForSession)
    ));
    assert!(
        manager
            .prepare_store_for_start(&mut store, &other, 1, memory, 1)
            .is_ok()
    );

    store.jobs.insert(
        other_job,
        synthetic_record(other, 1, ProcessStatus::Running, memory, 1),
    );
    assert!(matches!(
        manager.prepare_store_for_start(&mut store, &third, 1, 1, 1),
        Err(ProcessError::ProcessMemoryCapacityExceeded)
    ));

    store
        .jobs
        .get(&owner_job)
        .expect("owner record")
        .state
        .lock()
        .expect("owner state")
        .status = ProcessStatus::Exited;
    assert!(
        manager
            .prepare_store_for_start(&mut store, &third, 1, 1, 1)
            .is_ok()
    );
    drop(store);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn heavy_workload_slots_are_global_session_scoped_and_uncertainty_holds_capacity() {
    let root = workspace("heavy-workload-slots");
    let limits = HardLimits {
        max_active_process_jobs: 8,
        max_active_process_jobs_per_session: 8,
        max_active_heavy_process_jobs: 2,
        max_active_heavy_process_jobs_per_session: 1,
        ..HardLimits::default()
    };
    #[cfg(windows)]
    let manager = manager_with_host_memory(
        &root,
        limits,
        FakeHostMemoryObservation::Snapshot(HostMemorySnapshot {
            total_physical_bytes: 64 * 1024 * 1024 * 1024,
            available_physical_bytes: 64 * 1024 * 1024 * 1024,
        }),
    );
    #[cfg(not(windows))]
    let manager = ProcessManager::new(&root, limits, Vec::new()).expect("process manager");

    let session_a = SessionHandle::generate().expect("session A");
    let session_b = SessionHandle::generate().expect("session B");
    let session_c = SessionHandle::generate().expect("session C");
    let job_a = JobId::generate().expect("job A");
    let job_b = JobId::generate().expect("job B");

    let mut store = manager.jobs.lock().expect("job store");
    store.jobs.insert(
        job_a.clone(),
        synthetic_record_with_workload(
            session_a.clone(),
            0,
            ProcessStatus::Running,
            1,
            1,
            WorkloadClass::Heavy,
        ),
    );

    assert!(matches!(
        manager.prepare_store_for_start_with_workload(
            &mut store,
            &session_a,
            WorkloadClass::Heavy,
            1,
            1,
            1,
        ),
        Err(ProcessError::HeavyWorkloadCapacityExceededForSession)
    ));
    assert!(
        manager
            .prepare_store_for_start_with_workload(
                &mut store,
                &session_b,
                WorkloadClass::Heavy,
                1,
                1,
                1,
            )
            .is_ok()
    );

    store.jobs.insert(
        job_b,
        synthetic_record_with_workload(
            session_b,
            1,
            ProcessStatus::TerminationUncertain,
            1,
            1,
            WorkloadClass::Heavy,
        ),
    );

    assert!(matches!(
        manager.prepare_store_for_start_with_workload(
            &mut store,
            &session_c,
            WorkloadClass::Heavy,
            1,
            1,
            1,
        ),
        Err(ProcessError::HeavyWorkloadCapacityExceeded)
    ));
    assert!(
        manager
            .prepare_store_for_start_with_workload(
                &mut store,
                &session_c,
                WorkloadClass::Standard,
                1,
                1,
                1,
            )
            .is_ok(),
        "standard work must remain admissible when only heavy slots are saturated"
    );

    store
        .jobs
        .get(&job_a)
        .expect("job A record")
        .state
        .lock()
        .expect("job A state")
        .status = ProcessStatus::Exited;

    assert!(
        manager
            .prepare_store_for_start_with_workload(
                &mut store,
                &session_c,
                WorkloadClass::Heavy,
                1,
                1,
                1,
            )
            .is_ok(),
        "a proven-terminal heavy job must release its slot while uncertain work still holds one"
    );

    drop(store);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn cpu_reservations_are_global_session_scoped_and_uncertainty_holds_capacity() {
    let root = workspace("cpu-governor");
    let limits = HardLimits {
        max_active_process_jobs: 8,
        max_active_process_jobs_per_session: 8,
        max_process_cpu_percent_per_job: 25,
        max_active_process_cpu_percent: 50,
        max_active_process_cpu_percent_per_session: 25,
        ..HardLimits::default()
    };
    let manager = ProcessManager::new(&root, limits, Vec::new()).expect("process manager");
    let owner = SessionHandle::generate().expect("owner session");
    let other = SessionHandle::generate().expect("other session");
    let third = SessionHandle::generate().expect("third session");
    let owner_job = JobId::generate().expect("owner job id");
    let other_job = JobId::generate().expect("other job id");

    let mut store = manager.jobs.lock().expect("job store");
    store.jobs.insert(
        owner_job.clone(),
        synthetic_record(
            owner.clone(),
            0,
            ProcessStatus::TerminationUncertain,
            1,
            limits.max_process_cpu_percent_per_job,
        ),
    );
    assert!(matches!(
        manager.prepare_store_for_start(&mut store, &owner, 1, 1, 25),
        Err(ProcessError::CpuCapacityExceededForSession)
    ));
    assert!(
        manager
            .prepare_store_for_start(&mut store, &other, 1, 1, 25)
            .is_ok()
    );

    store.jobs.insert(
        other_job,
        synthetic_record(other, 1, ProcessStatus::Running, 1, 25),
    );
    assert!(matches!(
        manager.prepare_store_for_start(&mut store, &third, 1, 1, 25),
        Err(ProcessError::CpuCapacityExceeded)
    ));

    store
        .jobs
        .get(&owner_job)
        .expect("owner record")
        .state
        .lock()
        .expect("owner state")
        .status = ProcessStatus::Exited;
    assert!(
        manager
            .prepare_store_for_start(&mut store, &third, 1, 1, 25)
            .is_ok()
    );
    drop(store);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[tokio::test]
async fn termination_confirmation_wait_is_bounded_and_fail_closed() {
    let started = Instant::now();
    let confirmation = wait_for_termination_confirmation(
        std::future::pending::<std::io::Result<ExitStatus>>(),
        Duration::from_millis(20),
    )
    .await;
    assert!(confirmation.is_none());
    assert!(started.elapsed() < Duration::from_secs(1));

    let failed = wait_for_termination_confirmation(
        std::future::ready(Err(std::io::Error::other("wait failed"))),
        Duration::from_secs(1),
    )
    .await;
    assert!(failed.is_none());
}

#[tokio::test]
async fn termination_uncertain_holds_capacity_and_cannot_be_reaped() {
    let root = workspace("termination-uncertain");
    fs::write(root.join("fixture-sleep"), b"1").expect("write fixture mode");
    let limits = HardLimits {
        max_active_process_jobs: 2,
        max_active_process_jobs_per_session: 1,
        ..HardLimits::default()
    };
    let manager = ProcessManager::new(&root, limits, Vec::new()).expect("process manager");
    let owner = SessionHandle::generate().expect("owner session");
    let uncertain_job = JobId::generate().expect("uncertain job id");
    {
        let mut store = manager.jobs.lock().expect("job store");
        store.jobs.insert(
            uncertain_job.clone(),
            Arc::new(JobRecord {
                owner: owner.clone(),
                private_action: None,
                sequence: 0,
                workload_class: WorkloadClass::Standard,
                reserved_output_bytes: 1024,
                reserved_memory_bytes: 64 * 1024 * 1024,
                reserved_cpu_percent: limits.max_process_cpu_percent_per_job,
                #[cfg(windows)]
                cwd_pin: Mutex::new(None),
                output: Mutex::new(OutputState {
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    stdout_closed: true,
                    stderr_closed: true,
                }),
                state: Mutex::new(JobState {
                    status: ProcessStatus::TerminationUncertain,
                    exit_code: None,
                }),
                stop_requested: Arc::new(AtomicBool::new(true)),
                output_overflow: AtomicBool::new(true),
                output_read_failed: AtomicBool::new(false),
            }),
        );
    }

    assert_eq!(
        manager
            .active_session_job_count(&owner)
            .expect("active count"),
        1
    );
    assert_eq!(
        manager
            .remove_terminal_session_records(&owner)
            .expect("owner-scoped reap"),
        0
    );
    assert_eq!(
        manager
            .result(&owner, &uncertain_job)
            .expect("uncertain result")
            .status,
        ProcessStatus::TerminationUncertain
    );
    assert!(matches!(
        manager.start(spec(&root, owner.clone(), budget(5000, 1024))),
        Err(ProcessError::TooManyActiveJobsForSession)
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(windows)]
#[tokio::test]
async fn running_job_pins_cwd_namespace_until_terminal_state_is_proven() {
    let root = workspace("cwd-pin-runtime");
    let cwd = root.join("nested").join("cwd");
    fs::create_dir_all(&cwd).expect("create cwd");
    fs::write(cwd.join("fixture-sleep"), b"1").expect("sleep fixture");
    let manager =
        ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
    let owner = SessionHandle::generate().expect("owner session");
    let mut start = spec(&root, owner.clone(), budget(5_000, 64 * 1024));
    start.cwd = Some(WorkspacePath::parse("nested/cwd").expect("workspace cwd"));
    let job = manager.start(start).expect("start process");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let renamed = root.join("nested-renamed");
    assert!(
        fs::rename(root.join("nested"), &renamed).is_err(),
        "running job must keep the cwd ancestor namespace pinned"
    );

    assert!(manager.stop(&owner, &job).expect("stop process"));
    let result = await_terminal(&manager, &owner, &job).await;
    assert!(matches!(
        result.status,
        ProcessStatus::Stopped | ProcessStatus::Exited
    ));
    fs::rename(root.join("nested"), &renamed)
        .expect("terminal proof must release cwd namespace pin");
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

#[test]
fn cancellation_signals_uncertain_owner_without_discarding_proof_or_other_sessions() {
    // Fixed in-memory synthetic records: no external program or PID is used.
    // An uncertain owner retains its Job record and quota even if a stop is
    // requested. stop_requested is not a termination acknowledgement.
    let root = workspace("uncertain-session-cancel");
    let manager =
        ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
    let owner = SessionHandle::generate().expect("owner session");
    let other = SessionHandle::generate().expect("other session");
    let running = synthetic_record(owner.clone(), 0, ProcessStatus::Running, 1, 1);
    let uncertain = synthetic_record(owner.clone(), 1, ProcessStatus::TerminationUncertain, 1, 1);
    let already_signalled =
        synthetic_record(owner.clone(), 2, ProcessStatus::TerminationUncertain, 1, 1);
    let exited = synthetic_record(owner.clone(), 3, ProcessStatus::Exited, 1, 1);
    let failed = synthetic_record(owner.clone(), 4, ProcessStatus::Failed, 1, 1);
    let unrelated = synthetic_record(other.clone(), 5, ProcessStatus::Running, 1, 1);
    already_signalled
        .stop_requested
        .store(true, Ordering::Release);

    let ids = (0..6)
        .map(|_| JobId::generate().expect("synthetic job id"))
        .collect::<Vec<_>>();
    {
        let mut store = manager.jobs.lock().expect("job store");
        for (id, record) in ids.iter().zip([
            &running,
            &uncertain,
            &already_signalled,
            &exited,
            &failed,
            &unrelated,
        ]) {
            store.jobs.insert(id.clone(), Arc::clone(record));
        }
    }
    assert_eq!(
        manager
            .active_session_job_count(&owner)
            .expect("owner count"),
        3
    );
    assert_eq!(
        manager.cancel_session(&owner).expect("first cancellation"),
        2,
        "both Running and un-signalled TerminationUncertain need stop requests"
    );
    for record in [&running, &uncertain, &already_signalled] {
        assert!(record.stop_requested.load(Ordering::Acquire));
    }
    for record in [&exited, &failed, &unrelated] {
        assert!(!record.stop_requested.load(Ordering::Acquire));
    }
    assert_eq!(
        manager
            .cancel_session(&owner)
            .expect("idempotent cancellation"),
        0
    );
    assert_eq!(
        manager
            .active_session_job_count(&owner)
            .expect("retained capacity"),
        3,
        "requesting stop never proves a Job has terminated"
    );
    assert_eq!(
        manager
            .remove_terminal_session_records(&owner)
            .expect("remove only proven terminal jobs"),
        2
    );
    {
        let store = manager.jobs.lock().expect("retained store");
        for id in &ids[..3] {
            assert!(
                store.jobs.contains_key(id),
                "active/uncertain Job must remain owned"
            );
        }
        assert!(
            store.jobs.contains_key(&ids[5]),
            "unrelated owner unaffected"
        );
    }
    assert_eq!(
        manager
            .cancel_session(&other)
            .expect("other owner cancellation"),
        1
    );
    assert!(unrelated.stop_requested.load(Ordering::Acquire));
    assert_eq!(manager.cancel_session(&other).unwrap(), 0);
    drop(manager);
    fs::remove_dir_all(root).expect("remove workspace");
}

// #317 research only: the real registry is test-instrumented with an
// authority-owned session gate shared by direct revoke and synthetic
// JobStore owner commitment. These cases prove ordering, NOT Win32 safety.
fn research_owner_grant() -> optic_bridge_core::SessionGrant {
    optic_bridge_core::SessionGrant {
        handle: SessionHandle::generate().expect("fixed synthetic handle"),
        principal: optic_bridge_core::PrincipalId::new("owner-fence").expect("principal"),
        project: optic_bridge_core::ProjectId::new("research").expect("project"),
        capabilities: std::collections::BTreeSet::new(),
        expires_at: optic_bridge_core::MonotonicTime::from_millis(10_000),
        policy_epoch: 1,
    }
}

fn research_owner_wait_closed(registry: &crate::SessionRegistry, handle: &SessionHandle) {
    let until = std::time::Instant::now() + Duration::from_secs(2);
    while !matches!(
        registry.get_active(handle, optic_bridge_core::MonotonicTime::from_millis(1)),
        Err(crate::SessionRegistryError::Revoked)
    ) {
        assert!(
            std::time::Instant::now() < until,
            "close publication deadline"
        );
        thread::yield_now();
    }
}

#[test]
fn research_owner_fence_close_wins_and_late_admitted_producer_cannot_register() {
    let root = workspace("owner-fence-close");
    let manager = ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap();
    let registry = crate::SessionRegistry::new();
    let a = research_owner_grant();
    registry.register(a.clone()).unwrap();
    let now = optic_bridge_core::MonotonicTime::from_millis(1);
    let permit = registry.begin_admission(&a.handle, now).unwrap();
    let job = JobId::generate().unwrap();
    let candidate = synthetic_record(a.handle.clone(), 0, ProcessStatus::Running, 1, 1);
    let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
    thread::scope(|scope| {
        let closer = scope.spawn(|| {
            done_tx.send(registry.revoke(&a.handle)).unwrap();
        });
        research_owner_wait_closed(&registry, &a.handle);
        // An admission granted before closure is not a license to register
        // a *new* owner after the authority's close linearization point.
        assert_eq!(
            registry.research_commit_owner(&a.handle, now, || {
                manager.jobs.lock().unwrap().jobs.insert(job, candidate);
            }),
            Err(crate::SessionRegistryError::Revoked)
        );
        assert_eq!(manager.cancel_session(&a.handle).unwrap(), 0);
        assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 0);
        assert!(matches!(
            done_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        drop(permit);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap()
        );
        closer.join().unwrap();
    });
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn research_owner_fence_launch_wins_and_stop_reaches_registered_owner_before_drain() {
    let root = workspace("owner-fence-launch");
    let manager = ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap();
    let registry = crate::SessionRegistry::new();
    let a = research_owner_grant();
    let b = research_owner_grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = optic_bridge_core::MonotonicTime::from_millis(1);
    let permit = registry.begin_admission(&a.handle, now).unwrap();
    let job = JobId::generate().unwrap();
    let record = synthetic_record(
        a.handle.clone(),
        0,
        ProcessStatus::TerminationUncertain,
        1,
        1,
    );
    registry
        .research_commit_owner(&a.handle, now, || {
            manager
                .jobs
                .lock()
                .unwrap()
                .jobs
                .insert(job.clone(), Arc::clone(&record));
        })
        .unwrap();

    let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
    thread::scope(|scope| {
        let closer = scope.spawn(|| {
            done_tx.send(registry.revoke(&a.handle)).unwrap();
        });
        research_owner_wait_closed(&registry, &a.handle);
        assert_eq!(manager.cancel_session(&a.handle).unwrap(), 1);
        assert!(record.stop_requested.load(Ordering::Acquire));
        assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
        assert_eq!(manager.cancel_session(&a.handle).unwrap(), 0);
        assert!(registry.begin_admission(&b.handle, now).is_ok());
        assert!(matches!(
            done_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        drop(permit);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap()
        );
        closer.join().unwrap();
    });
    assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
    assert!(manager.jobs.lock().unwrap().jobs.contains_key(&job));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn research_owner_fence_racing_direct_revoke_waits_for_owner_commit_but_not_session_b() {
    let root = workspace("owner-fence-racing");
    let manager = ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap();
    let registry = crate::SessionRegistry::new();
    let a = research_owner_grant();
    let b = research_owner_grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = optic_bridge_core::MonotonicTime::from_millis(1);
    let permit = registry.begin_admission(&a.handle, now).unwrap();
    let job = JobId::generate().unwrap();
    let record = synthetic_record(a.handle.clone(), 0, ProcessStatus::Running, 1, 1);
    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
    let (close_tx, close_rx) = std::sync::mpsc::sync_channel(1);
    thread::scope(|scope| {
        let launch = scope.spawn(|| {
            registry.research_commit_owner(&a.handle, now, || {
                entered_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                manager
                    .jobs
                    .lock()
                    .unwrap()
                    .jobs
                    .insert(job.clone(), Arc::clone(&record));
            })
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let closer = scope.spawn(|| close_tx.send(registry.revoke(&a.handle)).unwrap());

        // A's commit holds a per-session gate, not the whole registry:
        // B still admits and commits while A is in the fixed test closure.
        assert!(registry.begin_admission(&b.handle, now).is_ok());
        registry
            .research_commit_owner(&b.handle, now, || ())
            .unwrap();
        release_tx.send(()).unwrap();
        launch.join().unwrap().unwrap();
        research_owner_wait_closed(&registry, &a.handle);
        // Now the already-committed owner is necessarily in JobStore;
        // a pre-drain stop reaches it without ambiguity.
        assert_eq!(manager.cancel_session(&a.handle).unwrap(), 1);
        assert!(record.stop_requested.load(Ordering::Acquire));
        assert!(matches!(
            close_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        drop(permit);
        assert!(
            close_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap()
        );
        closer.join().unwrap();
    });
    assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn research_owner_fence_expiry_and_poison_fail_closed_without_preventing_revoke() {
    let registry = crate::SessionRegistry::new();
    let a = research_owner_grant();
    registry.register(a.clone()).unwrap();
    let entered = std::sync::atomic::AtomicUsize::new(0);
    assert!(matches!(
        registry.research_commit_owner(
            &a.handle,
            optic_bridge_core::MonotonicTime::from_millis(10_000),
            || entered.fetch_add(1, Ordering::SeqCst)
        ),
        Err(crate::SessionRegistryError::Expired)
    ));
    assert_eq!(entered.load(Ordering::SeqCst), 0);
    let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = registry.research_commit_owner(
            &a.handle,
            optic_bridge_core::MonotonicTime::from_millis(1),
            || panic!("synthetic panic while committing owner"),
        );
    }));
    assert!(observed.is_err());
    assert!(matches!(
        registry.research_commit_owner(
            &a.handle,
            optic_bridge_core::MonotonicTime::from_millis(1),
            || panic!("no new effect after poisoned gate")
        ),
        Err(crate::SessionRegistryError::StateUnavailable)
    ));
    assert!(
        registry
            .revoke(&a.handle)
            .expect("fail-closed revoke after poison")
    );
    assert!(matches!(
        registry.get_active(&a.handle, optic_bridge_core::MonotonicTime::from_millis(1)),
        Err(crate::SessionRegistryError::Revoked)
    ));
}
