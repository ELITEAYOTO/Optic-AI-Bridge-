//! Additional finite root lifecycle schedules. Never invoke without outer Job.
use super::*;

struct ManualClock(AtomicU64);
impl Clock for ManualClock {
    fn now(&self) -> MonotonicTime {
        MonotonicTime::from_millis(self.0.load(Ordering::Acquire))
    }
}

pub(super) fn exercise(path: &Path, mode: &str) -> serde_json::Value {
    if mode.starts_with("root-drop-") {
        return drop_case(path, mode);
    }
    let registry = Arc::new(SessionRegistry::new());
    let manager = super::memory_sample::manager(path, HardLimits::default());
    let lifecycle = SessionLifecycleManager::new(
        Arc::clone(&registry),
        Arc::new(TaskLeaseRegistry::new()),
        Arc::clone(&manager),
    );
    // Frozen monotonic fixture deliberately separates the producer's clock
    // from its independent REAL controller timer. A spent controller budget
    // must be a latched stop regardless of a producer's stale clock sample.
    let clock = ManualClock(AtomicU64::new(0));
    let grant = lifecycle
        .provision(
            SessionGrantSpec {
                principal: PrincipalId::new("controller").unwrap(),
                project: ProjectId::new("synthetic").unwrap(),
                capabilities: BTreeSet::from([Capability::GitRead]),
                expires_at: MonotonicTime::from_millis(2000),
                policy_epoch: 1,
            },
            clock.now(),
        )
        .unwrap();
    let (mut root, client) = Root::new(&registry, &manager, &grant.handle, &clock);
    root.prepare(|| (), true).unwrap();
    match mode {
        "renewal-no-budget-reset" => {
            lifecycle
                .renew(&grant.handle, clock.now(), MonotonicTime::from_millis(4000))
                .unwrap();
            clock.0.store(2001, Ordering::Release);
            assert!(registry.get_active(&grant.handle, clock.now()).is_ok());
            assert!(matches!(root.commit(), Err(SessionRegistryError::Expired)));
            assert!(
                !root
                    .slot
                    .as_ref()
                    .unwrap()
                    .watch
                    .deadline_expired
                    .load(Ordering::Acquire)
            );
        }
        "expiry-running" => {
            root.commit().unwrap();
            root.start(path, mode, || ()).unwrap();
            wait(|| path.join("executed").exists());
            clock.0.store(2001, Ordering::Release);
            assert!(matches!(
                registry.get_active(&grant.handle, clock.now()),
                Err(SessionRegistryError::Expired)
            ));
            assert!(!root.observe().unwrap());
        }
        "controller-expired" | "controller-expired-before-commit" => {
            if mode == "controller-expired" {
                root.commit().unwrap();
            }
            wait(|| {
                root.slot
                    .as_ref()
                    .unwrap()
                    .watch
                    .deadline_expired
                    .load(Ordering::Acquire)
            });
            assert!(registry.get_active(&grant.handle, clock.now()).is_ok());
            if mode == "controller-expired" {
                assert_eq!(
                    root.start(path, mode, || ()).unwrap_err().kind(),
                    io::ErrorKind::Interrupted
                );
            } else {
                assert!(matches!(root.commit(), Err(SessionRegistryError::Expired)));
            }
        }
        _ => panic!("unknown fixed lifecycle schedule"),
    }
    if mode != "expiry-running" {
        assert!(
            !root.slot.as_ref().unwrap().native.creation_attempted(),
            "expired controller must prevent native creation attempt"
        );
    }
    root.request_shutdown();
    wait(|| root.observe().unwrap());
    drop(client);
    super::scenarios::finish(
        root,
        &lifecycle,
        path,
        mode,
        false,
        serde_json::json!({"controller_expiry_latched_before_start":mode.starts_with("controller-expired"),
        "producer_clock":"frozen-monotonic-fixture","independent_timer":"real-Instant"}),
    )
}

