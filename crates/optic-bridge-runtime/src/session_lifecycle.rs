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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionReapReport {
    pub inactive_sessions: usize,
    pub reaped_sessions: usize,
    pub deferred_sessions: usize,
    pub revoked_leases: usize,
    pub cancellation_requests: usize,
    pub removed_leases: usize,
    pub removed_process_records: usize,
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
        // The session itself is invalidated first so any later partial failure
        // still fails closed for new authorization checks.
        let session_changed = self.sessions.revoke(session)?;
        let revoked_leases = self.task_leases.revoke_session(session)?;
        let cancellation_requests = self.processes.cancel_session(session)?;
        Ok(SessionRevokeReport {
            session_changed,
            revoked_leases,
            cancellation_requests,
        })
    }

    /// Reap revoked or expired sessions whose process jobs are already terminal.
    ///
    /// This is a runtime API, not an MCP tool. Until admission and revocation are
    /// serialized by the Phase 3B2 gate, callers must only invoke it when no new
    /// session-scoped activity can be admitted concurrently.
    pub fn reap_inactive(
        &self,
        now: MonotonicTime,
    ) -> Result<SessionReapReport, SessionLifecycleError> {
        let handles = self.sessions.inactive_handles(now)?;
        let mut report = SessionReapReport {
            inactive_sessions: handles.len(),
            ..SessionReapReport::default()
        };

        for session in handles {
            report.revoked_leases = report
                .revoked_leases
                .saturating_add(self.task_leases.revoke_session(&session)?);
            report.cancellation_requests = report
                .cancellation_requests
                .saturating_add(self.processes.cancel_session(&session)?);

            if self.processes.active_session_job_count(&session)? != 0 {
                report.deferred_sessions = report.deferred_sessions.saturating_add(1);
                continue;
            }

            report.removed_process_records = report
                .removed_process_records
                .saturating_add(self.processes.remove_terminal_session_records(&session)?);
            report.removed_leases = report
                .removed_leases
                .saturating_add(self.task_leases.remove_session(&session)?);
            if self.sessions.remove_inactive(&session, now)? {
                report.reaped_sessions = report.reaped_sessions.saturating_add(1);
            }
        }

        Ok(report)
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
    use std::{
        env, fs,
        path::{Path, PathBuf},
        thread,
        time::Duration,
    };

    use optic_bridge_core::{
        HardLimits, JobId, LeaseScope, ResourceBudget, TaskLease, TaskLeaseId,
    };

    use super::*;
    use crate::{ProcessResult, ProcessStartSpec, ProcessStatus};

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
            resource_ceiling: process_budget(5000, 1024),
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    fn process_budget(timeout_ms: u64, output_bytes: u64) -> ResourceBudget {
        ResourceBudget {
            timeout_ms,
            output_bytes,
            memory_bytes: 64 * 1024 * 1024,
            process_count: 4,
        }
    }

    fn process_spec(root: &Path, session: SessionHandle) -> ProcessStartSpec {
        let executable = env::current_exe()
            .expect("current test executable")
            .canonicalize()
            .expect("canonical test executable")
            .to_string_lossy()
            .into_owned();
        let _ = root;
        ProcessStartSpec {
            session,
            executable,
            args: vec![
                "--exact".to_owned(),
                "session_lifecycle::tests::lifecycle_process_fixture_child".to_owned(),
                "--nocapture".to_owned(),
            ],
            cwd: None,
            env_allowlist: Vec::new(),
            resources: process_budget(5000, 1024),
        }
    }

    fn manager(
        root: &Path,
        limits: HardLimits,
    ) -> (
        SessionLifecycleManager,
        Arc<SessionRegistry>,
        Arc<TaskLeaseRegistry>,
        Arc<ProcessManager>,
    ) {
        let sessions =
            Arc::new(SessionRegistry::from_hard_limits(limits).expect("session registry"));
        let task_leases = Arc::new(TaskLeaseRegistry::new());
        let processes =
            Arc::new(ProcessManager::new(root, limits, Vec::new()).expect("process manager"));
        let lifecycle = SessionLifecycleManager::new(
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            Arc::clone(&processes),
        );
        (lifecycle, sessions, task_leases, processes)
    }

    async fn await_terminal(
        processes: &ProcessManager,
        session: &SessionHandle,
        job: &JobId,
    ) -> ProcessResult {
        for _ in 0..300 {
            let result = processes.result(session, job).expect("read process result");
            if result.status != ProcessStatus::Running {
                return result;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("lifecycle fixture process did not reach a terminal state");
    }

    #[test]
    fn lifecycle_process_fixture_child() {
        let cwd = env::current_dir().expect("fixture cwd");
        if cwd.join("fixture-sleep").exists() {
            thread::sleep(Duration::from_secs(2));
        }
    }

    #[test]
    fn provision_generates_and_registers_application_owned_session() {
        let root = workspace("provision");
        let (lifecycle, sessions, _, _) = manager(&root, HardLimits::default());
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
        let (lifecycle, sessions, _, _) = manager(&root, HardLimits::default());
        assert!(matches!(
            lifecycle.provision(spec(10), MonotonicTime::from_millis(10)),
            Err(SessionLifecycleError::ExpiredAtProvision)
        ));
        assert_eq!(sessions.len().expect("registry length"), 0);
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn active_session_is_never_reaped() {
        let root = workspace("active-reap");
        let (lifecycle, sessions, _, _) = manager(&root, HardLimits::default());
        let grant = lifecycle
            .provision(spec(100), MonotonicTime::from_millis(1))
            .expect("provision session");
        let report = lifecycle
            .reap_inactive(MonotonicTime::from_millis(99))
            .expect("reap inactive sessions");
        assert_eq!(report, SessionReapReport::default());
        sessions
            .get_active(&grant.handle, MonotonicTime::from_millis(99))
            .expect("active session must remain");
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn revoke_is_fail_closed_and_owner_scoped() {
        let root = workspace("revoke");
        let (lifecycle, sessions, task_leases, _) = manager(&root, HardLimits::default());
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

    #[test]
    fn expired_session_reap_revokes_and_removes_owned_leases() {
        let root = workspace("expired-reap");
        let (lifecycle, sessions, task_leases, _) = manager(&root, HardLimits::default());
        let grant = lifecycle
            .provision(spec(10), MonotonicTime::from_millis(1))
            .expect("provision session");
        let owned = lease(grant.handle.clone(), 100);
        task_leases.register(owned.clone()).expect("register lease");

        let report = lifecycle
            .reap_inactive(MonotonicTime::from_millis(10))
            .expect("reap expired session");
        assert_eq!(report.inactive_sessions, 1);
        assert_eq!(report.reaped_sessions, 1);
        assert_eq!(report.deferred_sessions, 0);
        assert_eq!(report.revoked_leases, 1);
        assert_eq!(report.removed_leases, 1);
        assert_eq!(sessions.len().expect("registry length"), 0);
        assert_eq!(
            task_leases
                .get_active(&owned.id, &grant.handle, MonotonicTime::from_millis(10))
                .expect_err("owned lease must be removed"),
            TaskLeaseRegistryError::UnknownLease
        );
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[tokio::test]
    async fn capacity_is_not_released_until_revoked_session_jobs_are_terminal() {
        let root = workspace("deferred-reap");
        fs::write(root.join("fixture-sleep"), b"1").expect("write fixture mode");
        let limits = HardLimits {
            max_sessions: 1,
            ..HardLimits::default()
        };
        let (lifecycle, sessions, task_leases, processes) = manager(&root, limits);
        let now = MonotonicTime::from_millis(1);
        let first = lifecycle.provision(spec(1000), now).expect("first session");
        let owned_lease = lease(first.handle.clone(), 1000);
        task_leases
            .register(owned_lease.clone())
            .expect("register first lease");
        let job = processes
            .start(process_spec(&root, first.handle.clone()))
            .expect("start lifecycle fixture");

        let revoke = lifecycle
            .revoke(&first.handle)
            .expect("revoke first session");
        assert!(revoke.session_changed);
        assert_eq!(revoke.revoked_leases, 1);
        assert_eq!(revoke.cancellation_requests, 1);

        let first_reap = lifecycle.reap_inactive(now).expect("first reap");
        assert_eq!(first_reap.inactive_sessions, 1);
        assert_eq!(first_reap.reaped_sessions, 0);
        assert_eq!(first_reap.deferred_sessions, 1);
        assert_eq!(sessions.len().expect("registry length"), 1);
        assert!(matches!(
            lifecycle.provision(spec(1000), now),
            Err(SessionLifecycleError::SessionRegistry(
                SessionRegistryError::CapacityExceeded
            ))
        ));

        let terminal = await_terminal(&processes, &first.handle, &job).await;
        assert_ne!(terminal.status, ProcessStatus::Running);

        let second_reap = lifecycle.reap_inactive(now).expect("second reap");
        assert_eq!(second_reap.inactive_sessions, 1);
        assert_eq!(second_reap.reaped_sessions, 1);
        assert_eq!(second_reap.deferred_sessions, 0);
        assert_eq!(second_reap.removed_process_records, 1);
        assert_eq!(second_reap.removed_leases, 1);
        assert_eq!(sessions.len().expect("registry length"), 0);
        assert!(matches!(
            processes.result(&first.handle, &job),
            Err(ProcessError::UnknownJob)
        ));
        assert_eq!(
            task_leases
                .get_active(&owned_lease.id, &first.handle, now)
                .expect_err("first lease must be removed"),
            TaskLeaseRegistryError::UnknownLease
        );

        lifecycle
            .provision(spec(1000), now)
            .expect("capacity must be reusable after terminal reap");
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }
}
