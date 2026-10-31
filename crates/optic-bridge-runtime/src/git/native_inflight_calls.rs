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
    Arc, Mutex,
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
            // A poisoned counter is unknown custody, never an all-clear.
            // This read does not lock or repair the counter, nor prove
            // native OS quiescence or revoke an already tracked call.
            || self.counts.is_poisoned()
            || self.original.as_ref().is_some_and(|original| {
                std::time::Instant::now() >= original.deadline
                    || original
                        .signals
                        .iter()
                        .any(|signal| signal.load(Ordering::Acquire))
            })
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
        if self.shared.is_stop_required() {
            return None;
        }
        let mut counts = self.shared.counts.lock().ok()?;
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

    /// None on poisoned accounting state is unknown, NOT an empty ledger.
    /// Even Some(0, 0) says nothing about live children, Job or pipe EOF.
    pub(crate) fn in_flight(&self) -> Option<InFlightNativeCalls> {
        let counts = self.shared.counts.lock().ok()?;
        Some(InFlightNativeCalls {
            creates: counts.creates,
            resumes: counts.resumes,
        })
    }
}

impl Drop for TrackedNativeCall<'_> {
    fn drop(&mut self) {
        // Never substitute a successful quiescence receipt if the mutex was
        // poisoned. The observer then sees None rather than false zero.
        if let Ok(mut counts) = self.shared.counts.lock() {
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
