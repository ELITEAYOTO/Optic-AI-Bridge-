use std::{collections::BTreeSet, sync::Arc};

use optic_bridge_core::{
    Capability, IdError, MonotonicTime, PrincipalId, ProjectId, SessionGrant, SessionHandle,
};
use thiserror::Error;

use crate::{
    ApprovalBroker, ApprovalBrokerError, ProcessError, ProcessManager, ReusableApprovalBroker,
    ReusableApprovalBrokerError, SessionRegistry, SessionRegistryError,
    SessionReusableApprovalService, SessionWorktreeError, SessionWorktreeManager,
    TaskLeaseRegistry, TaskLeaseRegistryError, session_registry::SessionRenewalError,
    task_lease_registry::TaskLeaseRenewalError,
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
    pub revoked_approvals: usize,
    pub revoked_reusable_approvals: usize,
    pub revoked_leases: usize,
    pub cancellation_requests: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionRenewalReport {
    pub previous_expires_at: MonotonicTime,
    pub expires_at: MonotonicTime,
    pub renewed_leases: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionReapReport {
    pub session_removed: bool,
    pub revoked_approvals: usize,
    pub revoked_reusable_approvals: usize,
    pub revoked_leases: usize,
    pub cancellation_requests: usize,
    pub active_jobs: u32,
    pub removed_process_records: usize,
    pub removed_leases: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuiescentSessionSeal {
    session: SessionHandle,
    revoke: SessionRevokeReport,
}

impl QuiescentSessionSeal {
    #[must_use]
    pub fn session(&self) -> &SessionHandle {
        &self.session
    }

    #[must_use]
    pub fn revoke_report(&self) -> SessionRevokeReport {
        self.revoke
    }
}

pub struct SessionLifecycleManager {
    sessions: Arc<SessionRegistry>,
    task_leases: Arc<TaskLeaseRegistry>,
    processes: Arc<ProcessManager>,
    approvals: Arc<ApprovalBroker>,
    reusable_approvals: Arc<ReusableApprovalBroker>,
    session_worktrees: Option<Arc<SessionWorktreeManager>>,
}

impl SessionLifecycleManager {
    #[must_use]
    pub fn new(
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        processes: Arc<ProcessManager>,
    ) -> Self {
        let approvals = Arc::new(ApprovalBroker::new());
        let reusable_approvals = approvals.reusable_approvals();
        Self::new_with_approval_brokers_and_worktrees(
            sessions,
            task_leases,
            processes,
            approvals,
            reusable_approvals,
            None,
        )
    }

    #[must_use]
    pub fn new_with_approval_broker(
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        processes: Arc<ProcessManager>,
        approvals: Arc<ApprovalBroker>,
    ) -> Self {
        let reusable_approvals = approvals.reusable_approvals();
        Self::new_with_approval_brokers_and_worktrees(
            sessions,
            task_leases,
            processes,
            approvals,
            reusable_approvals,
            None,
        )
    }

    #[must_use]
    pub fn new_with_approval_brokers(
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        processes: Arc<ProcessManager>,
        approvals: Arc<ApprovalBroker>,
        reusable_approvals: Arc<ReusableApprovalBroker>,
    ) -> Self {
        Self::new_with_approval_brokers_and_worktrees(
            sessions,
            task_leases,
            processes,
            approvals,
            reusable_approvals,
            None,
        )
    }

    #[must_use]
    pub fn new_with_approval_broker_and_worktrees(
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        processes: Arc<ProcessManager>,
        approvals: Arc<ApprovalBroker>,
        session_worktrees: Option<Arc<SessionWorktreeManager>>,
    ) -> Self {
        let reusable_approvals = approvals.reusable_approvals();
        Self::new_with_approval_brokers_and_worktrees(
            sessions,
            task_leases,
            processes,
            approvals,
            reusable_approvals,
            session_worktrees,
        )
    }

    #[must_use]
    pub fn new_with_approval_brokers_and_worktrees(
        sessions: Arc<SessionRegistry>,
        task_leases: Arc<TaskLeaseRegistry>,
        processes: Arc<ProcessManager>,
        approvals: Arc<ApprovalBroker>,
        reusable_approvals: Arc<ReusableApprovalBroker>,
        session_worktrees: Option<Arc<SessionWorktreeManager>>,
    ) -> Self {
        Self {
            sessions,
            task_leases,
            processes,
            approvals,
            reusable_approvals,
            session_worktrees,
        }
    }

    #[must_use]
    pub fn reusable_approval_service(&self) -> SessionReusableApprovalService {
        SessionReusableApprovalService::new(
            Arc::clone(&self.sessions),
            Arc::clone(&self.reusable_approvals),
        )
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

    pub fn renew(
        &self,
        session: &SessionHandle,
        now: MonotonicTime,
        new_expires_at: MonotonicTime,
    ) -> Result<SessionRenewalReport, SessionLifecycleError> {
        if new_expires_at <= now {
            return Err(SessionLifecycleError::RenewalExpiryNotFuture);
        }

        // Hold an admission permit across both registry updates. A concurrent revoke may
        // mark the session revoked, but it cannot finish authority cleanup until this
        // renewal attempt leaves the admission critical section.
        let admission = self.sessions.begin_admission(session, now)?;
        let previous_expires_at = admission.grant().expires_at;
        let validated = self
            .sessions
            .validate_renewal(session, now, new_expires_at)
            .map_err(map_session_renewal_error)?;
        debug_assert_eq!(validated.expires_at, previous_expires_at);
        let policy_epoch = admission.grant().policy_epoch;

        // Renew subordinate authority first. If this fails, the session expiry remains
        // untouched. If a concurrent revoke wins before the final session update, longer
        // lease expiries remain bounded by the revoked/old-expiry session and are therefore
        // not effective authority; revoke then owner-scopes their cleanup.
        let renewed_leases = self
            .task_leases
            .renew_active_session(session, now, policy_epoch, new_expires_at)
            .map_err(map_task_lease_renewal_error)?;
        let renewed = self
            .sessions
            .renew_active(session, now, new_expires_at)
            .map_err(map_session_renewal_error)?;

        debug_assert_eq!(renewed.policy_epoch, policy_epoch);
        Ok(SessionRenewalReport {
            previous_expires_at,
            expires_at: renewed.expires_at,
            renewed_leases,
        })
    }

    pub fn revoke(
        &self,
        session: &SessionHandle,
    ) -> Result<SessionRevokeReport, SessionLifecycleError> {
        // Invalidate the session and wait for already-admitted effects to leave their
        // admission critical section before touching leases or process jobs. New
        // admissions fail immediately once the revoked bit is set.
        let session_changed = self.sessions.revoke(session)?;

        // Admission is closed and all already-admitted effects have drained before
        // authority cleanup begins. Attempt every owner-scoped cleanup operation before
        // propagating an error so one registry failure cannot leave other authority live.
        let approval_result = self.approvals.revoke_session(session);
        let reusable_approval_result = self.reusable_approvals.revoke_session(session);
        let lease_result = self.task_leases.revoke_session(session);
        let process_result = self.processes.cancel_session(session);
        let revoked_approvals = approval_result?;
        let revoked_reusable_approvals = reusable_approval_result?;
        let revoked_leases = lease_result?;
        let cancellation_requests = process_result?;

        Ok(SessionRevokeReport {
            session_changed,
            revoked_approvals,
            revoked_reusable_approvals,
            revoked_leases,
            cancellation_requests,
        })
    }

    pub fn seal_quiescent(
        &self,
        session: &SessionHandle,
    ) -> Result<QuiescentSessionSeal, SessionLifecycleError> {
        let revoke = self.revoke(session)?;
        let active_jobs = self.processes.active_session_job_count(session)?;
        if active_jobs != 0 {
            return Err(SessionLifecycleError::SessionJobsStillActive { active_jobs });
        }
        Ok(QuiescentSessionSeal {
            session: session.clone(),
            revoke,
        })
    }

    pub fn reap_inactive(&self, now: MonotonicTime) -> Result<usize, SessionLifecycleError> {
        let handles = self.sessions.inactive_handles(now)?;
        let mut removed_sessions = 0;
        for handle in handles {
            if self.try_reap(&handle, now)?.session_removed {
                removed_sessions += 1;
            }
        }
        Ok(removed_sessions)
    }

    pub fn try_reap(
        &self,
        session: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<SessionReapReport, SessionLifecycleError> {
        match self.sessions.get_active(session, now) {
            Ok(_) => return Err(SessionLifecycleError::SessionStillActive),
            Err(SessionRegistryError::Revoked | SessionRegistryError::Expired) => {}
            Err(SessionRegistryError::UnknownSession) => return Ok(SessionReapReport::default()),
            Err(error) => return Err(error.into()),
        }

        // This is idempotent for an already-revoked session. For an expired session it
        // closes admission first, waits for admitted effects to drain, revokes leases,
        // and requests termination for every owned process job.
        let revoke = self.revoke(session)?;
        let active_jobs = self.processes.active_session_job_count(session)?;
        if active_jobs != 0 {
            return Ok(SessionReapReport {
                session_removed: false,
                revoked_approvals: revoke.revoked_approvals,
                revoked_reusable_approvals: revoke.revoked_reusable_approvals,
                revoked_leases: revoke.revoked_leases,
                cancellation_requests: revoke.cancellation_requests,
                active_jobs,
                removed_process_records: 0,
                removed_leases: 0,
            });
        }

        // Once admission is closed and no job is active, only owner-scoped terminal
        // history remains. Remove process records first, then leases, and finally the
        // session record so capacity is reclaimed only after subordinate state is gone.
        let removed_process_records = self.processes.remove_terminal_session_records(session)?;
        let removed_leases = self.task_leases.remove_session(session)?;
        if let Some(worktrees) = &self.session_worktrees {
            worktrees.release_if_owned(session)?;
        }
        let session_removed = self.sessions.remove_quiescent_inactive(session, now)?;

        Ok(SessionReapReport {
            session_removed,
            revoked_approvals: revoke.revoked_approvals,
            revoked_reusable_approvals: revoke.revoked_reusable_approvals,
            revoked_leases: revoke.revoked_leases,
            cancellation_requests: revoke.cancellation_requests,
            active_jobs: 0,
            removed_process_records,
            removed_leases,
        })
    }
}

#[derive(Debug, Error)]
pub enum SessionLifecycleError {
    #[error("session expiry must be later than the provisioning time")]
    ExpiredAtProvision,
    #[error("active sessions cannot be physically reaped")]
    SessionStillActive,
    #[error("session still owns {active_jobs} active process job(s) after revoke")]
    SessionJobsStillActive { active_jobs: u32 },
    #[error("session renewal expiry must be later than now")]
    RenewalExpiryNotFuture,
    #[error("session renewal must strictly extend the current expiry")]
    RenewalMustExtend,
    #[error("session renewal exceeds the configured future horizon")]
    RenewalBeyondHorizon,
    #[error("session renewal target became unavailable or inactive")]
    RenewalSessionInactive,
    #[error("session renewal lease set is not safely renewable")]
    RenewalLeaseSetInvalid,
    #[error("failed to generate application-owned session handle: {0}")]
    HandleGeneration(IdError),
    #[error(transparent)]
    SessionRegistry(#[from] SessionRegistryError),
    #[error(transparent)]
    ApprovalBroker(#[from] ApprovalBrokerError),
    #[error(transparent)]
    ReusableApprovalBroker(#[from] ReusableApprovalBrokerError),
    #[error(transparent)]
    TaskLeaseRegistry(#[from] TaskLeaseRegistryError),
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error(transparent)]
    SessionWorktree(#[from] SessionWorktreeError),
}

fn map_session_renewal_error(error: SessionRenewalError) -> SessionLifecycleError {
    match error {
        SessionRenewalError::ExpiryNotFuture => SessionLifecycleError::RenewalExpiryNotFuture,
        SessionRenewalError::BeyondHorizon => SessionLifecycleError::RenewalBeyondHorizon,
        SessionRenewalError::MustExtend => SessionLifecycleError::RenewalMustExtend,
        SessionRenewalError::StateUnavailable
        | SessionRenewalError::UnknownSession
        | SessionRenewalError::Revoked
        | SessionRenewalError::Expired => SessionLifecycleError::RenewalSessionInactive,
    }
}

fn map_task_lease_renewal_error(_error: TaskLeaseRenewalError) -> SessionLifecycleError {
    SessionLifecycleError::RenewalLeaseSetInvalid
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        sync::{Arc, mpsc},
        thread,
        time::Duration,
    };

    use optic_bridge_core::{
        ActionEnvelope, ActionId, Effect, HardLimits, JobId, LeaseScope, NetworkAccess,
        ProcessExecutionClass, ResourceBudget, TaskLease, TaskLeaseId,
    };

    use crate::{ApprovalSpec, ProcessResult, ProcessStartSpec, ProcessStatus};

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
            scopes: BTreeSet::from([LeaseScope::ProcessExecutable {
                executable: "fixture".to_owned(),
                class: ProcessExecutionClass::FixedTool,
            }]),
            resource_ceiling: ResourceBudget {
                timeout_ms: 5_000,
                output_bytes: 1024,
                memory_bytes: 64 * 1024 * 1024,
                process_count: 1,
            },
            workload_class: optic_bridge_core::WorkloadClass::Standard,
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
        Arc<ProcessManager>,
    ) {
        let sessions = Arc::new(SessionRegistry::new());
        let task_leases = Arc::new(TaskLeaseRegistry::new());
        let processes = Arc::new(
            ProcessManager::new(root, HardLimits::default(), Vec::new()).expect("process manager"),
        );
        let lifecycle = SessionLifecycleManager::new(
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            Arc::clone(&processes),
        );
        (lifecycle, sessions, task_leases, processes)
    }

    #[test]
    fn quiescent_seal_revokes_session_and_remains_reapable() {
        let root = workspace("quiescent-seal");
        let (lifecycle, sessions, _leases, _processes) = manager(&root);
        let now = MonotonicTime::from_millis(10);
        let grant = lifecycle
            .provision(spec(1_000), now)
            .expect("provision session");

        let seal = lifecycle
            .seal_quiescent(&grant.handle)
            .expect("quiescent seal");
        assert_eq!(seal.session(), &grant.handle);
        assert!(seal.revoke_report().session_changed);
        assert!(matches!(
            sessions.get_active(&grant.handle, now),
            Err(SessionRegistryError::Revoked)
        ));

        let reaped = lifecycle
            .try_reap(&grant.handle, now)
            .expect("reap sealed session");
        assert!(reaped.session_removed);
        fs::remove_dir_all(root).expect("cleanup");
    }

    fn approval_envelope(session: SessionHandle) -> ActionEnvelope {
        ActionEnvelope {
            action_id: ActionId::generate().expect("approval action"),
            session,
            task_lease: None,
            effect: Effect::ProcessRun {
                executable: "fixture".to_owned(),
                class: ProcessExecutionClass::FixedTool,
                network: NetworkAccess::Denied,
            },
            resources: ResourceBudget {
                timeout_ms: 1_000,
                output_bytes: 1_024,
                memory_bytes: 1_024,
                process_count: 1,
            },
            policy_epoch: 1,
        }
    }

    fn approval_spec(envelope: &ActionEnvelope, expires_at: u64) -> ApprovalSpec {
        ApprovalSpec {
            session: envelope.session.clone(),
            action_id: envelope.action_id.clone(),
            effect: envelope.effect.clone(),
            resources: envelope.resources,
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: envelope.policy_epoch,
        }
    }

    fn process_spec(session: SessionHandle) -> ProcessStartSpec {
        let executable = env::current_exe()
            .expect("current test executable")
            .canonicalize()
            .expect("canonical test executable")
            .to_string_lossy()
            .into_owned();
        ProcessStartSpec {
            session,
            class: ProcessExecutionClass::FixedTool,
            workload_class: optic_bridge_core::WorkloadClass::Standard,
            network: optic_bridge_core::NetworkAccess::Allowed,
            executable,
            args: vec![
                "--exact".to_owned(),
                "session_lifecycle::tests::process_fixture_child".to_owned(),
                "--nocapture".to_owned(),
            ],
            cwd: None,
            workspace_read_files: Vec::new(),
            env_allowlist: Vec::new(),
            resources: ResourceBudget {
                timeout_ms: 5_000,
                output_bytes: 1024,
                memory_bytes: 64 * 1024 * 1024,
                process_count: 1,
            },
        }
    }

    async fn await_terminal(
        processes: &ProcessManager,
        session: &SessionHandle,
        job: &JobId,
    ) -> ProcessResult {
        for _ in 0..300 {
            let result = processes.result(session, job).expect("process result");
            if result.status != ProcessStatus::Running {
                return result;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("process did not become terminal");
    }

    #[test]
    fn process_fixture_child() {
        let cwd = env::current_dir().expect("fixture cwd");
        if cwd.join("fixture-sleep").exists() {
            thread::sleep(Duration::from_secs(2));
        }
    }

    #[test]
    fn adversarial_process_fixture_child() {
        let cwd = env::current_dir().expect("fixture cwd");
        if cwd.join("fixture-adversarial-sleep").exists() {
            thread::sleep(Duration::from_secs(10));
        }
    }

    fn adversarial_process_spec(session: SessionHandle) -> ProcessStartSpec {
        let mut spec = process_spec(session);
        spec.args[1] = "session_lifecycle::tests::adversarial_process_fixture_child".to_owned();
        spec.resources.timeout_ms = 15_000;
        spec
    }

    #[test]
    fn provision_generates_and_registers_application_owned_session() {
        let root = workspace("provision");
        let (lifecycle, sessions, _, _) = manager(&root);
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
        let (lifecycle, sessions, _, _) = manager(&root);
        assert!(matches!(
            lifecycle.provision(spec(10), MonotonicTime::from_millis(10)),
            Err(SessionLifecycleError::ExpiredAtProvision)
        ));
        assert_eq!(sessions.len().expect("registry length"), 0);
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn renewal_extends_session_and_active_leases_without_mutating_authority() {
        let root = workspace("renew");
        let (lifecycle, sessions, task_leases, _) = manager(&root);
        let now = MonotonicTime::from_millis(10);
        let session = lifecycle.provision(spec(100), now).expect("session");
        let owned = lease(session.handle.clone(), 100);
        task_leases.register(owned.clone()).expect("lease");

        let report = lifecycle
            .renew(&session.handle, now, MonotonicTime::from_millis(200))
            .expect("renew lifecycle");
        assert_eq!(report.previous_expires_at, MonotonicTime::from_millis(100));
        assert_eq!(report.expires_at, MonotonicTime::from_millis(200));
        assert_eq!(report.renewed_leases, 1);

        let renewed_session = sessions
            .get_active(&session.handle, MonotonicTime::from_millis(150))
            .expect("renewed session");
        assert_eq!(renewed_session.expires_at, MonotonicTime::from_millis(200));
        assert_eq!(renewed_session.capabilities, session.capabilities);
        assert_eq!(renewed_session.policy_epoch, session.policy_epoch);
        let renewed_lease = task_leases
            .get_active(&owned.id, &session.handle, MonotonicTime::from_millis(150))
            .expect("renewed lease");
        assert_eq!(renewed_lease.expires_at, MonotonicTime::from_millis(200));
        assert_eq!(renewed_lease.capabilities, owned.capabilities);
        assert_eq!(renewed_lease.scopes, owned.scopes);
        assert_eq!(renewed_lease.resource_ceiling, owned.resource_ceiling);
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn renewal_beyond_horizon_is_rejected_before_any_lease_change() {
        let root = workspace("renew-horizon");
        let limits = HardLimits {
            max_session_renewal_horizon_ms: 50,
            ..HardLimits::default()
        };
        let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("limits"));
        let task_leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
        let processes =
            Arc::new(ProcessManager::new(&root, limits, Vec::new()).expect("process manager"));
        let lifecycle = SessionLifecycleManager::new(
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            processes,
        );
        let now = MonotonicTime::from_millis(10);
        let mut session_spec = spec(50);
        session_spec.expires_at = MonotonicTime::from_millis(50);
        let session = lifecycle.provision(session_spec, now).expect("session");
        let owned = lease(session.handle.clone(), 50);
        task_leases.register(owned.clone()).expect("lease");

        assert!(matches!(
            lifecycle.renew(&session.handle, now, MonotonicTime::from_millis(61),),
            Err(SessionLifecycleError::RenewalBeyondHorizon)
        ));
        let unchanged = task_leases
            .get_active(&owned.id, &session.handle, MonotonicTime::from_millis(20))
            .expect("lease unchanged");
        assert_eq!(unchanged.expires_at, MonotonicTime::from_millis(50));
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn renewal_failure_in_lease_set_leaves_session_expiry_unchanged() {
        let root = workspace("renew-lease-fail");
        let (lifecycle, sessions, task_leases, _) = manager(&root);
        let now = MonotonicTime::from_millis(10);
        let session = lifecycle.provision(spec(100), now).expect("session");
        let expired = lease(session.handle.clone(), 10);
        task_leases.register(expired).expect("expired lease record");

        assert!(matches!(
            lifecycle.renew(&session.handle, now, MonotonicTime::from_millis(200),),
            Err(SessionLifecycleError::RenewalLeaseSetInvalid)
        ));
        let unchanged = sessions
            .get_active(&session.handle, MonotonicTime::from_millis(50))
            .expect("session remains active at original horizon");
        assert_eq!(unchanged.expires_at, MonotonicTime::from_millis(100));
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn revoked_or_expired_session_cannot_be_renewed() {
        let root = workspace("renew-inactive");
        let (lifecycle, _, _, _) = manager(&root);
        let active = lifecycle
            .provision(spec(100), MonotonicTime::from_millis(1))
            .expect("active session");
        lifecycle.revoke(&active.handle).expect("revoke session");
        assert!(matches!(
            lifecycle.renew(
                &active.handle,
                MonotonicTime::from_millis(10),
                MonotonicTime::from_millis(200),
            ),
            Err(SessionLifecycleError::SessionRegistry(
                SessionRegistryError::Revoked
            ))
        ));

        let expired = lifecycle
            .provision(spec(10), MonotonicTime::from_millis(1))
            .expect("expiring session");
        assert!(matches!(
            lifecycle.renew(
                &expired.handle,
                MonotonicTime::from_millis(10),
                MonotonicTime::from_millis(200),
            ),
            Err(SessionLifecycleError::SessionRegistry(
                SessionRegistryError::Expired
            ))
        ));
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[tokio::test]
    async fn admitted_job_is_visible_to_revoke_then_reaped_without_cross_session_damage() {
        let root = workspace("admission-race");
        fs::write(root.join("fixture-sleep"), b"1").expect("write sleep marker");
        let (lifecycle, sessions, task_leases, processes) = manager(&root);
        let now = MonotonicTime::from_millis(1);
        let session_a = lifecycle.provision(spec(1_000), now).expect("session A");
        let session_b = lifecycle.provision(spec(1_000), now).expect("session B");
        let lease_a = lease(session_a.handle.clone(), 1_000);
        let lease_b = lease(session_b.handle.clone(), 1_000);
        task_leases.register(lease_a.clone()).expect("lease A");
        task_leases.register(lease_b.clone()).expect("lease B");

        let admission = sessions
            .begin_admission(&session_a.handle, now)
            .expect("admit A effect");
        let lifecycle = Arc::new(lifecycle);
        let revoke_lifecycle = Arc::clone(&lifecycle);
        let revoke_handle = session_a.handle.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let revoke_thread = thread::spawn(move || {
            let result = revoke_lifecycle.revoke(&revoke_handle);
            done_tx.send(result).expect("send revoke result");
        });

        let mut revoke_started = false;
        for _ in 0..100 {
            if matches!(
                sessions.get_active(&session_a.handle, now),
                Err(SessionRegistryError::Revoked)
            ) {
                revoke_started = true;
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(revoke_started, "revoke must close new admissions first");
        assert!(matches!(
            sessions.begin_admission(&session_a.handle, now),
            Err(SessionRegistryError::Revoked)
        ));
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));

        // Simulate an effect that was admitted immediately before revoke. The job is
        // inserted while revoke is blocked on this permit, so cancel_session must see it
        // after the permit is released.
        let job = processes
            .start(process_spec(session_a.handle.clone()))
            .expect("start pre-revoke admitted job");
        drop(admission);

        let report = done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("revoke must finish after admission drains")
            .expect("revoke result");
        revoke_thread.join().expect("revoke thread");
        assert!(report.session_changed);
        assert_eq!(report.revoked_leases, 1);
        assert_eq!(report.cancellation_requests, 1);

        let result = await_terminal(&processes, &session_a.handle, &job).await;
        assert_eq!(result.status, ProcessStatus::Stopped);

        let reap = lifecycle
            .try_reap(&session_a.handle, now)
            .expect("reap quiescent A");
        assert!(reap.session_removed);
        assert_eq!(reap.active_jobs, 0);
        assert_eq!(reap.removed_process_records, 1);
        assert_eq!(reap.removed_leases, 1);
        assert!(matches!(
            processes.result(&session_a.handle, &job),
            Err(ProcessError::UnknownJob)
        ));
        assert_eq!(
            sessions
                .get_active(&session_a.handle, now)
                .expect_err("A must be physically removed"),
            SessionRegistryError::UnknownSession
        );

        // B is independent and survives A's revoke/reap unchanged.
        sessions
            .get_active(&session_b.handle, now)
            .expect("B session must remain");
        task_leases
            .get_active(&lease_b.id, &session_b.handle, now)
            .expect("B lease must remain");

        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[tokio::test]
    async fn adversarial_multisession_pressure_revoke_and_reap_preserve_foreign_state() {
        let root = workspace("adversarial-multisession");
        fs::write(root.join("fixture-adversarial-sleep"), b"1")
            .expect("write adversarial sleep marker");

        let limits = HardLimits {
            max_sessions: 2,
            max_task_leases: 2,
            max_task_leases_per_session: 1,
            max_active_process_jobs: 3,
            max_active_process_jobs_per_session: 1,
            ..HardLimits::default()
        };
        let sessions =
            Arc::new(SessionRegistry::from_hard_limits(limits).expect("session registry"));
        let task_leases =
            Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("lease registry"));
        let processes =
            Arc::new(ProcessManager::new(&root, limits, Vec::new()).expect("process manager"));
        let lifecycle = SessionLifecycleManager::new(
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            Arc::clone(&processes),
        );

        let now = MonotonicTime::from_millis(1);
        let session_a = lifecycle.provision(spec(1_000), now).expect("session A");
        let session_b = lifecycle.provision(spec(1_000), now).expect("session B");
        let lease_a = lease(session_a.handle.clone(), 1_000);
        let lease_b = lease(session_b.handle.clone(), 1_000);
        task_leases.register(lease_a.clone()).expect("lease A");
        task_leases.register(lease_b.clone()).expect("lease B");
        assert_eq!(
            task_leases
                .get_active(&lease_b.id, &session_a.handle, now)
                .expect_err("A must not resolve B lease"),
            TaskLeaseRegistryError::WrongSession
        );

        let job_a = processes
            .start(adversarial_process_spec(session_a.handle.clone()))
            .expect("start A job");
        let job_b = processes
            .start(adversarial_process_spec(session_b.handle.clone()))
            .expect("start B job");

        assert!(matches!(
            processes.result(&session_a.handle, &job_b),
            Err(ProcessError::UnknownJob)
        ));
        assert!(matches!(
            processes.stop(&session_a.handle, &job_b),
            Err(ProcessError::UnknownJob)
        ));
        assert!(matches!(
            processes.result(&session_b.handle, &job_a),
            Err(ProcessError::UnknownJob)
        ));

        assert!(matches!(
            processes.start(adversarial_process_spec(session_a.handle.clone())),
            Err(ProcessError::TooManyActiveJobsForSession)
        ));
        assert_eq!(
            processes
                .result(&session_b.handle, &job_b)
                .expect("B result remains visible to B")
                .status,
            ProcessStatus::Running
        );

        assert!(matches!(
            lifecycle.provision(spec(1_000), now),
            Err(SessionLifecycleError::SessionRegistry(
                SessionRegistryError::CapacityExceeded
            ))
        ));

        let revoke = lifecycle.revoke(&session_a.handle).expect("revoke A");
        assert!(revoke.session_changed);
        assert_eq!(revoke.revoked_leases, 1);
        assert_eq!(revoke.cancellation_requests, 1);
        assert_eq!(
            await_terminal(&processes, &session_a.handle, &job_a)
                .await
                .status,
            ProcessStatus::Stopped
        );

        sessions
            .get_active(&session_b.handle, now)
            .expect("B session survives A revoke");
        task_leases
            .get_active(&lease_b.id, &session_b.handle, now)
            .expect("B lease survives A revoke");
        assert_eq!(
            processes
                .result(&session_b.handle, &job_b)
                .expect("B job survives A revoke")
                .status,
            ProcessStatus::Running
        );

        let reap = lifecycle.try_reap(&session_a.handle, now).expect("reap A");
        assert!(reap.session_removed);
        assert_eq!(reap.removed_process_records, 1);
        assert_eq!(reap.removed_leases, 1);
        assert!(matches!(
            task_leases.get_active(&lease_a.id, &session_a.handle, now),
            Err(TaskLeaseRegistryError::UnknownLease)
        ));
        assert!(matches!(
            processes.result(&session_a.handle, &job_a),
            Err(ProcessError::UnknownJob)
        ));

        let session_c = lifecycle
            .provision(spec(1_000), now)
            .expect("C reuses reaped session capacity");
        let lease_c = lease(session_c.handle.clone(), 1_000);
        task_leases
            .register(lease_c)
            .expect("C reuses reaped lease capacity");
        let job_c = processes
            .start(adversarial_process_spec(session_c.handle.clone()))
            .expect("C starts while B is still active");

        sessions
            .get_active(&session_b.handle, now)
            .expect("B session remains active after C admission");
        task_leases
            .get_active(&lease_b.id, &session_b.handle, now)
            .expect("B lease remains active after C admission");
        assert_eq!(
            processes
                .result(&session_b.handle, &job_b)
                .expect("B job remains visible after C admission")
                .status,
            ProcessStatus::Running
        );

        assert!(processes.stop(&session_c.handle, &job_c).expect("stop C"));
        assert!(processes.stop(&session_b.handle, &job_b).expect("stop B"));
        assert_eq!(
            await_terminal(&processes, &session_c.handle, &job_c)
                .await
                .status,
            ProcessStatus::Stopped
        );
        assert_eq!(
            await_terminal(&processes, &session_b.handle, &job_b)
                .await
                .status,
            ProcessStatus::Stopped
        );

        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn reap_reclaims_session_capacity_only_after_inactivation() {
        let root = workspace("capacity-reclaim");
        let limits = HardLimits {
            max_sessions: 1,
            ..HardLimits::default()
        };
        let sessions =
            Arc::new(SessionRegistry::from_hard_limits(limits).expect("session registry"));
        let task_leases = Arc::new(TaskLeaseRegistry::new());
        let processes =
            Arc::new(ProcessManager::new(&root, limits, Vec::new()).expect("process manager"));
        let lifecycle = SessionLifecycleManager::new(
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            Arc::clone(&processes),
        );
        let now = MonotonicTime::from_millis(1);
        let first = lifecycle
            .provision(spec(1_000), now)
            .expect("first session");
        assert!(matches!(
            lifecycle.provision(spec(1_000), now),
            Err(SessionLifecycleError::SessionRegistry(
                SessionRegistryError::CapacityExceeded
            ))
        ));
        assert!(matches!(
            lifecycle.try_reap(&first.handle, now),
            Err(SessionLifecycleError::SessionStillActive)
        ));

        lifecycle.revoke(&first.handle).expect("revoke first");
        let reap = lifecycle.try_reap(&first.handle, now).expect("reap first");
        assert!(reap.session_removed);
        assert_eq!(sessions.len().expect("registry length"), 0);
        lifecycle
            .provision(spec(1_000), now)
            .expect("capacity must be reusable after safe reap");
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn reap_inactive_cleans_expired_owner_state_without_touching_active_session() {
        let root = workspace("expired-sweep");
        let (lifecycle, sessions, task_leases, _) = manager(&root);
        let start = MonotonicTime::from_millis(1);
        let session_a = lifecycle.provision(spec(10), start).expect("session A");
        let session_b = lifecycle.provision(spec(1_000), start).expect("session B");
        let lease_a = lease(session_a.handle.clone(), 10);
        let lease_b = lease(session_b.handle.clone(), 1_000);
        task_leases.register(lease_a.clone()).expect("lease A");
        task_leases.register(lease_b.clone()).expect("lease B");

        let now = MonotonicTime::from_millis(10);
        assert_eq!(lifecycle.reap_inactive(now).expect("reap expired"), 1);
        assert_eq!(
            sessions
                .get_active(&session_a.handle, now)
                .expect_err("A must be removed"),
            SessionRegistryError::UnknownSession
        );
        assert_eq!(
            task_leases
                .get_active(&lease_a.id, &session_a.handle, start)
                .expect_err("A lease must be removed"),
            TaskLeaseRegistryError::UnknownLease
        );

        sessions
            .get_active(&session_b.handle, now)
            .expect("B session must remain active");
        task_leases
            .get_active(&lease_b.id, &session_b.handle, now)
            .expect("B lease must remain active");
        assert_eq!(
            lifecycle
                .reap_inactive(now)
                .expect("second sweep is idempotent"),
            0
        );
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn reap_reclaims_task_lease_capacity_without_touching_other_session() {
        let root = workspace("lease-capacity-reclaim");
        let limits = HardLimits {
            max_sessions: 2,
            max_task_leases: 1,
            max_task_leases_per_session: 1,
            ..HardLimits::default()
        };
        let sessions =
            Arc::new(SessionRegistry::from_hard_limits(limits).expect("session registry"));
        let task_leases =
            Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("lease registry"));
        let processes =
            Arc::new(ProcessManager::new(&root, limits, Vec::new()).expect("process manager"));
        let lifecycle = SessionLifecycleManager::new(
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            Arc::clone(&processes),
        );
        let now = MonotonicTime::from_millis(1);
        let session_a = lifecycle.provision(spec(1_000), now).expect("session A");
        let session_b = lifecycle.provision(spec(1_000), now).expect("session B");
        let lease_a = lease(session_a.handle.clone(), 1_000);
        task_leases.register(lease_a).expect("A lease");

        lifecycle.revoke(&session_a.handle).expect("revoke A");
        assert_eq!(
            task_leases
                .register(lease(session_b.handle.clone(), 1_000))
                .expect_err("revoked A must still hold storage before reap"),
            TaskLeaseRegistryError::CapacityExceeded
        );
        sessions
            .get_active(&session_b.handle, now)
            .expect("B session remains active");

        let reap = lifecycle.try_reap(&session_a.handle, now).expect("reap A");
        assert!(reap.session_removed);
        assert_eq!(reap.removed_leases, 1);
        let lease_b = lease(session_b.handle.clone(), 1_000);
        task_leases
            .register(lease_b.clone())
            .expect("B can reuse capacity after A reap");
        task_leases
            .get_active(&lease_b.id, &session_b.handle, now)
            .expect("B lease remains active");
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn revoke_is_fail_closed_and_owner_scoped() {
        let root = workspace("revoke");
        let (lifecycle, sessions, task_leases, _) = manager(&root);
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
    fn revoke_removes_only_owned_approval_grants() {
        let root = workspace("approval-owner-scope");
        let limits = HardLimits::default();
        let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
        let task_leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
        let processes =
            Arc::new(ProcessManager::new(&root, limits, Vec::new()).expect("processes"));
        let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits).expect("approvals"));
        let lifecycle = SessionLifecycleManager::new_with_approval_broker(
            Arc::clone(&sessions),
            task_leases,
            processes,
            Arc::clone(&approvals),
        );
        let now = MonotonicTime::from_millis(1);
        let session_a = lifecycle.provision(spec(100), now).expect("session A");
        let session_b = lifecycle.provision(spec(100), now).expect("session B");
        let action_a = approval_envelope(session_a.handle.clone());
        let action_b = approval_envelope(session_b.handle.clone());
        let approval_a = approvals
            .issue(approval_spec(&action_a, 100), now)
            .expect("approval A");
        let approval_b = approvals
            .issue(approval_spec(&action_b, 100), now)
            .expect("approval B");

        let report = lifecycle.revoke(&session_a.handle).expect("revoke A");
        assert_eq!(report.revoked_approvals, 1);
        assert_eq!(
            approvals
                .consume_exact(&approval_a.id, &action_a, now)
                .expect_err("A approval must be removed"),
            ApprovalBrokerError::UnknownApproval
        );
        approvals
            .consume_exact(&approval_b.id, &action_b, now)
            .expect("B approval must remain usable");
        sessions
            .get_active(&session_b.handle, now)
            .expect("B session remains active");
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }

    #[test]
    fn revoke_waits_for_admission_before_approval_cleanup() {
        let root = workspace("approval-admission-race");
        let limits = HardLimits {
            max_approval_grants: 2,
            max_approval_grants_per_session: 1,
            ..HardLimits::default()
        };
        let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
        let task_leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
        let processes =
            Arc::new(ProcessManager::new(&root, limits, Vec::new()).expect("processes"));
        let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits).expect("approvals"));
        let lifecycle = Arc::new(SessionLifecycleManager::new_with_approval_broker(
            Arc::clone(&sessions),
            task_leases,
            processes,
            Arc::clone(&approvals),
        ));
        let now = MonotonicTime::from_millis(1);
        let session = lifecycle.provision(spec(100), now).expect("session");
        let action = approval_envelope(session.handle.clone());
        let approval = approvals
            .issue(approval_spec(&action, 100), now)
            .expect("approval");
        let admission = sessions
            .begin_admission(&session.handle, now)
            .expect("admitted effect");

        let revoke_lifecycle = Arc::clone(&lifecycle);
        let revoke_session = session.handle.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let revoke_thread = thread::spawn(move || {
            done_tx
                .send(revoke_lifecycle.revoke(&revoke_session))
                .expect("send revoke result");
        });

        let mut revoke_started = false;
        for _ in 0..100 {
            if matches!(
                sessions.get_active(&session.handle, now),
                Err(SessionRegistryError::Revoked)
            ) {
                revoke_started = true;
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(revoke_started, "revoke must close admission first");
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));

        let second = approval_envelope(session.handle.clone());
        assert_eq!(
            approvals
                .issue(approval_spec(&second, 100), now)
                .expect_err("existing approval must stay live while admitted effect drains"),
            ApprovalBrokerError::SessionCapacityExceeded
        );

        drop(admission);
        let report = done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("revoke completes after admission drains")
            .expect("revoke result");
        revoke_thread.join().expect("revoke thread");
        assert_eq!(report.revoked_approvals, 1);
        assert_eq!(
            approvals
                .consume_exact(&approval.id, &action, now)
                .expect_err("approval must be removed after admission drains"),
            ApprovalBrokerError::UnknownApproval
        );
        fs::remove_dir_all(root).expect("remove lifecycle workspace");
    }
}
