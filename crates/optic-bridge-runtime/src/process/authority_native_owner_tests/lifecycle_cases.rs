//! Additional finite root lifecycle schedules. Never invoke without outer Job.
use super::*;

struct ManualClock(AtomicU64);
impl Clock for ManualClock {
    fn now(&self) -> MonotonicTime {
        MonotonicTime::from_millis(self.0.load(Ordering::Acquire))
    }
}

pub(super) fn exercise(path: &Path, mode: &str) -> serde_json::Value {
    assert_eq!(mode, "controller-expired");
    let registry = Arc::new(SessionRegistry::new());
    let manager = Arc::new(ProcessManager::new(path, HardLimits::default(), Vec::new()).unwrap());
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
                expires_at: MonotonicTime::from_millis(1000),
                policy_epoch: 1,
            },
            clock.now(),
        )
        .unwrap();
    let (mut root, client) = Root::new(&registry, &manager, &grant.handle, &clock);
    root.prepare(|| (), true).unwrap();
    root.commit().unwrap();
    wait(|| {
        root.slot
            .as_ref()
            .unwrap()
            .watch
            .deadline_expired
            .load(Ordering::Acquire)
    });
    assert!(registry.get_active(&grant.handle, clock.now()).is_ok());
    let result = root.start(path, mode, || ());
    assert!(
        !root.slot.as_ref().unwrap().native.creation_attempted(),
        "expired controller must prevent native creation attempt"
    );
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
    root.request_shutdown();
    wait(|| root.observe().unwrap());
    drop(client);
    super::scenarios::finish(
        root,
        &lifecycle,
        path,
        mode,
        false,
        serde_json::json!({"controller_expiry_latched_before_start":true,
        "producer_clock":"frozen-monotonic-fixture","independent_timer":"real-Instant"}),
    )
}
