use std::{collections::HashMap, path::Path, sync::Mutex};

use optic_bridge_core::{
    HardLimits, LeaseScope, LimitError, MonotonicTime, SessionHandle, TaskLease, TaskLeaseId,
};
use thiserror::Error;

use crate::process_authority::ProcessAuthority;

#[derive(Debug)]
pub struct TaskLeaseRegistry {
    leases: Mutex<HashMap<TaskLeaseId, LeaseRecord>>,
    max_leases: u32,
    max_leases_per_session: u32,
}

impl Default for TaskLeaseRegistry {
    fn default() -> Self {
        let limits = HardLimits::default();
        Self {
            leases: Mutex::new(HashMap::new()),
            max_leases: limits.max_task_leases,
            max_leases_per_session: limits.max_task_leases_per_session,
        }
    }
}

#[derive(Clone, Debug)]
struct LeaseRecord {
    lease: TaskLease,
    revoked: bool,
    process_authorities: HashMap<String, ProcessAuthority>,
}

impl TaskLeaseRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_hard_limits(limits: HardLimits) -> Result<Self, LimitError> {
        let limits = limits.validate_nonzero()?;
        Ok(Self {
            leases: Mutex::new(HashMap::new()),
            max_leases: limits.max_task_leases,
            max_leases_per_session: limits.max_task_leases_per_session,
        })
    }

    pub fn register(&self, lease: TaskLease) -> Result<(), TaskLeaseRegistryError> {
        let process_authorities = capture_process_authorities(&lease)?;
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        if leases.contains_key(&lease.id) {
            return Err(TaskLeaseRegistryError::AlreadyRegistered);
        }
        let max_leases = usize::try_from(self.max_leases)
            .map_err(|_| TaskLeaseRegistryError::CapacityExceeded)?;
        if leases.len() >= max_leases {
            return Err(TaskLeaseRegistryError::CapacityExceeded);
        }
        let session_count = leases
            .values()
            .filter(|record| record.lease.session == lease.session)
            .count();
        let max_leases_per_session = usize::try_from(self.max_leases_per_session)
            .map_err(|_| TaskLeaseRegistryError::SessionCapacityExceeded)?;
        if session_count >= max_leases_per_session {
            return Err(TaskLeaseRegistryError::SessionCapacityExceeded);
        }
        leases.insert(
            lease.id.clone(),
            LeaseRecord {
                lease,
                revoked: false,
                process_authorities,
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
        validate_active_record(record, session, now)?;
        Ok(record.lease.clone())
    }

    pub fn get_process_authority(
        &self,
        id: &TaskLeaseId,
        session: &SessionHandle,
        executable: &str,
        now: MonotonicTime,
    ) -> Result<ProcessAuthority, TaskLeaseRegistryError> {
        let leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let record = leases.get(id).ok_or(TaskLeaseRegistryError::UnknownLease)?;
        validate_active_record(record, session, now)?;
        record
            .process_authorities
            .get(executable)
            .cloned()
            .ok_or(TaskLeaseRegistryError::ProcessIdentityUnavailable)
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

    pub(crate) fn remove_revoked(&self, id: &TaskLeaseId) -> Result<bool, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let Some(record) = leases.get(id) else {
            return Ok(false);
        };
        if !record.revoked {
            return Err(TaskLeaseRegistryError::LeaseStillActive);
        }
        leases.remove(id);
        Ok(true)
    }

    pub(crate) fn remove_session(
        &self,
        session: &SessionHandle,
    ) -> Result<usize, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let before = leases.len();
        leases.retain(|_, record| &record.lease.session != session);
        Ok(before - leases.len())
    }
}

fn validate_active_record(
    record: &LeaseRecord,
    session: &SessionHandle,
    now: MonotonicTime,
) -> Result<(), TaskLeaseRegistryError> {
    if record.revoked {
        return Err(TaskLeaseRegistryError::Revoked);
    }
    if &record.lease.session != session {
        return Err(TaskLeaseRegistryError::WrongSession);
    }
    if record.lease.is_expired_at(now) {
        return Err(TaskLeaseRegistryError::Expired);
    }
    Ok(())
}

