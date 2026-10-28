#[cfg(any(windows, test))]
use std::sync::Arc;
use std::time::Instant;

use optic_bridge_core::MonotonicTime;

pub trait Clock: Send + Sync {
    fn now(&self) -> MonotonicTime;
}

#[derive(Clone, Debug)]
pub struct StdClock {
    origin: Instant,
    #[cfg(any(windows, test))]
    identity: Arc<()>,
}

impl StdClock {
    /// Process-local identity, preserved by Clone and never derived from a timestamp.
    #[cfg(any(windows, test))]
    pub(crate) fn identity(&self) -> ClockIdentity {
        ClockIdentity(Arc::clone(&self.identity))
    }

    /// Project an absolute service horizon from this clock's original origin.
    /// Cloning, delayed observation and watchdog arm cannot restart its budget.
    #[cfg(any(windows, test))]
    pub(crate) fn deadline_instant(&self, deadline: MonotonicTime) -> Option<Instant> {
        self.origin
            .checked_add(std::time::Duration::from_millis(deadline.as_millis()))
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            #[cfg(any(windows, test))]
            identity: Arc::new(()),
        }
    }
}

impl Default for StdClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for StdClock {
    fn now(&self) -> MonotonicTime {
        let elapsed_ms = self.origin.elapsed().as_millis();
        let elapsed_ms = u64::try_from(elapsed_ms).unwrap_or(u64::MAX);
        MonotonicTime::from_millis(elapsed_ms)
    }
}

/// Retained allocation identity prevents substitution and address reuse.
#[cfg(any(windows, test))]
#[derive(Clone, Debug)]
pub(crate) struct ClockIdentity(Arc<()>);

#[cfg(any(windows, test))]
impl ClockIdentity {
    pub(crate) fn matches(&self, clock: &StdClock) -> bool {
        Arc::ptr_eq(&self.0, &clock.identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_deadline_keeps_original_origin_across_clones_and_delay() {
        let origin = Instant::now()
            .checked_sub(std::time::Duration::from_secs(2))
            .unwrap();
        let clock = StdClock {
            origin,
            identity: Arc::new(()),
        };
        let deadline = MonotonicTime::from_millis(1_000);
        let expected = origin + std::time::Duration::from_secs(1);
        assert_eq!(clock.deadline_instant(deadline), Some(expected));
        assert!(expected < Instant::now(), "elapsed horizon remains elapsed");
        let cloned = clock.clone();
        assert_eq!(cloned.deadline_instant(deadline), Some(expected));
        assert_eq!(
            clock.deadline_instant(MonotonicTime::from_millis(0)),
            Some(origin)
        );
    }

    #[test]
    fn identical_logical_deadlines_do_not_establish_clock_identity() {
        let first = StdClock::new();
        let second = StdClock {
            origin: first.origin + std::time::Duration::from_secs(1),
            identity: Arc::new(()),
        };
        let deadline = MonotonicTime::from_millis(1_000);
        assert_ne!(
            first.deadline_instant(deadline),
            second.deadline_instant(deadline)
        );
        assert!(!first.identity().matches(&second));
        assert!(first.identity().matches(&first.clone()));
    }

    #[test]
    fn std_clock_is_monotonic() {
        let clock = StdClock::new();
        let first = clock.now();
        let second = clock.now();
        assert!(second >= first);
    }
}
