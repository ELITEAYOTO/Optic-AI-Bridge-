use std::{collections::HashMap, sync::Mutex};

use optic_bridge_core::{MonotonicTime, SessionHandle, TaskLease, TaskLeaseId};
use thiserror::Error;

#[derive(Debug, Default)]
pub struct TaskLeaseRegistry {
    leases: Mutex<HashMap<TaskLeaseId, LeaseRecord>>,
}

#[derive(Clone, Debug)]
struct LeaseRecord {
    lease: TaskLease,
    revoked: bool,
}

impl TaskLeaseRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, lease: TaskLease) -> Result<(), TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        if leases.contains_key(&lease.id) {
            return Err(TaskLeaseRegistryError::AlreadyRegistered);
        }
        leases.insert(
            lease.id.clone(),
            LeaseRecord {
                lease,
                revoked: false,
            },
        );
        Ok(())
    }

    pub fn get_active(
        &self,
        id: &TaskLeaseId,
        session: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<TaskLease, TaskLeaseRegistryError> {
        let leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let record = leases.get(id).ok_or(TaskLeaseRegistryError::UnknownLease)?;
        if record.revoked {
            return Err(TaskLeaseRegistryError::Revoked);
        }
        if &record.lease.session != session {
            return Err(TaskLeaseRegistryError::WrongSession);
        }
        if record.lease.is_expired_at(now) {
            return Err(TaskLeaseRegistryError::Expired);
        }
        Ok(record.lease.clone())
    }

    pub fn revoke(&self, id: &TaskLeaseId) -> Result<bool, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let Some(record) = leases.get_mut(id) else {
            return Ok(false);
        };
        let changed = !record.revoked;
        record.revoked = true;
        Ok(changed)
    }

    pub fn revoke_session(&self, session: &SessionHandle) -> Result<usize, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let mut changed = 0;
        for record in leases.values_mut() {
            if &record.lease.session == session && !record.revoked {
                record.revoked = true;
                changed += 1;
            }
        }
        Ok(changed)
    }

    pub fn remove_expired(&self, now: MonotonicTime) -> Result<usize, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let before = leases.len();
        leases.retain(|_, record| !record.lease.is_expired_at(now));
        Ok(before - leases.len())
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum TaskLeaseRegistryError {
    #[error("task lease registry state is unavailable")]
    StateUnavailable,
    #[error("task lease is already registered")]
    AlreadyRegistered,
    #[error("task lease is unknown")]
    UnknownLease,
    #[error("task lease has been revoked")]
    Revoked,
    #[error("task lease belongs to another session")]
    WrongSession,
    #[error("task lease has expired")]
    Expired,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use optic_bridge_core::{
        Capability, LeaseScope, ResourceBudget, SessionHandle, TaskLease, TaskLeaseId,
    };

    use super::*;

    fn lease(session: SessionHandle, expires_at: u64) -> TaskLease {
        TaskLease {
            id: TaskLeaseId::generate().expect("test entropy"),
            session,
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([LeaseScope::ProcessExecutable("test".to_owned())]),
            resource_ceiling: ResourceBudget {
                timeout_ms: 1000,
                output_bytes: 1024,
                memory_bytes: 1024,
                process_count: 1,
            },
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    #[test]
    fn cross_session_access_fails_closed() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("test entropy");
        let other = SessionHandle::generate().expect("test entropy");
        let lease = lease(owner, 100);
        registry.register(lease.clone()).expect("register lease");
        assert_eq!(
            registry
                .get_active(&lease.id, &other, MonotonicTime::from_millis(1))
                .expect_err("cross-session access must fail"),
            TaskLeaseRegistryError::WrongSession
        );
    }

    #[test]
    fn revoke_session_invalidates_its_leases() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("test entropy");
        let lease = lease(owner.clone(), 100);
        registry.register(lease.clone()).expect("register lease");
        assert_eq!(registry.revoke_session(&owner).expect("revoke session"), 1);
        assert_eq!(
            registry
                .get_active(&lease.id, &owner, MonotonicTime::from_millis(1))
                .expect_err("revoked lease must fail"),
            TaskLeaseRegistryError::Revoked
        );
    }
}
