use std::time::Instant;

use optic_bridge_core::MonotonicTime;

pub trait Clock: Send + Sync {
    fn now(&self) -> MonotonicTime;
}

#[derive(Clone, Debug)]
pub struct StdClock {
    origin: Instant,
}

impl StdClock {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_deadline_keeps_original_origin_across_clones_and_delay() {
        let origin = Instant::now()
            .checked_sub(std::time::Duration::from_secs(2))
            .unwrap();
        let clock = StdClock { origin };
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
    fn std_clock_is_monotonic() {
        let clock = StdClock::new();
        let first = clock.now();
        let second = clock.now();
        assert!(second >= first);
    }
}
