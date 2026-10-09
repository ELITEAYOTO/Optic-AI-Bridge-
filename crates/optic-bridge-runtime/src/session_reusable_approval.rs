use std::sync::Arc;

use optic_bridge_core::{MonotonicTime, ReusableApprovalGrant, SessionHandle, ToolProfile};
use thiserror::Error;

use crate::{
    ReusableApprovalBroker, ReusableApprovalBrokerError, ReusableApprovalSpec, SessionRegistry,
    SessionRegistryError,
};

/// Runtime boundary that binds reusable approvals to the live application session.
///
/// The caller never supplies a policy epoch. Both issuance and lookup hold a normal
/// session admission permit so concurrent revoke closes the door before reusable
/// approval authority can be created or consumed for another invocation.
pub struct SessionReusableApprovalService {
    sessions: Arc<SessionRegistry>,
    broker: Arc<ReusableApprovalBroker>,
}

impl SessionReusableApprovalService {
    #[must_use]
    pub fn new(
        sessions: Arc<SessionRegistry>,
        broker: Arc<ReusableApprovalBroker>,
    ) -> Self {
        Self { sessions, broker }
    }

    /// Issue one reusable profile approval owned by the active session.
    ///
    /// Expiry may narrow the session lifetime but may never extend past it.
    pub fn issue(
        &self,
        session: &SessionHandle,
        profile: ToolProfile,
        expires_at: MonotonicTime,
        now: MonotonicTime,
    ) -> Result<ReusableApprovalGrant, SessionReusableApprovalError> {
        if expires_at <= now {
            return Err(SessionReusableApprovalError::ExpiryNotFuture);
        }

        let admission = self.sessions.begin_admission(session, now)?;
        let grant = admission.grant();
        if expires_at > grant.expires_at {
            return Err(SessionReusableApprovalError::ExpiryBeyondSession);
        }

        self.broker
            .issue(
                ReusableApprovalSpec {
                    session: session.clone(),
                    profile,
                    expires_at,
                    policy_epoch: grant.policy_epoch,
                },
                now,
            )
            .map_err(SessionReusableApprovalError::Broker)
    }

    /// Resolve a reusable approval only while the owning session is active.
    ///
    /// The active session's current policy epoch is authoritative. A revoked or
    /// expired session fails before broker lookup, so stale stored grants cannot
    /// become effective authority even before physical cleanup runs.
    pub fn find_active_for_profile(
        &self,
        session: &SessionHandle,
        profile: &ToolProfile,
        now: MonotonicTime,
    ) -> Result<Option<ReusableApprovalGrant>, SessionReusableApprovalError> {
        let admission = self.sessions.begin_admission(session, now)?;
        self.broker
            .find_active_for_profile(session, profile, admission.grant().policy_epoch, now)
            .map_err(SessionReusableApprovalError::Broker)
    }

    #[must_use]
    pub fn broker(&self) -> &Arc<ReusableApprovalBroker> {
        &self.broker
    }
}

#[derive(Debug, Error)]
pub enum SessionReusableApprovalError {
    #[error("reusable approval expiry must be later than now")]
    ExpiryNotFuture,
    #[error("reusable approval expiry cannot exceed the owning session expiry")]
    ExpiryBeyondSession,
    #[error(transparent)]
    Session(#[from] SessionRegistryError),
    #[error(transparent)]
    Broker(#[from] ReusableApprovalBrokerError),
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use optic_bridge_core::{
        Capability, NetworkAccess, PrincipalId, ProcessExecutionClass, ProjectId, ResourceBudget,
        SessionGrant, ToolApprovalRequirement, ToolProfileName, ToolProfileSpec, WorkloadClass,
    };

    use super::*;

    fn profile(name: &str) -> ToolProfile {
        ToolProfile::from_spec(ToolProfileSpec {
            name: ToolProfileName::parse(name).expect("profile name"),
            executable: "C:/tools/cargo.exe".to_owned(),
            class: ProcessExecutionClass::FixedTool,
            workload_class: WorkloadClass::Heavy,
            exact_args: vec!["check".to_owned()],
            cwd: None,
            workspace_read_files: BTreeSet::new(),
            env_allowlist: BTreeSet::new(),
            network: NetworkAccess::Denied,
            resource_ceiling: ResourceBudget {
                timeout_ms: 5_000,
                output_bytes: 4_096,
                memory_bytes: 64 * 1024 * 1024,
                process_count: 1,
            },
            approval: ToolApprovalRequirement::HumanRequired,
        })
        .expect("valid profile")
    }

    fn active_session(expires_at: u64, policy_epoch: u64) -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().expect("session"),
            principal: PrincipalId::new("principal").expect("principal"),
            project: ProjectId::new("project").expect("project"),
            capabilities: HashSet::from([Capability::ProcessRun]),
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch,
        }
    }

