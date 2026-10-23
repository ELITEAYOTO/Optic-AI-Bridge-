//! RESEARCH ONLY: authority commit is short; native calls happen after it.
//! Owns genuine admission continuously. Not GitRead/task-policy authorization.
use super::*;
use crate::Clock;
use optic_bridge_core::JobId;

struct Metadata {
    owner: JobId,
    stop: Arc<AtomicBool>,
    deadline: MonotonicTime,
}

#[must_use = "ticket contains genuine admission; transfer into its prepared keeper"]
pub(crate) struct ResearchLaunchTicket<'a> {
    _admission: SessionAdmissionPermit<'a>,
    metadata: Metadata,
}
impl ResearchLaunchTicket<'_> {
    pub(crate) fn permits(&self, owner: &JobId, now: MonotonicTime) -> bool {
        &self.metadata.owner == owner
            && now < self.metadata.deadline
            && !self.metadata.stop.load(Ordering::Acquire)
    }
    pub(crate) fn deadline(&self) -> MonotonicTime {
        self.metadata.deadline
    }
}

// Both variants own admission. A denied commit must return it to the keeper,
// not drop it before the already-reserved owner's no-spawn/terminal proof.
#[must_use]
pub(crate) enum ResearchCommitResult<'a> {
    Committed(ResearchLaunchTicket<'a>),
    Denied {
        admission: SessionAdmissionPermit<'a>,
        reason: SessionRegistryError,
    },
}

