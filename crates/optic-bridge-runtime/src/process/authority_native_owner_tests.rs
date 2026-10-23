//! #317 RESEARCH ONLY: instrumented actual registry + atomic Win32 owner.
//! Ignored host MUST be owned by the independent outer Job before its gate.
//! No production API; finite fault windows, not arbitrary kernel-failure proof.
mod native_children;
use super::*;
use crate::{
    SessionAdmissionPermit, SessionGrantSpec, SessionLifecycleManager, SessionRegistry,
    SessionRegistryError, TaskLeaseRegistry,
};
use optic_bridge_core::{Capability, MonotonicTime, PrincipalId, ProjectId};
use optic_bridge_windows_test_support as native;
use std::{env, io, sync::mpsc, thread, time::Instant as StdInstant};

const MEMORY: u64 = native::MEMORY;
const OUTPUT: usize = native::OUTPUT;
const CPU: u32 = native::CPU;

fn wait(mut predicate: impl FnMut() -> bool) {
    let until = StdInstant::now() + Duration::from_secs(3);
    while !predicate() {
        assert!(
            StdInstant::now() < until,
            "native owner research stage deadline"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

struct Client(Arc<AtomicBool>);
impl Drop for Client {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

// The identified keeper outlives Client. Native/stop worker/admission stay here
// on Pending; no detached worker, forgotten memory or released reservation.
struct Owner<'a> {
    native: native::Native,
    watch: native::StopWatch,
    record: Arc<JobRecord>,
    id: JobId,
    stopped: Arc<AtomicBool>,
    observe_error: bool,
    query_error: bool,
    last_native_error: Option<i32>,
    terminate_error: Arc<AtomicBool>,
    failed: bool,
    terminal: bool,
    admission: Option<SessionAdmissionPermit<'a>>,
}

#[must_use = "Pending contains the actual native keeper; retain it until proof"]
enum OwnedShutdown<'a> {
    Complete,
    Pending {
        keeper: Box<Owner<'a>>,
        error: Option<io::ErrorKind>,
    },
}
impl<'a> Owner<'a> {
    fn prepare(
        registry: &'a SessionRegistry,
        manager: &ProcessManager,
        session: &SessionHandle,
        now: MonotonicTime,
    ) -> io::Result<(Box<Self>, Client)> {
        let resources = ResourceBudget {
            timeout_ms: 4_000,
            output_bytes: OUTPUT as u64,
            memory_bytes: MEMORY,
            process_count: 2,
        };
        assert!(
            resources
                .validate_nonzero()
                .unwrap()
                .fits_within(manager.limits.max_process_budget)
        );
        assert_eq!(CPU, manager.limits.max_process_cpu_percent_per_job);
        let admission = registry
            .begin_admission(session, now)
            .map_err(io::Error::other)?;
        let (revoked, _) = registry
            .watch_active(session, now)
            .map_err(io::Error::other)?;
        let id = JobId::generate().unwrap();
        let mut record =
            super::tests::synthetic_record(session.clone(), 0, ProcessStatus::Running, MEMORY, CPU);
        let initial = Arc::get_mut(&mut record).unwrap();
        initial.reserved_output_bytes = OUTPUT as u64;
        initial.output.get_mut().unwrap().stdout_closed = false;
        initial.output.get_mut().unwrap().stderr_closed = false;
        {
            let mut store = manager.jobs.lock().unwrap();
            manager
                .prepare_store_for_start_with_workload(
                    &mut store,
                    session,
                    WorkloadClass::Standard,
                    OUTPUT as u64,
                    MEMORY,
                    CPU,
                )
                .map_err(io::Error::other)?;
            assert!(store.jobs.insert(id.clone(), Arc::clone(&record)).is_none());
        }
        let native = match native::Native::prepare() {
            Ok(native) => native,
            Err(error) => {
                let mut out = record.output.lock().unwrap();
                out.stdout_closed = true;
                out.stderr_closed = true;
                record.state.lock().unwrap().status = ProcessStatus::Failed;
                return Err(error);
            }
        };
        let stopped = Arc::new(AtomicBool::new(false));
        let terminate_error = Arc::new(AtomicBool::new(false));
        let watch = native::StopWatch::new(
            Arc::clone(&native.job),
            [
                Arc::clone(&record.stop_requested),
                revoked,
                Arc::clone(&stopped),
            ],
            Arc::clone(&terminate_error),
        );
        let client = Client(Arc::clone(&stopped));
        Ok((
            Box::new(Self {
                native,
                watch,
                record,
                id,
                stopped,
                observe_error: false,
                query_error: false,
                last_native_error: None,
                terminate_error,
                failed: false,
                terminal: false,
                admission: Some(admission),
            }),
            client,
        ))
    }
    fn start(&mut self, root: &Path, mode: &str) -> io::Result<()> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "owner stopped"));
        }
        self.native.create(root, mode, mode == "create-error")?;
        self.native.resume(mode == "resume-error")
    }
    fn poll(&mut self) -> io::Result<bool> {
        if self.terminal {
            return Ok(true);
        }
        if self.observe_error {
            self.record.state.lock().unwrap().status = ProcessStatus::TerminationUncertain;
            return Ok(false);
        }
        if self.query_error {
            native::Native::rejected_query_probe()?;
        }
        self.native.capture()?;
        if self.native.overflow {
            self.stopped.store(true, Ordering::Release);
        }
        if !self.native.terminal()? {
            return Ok(false);
        }
        // Joining is attempted only AFTER is_finished; Pending retains budgets.
        if !self.watch.try_finish() {
            return Ok(false);
        }
        let mut output = self.record.output.lock().unwrap();
        output.stdout = std::mem::take(&mut self.native.stdout);
        output.stderr = std::mem::take(&mut self.native.stderr);
        output.stdout_closed = true;
        output.stderr_closed = true;
        self.record
            .output_overflow
            .store(self.native.overflow, Ordering::Release);
        let mut state = self.record.state.lock().unwrap();
        state.status = if self.failed || self.watch.errors.load(Ordering::Acquire) > 0 {
            ProcessStatus::Failed
        } else if self.native.overflow {
            ProcessStatus::OutputLimitExceeded
        } else {
            ProcessStatus::Stopped
        };
        state.exit_code = self.native.root_code()?.map(|v| v as i32);
        drop(state);
        drop(output);
        self.terminal = true;
        // Genuine owner-held admission drains only AFTER all proof + join.
        self.admission.take();
        Ok(true)
    }
    fn try_shutdown(&mut self) -> io::Result<bool> {
        self.stopped.store(true, Ordering::Release);
        self.native.close_writers();
        self.poll()
    }

    // Box allocated during prepare, BEFORE any native child. Returning Pending
    // performs no allocation or transfer to an unowned thread/global collection.
    fn begin_shutdown(mut self: Box<Self>) -> OwnedShutdown<'a> {
        match self.try_shutdown() {
            Ok(true) => OwnedShutdown::Complete,
            Ok(false) => OwnedShutdown::Pending {
                keeper: self,
                error: None,
            },
            Err(error) => {
                self.failed = true;
                self.last_native_error = error.raw_os_error();
                self.record.state.lock().unwrap().status = ProcessStatus::TerminationUncertain;
                OwnedShutdown::Pending {
                    keeper: self,
                    error: Some(error.kind()),
                }
            }
        }
    }
}

