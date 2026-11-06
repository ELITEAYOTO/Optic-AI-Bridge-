//! Process-owned, irreversible host-stop publication. No launch authority.
#[cfg(windows)]
use arc_swap::ArcSwap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[cfg(windows)]
use std::sync::{Mutex, Weak};

/// Owned by trusted process startup, never by an MCP request. Drop closes it.
/// A new source has a distinct identity and cannot reset existing watches.
#[derive(Debug)]
pub struct HostStopSource {
    stopped: Arc<AtomicBool>,
    // Private linearization gate: an in-progress stop denies native registration
    // before its public bit is emitted. Only the original source may set it.
    #[cfg(windows)]
    close_started: Arc<AtomicBool>,
    // The registration lock is never acquired by request_stop/Drop.
    #[cfg(windows)]
    registration_lock: Arc<Mutex<()>>,
    // Atomic immutable snapshots let stop close every previously registered
    // original ledger without waiting behind a stalled registrar.
    #[cfg(windows)]
    native_close_subscribers: Arc<ArcSwap<Vec<Weak<AtomicBool>>>>,
}

impl Default for HostStopSource {
    fn default() -> Self {
        Self {
            stopped: Arc::new(AtomicBool::new(false)),
            #[cfg(windows)]
            close_started: Arc::new(AtomicBool::new(false)),
            #[cfg(windows)]
            registration_lock: Arc::new(Mutex::new(())),
            #[cfg(windows)]
            native_close_subscribers: Arc::new(ArcSwap::from_pointee(Vec::new())),
        }
    }
}

impl HostStopSource {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn watch(&self) -> HostStopWatch {
        HostStopWatch {
            stopped: Arc::clone(&self.stopped),
            #[cfg(windows)]
            close_started: Arc::clone(&self.close_started),
            #[cfg(windows)]
            registration_lock: Arc::clone(&self.registration_lock),
            #[cfg(windows)]
            native_close_subscribers: Arc::clone(&self.native_close_subscribers),
        }
    }

    /// Publish once before cancelling service waiters. Does not drain work.
    pub fn request_stop(&self) -> bool {
        #[cfg(windows)]
        {
            // Publish an internal CLOSE_STARTING fence before the atomic
            // subscriber snapshot. A racing registrar either published its
            // ledger before this snapshot or sees the fence after publishing
            // and seals its own ledger before returning. SeqCst establishes
            // that total order without an OS call or a blocking registry lock.
            self.close_started.store(true, Ordering::SeqCst);
            let registered = self.native_close_subscribers.load_full();
            for weak in registered.iter() {
                if let Some(close) = weak.upgrade() {
                    close.store(true, Ordering::SeqCst);
                }
            }
            // Public stop is visible only after the original ledger closure
            // pass. The live host stop remains an independent watched signal.
            !self.stopped.swap(true, Ordering::SeqCst)
        }
        #[cfg(not(windows))]
        {
            !self.stopped.swap(true, Ordering::AcqRel)
        }
    }
}

impl Drop for HostStopSource {
    fn drop(&mut self) {
        self.request_stop();
    }
}

/// Read-only original host identity. A false observation is not a spawn grant.
#[derive(Clone, Debug)]
pub struct HostStopWatch {
    stopped: Arc<AtomicBool>,
    #[cfg(windows)]
    close_started: Arc<AtomicBool>,
    #[cfg(windows)]
    registration_lock: Arc<Mutex<()>>,
    #[cfg(windows)]
    native_close_subscribers: Arc<ArcSwap<Vec<Weak<AtomicBool>>>>,
}

