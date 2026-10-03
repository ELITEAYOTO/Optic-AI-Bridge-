use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use optic_bridge_core::{HardLimits, MonotonicTime};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportLimits {
    pub max_request_bytes: u64,
    pub max_response_bytes: u64,
    pub max_request_duration_ms: u64,
    pub max_concurrent_requests: u32,
}

impl From<HardLimits> for TransportLimits {
    fn from(value: HardLimits) -> Self {
        Self {
            max_request_bytes: value.max_request_bytes,
            max_response_bytes: value.max_response_bytes,
            max_request_duration_ms: value.max_request_duration_ms,
            max_concurrent_requests: value.max_concurrent_requests,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransportGuard {
    limits: TransportLimits,
    in_flight: Arc<AtomicU32>,
}

impl TransportGuard {
    pub fn new(limits: TransportLimits) -> Result<Self, TransportError> {
        if limits.max_request_bytes == 0
            || limits.max_response_bytes == 0
            || limits.max_request_duration_ms == 0
            || limits.max_concurrent_requests == 0
        {
            return Err(TransportError::InvalidLimits);
        }

        Ok(Self {
            limits,
            in_flight: Arc::new(AtomicU32::new(0)),
        })
    }

    pub fn begin_request(
        &self,
        request_bytes: u64,
        now: MonotonicTime,
    ) -> Result<RequestPermit, TransportError> {
        self.validate_request_bytes(request_bytes)?;
        self.begin_execution(now)
    }

    pub fn begin_execution(&self, now: MonotonicTime) -> Result<RequestPermit, TransportError> {
        let mut current = self.in_flight.load(Ordering::Acquire);
        loop {
            if current >= self.limits.max_concurrent_requests {
                return Err(TransportError::TooManyConcurrentRequests);
            }

            match self.in_flight.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }

        Ok(RequestPermit {
            in_flight: Arc::clone(&self.in_flight),
            deadline: now.saturating_add_millis(self.limits.max_request_duration_ms),
        })
    }

    pub fn validate_request_bytes(&self, request_bytes: u64) -> Result<(), TransportError> {
        if request_bytes > self.limits.max_request_bytes {
            return Err(TransportError::RequestTooLarge);
        }
        Ok(())
    }

    pub fn validate_response_bytes(&self, response_bytes: u64) -> Result<(), TransportError> {
        if response_bytes > self.limits.max_response_bytes {
            return Err(TransportError::ResponseTooLarge);
        }
        Ok(())
    }

    #[must_use]
    pub fn active_requests(&self) -> u32 {
        self.in_flight.load(Ordering::Acquire)
    }

    #[must_use]
    pub const fn limits(&self) -> TransportLimits {
        self.limits
    }
}

#[derive(Debug)]
pub struct RequestPermit {
    in_flight: Arc<AtomicU32>,
    deadline: MonotonicTime,
}

impl RequestPermit {
    #[must_use]
    pub const fn deadline(&self) -> MonotonicTime {
        self.deadline
    }

    #[must_use]
    pub fn is_expired_at(&self, now: MonotonicTime) -> bool {
        now >= self.deadline
    }
}

impl Drop for RequestPermit {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    #[error("transport limits must all be non-zero")]
    InvalidLimits,
    #[error("request exceeds the hard transport byte limit")]
    RequestTooLarge,
    #[error("response exceeds the hard transport byte limit")]
    ResponseTooLarge,
    #[error("too many requests are already in flight")]
    TooManyConcurrentRequests,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard(max_concurrent_requests: u32) -> TransportGuard {
        TransportGuard::new(TransportLimits {
            max_request_bytes: 16,
            max_response_bytes: 32,
            max_request_duration_ms: 100,
            max_concurrent_requests,
        })
        .expect("valid limits")
    }

    #[test]
    fn request_size_fails_closed() {
        let guard = guard(1);
        assert_eq!(
            guard
                .begin_request(17, MonotonicTime::from_millis(0))
                .expect_err("oversized request must fail"),
            TransportError::RequestTooLarge
        );
        assert_eq!(guard.active_requests(), 0);
    }

    #[test]
    fn execution_permit_does_not_revalidate_already_bounded_frames() {
        let guard = guard(1);
        let permit = guard
            .begin_execution(MonotonicTime::from_millis(10))
            .expect("execution permit should be granted");
        assert_eq!(guard.active_requests(), 1);
        drop(permit);
        assert_eq!(guard.active_requests(), 0);
    }

    #[test]
    fn concurrency_is_released_by_raii_permit() {
        let guard = guard(1);
        let permit = guard
            .begin_request(1, MonotonicTime::from_millis(10))
            .expect("first request should fit");
        assert_eq!(guard.active_requests(), 1);
        assert_eq!(
            guard
                .begin_request(1, MonotonicTime::from_millis(10))
                .expect_err("second request must be rejected"),
            TransportError::TooManyConcurrentRequests
        );
        drop(permit);
        assert_eq!(guard.active_requests(), 0);
        assert!(
            guard
                .begin_request(1, MonotonicTime::from_millis(10))
                .is_ok()
        );
    }

    #[test]
    fn permit_owns_a_monotonic_deadline() {
        let guard = guard(1);
        let permit = guard
            .begin_request(1, MonotonicTime::from_millis(500))
            .expect("request should fit");
        assert_eq!(permit.deadline(), MonotonicTime::from_millis(600));
        assert!(!permit.is_expired_at(MonotonicTime::from_millis(599)));
        assert!(permit.is_expired_at(MonotonicTime::from_millis(600)));
    }

    #[test]
    fn response_size_is_bounded_independently() {
        let guard = guard(1);
        assert!(guard.validate_response_bytes(32).is_ok());
        assert_eq!(
            guard
                .validate_response_bytes(33)
                .expect_err("oversized response must fail"),
            TransportError::ResponseTooLarge
        );
    }
}
