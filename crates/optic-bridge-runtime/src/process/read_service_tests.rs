//! Supervised tests of the actual private service kernel, not scoped mock.
use super::*;
use crate::{SessionLifecycleError, SessionLifecycleManager, TaskLeaseRegistry};
use optic_bridge_core::{Capability, PrincipalId, ProjectId, SessionGrant};
use std::{env, path::Path, time::Instant};

const BUDGET: ResourceBudget = ResourceBudget {
    timeout_ms: 4000,
    output_bytes: 4096,
    memory_bytes: 256 * 1024 * 1024,
    process_count: 1,
};

fn grant(label: &str) -> SessionGrant {
    SessionGrant {
        handle: SessionHandle::generate().unwrap(),
        principal: PrincipalId::new(label).unwrap(),
        project: ProjectId::new("synthetic").unwrap(),
        capabilities: BTreeSet::from([Capability::GitRead]),
        expires_at: MonotonicTime::from_millis(10_000),
        policy_epoch: 1,
    }
}

fn read(grant: &SessionGrant, marker: &Path, faults: Arc<TestFaults>) -> AuthorizedRead {
    let mut command = NativeCommand::new(env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "git_owner_registry::tests::native_registry_fixture_child",
            "--ignored",
            "--nocapture",
        ])
        .env_clear()
        .env("OPTIC317_REGISTRY_CHILD", "1")
        .env("OPTIC317_REGISTRY_STARTED", marker);
    command.current_dir(marker.parent().expect("synthetic fixture directory"));
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = env::var_os(key) {
            command.env(key, value);
        }
    }
    AuthorizedRead {
        session: grant.handle.clone(),
        now: MonotonicTime::from_millis(1),
        command,
        resources: BUDGET,
        workload: WorkloadClass::Standard,
        faults,
    }
}

