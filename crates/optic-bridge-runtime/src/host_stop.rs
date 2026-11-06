//! Process-owned, irreversible host-stop publication. No launch authority.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[cfg(windows)]
use std::sync::{Mutex, Weak};

/// Owned by trusted process startup, never by an MCP request. Drop closes it.
/// A new source has a distinct identity and cannot reset existing watches.
#[derive(Debug, Default)]
pub struct HostStopSource {
    stopped: Arc<AtomicBool>,
    // Host publication and the original native ledger close use one short
    // critical section. Neither native syscalls nor callbacks run under it.
    #[cfg(windows)]
    native_close_subscribers: Arc<Mutex<Vec<Weak<AtomicBool>>>>,
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
            native_close_subscribers: Arc::clone(&self.native_close_subscribers),
        }
    }

    /// Publish once before cancelling service waiters. Does not drain work.
    pub fn request_stop(&self) -> bool {
        #[cfg(windows)]
        {
            // Original ledger close bits must be sealed BEFORE the public
            // host-stop bit changes. Registration uses this same mutex.
            // Only short atomic stores occur here; never kernel or IO calls.
            let mut registered = match self.native_close_subscribers.lock() {
                Ok(registered) => registered,
                Err(poisoned) => poisoned.into_inner(),
            };
            registered.retain(|weak| {
                let Some(close) = weak.upgrade() else {
                    return false;
                };
                close.store(true, Ordering::Release);
                true
            });
            !self.stopped.swap(true, Ordering::AcqRel)
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
    native_close_subscribers: Arc<Mutex<Vec<Weak<AtomicBool>>>>,
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

    /// Subscribe the original native watchdog ledger's exact close bit.
    /// A racing host stop wins through the same mutex and cannot be
    /// published before this subscriber has been sealed. Fail closed on
    /// mutex contention/poison or already-closed authority. No spawn grant.
    #[cfg(windows)]
    pub(crate) fn try_bind_original_native_ledger_close(&self, close: &Arc<AtomicBool>) -> bool {
        let mut registered = match self.native_close_subscribers.try_lock() {
            Ok(registered) => registered,
            Err(_) => {
                close.store(true, Ordering::Release);
                return false;
            }
        };
        if self.stopped.load(Ordering::Acquire) || close.load(Ordering::Acquire) {
            close.store(true, Ordering::Release);
            return false;
        }
        registered.retain(|weak| weak.strong_count() > 0);
        registered.push(Arc::downgrade(close));
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
}
