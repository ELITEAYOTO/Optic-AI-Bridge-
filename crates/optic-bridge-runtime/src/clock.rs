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
    fn std_clock_is_monotonic() {
        let clock = StdClock::new();
        let first = clock.now();
        let second = clock.now();
        assert!(second >= first);
    }
}
