//! Process-owned, irreversible host-stop publication. No launch authority.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Owned by trusted process startup, never by an MCP request. Drop closes it.
/// A new source has a distinct identity and cannot reset existing watches.
#[derive(Debug, Default)]
pub struct HostStopSource {
    stopped: Arc<AtomicBool>,
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
        }
    }

    /// Publish once before cancelling service waiters. Does not drain work.
    pub fn request_stop(&self) -> bool {
        !self.stopped.swap(true, Ordering::AcqRel)
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
}
