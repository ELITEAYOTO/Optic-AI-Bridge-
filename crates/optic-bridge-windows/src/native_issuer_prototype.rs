//! Experimental original-generation native-call admission boundary.
//!
//! This module is deliberately compiled ONLY for Windows unit tests.
//! It is NOT wired into the live MCP GitRead route, nor does it bind
//! SessionRegistry, TaskLeaseRegistry, policy, host shutdown or OS custody.
//! No test-only success here constitutes Windows V1 acceptance.
//!
//! One non-Clone owner exposes a single atomic linearization point for
//! a close request and a new CreateSuspended/Resume call admission.
//! A call already admitted BEFORE close may physically complete AFTER it;
//! the in-flight ticket is retained until the syscall callback returns.
//! The issuer never holds a mutex across that callback.

use std::{
    io::{Error, ErrorKind, Result},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

const COUNT_BITS: u32 = 31;
const COUNT_MASK: u64 = (1_u64 << COUNT_BITS) - 1;
const RESUME_INCREMENT: u64 = 1_u64 << COUNT_BITS;
const CLOSED_BIT: u64 = 1_u64 << (2 * COUNT_BITS);
const COUNTERS_MASK: u64 = CLOSED_BIT - 1;

#[derive(Clone, Copy, Debug)]
pub(crate) enum NativePhase {
    CreateSuspended,
    Resume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrainError {
    TimedOut,
}

#[derive(Debug, Default)]
pub(crate) struct NativeIssuerPrototype {
    state: AtomicU64,
}

struct InFlight<'a> {
    original: &'a NativeIssuerPrototype,
    increment: u64,
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        // The original owner still lives for the complete callback.
        // A CLOSE bit can coexist with nonzero in-flight counters.
        let previous = self
            .original
            .state
            .fetch_sub(self.increment, Ordering::AcqRel);
        let shift = if self.increment == RESUME_INCREMENT {
            COUNT_BITS
        } else {
            0
        };
        debug_assert_ne!((previous >> shift) & COUNT_MASK, 0);
    }
}

impl NativeIssuerPrototype {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Irreversibly close new admission without waiting for Win32.
    /// Returns true only for the first transition.
    pub(crate) fn close(&self) -> bool {
        self.state.fetch_or(CLOSED_BIT, Ordering::AcqRel) & CLOSED_BIT == 0
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.state.load(Ordering::Acquire) & CLOSED_BIT != 0
    }

    pub(crate) fn counts(&self) -> (u32, u32) {
        let state = self.state.load(Ordering::Acquire);
        (
            (state & COUNT_MASK) as u32,
            ((state >> COUNT_BITS) & COUNT_MASK) as u32,
        )
    }

    fn admit(&self, phase: NativePhase) -> Result<InFlight<'_>> {
        let (shift, increment) = match phase {
            NativePhase::CreateSuspended => (0, 1),
            NativePhase::Resume => (COUNT_BITS, RESUME_INCREMENT),
        };
        let mut observed = self.state.load(Ordering::Acquire);
        loop {
            if observed & CLOSED_BIT != 0 {
                return Err(Error::new(
                    ErrorKind::PermissionDenied,
                    "original native issuer closed before call admission",
                ));
            }
            if ((observed >> shift) & COUNT_MASK) == COUNT_MASK {
                self.close();
                return Err(Error::other(
                    "native issuer counter exhausted; original owner permanently sealed",
                ));
            }
            match self.state.compare_exchange_weak(
                observed,
                observed + increment,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Ok(InFlight {
                        original: self,
                        increment,
                    });
                }
                Err(current) => observed = current,
            }
        }
    }

    /// Execute the OS callback only after an admission CAS succeeds.
    /// Do NOT confuse the CAS ordering with the physical Win32 call entry:
    /// a stop published AFTER admission can predate OS execution.
    pub(crate) fn issue<T>(
        &self,
        phase: NativePhase,
        call: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let _original_ticket = self.admit(phase)?;
        call()
    }

    /// Sealed + zero recorded calls. NOT root-exit, Job-empty, stdin,
    /// stdout EOF, stderr EOF, quotas, or crash-recovery evidence.
    pub(crate) fn seal_and_drain_until(
        &self,
        until: Instant,
    ) -> std::result::Result<(), DrainError> {
        self.close();
        loop {
            let observed = self.state.load(Ordering::Acquire);
            if Instant::now() >= until {
                return Err(DrainError::TimedOut);
            }
            if observed & COUNTERS_MASK == 0 {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, mpsc},
        thread,
    };

    #[test]
    fn close_precedes_create_and_resume_admission_and_never_reopens() {
        let issuer = NativeIssuerPrototype::new();
        assert_eq!(issuer.counts(), (0, 0));
        assert!(issuer.close());
        assert!(!issuer.close());
        let entered = std::sync::atomic::AtomicBool::new(false);
        for phase in [NativePhase::CreateSuspended, NativePhase::Resume] {
            for _ in 0..256 {
                let err = issuer
                    .issue(phase, || {
                        entered.store(true, Ordering::Release);
                        Ok(())
                    })
                    .unwrap_err();
                assert_eq!(err.kind(), ErrorKind::PermissionDenied);
            }
        }
        assert!(!entered.load(Ordering::Acquire));
        assert!(issuer.is_closed());
        assert_eq!(issuer.counts(), (0, 0));
        issuer
            .seal_and_drain_until(Instant::now() + Duration::from_secs(1))
            .expect("closed empty original");
    }

    #[test]
    fn close_is_prompt_while_admitted_resume_remains_counted_until_call_return() {
        let issuer = Arc::new(NativeIssuerPrototype::new());
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel::<()>(1);
        let call_issuer = Arc::clone(&issuer);
        let worker = thread::spawn(move || {
            call_issuer.issue(NativePhase::Resume, || {
                entered_tx.send(()).unwrap();
                release_rx
                    .recv_timeout(Duration::from_secs(3))
                    .expect("release in-flight callback");
                Ok(())
            })
        });
        entered_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("native callback entered with ticket");
        assert_eq!(issuer.counts(), (0, 1));
        assert!(
            issuer.close(),
            "close published while OS callback in flight"
        );
        assert_eq!(
            issuer.seal_and_drain_until(Instant::now() + Duration::from_millis(20)),
            Err(DrainError::TimedOut),
            "must not forge an empty native ledger during a blocked call"
        );
        release_tx.send(()).unwrap();
        worker
            .join()
            .expect("native callback thread joined")
            .expect("native callback completed");
        assert_eq!(issuer.counts(), (0, 0));
        issuer
            .seal_and_drain_until(Instant::now() + Duration::from_secs(1))
            .expect("original sealed and its syscall callback returned");
    }
}