fn capture_process_authorities(
    lease: &TaskLease,
) -> Result<HashMap<String, ProcessAuthority>, TaskLeaseRegistryError> {
    let mut authorities = HashMap::new();
    for scope in &lease.scopes {
        let LeaseScope::ProcessExecutable(executable) = scope else {
            continue;
        };
        if !Path::new(executable).is_absolute() {
            continue;
        }
        let authority = ProcessAuthority::capture(lease.id.clone(), executable)
            .map_err(|_| TaskLeaseRegistryError::ProcessIdentityUnavailable)?;
        authorities.insert(authority.canonical_path().to_owned(), authority);
    }
    Ok(authorities)
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum TaskLeaseRegistryError {
    #[error("task lease registry state is unavailable")]
    StateUnavailable,
    #[error("task lease is already registered")]
    AlreadyRegistered,
    #[error("task lease registry reached its hard global capacity")]
    CapacityExceeded,
    #[error("session reached its hard task lease capacity")]
    SessionCapacityExceeded,
    #[error("task lease is unknown")]
    UnknownLease,
    #[error("task lease has been revoked")]
    Revoked,
    #[error("task lease belongs to another session")]
    WrongSession,
    #[error("task lease has expired")]
    Expired,
    #[error("process executable identity is unavailable for this lease")]
    ProcessIdentityUnavailable,
    #[error("active task lease cannot be physically removed")]
    LeaseStillActive,
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
    fn registration_is_bounded_globally_and_per_session() {
        let limits = HardLimits {
            max_task_leases: 3,
            max_task_leases_per_session: 2,
            ..HardLimits::default()
        };
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("valid limits");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");

        registry
            .register(lease(session_a.clone(), 100))
            .expect("A1");
        registry
            .register(lease(session_a.clone(), 100))
            .expect("A2");
        assert_eq!(
            registry
                .register(lease(session_a, 100))
                .expect_err("A must hit its per-session ceiling"),
            TaskLeaseRegistryError::SessionCapacityExceeded
        );

        registry
            .register(lease(session_b.clone(), 100))
            .expect("B1");
        assert_eq!(
            registry
                .register(lease(session_b, 100))
                .expect_err("global ceiling must fail closed"),
            TaskLeaseRegistryError::CapacityExceeded
        );
    }

    #[test]
    fn revoked_leases_hold_capacity_until_owner_scoped_removal() {
        let limits = HardLimits {
            max_task_leases: 1,
            max_task_leases_per_session: 1,
            ..HardLimits::default()
        };
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("valid limits");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");
        let owned = lease(session_a.clone(), 100);
        registry.register(owned.clone()).expect("A lease");
        assert!(registry.revoke(&owned.id).expect("revoke A lease"));
        assert_eq!(
            registry
                .register(lease(session_b.clone(), 100))
                .expect_err("revocation alone must not reclaim storage"),
            TaskLeaseRegistryError::CapacityExceeded
        );

        assert_eq!(registry.remove_session(&session_a).expect("remove A"), 1);
        let foreign = lease(session_b.clone(), 100);
        registry
            .register(foreign.clone())
            .expect("B lease after reap");
        registry
            .get_active(&foreign.id, &session_b, MonotonicTime::from_millis(1))
            .expect("B lease remains active");
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
    fn only_revoked_lease_can_be_physically_removed_individually() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner session");
        let owned = lease(owner.clone(), 100);
        registry.register(owned.clone()).expect("owned lease");
        assert_eq!(
            registry
                .remove_revoked(&owned.id)
                .expect_err("active lease removal must fail"),
            TaskLeaseRegistryError::LeaseStillActive
        );
        assert!(registry.revoke(&owned.id).expect("revoke lease"));
        assert!(registry.remove_revoked(&owned.id).expect("remove revoked"));
        assert_eq!(
            registry
                .get_active(&owned.id, &owner, MonotonicTime::from_millis(1))
                .expect_err("removed lease must be unknown"),
            TaskLeaseRegistryError::UnknownLease
        );
    }

    #[test]
    fn remove_session_only_removes_owned_leases() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner session");
        let other = SessionHandle::generate().expect("other session");
        let owned = lease(owner.clone(), 100);
        let foreign = lease(other.clone(), 100);
        registry.register(owned.clone()).expect("owned lease");
        registry.register(foreign.clone()).expect("foreign lease");

        assert_eq!(registry.remove_session(&owner).expect("remove leases"), 1);
        assert_eq!(
            registry
                .get_active(&owned.id, &owner, MonotonicTime::from_millis(1))
                .expect_err("owned lease must be removed"),
            TaskLeaseRegistryError::UnknownLease
        );
        registry
            .get_active(&foreign.id, &other, MonotonicTime::from_millis(1))
            .expect("foreign lease must remain");
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
