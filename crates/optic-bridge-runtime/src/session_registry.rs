use std::{collections::HashMap, sync::Mutex};

use optic_bridge_core::{MonotonicTime, SessionGrant, SessionHandle};
use thiserror::Error;

#[derive(Debug, Default)]
pub struct SessionRegistry {
    sessions: Mutex<HashMap<SessionHandle, SessionRecord>>,
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

    pub fn register(&self, grant: SessionGrant) -> Result<(), SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        if sessions.contains_key(&grant.handle) {
            return Err(SessionRegistryError::AlreadyRegistered);
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

    pub fn remove_expired(&self, now: MonotonicTime) -> Result<usize, SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let before = sessions.len();
        sessions.retain(|_, record| !record.grant.is_expired_at(now));
        Ok(before - sessions.len())
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
    #[error("session handle is unknown")]
    UnknownSession,
    #[error("session has been revoked")]
    Revoked,
    #[error("session has expired")]
    Expired,
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
}
