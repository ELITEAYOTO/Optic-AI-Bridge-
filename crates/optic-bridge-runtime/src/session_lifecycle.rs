use std::{collections::BTreeSet, sync::Arc};

use optic_bridge_core::{
    Capability, IdError, MonotonicTime, PrincipalId, ProjectId, SessionGrant, SessionHandle,
};
use thiserror::Error;

use crate::{
    ProcessError, ProcessManager, SessionRegistry, SessionRegistryError, TaskLeaseRegistry,
    TaskLeaseRegistryError,
};

#[derive(Clone, Debug)]
pub struct SessionGrantSpec {
    pub principal: PrincipalId,
    pub project: ProjectId,
    pub capabilities: BTreeSet<Capability>,
    pub expires_at: MonotonicTime,
    pub policy_epoch: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionRevokeReport {
    pub session_changed: bool,
    pub revoked_leases: usize,
    pub cancellation_requests: usize,
}

pub struct SessionLifecycleManager {
    sessions: Arc<SessionRegistry>,
    task_leases: Arc<TaskLeaseRegistry>,
    processes: Arc<ProcessManager>,
}

impl SessionLifecycleManager {
    #[must_use]
    pub fn new(
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        processes: Arc<ProcessManager>,
    ) -> Self {
        Self {
            sessions,
            task_leases,
            processes,
        }
    }

    pub fn provision(
        &self,
        spec: SessionGrantSpec,
        now: MonotonicTime,
    ) -> Result<SessionGrant, SessionLifecycleError> {
        if spec.expires_at <= now {
            return Err(SessionLifecycleError::ExpiredAtProvision);
        }
        let handle = SessionHandle::generate().map_err(SessionLifecycleError::HandleGeneration)?;
        let grant = SessionGrant {
            handle,
            principal: spec.principal,
            project: spec.project,
            capabilities: spec.capabilities,
            expires_at: spec.expires_at,
            policy_epoch: spec.policy_epoch,
        };
        self.sessions.register(grant.clone())?;
        Ok(grant)
    }

    pub fn revoke(
        &self,
        session: &SessionHandle,
    ) -> Result<SessionRevokeReport, SessionLifecycleError> {
        // Invalidate the session first so any later partial failure still fails closed
        // for new authorization checks.
        let session_changed = self.sessions.revoke(session)?;

        // Attempt both owner-scoped cleanup operations before propagating either error.
        // A lease-registry failure must not prevent us from requesting process stop.
        let lease_result = self.task_leases.revoke_session(session);
        let process_result = self.processes.cancel_session(session);
        let revoked_leases = lease_result?;
        let cancellation_requests = process_result?;

        Ok(SessionRevokeReport {
            session_changed,
            revoked_leases,
            cancellation_requests,
        })
    }
}

#[derive(Debug, Error)]
pub enum SessionLifecycleError {
    #[error("session expiry must be later than the provisioning time")]
    ExpiredAtProvision,
    #[error("failed to generate application-owned session handle: {0}")]
    HandleGeneration(IdError),
    #[error(transparent)]
    SessionRegistry(#[from] SessionRegistryError),
    #[error(transparent)]
    TaskLeaseRegistry(#[from] TaskLeaseRegistryError),
    #[error(transparent)]
    Process(#[from] ProcessError),
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path, path::PathBuf};

    use optic_bridge_core::{HardLimits, LeaseScope, ResourceBudget, TaskLease, TaskLeaseId};

    use super::*;

    fn workspace(label: &str) -> PathBuf {
        let token = SessionHandle::generate().expect("test entropy").to_token();
        let root = env::temp_dir().join(format!("optic-session-lifecycle-{label}-{token}"));
        fs::create_dir_all(&root).expect("create lifecycle workspace");
        root
    }

    fn spec(expires_at: u64) -> SessionGrantSpec {
        SessionGrantSpec {
            principal: PrincipalId::new("principal").expect("valid principal"),
            project: ProjectId::new("project").expect("valid project"),
            capabilities: BTreeSet::from([Capability::FileRead, Capability::ProcessRun]),
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    fn lease(session: SessionHandle, expires_at: u64) -> TaskLease {
        TaskLease {
            id: TaskLeaseId::generate().expect("test entropy"),
            session,
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([LeaseScope::ProcessExecutable("fixture".to_owned())]),
            resource_ceiling: ResourceBudget {
                timeout_ms: 5_000,
                output_bytes: 1024,
                memory_bytes: 64 * 1024 * 1024,
                process_count: 1,
            },
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    fn manager(
        root: &Path,
    ) -> (
        SessionLifecycleManager,
        Arc<SessionRegistry>,
        Arc<TaskLeaseRegistry>,
    ) {
        let sessions = Arc::new(SessionRegistry::new());
        let task_leases = Arc::new(TaskLeaseRegistry::new());
        let processes = Arc::new(
            ProcessManager::new(root, HardLimits::default(), Vec::new()).expect("process manager"),
        );
        let lifecycle = SessionLifecycleManager::new(
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            processes,
        );
        (lifecycle, sessions, task_leases)
    }

    #[test]
    fn provision_generates_and_registers_application_owned_session() {
        let root = workspace("provision");
        let (lifecycle, sessions, _) = manager(&root);
        let now = MonotonicTime::from_millis(10);
        let grant = lifecycle
            .provision(spec(100), now)
            .expect("provision session");
        let loaded = sessions
            .get_active(&grant.handle, now)
            .expect("session must be registered and active");
        assert_eq!(loaded.handle, grant.handle);

        let second = lifecycle.provision(spec(100), now).expect("second session");
        assert_ne!(second.handle, grant.handle);
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn provision_rejects_already_expired_session() {
        let root = workspace("expired-provision");
        let (lifecycle, sessions, _) = manager(&root);
        assert!(matches!(
            lifecycle.provision(spec(10), MonotonicTime::from_millis(10)),
            Err(SessionLifecycleError::ExpiredAtProvision)
        ));
        assert_eq!(sessions.len().expect("registry length"), 0);
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn revoke_is_fail_closed_and_owner_scoped() {
        let root = workspace("revoke");
        let (lifecycle, sessions, task_leases) = manager(&root);
        let now = MonotonicTime::from_millis(1);
        let session_a = lifecycle.provision(spec(100), now).expect("session A");
        let session_b = lifecycle.provision(spec(100), now).expect("session B");
        let lease_a = lease(session_a.handle.clone(), 100);
        let lease_b = lease(session_b.handle.clone(), 100);
        task_leases.register(lease_a.clone()).expect("lease A");
        task_leases.register(lease_b.clone()).expect("lease B");

        let report = lifecycle.revoke(&session_a.handle).expect("revoke A");
        assert!(report.session_changed);
        assert_eq!(report.revoked_leases, 1);
        assert_eq!(report.cancellation_requests, 0);
        assert_eq!(
            sessions
                .get_active(&session_a.handle, now)
                .expect_err("A must be revoked"),
            SessionRegistryError::Revoked
        );
        sessions
            .get_active(&session_b.handle, now)
            .expect("B must remain active");
        assert_eq!(
            task_leases
                .get_active(&lease_a.id, &session_a.handle, now)
                .expect_err("A lease must be revoked"),
            TaskLeaseRegistryError::Revoked
        );
        task_leases
            .get_active(&lease_b.id, &session_b.handle, now)
            .expect("B lease must remain active");
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }
}
