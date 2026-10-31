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

#[derive(Default)]
struct Shared {
    stop_requested: AtomicBool,
    counts: Mutex<CallCounts>,
}

/// This accounting gate is deliberately not attached to the live MCP or native
/// owner. A future guardian must bind every original revoke source and retain
/// native custody after calls finish; this tracker alone cannot do either.
#[derive(Clone, Default)]
pub(crate) struct NativeCallStopTracker {
    shared: Arc<Shared>,
}

/// Retained by the thread actually performing an OS call. The ticket does not
/// authorize that call. In particular, a stop can win after admission and
/// before an already admitted CreateProcessW/ResumeThread enters the kernel.
#[must_use = "keep the call counted until the native API returns"]
pub(crate) struct TrackedNativeCall {
    shared: Arc<Shared>,
    phase: NativeCallPhase,
}

impl NativeCallStopTracker {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Register potential in-flight work, not permission to launch. The second
    /// stop check under the short mutex closes admission after a prior stop.
    /// A concurrent stop after that check makes this an already-admitted call
    /// which the eventual guardian must reconcile or forcibly terminate.
    pub(crate) fn track(&self, phase: NativeCallPhase) -> Option<TrackedNativeCall> {
        if self.shared.stop_requested.load(Ordering::Acquire) {
            return None;
        }
        let mut counts = self.shared.counts.lock().ok()?;
        if self.shared.stop_requested.load(Ordering::Acquire) {
            return None;
        }
        let counter = match phase {
            NativeCallPhase::CreateSuspended => &mut counts.creates,
            NativeCallPhase::Resume => &mut counts.resumes,
        };
        *counter = counter.checked_add(1)?;
        Some(TrackedNativeCall {
            shared: Arc::clone(&self.shared),
            phase,
        })
    }

    /// Irreversibly closes admission before touching the count mutex. It does
    /// NOT synchronously stop an already admitted OS call or complete revoke.
    pub(crate) fn request_stop(&self) {
        self.shared.stop_requested.store(true, Ordering::Release);
    }

    pub(crate) fn stop_requested(&self) -> bool {
        self.shared.stop_requested.load(Ordering::Acquire)
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

impl Drop for TrackedNativeCall {
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
        let worker = std::thread::spawn(move || {
            // Model an in-flight, arbitrarily delayed native call without
            // creating a child or holding the authority/count mutex.
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
        let other_observer = tracker.clone();
        tracker.request_stop();
        assert!(other_observer.track(NativeCallPhase::CreateSuspended).is_none());
        assert!(other_observer.track(NativeCallPhase::Resume).is_none());
    }
}