impl SessionRegistry {
    #[cfg(test)]
    pub(crate) fn research_commit_ticket<'a>(
        &self,
        admission: SessionAdmissionPermit<'a>,
        owner: &JobId,
        clock: &dyn Clock,
        owner_deadline: MonotonicTime,
    ) -> ResearchCommitResult<'a> {
        let metadata = (|| {
            if !std::ptr::eq(admission.registry, self) {
                return Err(SessionRegistryError::UnknownSession);
            }
            let gate = self
                .research_gate(&admission.handle)?
                .ok_or(SessionRegistryError::UnknownSession)?;
            let _guard = gate
                .lock()
                .map_err(|_| SessionRegistryError::StateUnavailable)?;
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| SessionRegistryError::StateUnavailable)?;
            let record = sessions
                .get(&admission.handle)
                .ok_or(SessionRegistryError::UnknownSession)?;
            if !Arc::ptr_eq(&gate, &record.research_commit_gate)
                || record.grant.principal != admission.grant.principal
                || record.grant.project != admission.grant.project
                || record.grant.policy_epoch != admission.grant.policy_epoch
            {
                return Err(SessionRegistryError::UnknownSession);
            }
            if record.revoked {
                return Err(SessionRegistryError::Revoked);
            }
            // Read INSIDE commit, after any gate wait. Never reset the supplied
            // absolute owner budget, nor silently extend an old admission on renewal.
            let now = clock.now();
            let deadline = MonotonicTime::from_millis(
                owner_deadline
                    .as_millis()
                    .min(record.grant.expires_at.as_millis())
                    .min(admission.grant.expires_at.as_millis()),
            );
            if now >= deadline {
                return Err(SessionRegistryError::Expired);
            }
            Ok(Metadata {
                owner: owner.clone(),
                stop: Arc::clone(&record.revocation_signal),
                deadline,
            })
        })();
        match metadata {
            Ok(metadata) => ResearchCommitResult::Committed(ResearchLaunchTicket {
                _admission: admission,
                metadata,
            }),
            Err(reason) => ResearchCommitResult::Denied { admission, reason },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_bridge_core::{Capability, PrincipalId, ProjectId};
    use std::{
        collections::BTreeSet,
        sync::atomic::AtomicU64,
        thread,
        time::{Duration, Instant},
    };

    struct FixedClock(AtomicU64);
    impl Clock for FixedClock {
        fn now(&self) -> MonotonicTime {
            MonotonicTime::from_millis(self.0.load(Ordering::Acquire))
        }
    }
    fn grant() -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().unwrap(),
            principal: PrincipalId::new("ticket-test").unwrap(),
            project: ProjectId::new("synthetic").unwrap(),
            capabilities: BTreeSet::from([Capability::GitRead]),
            expires_at: MonotonicTime::from_millis(1_000),
            policy_epoch: 1,
        }
    }

    #[test]
    fn ticket_owns_admission_across_prompt_close_without_holding_gate() {
        let registry = SessionRegistry::new();
        let grant = grant();
        registry.register(grant.clone()).unwrap();
        let clock = FixedClock(AtomicU64::new(1));
        let id = JobId::generate().unwrap();
        let permit = registry
            .begin_admission(&grant.handle, clock.now())
            .unwrap();
        let ticket = match registry.research_commit_ticket(
            permit,
            &id,
            &clock,
            MonotonicTime::from_millis(100),
        ) {
            ResearchCommitResult::Committed(ticket) => ticket,
            ResearchCommitResult::Denied { .. } => panic!("valid commit denied"),
        };
        assert!(ticket.permits(&id, clock.now()));
        assert!(!ticket.permits(&JobId::generate().unwrap(), clock.now()));
        assert_eq!(
            registry.research_close_admission(&grant.handle).unwrap(),
            (true, 1)
        );
        assert!(!ticket.permits(&id, clock.now()));
        assert_eq!(
            registry
                .remove_quiescent_inactive(&grant.handle, clock.now())
                .unwrap_err(),
            SessionRegistryError::AdmissionsInFlight
        );
        drop(ticket); // inert unit: no native effect exists here
        assert!(!registry.revoke(&grant.handle).unwrap());
    }

    #[test]
    fn denied_commit_returns_the_original_permit() {
        let registry = SessionRegistry::new();
        let grant = grant();
        registry.register(grant.clone()).unwrap();
        let clock = FixedClock(AtomicU64::new(1));
        let permit = registry
            .begin_admission(&grant.handle, clock.now())
            .unwrap();
        registry.research_close_admission(&grant.handle).unwrap();
        let rejected = registry.research_commit_ticket(
            permit,
            &JobId::generate().unwrap(),
            &clock,
            MonotonicTime::from_millis(100),
        );
        match rejected {
            ResearchCommitResult::Denied { admission, reason } => {
                assert_eq!(reason, SessionRegistryError::Revoked);
                assert_eq!(
                    registry.research_close_admission(&grant.handle).unwrap(),
                    (false, 1)
                );
                drop(admission);
            }
            ResearchCommitResult::Committed(_) => panic!("closed session committed"),
        }
    }

    #[test]
    fn foreign_registry_permit_cannot_mint_ticket() {
        let a = SessionRegistry::new();
        let b = SessionRegistry::new();
        let grant = grant();
        a.register(grant.clone()).unwrap();
        b.register(grant.clone()).unwrap();
        let clock = FixedClock(AtomicU64::new(1));
        let permit = a.begin_admission(&grant.handle, clock.now()).unwrap();
        match b.research_commit_ticket(
            permit,
            &JobId::generate().unwrap(),
            &clock,
            MonotonicTime::from_millis(100),
        ) {
            ResearchCommitResult::Denied { admission, reason } => {
                assert_eq!(reason, SessionRegistryError::UnknownSession);
                assert_eq!(
                    a.research_close_admission(&grant.handle).unwrap(),
                    (true, 1)
                );
                assert!(b.get_active(&grant.handle, clock.now()).is_ok());
                drop(admission);
            }
            ResearchCommitResult::Committed(_) => panic!("foreign authority committed"),
        }
    }

    #[test]
    fn clock_is_rechecked_after_gate_wait_and_budget_is_not_restarted() {
        let registry = SessionRegistry::new();
        let grant = grant();
        registry.register(grant.clone()).unwrap();
        let clock = FixedClock(AtomicU64::new(1));
        let permit = registry
            .begin_admission(&grant.handle, clock.now())
            .unwrap();
        let gate = registry.research_gate(&grant.handle).unwrap().unwrap();
        let held = gate.lock().unwrap();
        thread::scope(|scope| {
            let registry_ref = &registry;
            let clock_ref = &clock;
            let blocked = scope.spawn(move || {
                registry_ref.research_commit_ticket(
                    permit,
                    &JobId::generate().unwrap(),
                    clock_ref,
                    MonotonicTime::from_millis(50),
                )
            });
            let until = Instant::now() + Duration::from_secs(2);
            while Arc::strong_count(&gate) != 3 {
                assert!(Instant::now() < until);
                thread::yield_now();
            }
            clock.0.store(50, Ordering::Release);
            drop(held);
            match blocked.join().unwrap() {
                ResearchCommitResult::Denied { admission, reason } => {
                    assert_eq!(reason, SessionRegistryError::Expired);
                    drop(admission);
                }
                ResearchCommitResult::Committed(_) => panic!("stale captured clock authorized"),
            }
        });
        clock.0.store(60, Ordering::Release);
        let permit = registry
            .begin_admission(&grant.handle, clock.now())
            .unwrap();
        match registry.research_commit_ticket(
            permit,
            &JobId::generate().unwrap(),
            &clock,
            MonotonicTime::from_millis(65),
        ) {
            ResearchCommitResult::Committed(ticket) => {
                assert_eq!(ticket.deadline(), MonotonicTime::from_millis(65));
            }
            ResearchCommitResult::Denied { .. } => panic!("fresh remaining budget denied"),
        }
    }
}
