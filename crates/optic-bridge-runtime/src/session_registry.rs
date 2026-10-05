use std::{collections::HashMap, sync::Mutex};

use optic_bridge_core::{HardLimits, LimitError, MonotonicTime, SessionGrant, SessionHandle};
use thiserror::Error;

#[derive(Debug)]
pub struct SessionRegistry {
    sessions: Mutex<HashMap<SessionHandle, SessionRecord>>,
    max_sessions: u32,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            max_sessions: HardLimits::default().max_sessions,
        }
    }
}

#[derive(Clone, Debug)]
struct SessionRecord {
    grant: SessionGrant,
    revoked: bool,
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
            max_sessions: limits.max_sessions,
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

    pub(crate) fn remove_inactive(
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
        sessions.remove(handle);
        Ok(true)
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
    #[error("session is still active and cannot be removed")]
    StillActive,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

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
    fn inactive_removal_refuses_active_session() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        assert_eq!(
            registry
                .remove_inactive(&grant.handle, MonotonicTime::from_millis(99))
                .expect_err("active session must not be removed"),
            SessionRegistryError::StillActive
        );
        registry
            .get_active(&grant.handle, MonotonicTime::from_millis(99))
            .expect("active session must remain registered");
    }

    #[test]
    fn inactive_handles_include_revoked_and_expired_sessions() {
        let registry = SessionRegistry::new();
        let revoked = grant(1000);
        let expired = grant(10);
        let active = grant(1000);
        registry.register(revoked.clone()).expect("revoked session");
        registry.register(expired.clone()).expect("expired session");
        registry.register(active.clone()).expect("active session");
        registry.revoke(&revoked.handle).expect("revoke session");

        let inactive = registry
            .inactive_handles(MonotonicTime::from_millis(100))
            .expect("inactive handles");
        assert!(inactive.contains(&revoked.handle));
        assert!(inactive.contains(&expired.handle));
        assert!(!inactive.contains(&active.handle));
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