impl HostStopWatch {
    #[must_use]
    pub fn is_stop_requested(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    #[cfg(any(windows, test))]
    pub(crate) fn same_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.stopped, &other.stopped)
    }

    #[cfg(windows)]
    pub(crate) fn stop_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stopped)
    }

    /// Register a weak link to the exact original native ledger before it
    /// can escape its no-child owner. Unlike host stop, registration may
    /// conservatively fail on an occupied or poisoned short writer mutex.
    /// The post-publish SeqCst recheck closes a ledger added concurrently
    /// with stop's lock-free snapshot. This is not a Win32 launch grant.
    #[cfg(windows)]
    pub(crate) fn try_bind_original_native_ledger_close(&self, close: &Arc<AtomicBool>) -> bool {
        let _registration = match self.registration_lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                close.store(true, Ordering::SeqCst);
                return false;
            }
        };
        if self.close_started.load(Ordering::SeqCst)
            || self.stopped.load(Ordering::SeqCst)
            || close.load(Ordering::SeqCst)
        {
            close.store(true, Ordering::SeqCst);
            return false;
        }
        let old = self.native_close_subscribers.load_full();
        let mut updated = old
            .iter()
            .filter(|weak| weak.strong_count() > 0)
            .cloned()
            .collect::<Vec<_>>();
        updated.push(Arc::downgrade(close));
        self.native_close_subscribers.store(Arc::new(updated));
        if self.close_started.load(Ordering::SeqCst) || self.stopped.load(Ordering::SeqCst) {
            close.store(true, Ordering::SeqCst);
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_stop_is_irreversible_and_original_watches_cannot_be_retargeted() {
        let source = HostStopSource::new();
        let original = source.watch();
        let clone = original.clone();
        let foreign = HostStopSource::new();
        assert!(original.same_source(&clone));
        assert!(!original.same_source(&foreign.watch()));
        assert!(!original.is_stop_requested());
        assert!(source.request_stop());
        assert!(!source.request_stop());
        assert!(original.is_stop_requested() && clone.is_stop_requested());
        assert!(!foreign.watch().is_stop_requested());
    }
    #[test]
    fn owner_drop_publishes_stop_to_retained_watch() {
        let watch = {
            let source = HostStopSource::new();
            source.watch()
        };
        assert!(watch.is_stop_requested());
    }

    #[cfg(windows)]
    #[test]
    fn original_host_source_seals_exact_ledger_before_stop_or_drop_publication() {
        let source = HostStopSource::new();
        let original = source.watch();
        let foreign = HostStopSource::new();
        let foreign_watch = foreign.watch();
        let close = Arc::new(AtomicBool::new(false));
        assert!(original.try_bind_original_native_ledger_close(&close));
        assert!(!close.load(Ordering::Acquire));
        assert!(source.request_stop());
        assert!(original.is_stop_requested());
        assert!(close.load(Ordering::Acquire));
        let late_close = Arc::new(AtomicBool::new(false));
        assert!(!original.try_bind_original_native_ledger_close(&late_close));
        assert!(late_close.load(Ordering::Acquire));
        assert!(!foreign_watch.is_stop_requested());

        let drop_close = Arc::new(AtomicBool::new(false));
        let drop_watch = {
            let owned = HostStopSource::new();
            let watch = owned.watch();
            assert!(watch.try_bind_original_native_ledger_close(&drop_close));
            watch
        };
        assert!(drop_watch.is_stop_requested());
        assert!(drop_close.load(Ordering::Acquire));
    }

    #[cfg(windows)]
    #[test]
    fn concurrent_host_stop_and_registration_never_leave_open_original_ledger() {
        use std::sync::Barrier;
        for _ in 0..64 {
            let source = Arc::new(HostStopSource::new());
            let watch = source.watch();
            let close = Arc::new(AtomicBool::new(false));
            let barrier = Arc::new(Barrier::new(2));
            std::thread::scope(|scope| {
                let stop_source = Arc::clone(&source);
                let stop_barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    stop_barrier.wait();
                    assert!(stop_source.request_stop());
                });
                barrier.wait();
                let admitted = watch.try_bind_original_native_ledger_close(&close);
                if !admitted {
                    assert!(close.load(Ordering::Acquire));
                }
            });
            assert!(watch.is_stop_requested());
            assert!(close.load(Ordering::Acquire));
        }
    }

    // #323 contract RED: a short but intentionally held registration mutex
    // cannot postpone host stop publication; a native issuer may be stalled.
    // Test-only controlled contention: no Git child, no Win32 call.
    #[cfg(windows)]
    #[test]
    fn original_host_stop_publishes_while_registration_mutex_is_held() {
        use std::sync::mpsc;
        use std::time::Duration;
        let source = Arc::new(HostStopSource::new());
        let watch = source.watch();
        let close = Arc::new(AtomicBool::new(false));
        assert!(watch.try_bind_original_native_ledger_close(&close));
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let held = source.registration_lock.lock().unwrap();
        let other = Arc::clone(&source);
        let worker = std::thread::spawn(move || {
            ready_tx.send(()).expect("consumer exists");
            tx.send(other.request_stop())
                .expect("channel consumer exists");
        });
        // Count only after the publisher thread is scheduled; do not
        // confuse Windows runner scheduling with a mutex dependency.
        let ready = ready_rx.recv_timeout(Duration::from_secs(2)).is_ok();
        // Release held lock and join before asserting so RED never leaks work.
        let promptly_published =
            ready && rx.recv_timeout(Duration::from_millis(500)).ok() == Some(true);
        let observed_while_held = watch.is_stop_requested() && close.load(Ordering::Acquire);
        drop(held);
        worker.join().expect("bounded host stop worker");
        assert!(
            promptly_published && observed_while_held,
            "ISSUE323_HOSTSTOP_BLOCKING_REGISTRATION_LOCK: host publication and original ledger close must not wait for a held subscription mutex"
        );
    }

    #[cfg(windows)]
    #[test]
    fn original_host_binding_contention_or_poison_fails_closed_without_retargeting() {
        // A busy host register lock cannot force a waiting prelaunch owner
        // into a newly admitted native call: that exact ledger is sealed.
        let busy_source = HostStopSource::new();
        let busy_watch = busy_source.watch();
        let busy_close = Arc::new(AtomicBool::new(false));
        let held = busy_source.registration_lock.lock().unwrap();
        assert!(!busy_watch.try_bind_original_native_ledger_close(&busy_close));
        assert!(busy_close.load(Ordering::Acquire));
        drop(held);
        assert!(busy_source.request_stop());

        // A poisoned registration lock denies any replacement ledger.
        // Original stop publication does not acquire or recover that lock;
        // it seals the previously published snapshot independently.
        let source = HostStopSource::new();
        let watch = source.watch();
        let original_close = Arc::new(AtomicBool::new(false));
        assert!(watch.try_bind_original_native_ledger_close(&original_close));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _guard = source.registration_lock.lock().unwrap();
                panic!("intentional test-only subscriber-lock poison");
            }))
            .is_err()
        );
        let close = Arc::new(AtomicBool::new(false));
        assert!(!watch.try_bind_original_native_ledger_close(&close));
        assert!(close.load(Ordering::Acquire));
        assert!(source.request_stop());
        assert!(watch.is_stop_requested());
        assert!(
            original_close.load(Ordering::Acquire),
            "poisoned writer lock cannot prevent sealing already published original ledger"
        );
    }
}
