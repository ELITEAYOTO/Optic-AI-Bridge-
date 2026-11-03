//! Inert accounting for native OS calls which may outlive a stop request.
//!
//! This is a prerequisite for a future Windows GitRead guardian, NOT an
//! authorization issuer: no session/TaskLease/policy/host authority is bound
//! here, and possession of a ticket MUST NOT be used as a spawn permit.
//!
//! A stop request closes admission immediately without waiting for an in-flight
//! Win32 call. Existing calls remain counted until their owners actually return.
//! Zero in-flight calls is NOT proof of process termination or pipe EOF.

use std::sync::{
    Arc, Mutex, TryLockError,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy)]
pub(crate) enum NativeCallPhase {
    CreateSuspended,
    Resume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InFlightNativeCalls {
    pub(crate) creates: usize,
    pub(crate) resumes: usize,
}

#[derive(Default)]
struct CallCounts {
    creates: usize,
    resumes: usize,
}

/// Only the original accounting ledger is sealed and empty. NOT authorization
/// and NOT a native process / Job / stdout / stderr terminal receipt.
#[must_use = "hold the original ledger until native OS and pipe teardown"]
#[allow(dead_code)] // Prepared for the future native owner; intentionally not wired to GitRead.
pub(crate) struct NativeCallDrainReceipt<'a> {
    original: &'a NativeCallStopTracker,
}

impl NativeCallDrainReceipt<'_> {
    #[cfg(test)]
    fn is_sealed(&self) -> bool {
        self.original.shared.stop_requested.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)] // Not a public MCP API, consumed by private native guardian later.
pub(crate) enum NativeCallDrainError {
    TimedOut,
    Unknown,
}

/// Four retained, original and irreversible stop sources: session generation,
/// TaskLease generation, counted Job and process-owned host.
struct OriginalNativeStops {
    signals: [Arc<AtomicBool>; 4],
    deadline: std::time::Instant,
}

struct Shared {
    stop_requested: Arc<AtomicBool>,
    counts: Mutex<CallCounts>,
    original: Option<OriginalNativeStops>,
}

impl Shared {
    fn new(original: Option<OriginalNativeStops>) -> Self {
        Self {
            stop_requested: Arc::new(AtomicBool::new(false)),
            counts: Mutex::new(CallCounts::default()),
            original,
        }
    }

    fn is_stop_required(&self) -> bool {
        self.stop_requested.load(Ordering::Acquire)
            // A poisoned counter is unknown custody. Publish the same
            // irreversible internal stop which the already-armed original
            // Job watchdog observes; true as a return value alone is not
            // enough to request native termination.
            || self.stop_if_poisoned()
            || self.original.as_ref().is_some_and(|original| {
                std::time::Instant::now() >= original.deadline
                    || original
                        .signals
                        .iter()
                        .any(|signal| signal.load(Ordering::Acquire))
            })
    }