// The actual retaining destructor, not a detached proxy. All scoped workers
// stay owned until proof, or until the pre-armed external Job kills this host.
fn drop_case(path: &Path, mode: &str) -> serde_json::Value {
    let registry = Arc::new(SessionRegistry::new());
    let manager = super::memory_sample::manager(path, HardLimits::default());
    let lifecycle = SessionLifecycleManager::new(
        Arc::clone(&registry),
        Arc::new(TaskLeaseRegistry::new()),
        Arc::clone(&manager),
    );
    let clock = StdClock::new();
    let grant = super::scenarios::provision(&lifecycle, &clock, "drop-owner");
    let independent = super::scenarios::provision(&lifecycle, &clock, "independent");
    let (mut root, client) = Root::new(&registry, &manager, &grant.handle, &clock);
    root.prepare(|| (), true).unwrap();
    root.commit().unwrap();
    root.start(path, mode, || ()).unwrap();
    wait(|| path.join("executed").exists());
    let query = Arc::clone(&root.fail_query);
    let errors = Arc::clone(&root.errors);
    let k = root.slot.as_ref().unwrap();
    let record = Arc::clone(&k.record);
    let stop_errors = Arc::clone(&k.watch.errors);
    let id = k.id.clone();
    let persistent = mode == "root-drop-persistent";
    query.store(true, Ordering::Release);
    root.fail_stop.store(persistent, Ordering::Release);
    assert!(!root.request_shutdown());
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let (revoke_tx, revoke_rx) = mpsc::sync_channel(1);
    thread::scope(|scope| {
        let registry_ref = &registry;
        let handle = &grant.handle;
        let closer = scope.spawn(move || {
            revoke_tx
                .send(registry_ref.revoke(handle).unwrap())
                .unwrap();
        });
        super::scenarios::wait_closed(&registry, handle, &clock);
        let keeper = scope.spawn(move || {
            drop(root);
            done_tx.send(()).unwrap();
        });
        wait(|| {
            errors.load(Ordering::Acquire) >= 4
                && (!persistent || stop_errors.load(Ordering::Acquire) >= 4)
        });
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(matches!(
            revoke_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert_eq!(manager.active_session_job_count(handle).unwrap(), 1);
        assert!(
            registry
                .remove_quiescent_inactive(handle, clock.now())
                .is_err()
        );
        let other = registry
            .begin_admission(&independent.handle, clock.now())
            .unwrap();
        drop(other);
        if persistent {
            let proof = serde_json::json!({"mode":mode,"production_wired":false,"completion_kind":"host-exit-pending",
                "real_registry":true,"real_admission":true,"real_win32":true,"native_created":true,
                "native_create_attempted":true,"atomic_job_association":true,"native_terminal_and_both_eof":false,
                "stop_worker_joined":false,"active_ownership_released_after_proof":false,"root_drop_completed":false,
                "status":"TerminationUncertain","combined_output_bytes":0,"observation_errors":errors.load(Ordering::Acquire),
                "stop_errors":stop_errors.load(Ordering::Acquire),"retained_active_jobs":1,"independent_admission_completed":true});
            fs::write(path.join("pending-ready"), proof.to_string()).unwrap();
            thread::sleep(Duration::from_secs(10));
            panic!("outer supervisor failed to recover blocked retaining Drop");
        }
        fs::write(path.join("drop-ready"), b"retaining").unwrap();
        wait(|| path.join("drop-confirmed").exists());
        query.store(false, Ordering::Release);
        done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(revoke_rx.recv_timeout(Duration::from_secs(2)).unwrap());
        keeper.join().unwrap();
        closer.join().unwrap();
    });
    drop(client);
    let result = manager.result(&grant.handle, &id).unwrap();
    assert_eq!(manager.active_session_job_count(&grant.handle).unwrap(), 0);
    let output = record.output.lock().unwrap();
    assert!(output.stdout_closed && output.stderr_closed);
    let bytes = output.stdout.len() + output.stderr.len();
    assert!(bytes <= native::OUTPUT);
    assert!(lifecycle.seal_quiescent(&grant.handle).is_ok());
    serde_json::json!({"mode":mode,"production_wired":false,"completion_kind":"native-or-no-spawn-proof",
        "real_registry":true,"real_admission":true,"real_win32":true,"native_created":true,
        "native_create_attempted":true,"atomic_job_association":true,"native_terminal_and_both_eof":true,
        "stop_worker_joined":true,"active_ownership_released_after_proof":true,"root_drop_completed":true,
        "legacy_manual_cancel_rescue":false,"status":format!("{:?}",result.status),"combined_output_bytes":bytes,
        "observation_errors":errors.load(Ordering::Acquire),"stop_errors":stop_errors.load(Ordering::Acquire),
        "independent_admission_completed":true})
}