fn wait_for(mut check: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(5);
    while !check() {
        assert!(Instant::now() < until, "service kernel observation bound");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn shared_budget_rejects_other_domain_overcommit_before_native_prepare() {
    // Only this negative accounting test uses a fixed host observation. It
    // must not depend on unrelated desktop RAM pressure, and an admission
    // regression must still be unable to execute any child.
    #[derive(Debug)]
    struct AvailableHost;
    impl HostMemoryProvider for AvailableHost {
        fn snapshot(&self) -> std::io::Result<HostMemorySnapshot> {
            Ok(HostMemorySnapshot {
                total_physical_bytes: 32 * 1024 * 1024 * 1024,
                available_physical_bytes: 16 * 1024 * 1024 * 1024,
            })
        }
    }
    for case in ["cpu", "memory", "output"] {
        let root = env::temp_dir().join(format!(
            "optic317-budget-{}",
            SessionHandle::generate().unwrap().to_token()
        ));
        fs::create_dir(&root).unwrap();
        let limits = HardLimits {
            max_process_budget: BUDGET,
            max_active_process_memory_bytes: if case == "memory" {
                BUDGET.memory_bytes
            } else {
                2 * BUDGET.memory_bytes
            },
            max_active_process_memory_bytes_per_session: BUDGET.memory_bytes,
            max_active_output_ram_bytes: if case == "output" {
                BUDGET.output_bytes
            } else {
                2 * BUDGET.output_bytes
            },
            max_active_output_ram_bytes_per_session: BUDGET.output_bytes,
            max_active_process_cpu_percent: if case == "cpu" { 25 } else { 50 },
            max_active_process_cpu_percent_per_session: 25,
            ..HardLimits::default()
        };
        let mut manager = ProcessManager::new(&root, limits, Vec::new()).unwrap();
        manager.host_memory_provider = Arc::new(AvailableHost);
        let manager = Arc::new(manager);
        let sessions = Arc::new(SessionRegistry::new());
        let ordinary = grant("ordinary");
        let git = grant("git");
        sessions.register(git.clone()).unwrap();
        let mut record = super::super::tests::synthetic_record(
            ordinary.handle,
            0,
            ProcessStatus::Running,
            BUDGET.memory_bytes,
            25,
        );
        Arc::get_mut(&mut record).unwrap().reserved_output_bytes = BUDGET.output_bytes;
        manager
            .jobs
            .lock()
            .unwrap()
            .jobs
            .insert(JobId::generate().unwrap(), record);
        let service = ReadServiceRoot::new(Arc::clone(&manager), sessions);
        let mut token = read(
            &git,
            &root.join("must-not-start"),
            Arc::new(TestFaults::default()),
        );
        token.command = NativeCommand::new(root.join("absent-synthetic-executable.exe"));
        let result = service.submit(token);
        assert!(matches!(
            (case, result),
            (
                "cpu",
                Err(ServiceError::Process(ProcessError::CpuCapacityExceeded))
            ) | (
                "memory",
                Err(ServiceError::Process(
                    ProcessError::ProcessMemoryCapacityExceeded
                ))
            ) | (
                "output",
                Err(ServiceError::Process(
                    ProcessError::OutputMemoryLimitExceeded
                ))
            )
        ));
        assert_eq!(manager.jobs.lock().unwrap().jobs.len(), 1);
        assert_eq!(manager.active_session_job_count(&git.handle).unwrap(), 0);
    }
}

#[test]
#[ignore = "requires independent owning Job before gate"]
fn service_kernel_outcome_and_pipe_matrix() {
    let gate = PathBuf::from(env::var_os("OPTIC317_SERVICE_GATE").expect("outer Job"));
    wait_for(|| gate.exists());
    let root = gate.parent().unwrap().join("kernel-matrix");
    fs::create_dir(&root).unwrap();
    let manager = Arc::new(ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap());
    let sessions = Arc::new(SessionRegistry::new());
    let a = grant("matrix");
    sessions.register(a.clone()).unwrap();
    let service = ReadServiceRoot::new(Arc::clone(&manager), Arc::clone(&sessions));
    for (mode, expected, code) in [
        ("normal", ProcessStatus::Exited, Some(0)),
        ("nonzero", ProcessStatus::Exited, Some(7)),
        ("output", ProcessStatus::OutputLimitExceeded, None),
        ("both-output", ProcessStatus::OutputLimitExceeded, None),
        ("stdout", ProcessStatus::Exited, Some(0)),
        ("stderr", ProcessStatus::Exited, Some(0)),
        ("timeout", ProcessStatus::TimedOut, None),
        ("null-stdin", ProcessStatus::Exited, Some(0)),
    ] {
        let started = Instant::now();
        let mut token = read(&a, &root.join("unused"), Arc::new(TestFaults::default()));
        token.command = NativeCommand::new(env::var_os("OPTIC317_PYTHON").expect("known Python"));
        token.command.current_dir(&root);
        token
            .command
            .arg(env::var_os("OPTIC317_FIXTURE").expect("synthetic fixture"))
            .arg(mode)
            .env_clear();
        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = env::var_os(key) {
                token.command.env(key, value);
            }
        }
        token.resources.process_count = 2; // parent and bounded synthetic descendant
        if mode == "timeout" {
            token.resources.timeout_ms = 200;
        }
        let id = service.submit(token).unwrap();
        wait_for(|| manager.active_session_job_count(&a.handle).unwrap() == 0);
        let result = manager.result(&a.handle, &id).unwrap();
        assert_eq!(result.status, expected, "case {mode}");
        if let Some(code) = code {
            assert_eq!(result.exit_code, Some(code), "case {mode}");
        }
        let stdout = manager
            .read(&a.handle, &id, ProcessStream::Stdout, 0, 4096)
            .unwrap();
        let stderr = manager
            .read(&a.handle, &id, ProcessStream::Stderr, 0, 4096)
            .unwrap();
        assert!(stdout.eof && stderr.eof);
        assert!(stdout.bytes.len() + stderr.bytes.len() <= 4096);
        assert_eq!(
            result.output_truncated,
            matches!(mode, "output" | "both-output")
        );
        println!(
            "OPTIC317_SERVICE_CASE {{\"case\":\"{mode}\",\"elapsed_ms\":{},\"record_terminal\":true,\"combined_output_bytes\":{},\"eof\":true}}",
            started.elapsed().as_millis(),
            stdout.bytes.len() + stderr.bytes.len()
        );
    }
    wait_for(|| service.try_shutdown().unwrap() == 0);
    println!(
        "OPTIC317_SERVICE kernel_matrix=true cases=8 result_mapping=true combined_output_bounded=true workers_joined=true production_git_wired=false"
    );
}

#[test]
#[ignore = "requires independent owning Job before gate"]
fn service_kernel_prestart_stop_and_deadline_classification() {
    let gate = PathBuf::from(env::var_os("OPTIC317_SERVICE_GATE").expect("outer Job"));
    wait_for(|| gate.exists());
    let root = gate.parent().unwrap().join("prestart");
    fs::create_dir(&root).unwrap();
    let manager = Arc::new(ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap());
    let sessions = Arc::new(SessionRegistry::new());
    let mut observations = Vec::new();
    for (mode, expected) in [
        ("prestart-stop", ProcessStatus::Stopped),
        ("prestart-deadline", ProcessStatus::TimedOut),
    ] {
        let a = grant(mode);
        sessions.register(a.clone()).unwrap();
        let service = ReadServiceRoot::new(Arc::clone(&manager), Arc::clone(&sessions));
        let faults = Arc::new(TestFaults::default());
        faults.hold_start.store(true, Ordering::Release);
        let mut token = read(&a, &root.join(mode), Arc::clone(&faults));
        if mode == "prestart-deadline" {
            token.resources.timeout_ms = 100;
        }
        let id = service.submit(token).unwrap();
        let submitted = Instant::now();
        if mode == "prestart-stop" {
            assert!(service.try_shutdown().unwrap() >= 1);
        } else {
            // Bound the test; intentionally let the EXISTING native budget
            // expire before releasing the synthetic scheduling gate.
            while submitted.elapsed() < Duration::from_millis(150) {
                thread::sleep(Duration::from_millis(5));
            }
        }
        faults.hold_start.store(false, Ordering::Release);
        wait_for(|| manager.active_session_job_count(&a.handle).unwrap() == 0);
        let result = manager.result(&a.handle, &id).unwrap();
        assert!(
            !root.join(mode).exists(),
            "cancelled/expired root must not execute"
        );
        observations.push((result.status, expected));
        println!(
            "OPTIC317_SERVICE_CASE {{\"case\":\"{mode}\",\"actual\":\"{:?}\",\"expected\":\"{expected:?}\",\"native_started\":false}}",
            result.status
        );
        wait_for(|| service.try_shutdown().unwrap() == 0);
    }
    assert!(
        observations
            .iter()
            .all(|(actual, expected)| actual == expected)
    );
    println!(
        "OPTIC317_SERVICE prestart_cases=2 cancellation_and_deadline_distinct=true no_native_execution=true workers_joined=true production_git_wired=false"
    );
}

// Release synthetic errors before the root's joined Drop on assertion unwind.
struct Recover(Arc<TestFaults>);
impl Drop for Recover {
    fn drop(&mut self) {
        self.0.observation_error.store(false, Ordering::Release);
        self.0.hold_start.store(false, Ordering::Release);
    }
}

#[test]
#[ignore = "requires independent owning Job before gate"]
fn service_kernel_retains_owners_through_request_loss_and_shutdown() {
    let gate = PathBuf::from(env::var_os("OPTIC317_SERVICE_GATE").expect("outer Job gate"));
    wait_for(|| gate.exists());
    let root = gate.parent().unwrap().join("kernel-workspace");
    fs::create_dir(&root).unwrap();
    let limits = HardLimits {
        max_active_process_jobs: 2,
        max_active_process_jobs_per_session: 1,
        ..HardLimits::default()
    };
    let manager = Arc::new(ProcessManager::new(&root, limits, Vec::new()).unwrap());
    let sessions = Arc::new(SessionRegistry::new());
    let lifecycle = SessionLifecycleManager::new(
        Arc::clone(&sessions),
        Arc::new(TaskLeaseRegistry::new()),
        Arc::clone(&manager),
    );
    let a = grant("a");
    let b = grant("b");
    sessions.register(a.clone()).unwrap();
    sessions.register(b.clone()).unwrap();
    let service = ReadServiceRoot::new(Arc::clone(&manager), Arc::clone(&sessions));
    let fault_a = Arc::new(TestFaults::default());
    let fault_b = Arc::new(TestFaults::default());
    let _recover_a = Recover(Arc::clone(&fault_a));
    let _recover_b = Recover(Arc::clone(&fault_b));
    let marker_a = root.join("a-started");
    let marker_b = root.join("b-started");
    // The "request" owns an ID only. Service worker owns actual native payload.
    let id_a = service
        .submit(read(&a, &marker_a, Arc::clone(&fault_a)))
        .unwrap();
    assert!(matches!(
        service.submit(read(
            &a,
            &root.join("a-refused"),
            Arc::new(TestFaults::default())
        )),
        Err(ServiceError::Process(
            ProcessError::TooManyActiveJobsForSession
        ))
    ));
    let id_b = service
        .submit(read(&b, &marker_b, Arc::clone(&fault_b)))
        .unwrap();
    let (reply, request) = std::sync::mpsc::sync_channel(1);
    reply.send(id_a.clone()).unwrap();
    drop(request);
    drop(reply);
    wait_for(|| marker_a.exists() && marker_b.exists());
    assert!(matches!(
        manager.result(&b.handle, &id_a),
        Err(ProcessError::UnknownJob)
    ));
    assert!(matches!(
        manager.stop(&b.handle, &id_a),
        Err(ProcessError::UnknownJob)
    ));
    fault_a.observation_error.store(true, Ordering::Release);
    fault_a.panic_once.store(true, Ordering::Release);
    wait_for(|| fault_a.panics_caught.load(Ordering::Acquire) == 1);
    assert_eq!(
        manager.result(&a.handle, &id_a).unwrap().status,
        ProcessStatus::TerminationUncertain
    );
    lifecycle.revoke(&a.handle).unwrap();
    assert!(matches!(
        service.submit(read(&a, &marker_a, Arc::clone(&fault_a))),
        Err(ServiceError::Session(SessionRegistryError::Revoked))
    ));
    assert!(matches!(
        lifecycle.seal_quiescent(&a.handle),
        Err(SessionLifecycleError::SessionJobsStillActive { active_jobs: 1 })
    ));
    let now = MonotonicTime::from_millis(1);
    assert_eq!(lifecycle.try_reap(&a.handle, now).unwrap().active_jobs, 1);
    assert_eq!(
        manager.remove_terminal_session_records(&a.handle).unwrap(),
        0
    );
    assert_eq!(
        manager.result(&b.handle, &id_b).unwrap().status,
        ProcessStatus::Running
    );
    {
        let mut store = manager.jobs.lock().unwrap();
        assert_eq!(
            reserved_process_memory_bytes(&store).unwrap(),
            2 * BUDGET.memory_bytes
        );
        assert_eq!(reserved_cpu_percent(&store).unwrap(), 50);
        assert_eq!(reserved_output_bytes(&store), 2 * BUDGET.output_bytes);
        // Actual admission for ordinary jobs sees kernel reservations.
        assert!(matches!(
            manager.prepare_store_for_start(
                &mut store,
                &a.handle,
                1,
                manager.limits.max_active_process_memory_bytes,
                1
            ),
            Err(ProcessError::TooManyActiveJobs)
        ));
    }
    fault_a.observation_error.store(false, Ordering::Release);
    wait_for(|| manager.active_session_job_count(&a.handle).unwrap() == 0);
    assert_eq!(
        manager.result(&a.handle, &id_a).unwrap().status,
        ProcessStatus::Failed
    );
    assert!(lifecycle.seal_quiescent(&a.handle).is_ok());
    assert!(lifecycle.try_reap(&a.handle, now).unwrap().session_removed);
    assert!(sessions.get_active(&b.handle, now).is_ok());
    // Stop observation while closing the root; shutdown cannot release B.
    fault_b.observation_error.store(true, Ordering::Release);
    wait_for(|| {
        manager.result(&b.handle, &id_b).unwrap().status == ProcessStatus::TerminationUncertain
    });
    assert!(service.try_shutdown().unwrap() >= 1);
    assert!(matches!(
        service.submit(read(&b, &marker_b, Arc::clone(&fault_b))),
        Err(ServiceError::Stopping)
    ));
    assert_eq!(manager.active_session_job_count(&b.handle).unwrap(), 1);
    assert!(matches!(
        lifecycle.seal_quiescent(&b.handle),
        Err(SessionLifecycleError::SessionJobsStillActive { active_jobs: 1 })
    ));
    fs::write(
        gate.with_extension("kernel-stopped"),
        b"uncertainty retained",
    )
    .unwrap();
    // Outer supervisor observes native child cleanup while driver cannot poll.
    wait_for(|| gate.with_extension("kernel-confirmed").exists());
    fault_b.observation_error.store(false, Ordering::Release);
    wait_for(|| service.try_shutdown().unwrap() == 0);
    assert!(lifecycle.seal_quiescent(&b.handle).is_ok());
    assert!(lifecycle.try_reap(&b.handle, now).unwrap().session_removed);
    // Separate clean Drop: the earlier error already requested native stop.
    let c = grant("c");
    sessions.register(c.clone()).unwrap();
    let marker_c = root.join("c-started");
    let clean = ReadServiceRoot::new(Arc::clone(&manager), Arc::clone(&sessions));
    let id_c = clean
        .submit(read(&c, &marker_c, Arc::new(TestFaults::default())))
        .unwrap();
    wait_for(|| marker_c.exists());
    assert_eq!(manager.active_session_job_count(&c.handle).unwrap(), 1);
    let bookkeeping = clean.workers.lock().unwrap();
    fs::write(
        gate.with_extension("shutdown-lock-ready"),
        b"active child, lock held",
    )
    .unwrap();
    wait_for(|| gate.with_extension("shutdown-lock-observed").exists());
    assert!(matches!(clean.try_shutdown(), Err(ServiceError::Busy)));
    fs::write(
        gate.with_extension("shutdown-lock-stop"),
        b"stop requested without lock",
    )
    .unwrap();
    wait_for(|| gate.with_extension("shutdown-lock-confirmed").exists());
    drop(bookkeeping);
    drop(clean);
    assert_eq!(manager.active_session_job_count(&c.handle).unwrap(), 0);
    assert_eq!(
        manager.result(&c.handle, &id_c).unwrap().status,
        ProcessStatus::Stopped
    );
    // Direct Drop alone is also a joined fallback when native recovery works.
    let direct = ReadServiceRoot::new(Arc::clone(&manager), Arc::clone(&sessions));
    let direct_marker = root.join("direct-drop-started");
    let direct_id = direct
        .submit(read(&c, &direct_marker, Arc::new(TestFaults::default())))
        .unwrap();
    wait_for(|| direct_marker.exists());
    drop(direct);
    assert_eq!(
        manager.result(&c.handle, &direct_id).unwrap().status,
        ProcessStatus::Stopped
    );
    // Submit admission is NOT a lifetime permit. Freeze the actual worker
    // before start, revoke with NO test-held admission, then let it proceed.
    // This proves the pre-start cancellation path, not atomic check/resume.
    let d = grant("predrain");
    sessions.register(d.clone()).unwrap();
    let predrain = ReadServiceRoot::new(Arc::clone(&manager), Arc::clone(&sessions));
    let predrain_marker = root.join("predrain-started");
    let delayed = Arc::new(TestFaults::default());
    delayed.hold_start.store(true, Ordering::Release);
    let _recover_delayed = Recover(Arc::clone(&delayed));
    let predrain_id = predrain
        .submit(read(&d, &predrain_marker, Arc::clone(&delayed)))
        .unwrap();
    let (flag, _) = sessions.watch_active(&d.handle, now).unwrap();
    lifecycle.revoke(&d.handle).unwrap();
    assert!(flag.load(Ordering::Acquire));
    assert!(!predrain_marker.exists());
    assert_eq!(manager.active_session_job_count(&d.handle).unwrap(), 1);
    assert!(matches!(
        lifecycle.seal_quiescent(&d.handle),
        Err(SessionLifecycleError::SessionJobsStillActive { active_jobs: 1 })
    ));
    assert!(!lifecycle.try_reap(&d.handle, now).unwrap().session_removed);
    assert!(matches!(
        predrain.submit(read(&d, &predrain_marker, Arc::clone(&delayed))),
        Err(ServiceError::Session(SessionRegistryError::Revoked))
    ));
    delayed.hold_start.store(false, Ordering::Release);
    wait_for(|| manager.active_session_job_count(&d.handle).unwrap() == 0);
    assert!(!predrain_marker.exists());
    assert_eq!(
        manager.result(&d.handle, &predrain_id).unwrap().status,
        ProcessStatus::Stopped
    );
    wait_for(|| predrain.try_shutdown().unwrap() == 0);
    assert!(lifecycle.try_reap(&d.handle, now).unwrap().session_removed);

    // Demonstrate the Drop liveness boundary without abandoning its owner:
    // a named, joined test thread retains the root until observation recovers.
    let closing = ReadServiceRoot::new(Arc::clone(&manager), Arc::clone(&sessions));
    let closing_fault = Arc::new(TestFaults::default());
    let closing_marker = root.join("closing-started");
    let closing_id = closing
        .submit(read(&c, &closing_marker, Arc::clone(&closing_fault)))
        .unwrap();
    wait_for(|| closing_marker.exists());
    closing_fault
        .observation_error
        .store(true, Ordering::Release);
    wait_for(|| {
        manager.result(&c.handle, &closing_id).unwrap().status
            == ProcessStatus::TerminationUncertain
    });
    assert!(closing.try_shutdown().unwrap() > 0);
    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
    let (dropped_tx, dropped_rx) = std::sync::mpsc::sync_channel(1);
    thread::scope(|scope| {
        // On assertion unwind this guard clears the fault BEFORE scope joins.
        let _recover_closing = Recover(Arc::clone(&closing_fault));
        let dropping = scope.spawn(move || {
            entered_tx.send(()).unwrap();
            drop(closing);
            dropped_tx.send(()).unwrap();
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(
            dropped_rx.recv_timeout(Duration::from_millis(100)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        assert_eq!(manager.active_session_job_count(&c.handle).unwrap(), 1);
        closing_fault
            .observation_error
            .store(false, Ordering::Release);
        dropped_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        dropping.join().unwrap();
    });
    assert_eq!(manager.active_session_job_count(&c.handle).unwrap(), 0);
    let failed_root = ReadServiceRoot::new(Arc::clone(&manager), Arc::clone(&sessions));
    let mut missing = read(
        &c,
        &root.join("never-started"),
        Arc::new(TestFaults::default()),
    );
    missing.command = NativeCommand::new(root.join("nonexistent.exe"));
    let missing_id = failed_root.submit(missing).unwrap();
    wait_for(|| manager.active_session_job_count(&c.handle).unwrap() == 0);
    assert_eq!(
        manager.result(&c.handle, &missing_id).unwrap().status,
        ProcessStatus::Failed
    );
    wait_for(|| failed_root.try_shutdown().unwrap() == 0);
    println!(
        "OPTIC317_SERVICE kernel=true caller_reply_dropped=true worker_panic_caught=1 shared_accounting=true uncertainty_retained=true seal_reap_gated=true revoke_without_test_permit=true delayed_start_cancelled=true shutdown_pending_retains_root=true shutdown_while_bookkeeping_locked=true drop_waits_for_observation=true clean_root_drop_joined=true failed_spawn_recovered=true workers_joined=true production_git_wired=false"
    );
}