    fn stop_if_poisoned(&self) -> bool {
        if self.counts.is_poisoned() {
            self.stop_requested.store(true, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// A short transient counter lock should not falsely poison native
    /// custody. But final observations and ticket retirement MUST NOT block
    /// indefinitely: after a fixed, non-renewable 25 ms budget, keep the
    /// accounting uncertain and publish the original watchdog stop.
    fn lock_counts_bounded(&self) -> Option<std::sync::MutexGuard<'_, CallCounts>> {
        let until = std::time::Instant::now() + std::time::Duration::from_millis(25);
        loop {
            match self.counts.try_lock() {
                Ok(guard) => return Some(guard),
                Err(TryLockError::Poisoned(_)) => {
                    self.stop_requested.store(true, Ordering::Release);
                    return None;
                }
                Err(TryLockError::WouldBlock) if std::time::Instant::now() < until => {
                    std::thread::yield_now();
                }
                Err(TryLockError::WouldBlock) => {
                    self.stop_requested.store(true, Ordering::Release);
                    return None;
                }
            }
        }
    }
}

/// This accounting gate is deliberately not attached to the live MCP or native
/// execution path. Production construction requires the original stop sources.
/// It is neither Clone nor Default: a detached copy cannot issue new tickets.
pub(crate) struct NativeCallStopTracker {
    shared: Shared,
}

/// A call accounting ticket BORROWS its tracker. A future guardian must keep
/// this tracker inside its uniquely owned native custody until the OS call
/// returns; a tracked call cannot outlive that Rust owner via this API.
///
/// This is NOT launch authorization or OS termination/EOF proof. Stop can
/// still win between a tracked admission and the actual kernel entry.
#[must_use = "keep the borrowed call ticket until the native API returns"]
pub(crate) struct TrackedNativeCall<'a> {
    shared: &'a Shared,
    phase: NativeCallPhase,
}

impl NativeCallStopTracker {
    // A source-free tracker is confined to the unit-only concurrency fixture.
    // The real precreated owner must bind its ORIGINAL stop signals and horizon.
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            shared: Shared::new(None),
        }
    }

    pub(crate) fn with_original_stops(
        signals: [Arc<AtomicBool>; 4],
        deadline: std::time::Instant,
    ) -> Self {
        Self {
            shared: Shared::new(Some(OriginalNativeStops { signals, deadline })),
        }
    }

    /// Register potential in-flight work, not permission to launch. The second
    /// stop check under the short mutex closes admission after a prior stop.
    /// A concurrent stop after that check makes this an already-admitted call
    /// which the eventual guardian must reconcile or forcibly terminate.
    pub(crate) fn track(&self, phase: NativeCallPhase) -> Option<TrackedNativeCall<'_>> {
        self.track_with_before_commit(phase, || {})
    }

    // The callback is internal and empty in production. A Windows concurrency
    // test uses it as a deterministic barrier immediately before the final
    // ticket commitment, while the private counter mutex is still held.
    fn track_with_before_commit(
        &self,
        phase: NativeCallPhase,
        before_commit: impl FnOnce(),
    ) -> Option<TrackedNativeCall<'_>> {
        if self.shared.is_stop_required() {
            return None;
        }
        // Admission cannot block behind a wedged counter owner. No caller
        // may interpret contention as an untracked Create/Resume permit:
        // permanently stop this original owner and deny the new ticket.
        let mut counts = match self.shared.counts.try_lock() {
            Ok(counts) => counts,
            Err(TryLockError::Poisoned(_)) | Err(TryLockError::WouldBlock) => {
                self.request_stop();
                return None;
            }
        };
        if self.shared.is_stop_required() {
            return None;
        }
        let counter = match phase {
            NativeCallPhase::CreateSuspended => &mut counts.creates,
            NativeCallPhase::Resume => &mut counts.resumes,
        };
        let Some(next) = counter.checked_add(1) else {
            // An exhausted ledger cannot safely admit another call. Seal
            // admission permanently, even if counts subsequently decrease.
            // This is not an OS launch/revocation synchronization barrier.
            self.request_stop();
            return None;
        };
        *counter = next;
        before_commit();
        // Compare-and-exchange of the SAME stop bit used by request_stop
        // orders internal stop against ticket commitment, unlike a separate
        // load followed by publication. This RMW is NOT a cross-registry
        // issuer. The original session, TaskLease, Job and host generations
        // and original deadline are independent and may stop AFTER the last
        // pre-counter check. Observe them again AFTER the RMW, while holding
        // the counter mutex: any observed stop rolls back the pending ticket.
        // A later stop still leaves an already-admitted call requiring native
        // watchdog/guardian reconciliation. This does NOT synchronize real
        // CreateProcessW/ResumeThread against external revocation.
        if self
            .shared
            .stop_requested
            .compare_exchange(false, false, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
            || self.shared.is_stop_required()
        {
            *counter -= 1;
            return None;
        }
        Some(TrackedNativeCall {
            shared: &self.shared,
            phase,
        })
    }

    /// Irreversibly closes admission before touching the count mutex. It does
    /// NOT synchronously stop an already admitted OS call or complete revoke.
    pub(crate) fn request_stop(&self) {
        self.shared.stop_requested.store(true, Ordering::Release);
    }

    /// Seal this ORIGINAL ledger before acquiring its count lock, then wait
    /// for BOTH already-admitted native Create and Resume tickets to retire.
    /// Admission cannot reopen, even if the wait exceeds the absolute bound.
    ///
    /// This is not a synchronized session/lease/policy/host revoke fence, a
    /// native Win32 launch permit, or proof of root/Job/pipe termination.
    #[allow(dead_code)] // Not wired into the live GitRead path.
    pub(crate) fn seal_and_drain_until(
        &self,
        until: std::time::Instant,
    ) -> Result<NativeCallDrainReceipt<'_>, NativeCallDrainError> {
        self.request_stop();
        // A caller-supplied cleanup bound can shorten but MUST NEVER renew
        // the original service-clock deadline captured at admission.
        let until = self
            .shared
            .original
            .as_ref()
            .map_or(until, |original| until.min(original.deadline));
        // Mutex::lock and Condvar::wait_timeout can both wait *past* the
        // requested bound while reacquiring a contended counter mutex.
        // Try-lock with an absolute-deadline-bound probe instead: a stalled
        // counter owner must never turn a bounded drain into an unbounded one.
        loop {
            if self.shared.counts.is_poisoned() {
                self.request_stop();
                return Err(NativeCallDrainError::Unknown);
            }
            if std::time::Instant::now() >= until {
                return Err(NativeCallDrainError::TimedOut);
            }
            match self.shared.counts.try_lock() {
                Ok(counts) => {
                    if counts.creates == 0 && counts.resumes == 0 {
                        // Do not publish a positive receipt if the absolute
                        // deadline expired during the counter observation.
                        if std::time::Instant::now() >= until {
                            return Err(NativeCallDrainError::TimedOut);
                        }
                        return Ok(NativeCallDrainReceipt { original: self });
                    }
                }
                Err(TryLockError::Poisoned(_)) => {
                    self.request_stop();
                    return Err(NativeCallDrainError::Unknown);
                }
                Err(TryLockError::WouldBlock) => {
                    // Still owned by another thread: this is not an empty ledger.
                }
            }
            let remaining = until.saturating_duration_since(std::time::Instant::now());
            std::thread::sleep(remaining.min(std::time::Duration::from_millis(1)));
        }
    }

    /// Private stop source for the original Job watchdog, not a launch grant.
    /// Cloning the Arc retains the exact internal fail-closed event; the
    /// independent watchdog cannot outlive its original Job custody.
    pub(crate) fn internal_stop_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shared.stop_requested)
    }

    /// Deterministic poison of the real ledger mutex without creating an OS
    /// call, exposed only to hosted Rust test builds.
    #[cfg(test)]
    pub(crate) fn poison_ledger_for_test(&self) {
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| {
                let _guard = self.shared.counts.lock().expect("fresh ledger");
                panic!("intentional test-only native ledger poison");
            });
            assert!(worker.join().is_err(), "fixture must poison the ledger");
        });
        assert!(self.shared.counts.is_poisoned());
    }

    /// Original revoke and original absolute expiry close future admission,
    /// even if nobody explicitly called request_stop on this tracker.
    pub(crate) fn stop_requested(&self) -> bool {
        self.shared.is_stop_required()
    }

    /// None on poisoned or persistently contended accounting is unknown,
    /// NOT an empty ledger. A short bounded observation tolerates transient
    /// contention; beyond its fixed bound the ORIGINAL watchdog is stopped.
    /// Even Some(0, 0) says nothing about live children, Job or pipe EOF.
    pub(crate) fn in_flight(&self) -> Option<InFlightNativeCalls> {
        let counts = self.shared.lock_counts_bounded()?;
        Some(InFlightNativeCalls {
            creates: counts.creates,
            resumes: counts.resumes,
        })
    }
}