fn exercise(root: &Path, mode: &str) -> serde_json::Value {
    let mode = mode.to_owned();
    assert!(matches!(
        mode.as_str(),
        "close-before-create"
            | "close-before-resume"
            | "launch-wins"
            | "close-during-native"
            | "create-error"
            | "resume-error"
            | "stop-error"
            | "query-error"
            | "observation-uncertain"
            | "client-drop"
            | "descendant-pipes"
            | "output-limit"
            | "panic-under-gate"
    ));
    let manager = Arc::new(ProcessManager::new(root, HardLimits::default(), Vec::new()).unwrap());
    let registry = Arc::new(SessionRegistry::new());
    let lifecycle = SessionLifecycleManager::new(
        Arc::clone(&registry),
        Arc::new(TaskLeaseRegistry::new()),
        Arc::clone(&manager),
    );
    let now = MonotonicTime::from_millis(1);
    let provision = || {
        lifecycle
            .provision(
                SessionGrantSpec {
                    principal: PrincipalId::new("native-research").unwrap(),
                    project: ProjectId::new("synthetic").unwrap(),
                    capabilities: BTreeSet::from([Capability::GitRead]),
                    expires_at: MonotonicTime::from_millis(10_000),
                    policy_epoch: 1,
                },
                now,
            )
            .unwrap()
    };
    let a = provision();
    let b = provision();
    let handles_before = native::Native::current_handle_count().unwrap();
    let (mut owner, client) = Owner::prepare(&registry, &manager, &a.handle, now).unwrap();
    let mut client = Some(client);
    let mut close_during_call_observed = false;
    let mut error_code = None;
    if mode == "close-before-create" {
        assert_eq!(
            registry.research_close_admission(&a.handle).unwrap(),
            (true, 1)
        );
        assert!(matches!(
            registry.research_commit_launch(&a.handle, now, || owner.start(root, &mode)),
            Err(SessionRegistryError::Revoked)
        ));
    } else if mode == "close-before-resume" {
        registry
            .research_commit_launch(&a.handle, now, || owner.native.create(root, &mode, false))
            .unwrap()
            .unwrap();
        assert_eq!(owner.native.job.active().unwrap(), 1);
        registry.research_close_admission(&a.handle).unwrap();
        assert!(matches!(
            registry.research_commit_launch(&a.handle, now, || owner.native.resume(false)),
            Err(SessionRegistryError::Revoked)
        ));
    } else if mode == "close-during-native" {
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (attempt_tx, attempt_rx) = mpsc::sync_channel(1);
        let (closed_tx, closed_rx) = mpsc::sync_channel(1);
        let native_registry = registry.as_ref();
        let native_handle = &a.handle;
        let native_mode = mode.clone();
        owner = thread::scope(|scope| {
            let launch = scope.spawn(move || {
                let mut retained = owner;
                native_registry
                    .research_commit_launch(native_handle, now, || {
                        retained.native.create(root, &native_mode, false).unwrap();
                        entered_tx.send(()).unwrap();
                        release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                        retained.native.resume(false).unwrap();
                    })
                    .unwrap();
                retained
            });
            entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            let close = scope.spawn(|| {
                attempt_tx.send(()).unwrap();
                closed_tx.send(registry.revoke(&a.handle)).unwrap();
            });
            attempt_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            wait(|| registry.research_gate_reference_count(&a.handle).unwrap() == 3);
            // Gate-held native phase delays A close, but NEVER holds global mutex.
            registry
                .research_commit_launch(&b.handle, now, || ())
                .unwrap();
            {
                let mut store = manager
                    .jobs
                    .try_lock()
                    .expect("no JobStore mutex across native call");
                manager
                    .prepare_store_for_start_with_workload(
                        &mut store,
                        &b.handle,
                        WorkloadClass::Standard,
                        OUTPUT as u64,
                        MEMORY,
                        CPU,
                    )
                    .unwrap();
            }
            assert!(registry.get_active(&a.handle, now).is_ok());
            assert!(matches!(
                closed_rx.try_recv(),
                Err(mpsc::TryRecvError::Empty)
            ));
            close_during_call_observed = true;
            release_tx.send(()).unwrap();
            let mut retained = launch.join().unwrap();
            wait(|| {
                matches!(
                    registry.get_active(&a.handle, now),
                    Err(SessionRegistryError::Revoked)
                )
            });
            wait(|| retained.try_shutdown().unwrap());
            closed_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap();
            close.join().unwrap();
            retained
        });
    } else if mode == "panic-under-gate" {
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            registry
                .research_commit_launch(&a.handle, now, || {
                    owner.start(root, &mode).unwrap();
                    panic!("fixed test panic; owner is outside unwind boundary");
                })
                .unwrap();
        }));
        assert!(caught.is_err());
        owner.failed = true;
        registry.research_close_admission(&a.handle).unwrap();
    } else {
        let result = registry
            .research_commit_launch(&a.handle, now, || owner.start(root, &mode))
            .unwrap();
        if matches!(mode.as_str(), "create-error" | "resume-error") {
            error_code = result.as_ref().err().and_then(io::Error::raw_os_error);
            assert!(result.is_err());
            owner.failed = true;
        } else {
            result.unwrap();
            wait(|| root.join("executed").exists());
        }
    }
    if mode == "output-limit" {
        wait(|| {
            owner.poll().unwrap();
            owner.native.overflow
        });
    }
    if mode == "descendant-pipes" {
        wait(|| {
            owner.native.capture().unwrap();
            owner.native.root_exited().unwrap() && root.join("descendant").exists()
        });
        assert_eq!(owner.native.root_code().unwrap(), Some(0));
        assert_eq!(owner.native.job.active().unwrap(), 1);
        assert!(!owner.native.out_eof && !owner.native.err_eof);
        fs::write(root.join("retained-ready"), b"root exited, writer retained").unwrap();
        wait(|| root.join("retained-confirmed").exists());
    }
    if matches!(
        mode.as_str(),
        "stop-error" | "query-error" | "observation-uncertain" | "client-drop"
    ) {
        owner.observe_error = mode != "query-error";
        owner.query_error = mode == "query-error";
        if mode == "stop-error" {
            owner.terminate_error.store(true, Ordering::Release);
        }
        if mode == "client-drop" {
            drop(client.take());
            assert!(owner.stopped.load(Ordering::Acquire));
            assert!(!owner.poll().unwrap());
            assert!(registry.get_active(&a.handle, now).is_ok());
        } else {
            assert_eq!(
                registry.research_close_admission(&a.handle).unwrap(),
                (true, 1)
            );
            if mode != "query-error" {
                assert!(!owner.try_shutdown().unwrap());
            }
        }
        if mode == "stop-error" {
            wait(|| owner.watch.errors.load(Ordering::Acquire) > 0);
            assert_eq!(owner.native.job.active().unwrap(), 1);
        } else {
            fs::write(root.join("uncertain-ready"), b"keeper retained").unwrap();
            wait(|| root.join("uncertain-confirmed").exists());
        }
        if mode == "client-drop" {
            // The external witness saw cleanup caused by Client Drop alone.
            assert!(registry.get_active(&a.handle, now).is_ok());
            assert_eq!(
                registry.research_close_admission(&a.handle).unwrap(),
                (true, 1)
            );
        }
        let address = owner.as_ref() as *const Owner<'_>;
        let record_address = Arc::as_ptr(&owner.record);
        owner = match owner.begin_shutdown() {
            OwnedShutdown::Pending { keeper, error } => {
                assert_eq!(error.is_some(), mode == "query-error");
                if mode == "query-error" {
                    assert_eq!(keeper.last_native_error, Some(6));
                }
                assert_eq!(keeper.as_ref() as *const Owner<'_>, address);
                assert_eq!(Arc::as_ptr(&keeper.record), record_address);
                assert!(keeper.admission.is_some());
                keeper
            }
            OwnedShutdown::Complete => panic!("observation unavailable cannot complete shutdown"),
        };
        // Repeated pending replies retain the SAME root, not merely an enum.
        for _ in 0..2 {
            owner = match owner.begin_shutdown() {
                OwnedShutdown::Pending { keeper, .. } => {
                    assert_eq!(keeper.as_ref() as *const Owner<'_>, address);
                    assert_eq!(Arc::as_ptr(&keeper.record), record_address);
                    keeper
                }
                OwnedShutdown::Complete => panic!("unavailable observation cannot complete"),
            };
        }
        assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
        assert_eq!(
            manager.remove_terminal_session_records(&a.handle).unwrap(),
            0
        );
        assert_eq!(
            registry
                .remove_quiescent_inactive(&a.handle, now)
                .unwrap_err(),
            SessionRegistryError::AdmissionsInFlight
        );
        {
            let store = manager.jobs.lock().unwrap();
            assert_eq!(reserved_process_memory_bytes(&store).unwrap(), MEMORY);
            assert_eq!(reserved_cpu_percent(&store).unwrap(), CPU);
            assert_eq!(reserved_output_bytes(&store), OUTPUT as u64);
        }
        registry
            .research_commit_launch(&b.handle, now, || ())
            .unwrap();
        {
            let mut store = manager
                .jobs
                .try_lock()
                .expect("pending owner does not lock global accounting");
            manager
                .prepare_store_for_start_with_workload(
                    &mut store,
                    &b.handle,
                    WorkloadClass::Standard,
                    OUTPUT as u64,
                    MEMORY,
                    CPU,
                )
                .unwrap();
        }
        thread::scope(|scope| {
            let (entered_tx, entered_rx) = mpsc::sync_channel(1);
            let (done_tx, done_rx) = mpsc::sync_channel(1);
            let closing_lifecycle = &lifecycle;
            let closing_session = &a.handle;
            let closer = scope.spawn(move || {
                entered_tx.send(()).unwrap();
                done_tx
                    .send(closing_lifecycle.revoke(closing_session))
                    .unwrap();
            });
            entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
            owner.terminate_error.store(false, Ordering::Release);
            owner.observe_error = false;
            owner.query_error = false;
            wait(|| owner.try_shutdown().unwrap());
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap();
            closer.join().unwrap();
        });
    }
    if !owner.terminal {
        registry.research_close_admission(&a.handle).unwrap();
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        thread::scope(|scope| {
            let closer = scope.spawn(|| done_tx.send(lifecycle.revoke(&a.handle)).unwrap());
            wait(|| owner.try_shutdown().unwrap());
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap();
            closer.join().unwrap();
        });
    }
    assert!(owner.terminal && owner.admission.is_none());
    assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 0);
    let result = manager.result(&a.handle, &owner.id).unwrap();
    let output = owner.record.output.lock().unwrap();
    assert!(output.stdout_closed && output.stderr_closed);
    let bytes = output.stdout.len() + output.stderr.len();
    assert!(bytes <= OUTPUT);
    drop(output);
    if matches!(mode.as_str(), "close-before-create" | "close-before-resume") {
        assert!(!root.join("executed").exists());
    }
    if matches!(
        mode.as_str(),
        "create-error" | "resume-error" | "stop-error" | "query-error" | "panic-under-gate"
    ) {
        assert_eq!(result.status, ProcessStatus::Failed);
    }
    if mode == "output-limit" {
        assert_eq!(result.status, ProcessStatus::OutputLimitExceeded);
    }
    assert!(lifecycle.seal_quiescent(&a.handle).is_ok());
    assert!(lifecycle.try_reap(&a.handle, now).unwrap().session_removed);
    assert!(registry.get_active(&b.handle, now).is_ok());
    if error_code.is_none() {
        error_code = owner.last_native_error;
    }
    if error_code.is_none() {
        let code = owner.watch.first_error.load(Ordering::Acquire);
        if code != 0 {
            error_code = Some(code as i32);
        }
    }
    let associated = owner.native.association_observed;
    assert!(matches!(owner.begin_shutdown(), OwnedShutdown::Complete));
    drop(client);
    let handles_after = native::Native::current_handle_count().unwrap();
    serde_json::json!({"mode":mode,"instrumented_registry":true,"real_win32":true,
        "native_created":associated,"atomic_job_association":associated,
        "native_terminal_and_both_eof":true,"stop_worker_joined":true,
        "shared_accounting_released_after_proof":true,"other_session_active":true,
        "close_during_native_observed":close_during_call_observed,"native_error_code":error_code,
        "combined_output_bytes":bytes,"status":format!("{:?}",result.status),
        "handles_before_owner":handles_before,"handles_after_owner":handles_after,
        "production_wired":false})
}

