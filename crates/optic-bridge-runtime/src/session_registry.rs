use std::{
    collections::HashMap,
    sync::{Condvar, Mutex},
};

use optic_bridge_core::{HardLimits, LimitError, MonotonicTime, SessionGrant, SessionHandle};
use thiserror::Error;

#[derive(Debug)]
pub struct SessionRegistry {
    sessions: Mutex<HashMap<SessionHandle, SessionRecord>>,
    admissions_drained: Condvar,
    max_sessions: u32,
    max_renewal_horizon_ms: u64,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            admissions_drained: Condvar::new(),
            max_sessions: HardLimits::default().max_sessions,
            max_renewal_horizon_ms: HardLimits::default().max_session_renewal_horizon_ms,
        }
    }
}

#[derive(Clone, Debug)]
struct SessionRecord {
    grant: SessionGrant,
    revoked: bool,
    active_admissions: u32,
}

impl SessionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_hard_limits(limits: HardLimits) -> Result<Self, LimitError> {
        let limits = limits.validate_nonzero()?;
        Ok(Self {
            sessions: Mutex::new(HashMap::new()),
            admissions_drained: Condvar::new(),
            max_sessions: limits.max_sessions,
            max_renewal_horizon_ms: limits.max_session_renewal_horizon_ms,
        })
    }

    pub fn register(&self, grant: SessionGrant) -> Result<(), SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        if sessions.contains_key(&grant.handle) {
            return Err(SessionRegistryError::AlreadyRegistered);
        }
        let max_sessions = usize::try_from(self.max_sessions)
            .map_err(|_| SessionRegistryError::CapacityExceeded)?;
        if sessions.len() >= max_sessions {
            return Err(SessionRegistryError::CapacityExceeded);
        }
        sessions.insert(
            grant.handle.clone(),
            SessionRecord {
                grant,
                revoked: false,
                active_admissions: 0,
            },
        );
        Ok(())
    }

    pub fn get_active(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<SessionGrant, SessionRegistryError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let record = sessions
            .get(handle)
            .ok_or(SessionRegistryError::UnknownSession)?;
        if record.revoked {
            return Err(SessionRegistryError::Revoked);
        }
        if record.grant.is_expired_at(now) {
            return Err(SessionRegistryError::Expired);
        }
        Ok(record.grant.clone())
    }

    pub fn begin_admission(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<SessionAdmissionPermit<'_>, SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let record = sessions
            .get_mut(handle)
            .ok_or(SessionRegistryError::UnknownSession)?;
        if record.revoked {
            return Err(SessionRegistryError::Revoked);
        }
        if record.grant.is_expired_at(now) {
            return Err(SessionRegistryError::Expired);
        }
        record.active_admissions = record
            .active_admissions
            .checked_add(1)
            .ok_or(SessionRegistryError::AdmissionCounterOverflow)?;
        let grant = record.grant.clone();
        Ok(SessionAdmissionPermit {
            registry: self,
            handle: handle.clone(),
            grant,
        })
    }

    pub(crate) fn validate_renewal(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
        new_expires_at: MonotonicTime,
    ) -> Result<SessionGrant, SessionRenewalError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRenewalError::StateUnavailable)?;
        let record = sessions
            .get(handle)
            .ok_or(SessionRenewalError::UnknownSession)?;
        validate_renewal_record(record, now, new_expires_at, self.max_renewal_horizon_ms)?;
        Ok(record.grant.clone())
    }

    pub(crate) fn renew_active(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
        new_expires_at: MonotonicTime,
    ) -> Result<SessionGrant, SessionRenewalError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRenewalError::StateUnavailable)?;
        let record = sessions
            .get_mut(handle)
            .ok_or(SessionRenewalError::UnknownSession)?;
        validate_renewal_record(record, now, new_expires_at, self.max_renewal_horizon_ms)?;
        record.grant.expires_at = new_expires_at;
        Ok(record.grant.clone())
    }

    pub fn revoke(&self, handle: &SessionHandle) -> Result<bool, SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let Some(record) = sessions.get_mut(handle) else {
            return Ok(false);
        };
        let changed = !record.revoked;
        record.revoked = true;

        loop {
            let active_admissions = sessions
                .get(handle)
                .ok_or(SessionRegistryError::UnknownSession)?
                .active_admissions;
            if active_admissions == 0 {
                break;
            }
            sessions = self
                .admissions_drained
                .wait(sessions)
                .map_err(|_| SessionRegistryError::StateUnavailable)?;
        }
        Ok(changed)
    }

    pub(crate) fn inactive_handles(
        &self,
        now: MonotonicTime,
    ) -> Result<Vec<SessionHandle>, SessionRegistryError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        Ok(sessions
            .iter()
            .filter(|(_, record)| record.revoked || record.grant.is_expired_at(now))
            .map(|(handle, _)| handle.clone())
            .collect())
    }

    pub(crate) fn remove_quiescent_inactive(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<bool, SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let Some(record) = sessions.get(handle) else {
            return Ok(false);
        };
        if !record.revoked && !record.grant.is_expired_at(now) {
            return Err(SessionRegistryError::StillActive);
        }
        if record.active_admissions != 0 {
            return Err(SessionRegistryError::AdmissionsInFlight);
        }
        sessions.remove(handle);
        Ok(true)
    }

    fn finish_admission(&self, handle: &SessionHandle) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(record) = sessions.get_mut(handle) else {
            return;
        };
        if record.active_admissions == 0 {
            debug_assert!(false, "session admission counter underflow");
            return;
        }
        record.active_admissions -= 1;
        if record.active_admissions == 0 {
            self.admissions_drained.notify_all();
        }
    }

    pub fn len(&self) -> Result<usize, SessionRegistryError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        Ok(sessions.len())
    }

    pub fn is_empty(&self) -> Result<bool, SessionRegistryError> {
        Ok(self.len()? == 0)
    }
}