    fn service_with_session(
        expires_at: u64,
        policy_epoch: u64,
    ) -> (
        SessionReusableApprovalService,
        Arc<SessionRegistry>,
        Arc<ReusableApprovalBroker>,
        SessionGrant,
    ) {
        let sessions = Arc::new(SessionRegistry::new());
        let broker = Arc::new(ReusableApprovalBroker::new());
        let session = active_session(expires_at, policy_epoch);
        sessions.register(session.clone()).expect("register session");
        let service = SessionReusableApprovalService::new(
            Arc::clone(&sessions),
            Arc::clone(&broker),
        );
        (service, sessions, broker, session)
    }

    #[test]
    fn issuance_uses_application_session_epoch_and_bounds_expiry() {
        let (service, _sessions, _broker, session) = service_with_session(100, 7);
        let approved_profile = profile("cargo-check");
        let grant = service
            .issue(
                &session.handle,
                approved_profile.clone(),
                MonotonicTime::from_millis(90),
                MonotonicTime::from_millis(1),
            )
            .expect("issue reusable approval");
        assert_eq!(grant.policy_epoch, 7);
        assert_eq!(grant.expires_at, MonotonicTime::from_millis(90));
        assert!(grant.matches_profile(
            &session.handle,
            &approved_profile,
            7,
            MonotonicTime::from_millis(2)
        ));

        assert!(matches!(
            service.issue(
                &session.handle,
                approved_profile,
                MonotonicTime::from_millis(101),
                MonotonicTime::from_millis(1),
            ),
            Err(SessionReusableApprovalError::ExpiryBeyondSession)
        ));
    }

    #[test]
    fn revoked_session_invalidates_grant_before_physical_broker_cleanup() {
        let (service, sessions, broker, session) = service_with_session(100, 3);
        let approved_profile = profile("cargo-check");
        service
            .issue(
                &session.handle,
                approved_profile.clone(),
                MonotonicTime::from_millis(90),
                MonotonicTime::from_millis(1),
            )
            .expect("issue reusable approval");
        assert!(
            service
                .find_active_for_profile(
                    &session.handle,
                    &approved_profile,
                    MonotonicTime::from_millis(2),
                )
                .expect("active lookup")
                .is_some()
        );

        assert!(sessions.revoke(&session.handle).expect("revoke session"));
        assert!(matches!(
            service.find_active_for_profile(
                &session.handle,
                &approved_profile,
                MonotonicTime::from_millis(3),
            ),
            Err(SessionReusableApprovalError::Session(
                SessionRegistryError::Revoked
            ))
        ));

        // The raw broker still contains the grant until lifecycle cleanup, proving
        // that effective invalidation comes from the active-session boundary rather
        // than relying on timing of physical map cleanup.
        assert!(
            broker
                .find_active_for_profile(
                    &session.handle,
                    &approved_profile,
                    3,
                    MonotonicTime::from_millis(3),
                )
                .expect("raw broker lookup")
                .is_some()
        );
    }

    #[test]
    fn expired_session_cannot_use_stored_reusable_approval() {
        let (service, _sessions, broker, session) = service_with_session(10, 5);
        let approved_profile = profile("cargo-check");
        service
            .issue(
                &session.handle,
                approved_profile.clone(),
                MonotonicTime::from_millis(10),
                MonotonicTime::from_millis(1),
            )
            .expect("issue reusable approval");

        assert!(matches!(
            service.find_active_for_profile(
                &session.handle,
                &approved_profile,
                MonotonicTime::from_millis(10),
            ),
            Err(SessionReusableApprovalError::Session(
                SessionRegistryError::Expired
            ))
        ));
        assert!(
            broker
                .find_active_for_profile(
                    &session.handle,
                    &approved_profile,
                    5,
                    MonotonicTime::from_millis(9),
                )
                .expect("broker still has not-yet-expired grant")
                .is_some()
        );
    }

    #[test]
    fn lookup_remains_exact_to_profile_fingerprint() {
        let (service, _sessions, _broker, session) = service_with_session(100, 9);
        let approved_profile = profile("cargo-check");
        service
            .issue(
                &session.handle,
                approved_profile.clone(),
                MonotonicTime::from_millis(90),
                MonotonicTime::from_millis(1),
            )
            .expect("issue reusable approval");

        let mut changed_spec = approved_profile.spec().clone();
        changed_spec.resource_ceiling.timeout_ms += 1;
        let widened_profile = ToolProfile::from_spec(changed_spec).expect("widened profile");
        assert_eq!(widened_profile.name(), approved_profile.name());
        assert!(
            service
                .find_active_for_profile(
                    &session.handle,
                    &widened_profile,
                    MonotonicTime::from_millis(2),
                )
                .expect("lookup")
                .is_none()
        );
    }
}