impl Drop for TrackedNativeCall<'_> {
    fn drop(&mut self) {
        // Retiring a completed native call cannot wait indefinitely behind
        // another counter owner. Brief contention is retried within a hard
        // fixed bound; poison or prolonged contention preserves the original
        // count and irreversibly signals the ORIGINAL Job watchdog.
        // Never invent a zero count or native drain receipt after uncertainty.
        if let Some(mut counts) = self.shared.lock_counts_bounded() {
            let counter = match self.phase {
                NativeCallPhase::CreateSuspended => &mut counts.creates,
                NativeCallPhase::Resume => &mut counts.resumes,
            };
            *counter = counter
                .checked_sub(1)
                .expect("one retained ticket per accounted native call");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn sealed_drain_cannot_certify_in_flight_create_or_resume_and_closes_admission() {
        for phase in [NativeCallPhase::CreateSuspended, NativeCallPhase::Resume] {
            let tracker = NativeCallStopTracker::new();
            let ticket = tracker.track(phase).expect("real original ledger ticket");
            let (entered_tx, entered_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let returned = AtomicBool::new(false);
            std::thread::scope(|scope| {
                let worker = scope.spawn(move || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                    drop(ticket);
                });
                entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                let returned_ref = &returned;
                let original = &tracker;
                let drain = scope.spawn(move || {
                    let receipt = original
                        .seal_and_drain_until(std::time::Instant::now() + Duration::from_secs(2))
                        .expect("in-flight call returned within original bound");
                    assert!(receipt.is_sealed());
                    returned_ref.store(true, Ordering::Release);
                });
                let bound = std::time::Instant::now() + Duration::from_secs(1);
                while !tracker.stop_requested() {
                    assert!(std::time::Instant::now() < bound, "seal did not begin");
                    std::thread::yield_now();
                }
                assert!(!returned.load(Ordering::Acquire));
                assert!(tracker.track(NativeCallPhase::CreateSuspended).is_none());
                assert!(tracker.track(NativeCallPhase::Resume).is_none());
                let counted = tracker.in_flight().expect("unpoisoned ledger");
                assert_eq!(counted.creates + counted.resumes, 1);
                release_tx.send(()).unwrap();
                worker.join().unwrap();
                drain.join().unwrap();
            });
            assert!(returned.load(Ordering::Acquire));
            let drained = tracker.in_flight().expect("complete counts");
            assert_eq!(drained.creates + drained.resumes, 0);
            assert!(tracker.stop_requested());
            assert!(tracker.track(phase).is_none());
        }
    }

    #[test]
    fn internal_stop_between_count_and_ticket_commit_must_deny_and_rollback() {
        // This barrier is AFTER the existing stop checks and counter
        // increment, but BEFORE the ticket is returned to the caller.
        // A stop published here must win over a later ticket commitment.
        // The test never creates an OS process or grants launch authority.
        for phase in [NativeCallPhase::CreateSuspended, NativeCallPhase::Resume] {
            let tracker = NativeCallStopTracker::new();
            let (counted_tx, counted_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            std::thread::scope(|scope| {
                let original = &tracker;
                let registrar = scope.spawn(move || {
                    original
                        .track_with_before_commit(phase, || {
                            counted_tx.send(()).expect("counter incremented");
                            release_rx
                                .recv_timeout(Duration::from_secs(3))
                                .expect("bounded release of ticket commitment");
                        })
                        .is_some()
                });
                counted_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("ticket reaches pre-commit barrier");
                // Must not need the held counter mutex to publish stop.
                tracker.request_stop();
                assert!(tracker.stop_requested());
                release_tx.send(()).expect("release fixture after stop");
                let accepted = registrar.join().expect("registrar completes");
                assert!(
                    !accepted,
                    "a stopped original owner must not issue a new native ticket after stop"
                );
                assert_eq!(
                    tracker.in_flight(),
                    Some(InFlightNativeCalls {
                        creates: 0,
                        resumes: 0
                    }),
                    "failed post-stop commitment must roll back its counted increment"
                );
                assert!(tracker.track(phase).is_none(), "stop is irreversible");
            });
        }
    }

    #[test]
    fn ticket_registration_fails_closed_without_waiting_for_contended_counter_mutex() {
        // The original counter mutex can be held by another native accounting
        // thread. New Create/Resume admission cannot wait indefinitely behind
        // that holder, even though this is NOT an OS-launch authorization gate.
        for phase in [NativeCallPhase::CreateSuspended, NativeCallPhase::Resume] {
            let tracker = NativeCallStopTracker::new();
            let (held_tx, held_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let (result_tx, result_rx) = mpsc::channel();

            std::thread::scope(|scope| {
                let original = &tracker;
                let holder = scope.spawn(move || {
                    let _held = original.shared.counts.lock().expect("fresh counter mutex");
                    held_tx.send(()).expect("signal held counter lock");
                    release_rx
                        .recv_timeout(Duration::from_secs(3))
                        .expect("bounded test supervisor releases held lock");
                });
                held_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("fixture holds lock before ticket admission");
                let original = &tracker;
                let registrar = scope.spawn(move || {
                    result_tx
                        .send(original.track(phase).is_some())
                        .expect("bounded admission outcome");
                });

                // Independent test supervisor observes admission *before*
                // permitting the counter mutex holder to resume. Even if
                // the old blocking lock regresses, release and join safely.
                let observed = result_rx.recv_timeout(Duration::from_secs(1));
                release_tx.send(()).expect("always release fixture mutex");
                holder.join().expect("fixture holder joined");
                registrar.join().expect("ticket caller joined");

                assert_eq!(
                    observed,
                    Ok(false),
                    "native ticket must fail closed before the held mutex is released"
                );
                assert!(
                    tracker.stop_requested(),
                    "contention must seal original owner"
                );
                assert_eq!(
                    tracker.in_flight().expect("unpoisoned original counters"),
                    InFlightNativeCalls {
                        creates: 0,
                        resumes: 0
                    }
                );
                assert!(tracker.track(phase).is_none(), "stop cannot be reversed");
            });
        }
    }

    #[test]
    fn native_inflight_observation_must_not_wait_for_contended_original_mutex() {
        // This is an original native-call accounting ledger, not a process.
        // The owner must not block on a stalled observer when needing to
        // decide whether already-admitted OS calls remain uncertain.
        let tracker = NativeCallStopTracker::new();
        let watchdog_stop = tracker.internal_stop_signal();
        let (locked_tx, locked_rx) = mpsc::channel();
        let (unlock_tx, unlock_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        std::thread::scope(|scope| {
            let original = &tracker;
            let holder = scope.spawn(move || {
                let _guard = original.shared.counts.lock().expect("original mutex");
                locked_tx.send(()).expect("announce held accounting lock");
                unlock_rx
                    .recv_timeout(Duration::from_secs(3))
                    .expect("test supervisor releases held counter");
            });
            locked_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("original counter must be held before status probe");
            let observer = scope.spawn(|| {
                result_tx
                    .send(tracker.in_flight())
                    .expect("return original accounting observation");
            });

            // The old .lock() waits until release and can deadlock a real
            // owner. Release even when RED to avoid leaving a stuck fixture.
            let observed = result_rx.recv_timeout(Duration::from_secs(1));
            let stop_before_unlock = watchdog_stop.load(Ordering::Acquire);
            unlock_tx.send(()).expect("always unlock held ledger");
            holder.join().expect("bounded fixture holder cleanup");
            observer.join().expect("bounded fixture observer cleanup");

            assert_eq!(
                observed,
                Ok(None),
                "contended accounting must report Unknown promptly, never wait for the mutex"
            );
            assert!(
                stop_before_unlock,
                "unknown accounting must notify the ORIGINAL Job watchdog before unlock"
            );
        });
        assert!(tracker.track(NativeCallPhase::Resume).is_none());
        assert_eq!(
            tracker.in_flight(),
            Some(InFlightNativeCalls {
                creates: 0,
                resumes: 0,
            }),
            "a later successful count read does not reopen irrevocable stop"
        );
        assert!(tracker.stop_requested());
    }

    #[test]
    fn retiring_native_ticket_must_not_block_or_claim_quiescence_on_contended_mutex() {
        // An already-counted native call can finish while another thread
        // indefinitely holds the accounting lock. Its destructor must
        // promptly publish original stop instead of blocking the OS-call
        // owner on that lock. The missing decrement remains conservatively
        // counted, never forged as an empty drain receipt.
        for phase in [NativeCallPhase::CreateSuspended, NativeCallPhase::Resume] {
            let tracker = NativeCallStopTracker::new();
            let watchdog_stop = tracker.internal_stop_signal();
            let ticket = tracker.track(phase).expect("counted original call");
            let (locked_tx, locked_rx) = mpsc::channel();
            let (unlock_tx, unlock_rx) = mpsc::channel();
            let (completed_tx, completed_rx) = mpsc::channel();
            std::thread::scope(|scope| {
                let original = &tracker;
                let holder = scope.spawn(move || {
                    let _guard = original.shared.counts.lock().expect("original counts");
                    locked_tx
                        .send(())
                        .expect("announce locked original accounting");
                    unlock_rx
                        .recv_timeout(Duration::from_secs(3))
                        .expect("test supervisor releases held counter");
                });
                locked_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("counter held before native call returns");
                let closer = scope.spawn(move || {
                    drop(ticket);
                    completed_tx
                        .send(())
                        .expect("report ticket retirement completed");
                });

                let observed = completed_rx.recv_timeout(Duration::from_secs(1));
                let stop_before_unlock = watchdog_stop.load(Ordering::Acquire);
                unlock_tx
                    .send(())
                    .expect("always release held mutex on RED");
                holder.join().expect("bounded fixture holder cleanup");
                closer.join().expect("bounded native ticket closer cleanup");

                assert_eq!(
                    observed,
                    Ok(()),
                    "original native ticket destructor must not wait for contended counter lock"
                );
                assert!(
                    stop_before_unlock,
                    "failed ticket retirement must publish ORIGINAL watchdog stop"
                );
            });
            let remaining = tracker.in_flight().expect("original ledger not poisoned");
            assert_eq!(
                remaining,
                match phase {
                    NativeCallPhase::CreateSuspended => InFlightNativeCalls {
                        creates: 1,
                        resumes: 0,
                    },
                    NativeCallPhase::Resume => InFlightNativeCalls {
                        creates: 0,
                        resumes: 1,
                    },
                },
                "uncertain ticket retirement must retain its original count"
            );
            assert!(tracker.track(phase).is_none(), "stop must stay terminal");
            assert!(
                matches!(
                    tracker.seal_and_drain_until(
                        std::time::Instant::now() + Duration::from_millis(40)
                    ),
                    Err(NativeCallDrainError::TimedOut)
                ),
                "a failed retirement must never publish empty native-call drain"
            );
        }
    }

    #[test]
    fn sealed_drain_does_not_wait_for_contended_mutex_past_absolute_bound() {
        let tracker = NativeCallStopTracker::new();
        let (locked_tx, locked_rx) = mpsc::channel();
        let (unlock_tx, unlock_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        std::thread::scope(|scope| {
            let holder_tracker = &tracker;
            let holder = scope.spawn(move || {
                let _counter_guard = holder_tracker.shared.counts.lock().expect("fresh ledger");
                locked_tx.send(()).expect("announce held counter lock");
                unlock_rx
                    .recv_timeout(Duration::from_secs(3))
                    .expect("fixture must release held lock");
            });
            locked_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("count mutex held before drain");
            let drainer = scope.spawn(|| {
                let result = tracker
                    .seal_and_drain_until(std::time::Instant::now() + Duration::from_millis(60))
                    .map(|_| ());
                result_tx.send(result).expect("send drain outcome");
            });
            // Stop must be publishable before the blocked lock acquisition.
            let started = std::time::Instant::now();
            let stop_published = loop {
                if tracker.shared.stop_requested.load(Ordering::Acquire) {
                    break true;
                }
                if started.elapsed() >= Duration::from_secs(2) {
                    break false;
                }
                std::thread::yield_now();
            };
            // Bounded external supervisor: release the test's synthetic lock
            // even when the original implementation would wait forever.
            let observed = result_rx.recv_timeout(Duration::from_secs(1));
            unlock_tx
                .send(())
                .expect("release fixture even on regression");
            holder.join().expect("fixture holder completion");
            drainer.join().expect("drainer termination after release");
            assert!(stop_published, "original drain never sealed admission");
            assert!(
                matches!(observed, Ok(Err(NativeCallDrainError::TimedOut))),
                "drain must time out before releasing a contended counter mutex; observed: {observed:?}"
            );
            assert!(tracker.track(NativeCallPhase::Resume).is_none());
        });
    }

    #[test]
    fn sealed_drain_times_out_without_releasing_original_ticket_and_poison_is_unknown() {
        let tracker = NativeCallStopTracker::new();
        let ticket = tracker.track(NativeCallPhase::CreateSuspended).unwrap();
        assert!(matches!(
            tracker.seal_and_drain_until(std::time::Instant::now() + Duration::from_millis(50)),
            Err(NativeCallDrainError::TimedOut)
        ));
        assert!(tracker.stop_requested());
        assert!(tracker.track(NativeCallPhase::Resume).is_none());
        assert_eq!(tracker.in_flight().unwrap().creates, 1);
        drop(ticket);
        assert!(
            tracker
                .seal_and_drain_until(std::time::Instant::now() + Duration::from_secs(1))
                .is_ok()
        );
        assert!(matches!(
            tracker.seal_and_drain_until(std::time::Instant::now()),
            Err(NativeCallDrainError::TimedOut)
        ));

        let poisoned = NativeCallStopTracker::new();
        poisoned.poison_ledger_for_test();
        assert!(matches!(
            poisoned.seal_and_drain_until(std::time::Instant::now() + Duration::from_secs(1)),
            Err(NativeCallDrainError::Unknown)
        ));
        assert!(poisoned.internal_stop_signal().load(Ordering::Acquire));
    }

    #[test]
    fn sealed_drain_never_extends_the_original_admitted_horizon() {
        let original_stops: [Arc<AtomicBool>; 4] =
            std::array::from_fn(|_| Arc::new(AtomicBool::new(false)));
        // A previously expired ORIGINAL bound must not be replaced by a fresh
        // cleanup request, even if there are no in-flight calls left.
        let original_deadline = std::time::Instant::now() - Duration::from_millis(1);
        let tracker = NativeCallStopTracker::with_original_stops(original_stops, original_deadline);
        assert!(matches!(
            tracker.seal_and_drain_until(std::time::Instant::now() + Duration::from_secs(10)),
            Err(NativeCallDrainError::TimedOut)
        ));
        assert!(tracker.stop_requested());
        assert!(tracker.track(NativeCallPhase::Resume).is_none());
    }

    #[test]
    fn stop_during_blocked_create_remains_publishable_and_denies_resume() {
        let tracker = NativeCallStopTracker::new();
        let create = tracker.track(NativeCallPhase::CreateSuspended).unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        std::thread::scope(|scope| {
            // Scoped thread may borrow the uniquely retained ticket; it must
            // complete before the tracker can be destroyed by its owner.
            let worker = scope.spawn(move || {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                drop(create);
            });
            entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            tracker.request_stop();
            assert!(tracker.stop_requested());
            assert_eq!(
                tracker.in_flight(),
                Some(InFlightNativeCalls {
                    creates: 1,
                    resumes: 0,
                })
            );
            assert!(tracker.track(NativeCallPhase::CreateSuspended).is_none());
            assert!(tracker.track(NativeCallPhase::Resume).is_none());
            release_tx.send(()).unwrap();
            worker.join().unwrap();
        });
        assert_eq!(
            tracker.in_flight(),
            Some(InFlightNativeCalls {
                creates: 0,
                resumes: 0,
            })
        );
    }

    #[test]
    fn stop_during_admitted_resume_retains_uncertain_inflight_call() {
        let tracker = NativeCallStopTracker::new();
        let resume = tracker.track(NativeCallPhase::Resume).unwrap();
        tracker.request_stop();
        tracker.request_stop(); // no reset, no newly admitted work
        assert_eq!(
            tracker.in_flight(),
            Some(InFlightNativeCalls {
                creates: 0,
                resumes: 1,
            })
        );
        assert!(tracker.track(NativeCallPhase::Resume).is_none());
        drop(resume);
        assert_eq!(
            tracker.in_flight(),
            Some(InFlightNativeCalls {
                creates: 0,
                resumes: 0,
            })
        );
        assert!(tracker.stop_requested());
    }

    #[test]
    fn stop_before_call_registration_is_irreversible() {
        let tracker = NativeCallStopTracker::new();
        let other_observer = &tracker;
        tracker.request_stop();
        assert!(
            other_observer
                .track(NativeCallPhase::CreateSuspended)
                .is_none()
        );
        assert!(other_observer.track(NativeCallPhase::Resume).is_none());
    }

    #[test]
    fn poisoned_inflight_ledger_requires_stop_and_never_reports_empty() {
        let tracker = NativeCallStopTracker::new();
        let watchdog_stop = tracker.internal_stop_signal();
        assert!(!watchdog_stop.load(Ordering::Acquire));
        // Poison the actual accounting mutex while retaining a native-call
        // ticket. This is a local model: no process or Windows API is called.
        let pending = tracker.track(NativeCallPhase::CreateSuspended).unwrap();
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| {
                let _guard = tracker.shared.counts.lock().unwrap();
                panic!("intentional test-only accounting poison");
            });
            assert!(worker.join().is_err());
        });
        assert!(
            tracker.stop_requested(),
            "unknown/poisoned accounting must fail closed as stop required"
        );
        assert!(
            watchdog_stop.load(Ordering::Acquire),
            "observed poison must irreversibly signal the original native Job watchdog"
        );
        assert_eq!(tracker.in_flight(), None, "unknown is never zero");
        assert!(tracker.track(NativeCallPhase::CreateSuspended).is_none());
        assert!(tracker.track(NativeCallPhase::Resume).is_none());
        drop(pending);
        assert_eq!(
            tracker.in_flight(),
            None,
            "poison remains unknown after drop"
        );
        tracker.request_stop();
        assert!(tracker.stop_requested());
    }

    #[test]
    fn dropping_native_call_ticket_on_poisoned_ledger_publishes_original_stop() {
        let tracker = NativeCallStopTracker::new();
        let original_watchdog_stop = tracker.internal_stop_signal();
        let ticket = tracker
            .track(NativeCallPhase::CreateSuspended)
            .expect("original native call was admitted before poison");
        tracker.poison_ledger_for_test();
        assert!(!original_watchdog_stop.load(Ordering::Acquire));
        // Drop is the FIRST observer of the poisoned ledger. The original
        // watchdog must not require a later status() or in_flight() request
        // to learn that retiring this OS call has unknown accounting.
        drop(ticket);
        assert!(
            original_watchdog_stop.load(Ordering::Acquire),
            "a failed native-call ticket retirement must wake the original Job watchdog"
        );
        assert_eq!(tracker.in_flight(), None, "poison never becomes empty");
        assert!(tracker.track(NativeCallPhase::Resume).is_none());
    }

    #[test]
    fn poisoned_ledger_inflight_observation_seals_original_watchdog_stop() {
        let tracker = NativeCallStopTracker::new();
        let watchdog_stop = tracker.internal_stop_signal();
        tracker.poison_ledger_for_test();
        assert!(!watchdog_stop.load(Ordering::Acquire));
        assert_eq!(tracker.in_flight(), None, "unknown is never zero");
        assert!(
            watchdog_stop.load(Ordering::Acquire),
            "first poisoned accounting read must publish the original Job stop"
        );
        assert!(tracker.track(NativeCallPhase::Resume).is_none());
        assert!(tracker.stop_requested());
    }

    #[test]
    fn original_authority_revoke_before_native_ticket_commit_denies_all_generations() {
        // Actual concurrent revoke: the registrar holds the ORIGINAL counter
        // mutex after incrementing it and before the final ticket decision.
        // No kernel process is created, and a ticket is not spawn authority.
        for phase in [NativeCallPhase::CreateSuspended, NativeCallPhase::Resume] {
            for source in 0..4 {
                let original_stops: [Arc<AtomicBool>; 4] =
                    std::array::from_fn(|_| Arc::new(AtomicBool::new(false)));
                let tracker = NativeCallStopTracker::with_original_stops(
                    original_stops.clone(),
                    std::time::Instant::now() + Duration::from_secs(5),
                );
                let (counted_tx, counted_rx) = mpsc::channel();
                let (release_tx, release_rx) = mpsc::channel();
                std::thread::scope(|scope| {
                    let original = &tracker;
                    let registrar = scope.spawn(move || {
                        original.track_with_before_commit(phase, || {
                            counted_tx
                                .send(())
                                .expect("counter held before original revoke");
                            release_rx
                                .recv_timeout(Duration::from_secs(3))
                                .expect("supervisor releases native ticket decision");
                        })
                    });
                    counted_rx
                        .recv_timeout(Duration::from_secs(2))
                        .expect("ticket reached final decision barrier");
                    // Original session, TaskLease, Job and host signal occupy
                    // the four distinct indexes, and NONE is the internal
                    // stop_requested bit used by the final CAS.
                    original_stops[source].store(true, Ordering::Release);
                    assert!(
                        tracker.stop_requested(),
                        "original authority source {source} must stop admission"
                    );
                    release_tx.send(()).expect("release final ticket boundary");
                    let accepted = registrar.join().expect("registrar returns");
                    assert!(
                        accepted.is_none(),
                        "original authority {source} revoked before ticket commitment"
                    );
                });
                assert_eq!(
                    tracker.in_flight(),
                    Some(InFlightNativeCalls {
                        creates: 0,
                        resumes: 0,
                    }),
                    "rejected ticket must roll back its exact original counter"
                );
                assert!(
                    tracker.track(phase).is_none(),
                    "revoked original generation must stay denied"
                );
            }
        }
    }

    #[test]
    fn original_deadline_expiring_before_native_ticket_commit_denies_and_rolls_back() {
        for phase in [NativeCallPhase::CreateSuspended, NativeCallPhase::Resume] {
            let original_stops: [Arc<AtomicBool>; 4] =
                std::array::from_fn(|_| Arc::new(AtomicBool::new(false)));
            let tracker = NativeCallStopTracker::with_original_stops(
                original_stops,
                std::time::Instant::now() + Duration::from_millis(80),
            );
            let reached_commit_boundary = AtomicBool::new(false);
            let accepted = tracker.track_with_before_commit(phase, || {
                // This test-only delay lies AFTER both existing stop checks,
                // while the original counter mutex is held.
                reached_commit_boundary.store(true, Ordering::Release);
                std::thread::sleep(Duration::from_millis(130));
            });
            assert!(
                reached_commit_boundary.load(Ordering::Acquire),
                "the expiry fixture must reach the actual counted ticket boundary"
            );
            assert!(
                accepted.is_none(),
                "original absolute deadline expired before native ticket commitment"
            );
            assert_eq!(
                tracker.in_flight(),
                Some(InFlightNativeCalls {
                    creates: 0,
                    resumes: 0
                }),
                "expiry before commitment must not leave an in-flight ticket"
            );
            assert!(tracker.stop_requested(), "original expiry remains terminal");
        }
    }

    #[test]
    fn original_generation_stop_keeps_admitted_call_counted_after_reuse() {
        let signals: [Arc<AtomicBool>; 4] =
            std::array::from_fn(|_| Arc::new(AtomicBool::new(false)));
        let original = NativeCallStopTracker::with_original_stops(
            signals.clone(),
            std::time::Instant::now() + Duration::from_secs(5),
        );
        let entered = original.track(NativeCallPhase::CreateSuspended).unwrap();
        // The original TaskLease generation is revoked; replacing the public
        // ID with an unrelated fresh flag must not redeem the old owner.
        signals[1].store(true, Ordering::Release);
        let fresh_lease_generation = Arc::new(AtomicBool::new(false));
        assert!(!fresh_lease_generation.load(Ordering::Acquire));
        assert!(original.stop_requested());
        assert!(original.track(NativeCallPhase::Resume).is_none());
        assert_eq!(
            original.in_flight(),
            Some(InFlightNativeCalls {
                creates: 1,
                resumes: 0,
            })
        );
        drop(entered);
        assert_eq!(
            original.in_flight(),
            Some(InFlightNativeCalls {
                creates: 0,
                resumes: 0,
            })
        );
        assert!(original.stop_requested());
    }

    #[test]
    fn exhausted_native_call_counter_closes_all_future_admission() {
        // Model the integer boundary without creating any native process or
        // attempting an impractical number of tracked calls.
        for phase in [NativeCallPhase::CreateSuspended, NativeCallPhase::Resume] {
            let tracker = NativeCallStopTracker::new();
            {
                let mut counts = tracker.shared.counts.lock().unwrap();
                match phase {
                    NativeCallPhase::CreateSuspended => counts.creates = usize::MAX,
                    NativeCallPhase::Resume => counts.resumes = usize::MAX,
                }
            }
            assert!(tracker.track(phase).is_none(), "counter cannot wrap");
            assert!(
                tracker.stop_requested(),
                "exhausted call accounting must fail closed as stop required"
            );
            // Even after a subsequent count decrease the original owner
            // must not reopen admission once capacity was exhausted.
            {
                let mut counts = tracker.shared.counts.lock().unwrap();
                match phase {
                    NativeCallPhase::CreateSuspended => counts.creates = 0,
                    NativeCallPhase::Resume => counts.resumes = 0,
                }
            }
            assert!(tracker.track(NativeCallPhase::CreateSuspended).is_none());
            assert!(tracker.track(NativeCallPhase::Resume).is_none());
        }
    }

    #[test]
    fn original_absolute_deadline_denies_new_calls_without_manual_stop() {
        let signals: [Arc<AtomicBool>; 4] =
            std::array::from_fn(|_| Arc::new(AtomicBool::new(false)));
        let original = NativeCallStopTracker::with_original_stops(
            signals,
            std::time::Instant::now() + Duration::from_millis(200),
        );
        let entered = original.track(NativeCallPhase::Resume).unwrap();
        std::thread::sleep(Duration::from_millis(250));
        assert!(
            original.stop_requested(),
            "the original horizon cannot renew"
        );
        assert!(original.track(NativeCallPhase::CreateSuspended).is_none());
        assert_eq!(
            original.in_flight(),
            Some(InFlightNativeCalls {
                creates: 0,
                resumes: 1,
            })
        );
        drop(entered);
        assert_eq!(
            original.in_flight(),
            Some(InFlightNativeCalls {
                creates: 0,
                resumes: 0,
            })
        );
    }
}