fn validate_renewal_record(
    record: &SessionRecord,
    now: MonotonicTime,
    new_expires_at: MonotonicTime,
    max_renewal_horizon_ms: u64,
) -> Result<(), SessionRenewalError> {
    if new_expires_at <= now {
        return Err(SessionRenewalError::ExpiryNotFuture);
    }
    if new_expires_at > now.saturating_add_millis(max_renewal_horizon_ms) {
        return Err(SessionRenewalError::BeyondHorizon);
    }
    if record.revoked {
        return Err(SessionRenewalError::Revoked);
    }
    if record.grant.is_expired_at(now) {
        return Err(SessionRenewalError::Expired);
    }
    if new_expires_at <= record.grant.expires_at {
        return Err(SessionRenewalError::MustExtend);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionRenewalError {
    #[error("session renewal state is unavailable")]
    StateUnavailable,
    #[error("session renewal target is unknown")]
    UnknownSession,
    #[error("revoked sessions cannot be renewed")]
    Revoked,
    #[error("expired sessions cannot be renewed")]
    Expired,
    #[error("session renewal expiry must be later than now")]
    ExpiryNotFuture,
    #[error("session renewal exceeds the configured future horizon")]
    BeyondHorizon,
    #[error("session renewal must strictly extend the current expiry")]
    MustExtend,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SessionRegistryError {
    #[error("session registry state is unavailable")]
    StateUnavailable,
    #[error("session handle is already registered")]
    AlreadyRegistered,
    #[error("session registry reached its hard session capacity")]
    CapacityExceeded,
    #[error("session handle is unknown")]
    UnknownSession,
    #[error("session has been revoked")]
    Revoked,
    #[error("session has expired")]
    Expired,
    #[error("session admission counter reached its representable limit")]
    AdmissionCounterOverflow,
    #[error("session still has admitted effects in flight")]
    AdmissionsInFlight,
    #[error("session is still active and cannot be removed")]
    StillActive,
}

pub struct SessionAdmissionPermit<'a> {
    registry: &'a SessionRegistry,
    handle: SessionHandle,
    grant: SessionGrant,
}

impl SessionAdmissionPermit<'_> {
    #[must_use]
    pub fn grant(&self) -> &SessionGrant {
        &self.grant
    }
}

impl Drop for SessionAdmissionPermit<'_> {
    fn drop(&mut self) {
        self.registry.finish_admission(&self.handle);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        sync::{Arc, mpsc},
        thread,
        time::Duration,
    };

    use optic_bridge_core::{Capability, PrincipalId, ProjectId};

    use super::*;

    fn grant(expires_at: u64) -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().expect("test entropy"),
            principal: PrincipalId::new("principal").expect("valid principal"),
            project: ProjectId::new("project").expect("valid project"),
            capabilities: BTreeSet::from([Capability::FileRead]),
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    #[test]
    fn active_session_round_trips() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        let loaded = registry
            .get_active(&grant.handle, MonotonicTime::from_millis(99))
            .expect("session should be active");
        assert_eq!(loaded.handle, grant.handle);
    }

    #[test]
    fn revoke_fails_closed_immediately() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        assert!(registry.revoke(&grant.handle).expect("revoke session"));
        assert_eq!(
            registry
                .get_active(&grant.handle, MonotonicTime::from_millis(1))
                .expect_err("revoked session must fail"),
            SessionRegistryError::Revoked
        );
    }

    #[test]
    fn expiry_is_checked_against_monotonic_time() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        assert_eq!(
            registry
                .get_active(&grant.handle, MonotonicTime::from_millis(100))
                .expect_err("expired session must fail"),
            SessionRegistryError::Expired
        );
    }

    #[test]
    fn inactive_handles_include_only_revoked_and_expired_sessions() {
        let registry = SessionRegistry::new();
        let active = grant(1_000);
        let expired = grant(10);
        let revoked = grant(1_000);
        registry.register(active.clone()).expect("active session");
        registry.register(expired.clone()).expect("expired session");
        registry.register(revoked.clone()).expect("revoked session");
        registry.revoke(&revoked.handle).expect("revoke session");

        let inactive = registry
            .inactive_handles(MonotonicTime::from_millis(10))
            .expect("inactive handles");
        assert_eq!(inactive.len(), 2);
        assert!(inactive.contains(&expired.handle));
        assert!(inactive.contains(&revoked.handle));
        assert!(!inactive.contains(&active.handle));
    }

    #[test]
    fn duplicate_handle_is_rejected() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("first insert");
        assert_eq!(
            registry.register(grant).expect_err("duplicate must fail"),
            SessionRegistryError::AlreadyRegistered
        );
    }

    #[test]
    fn revoke_closes_new_admissions_and_waits_for_existing_permit() {
        let registry = Arc::new(SessionRegistry::new());
        let grant = grant(1_000);
        registry.register(grant.clone()).expect("register session");
        let now = MonotonicTime::from_millis(1);
        let permit = registry
            .begin_admission(&grant.handle, now)
            .expect("admit effect");

        let worker_registry = Arc::clone(&registry);
        let worker_handle = grant.handle.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let result = worker_registry.revoke(&worker_handle);
            done_tx.send(result).expect("send revoke result");
        });

        let mut observed_revoked = false;
        for _ in 0..100 {
            if matches!(
                registry.get_active(&grant.handle, now),
                Err(SessionRegistryError::Revoked)
            ) {
                observed_revoked = true;
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            observed_revoked,
            "revoke must close admission before waiting"
        );
        assert!(matches!(
            registry.begin_admission(&grant.handle, now),
            Err(SessionRegistryError::Revoked)
        ));
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));

        drop(permit);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("revoke must drain after permit drop")
                .expect("revoke result")
        );
        worker.join().expect("revoke thread");
    }

    #[test]
    fn renewal_extends_only_active_session_expiry() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        let renewed = registry
            .renew_active(
                &grant.handle,
                MonotonicTime::from_millis(10),
                MonotonicTime::from_millis(200),
            )
            .expect("renew session");
        assert_eq!(renewed.expires_at, MonotonicTime::from_millis(200));
        assert_eq!(renewed.capabilities, grant.capabilities);
        assert_eq!(renewed.policy_epoch, grant.policy_epoch);
    }

    #[test]
    fn renewal_cannot_exceed_configured_future_horizon() {
        let limits = HardLimits {
            max_session_renewal_horizon_ms: 50,
            ..HardLimits::default()
        };
        let registry = SessionRegistry::from_hard_limits(limits).expect("limits");
        let grant = grant(100);
        registry.register(grant.clone()).expect("session");
        assert_eq!(
            registry
                .renew_active(
                    &grant.handle,
                    MonotonicTime::from_millis(10),
                    MonotonicTime::from_millis(61),
                )
                .expect_err("horizon must fail"),
            SessionRenewalError::BeyondHorizon
        );
    }

    #[test]
    fn renewal_cannot_shorten_revoke_or_resurrect_expired_session() {
        let registry = SessionRegistry::new();
        let active = grant(100);
        registry.register(active.clone()).expect("active session");
        assert_eq!(
            registry
                .renew_active(
                    &active.handle,
                    MonotonicTime::from_millis(10),
                    MonotonicTime::from_millis(100),
                )
                .expect_err("equal expiry must fail"),
            SessionRenewalError::MustExtend
        );

        let revoked = grant(100);
        registry.register(revoked.clone()).expect("revoked session");
        registry.revoke(&revoked.handle).expect("revoke");
        assert_eq!(
            registry
                .renew_active(
                    &revoked.handle,
                    MonotonicTime::from_millis(10),
                    MonotonicTime::from_millis(200),
                )
                .expect_err("revoked must fail"),
            SessionRenewalError::Revoked
        );

        let expired = grant(10);
        registry.register(expired.clone()).expect("expired session");
        assert_eq!(
            registry
                .renew_active(
                    &expired.handle,
                    MonotonicTime::from_millis(10),
                    MonotonicTime::from_millis(200),
                )
                .expect_err("expired must fail"),
            SessionRenewalError::Expired
        );
    }

    #[test]
    fn registration_is_bounded_by_hard_session_capacity() {
        let limits = HardLimits {
            max_sessions: 2,
            ..HardLimits::default()
        };
        let registry = SessionRegistry::from_hard_limits(limits).expect("valid limits");
        registry.register(grant(100)).expect("first session");
        registry.register(grant(100)).expect("second session");
        assert_eq!(
            registry
                .register(grant(100))
                .expect_err("capacity must fail closed"),
            SessionRegistryError::CapacityExceeded
        );
        assert_eq!(registry.len().expect("registry length"), 2);
    }
}