#[test]
#[ignore = "requires pre-armed external Job; runs fixed synthetic processes only"]
fn authority_native_owner_host() {
    let gate = PathBuf::from(env::var_os("OPTIC317_OWNER_GATE").unwrap());
    wait(|| gate.exists());
    let root = gate.parent().unwrap();
    let mode = env::var("OPTIC317_OWNER_MODE").unwrap();
    let proof = if mode == "handle-recovery" {
        let warm = root.join("warm");
        let measured = root.join("measured");
        fs::create_dir(&warm).unwrap();
        fs::create_dir(&measured).unwrap();
        let first = exercise(&warm, "launch-wins");
        let mut second = exercise(&measured, "launch-wins");
        assert_eq!(
            second["handles_before_owner"], second["handles_after_owner"],
            "post-warmup owner must return all measured handles"
        );
        second["mode"] = serde_json::json!(mode);
        second["warmup_handles_before"] = first["handles_before_owner"].clone();
        second["warmup_handles_after"] = first["handles_after_owner"].clone();
        second["handle_count_stable_after_warmup"] = serde_json::json!(true);
        second
    } else {
        exercise(root, &mode)
    };
    fs::write(root.join("ready"), proof.to_string()).unwrap();
    // External supervisor observes host-only, then owns this host's termination.
    thread::sleep(Duration::from_secs(10));
    panic!("external supervisor did not recover host");
}
