use std::{collections::HashMap, sync::Mutex};

use optic_bridge_core::{
    HardLimits, IdError, LimitError, MonotonicTime, ReusableApprovalGrant, ReusableApprovalId,
    SessionHandle, ToolProfile,
};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct ReusableApprovalSpec {
    pub session: SessionHandle,
    pub profile: ToolProfile,
    pub expires_at: MonotonicTime,
    pub policy_epoch: u64,
}

#[derive(Debug)]
pub struct ReusableApprovalBroker {
    grants: Mutex<HashMap<ReusableApprovalId, ReusableApprovalGrant>>,
    max_grants: u32,
    max_grants_per_session: u32,
}

impl Default for ReusableApprovalBroker {
    fn default() -> Self {
        let limits = HardLimits::default();
        Self {
            grants: Mutex::new(HashMap::new()),
            max_grants: limits.max_approval_grants,
            max_grants_per_session: limits.max_approval_grants_per_session,
        }
    }
}

impl ReusableApprovalBroker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_hard_limits(limits: HardLimits) -> Result<Self, LimitError> {
        let limits = limits.validate_nonzero()?;
        Ok(Self {
            grants: Mutex::new(HashMap::new()),
            max_grants: limits.max_approval_grants,
            max_grants_per_session: limits.max_approval_grants_per_session,
        })
    }

    pub fn issue(
        &self,
        spec: ReusableApprovalSpec,
        now: MonotonicTime,
    ) -> Result<ReusableApprovalGrant, ReusableApprovalBrokerError> {
        if spec.expires_at <= now {
            return Err(ReusableApprovalBrokerError::ExpiredAtIssue);
        }

        let mut grants = self
            .grants
            .lock()
            .map_err(|_| ReusableApprovalBrokerError::StateUnavailable)?;
        grants.retain(|_, grant| !grant.is_expired_at(now));

        let max_grants = usize::try_from(self.max_grants)
            .map_err(|_| ReusableApprovalBrokerError::CapacityExceeded)?;
        if grants.len() >= max_grants {
            return Err(ReusableApprovalBrokerError::CapacityExceeded);
        }
        let owned = grants
            .values()
            .filter(|grant| grant.session == spec.session)
            .count();
        let max_owned = usize::try_from(self.max_grants_per_session)
            .map_err(|_| ReusableApprovalBrokerError::SessionCapacityExceeded)?;
        if owned >= max_owned {
            return Err(ReusableApprovalBrokerError::SessionCapacityExceeded);
        }

        let id = ReusableApprovalId::generate()
            .map_err(ReusableApprovalBrokerError::IdGeneration)?;
        let grant = ReusableApprovalGrant {
            id: id.clone(),
            session: spec.session,
            profile: spec.profile.name().clone(),
            profile_fingerprint: spec.profile.fingerprint(),
            expires_at: spec.expires_at,
            policy_epoch: spec.policy_epoch,
        };
        grants.insert(id, grant.clone());
        Ok(grant)
    }

    /// Returns an active reusable grant only for the exact application session,
    /// immutable ToolProfile fingerprint and current policy epoch.
    ///
    /// Unlike one-shot approvals, a successful lookup does not consume the grant.
    /// Every covered invocation must still pass the normal policy/profile/resource
    /// checks before this broker is consulted.
    pub fn find_active_for_profile(
        &self,
        session: &SessionHandle,
        profile: &ToolProfile,
        policy_epoch: u64,
        now: MonotonicTime,
    ) -> Result<Option<ReusableApprovalGrant>, ReusableApprovalBrokerError> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| ReusableApprovalBrokerError::StateUnavailable)?;
        grants.retain(|_, grant| !grant.is_expired_at(now));
        Ok(grants
            .values()
            .find(|grant| grant.matches_profile(session, profile, policy_epoch, now))
            .cloned())
    }

    pub fn revoke(
        &self,
        id: &ReusableApprovalId,
    ) -> Result<bool, ReusableApprovalBrokerError> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| ReusableApprovalBrokerError::StateUnavailable)?;
        Ok(grants.remove(id).is_some())
    }

    pub fn revoke_session(
        &self,
        session: &SessionHandle,
    ) -> Result<usize, ReusableApprovalBrokerError> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| ReusableApprovalBrokerError::StateUnavailable)?;
        let before = grants.len();
        grants.retain(|_, grant| &grant.session != session);
        Ok(before - grants.len())
    }

    #[cfg(test)]
    fn len(&self) -> Result<usize, ReusableApprovalBrokerError> {
        self.grants
            .lock()
            .map(|grants| grants.len())
            .map_err(|_| ReusableApprovalBrokerError::StateUnavailable)
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ReusableApprovalBrokerError {
    #[error("reusable approval broker state is unavailable")]
    StateUnavailable,
    #[error("reusable approval expiry must be later than issuance")]
    ExpiredAtIssue,
    #[error("reusable approval broker reached its hard global capacity")]
    CapacityExceeded,
    #[error("session reached its hard reusable approval capacity")]
    SessionCapacityExceeded,
    #[error("failed to generate application-owned reusable approval id: {0}")]
    IdGeneration(IdError),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use optic_bridge_core::{
        NetworkAccess, ProcessExecutionClass, ResourceBudget, ToolApprovalRequirement,
        ToolProfileName, ToolProfileSpec, WorkloadClass,
    };

    use super::*;

    fn profile(name: &str, timeout_ms: u64) -> ToolProfile {
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
                timeout_ms,
                output_bytes: 4096,
                memory_bytes: 1024 * 1024,
                process_count: 1,
            },
            approval: ToolApprovalRequirement::HumanRequired,
        })
        .expect("valid profile")
    }

    fn spec(session: SessionHandle, profile: ToolProfile) -> ReusableApprovalSpec {
        ReusableApprovalSpec {
            session,
            profile,
            expires_at: MonotonicTime::from_millis(100),
            policy_epoch: 7,
        }
    }

    #[test]
    fn successful_lookup_is_reusable_and_exact() {
        let broker = ReusableApprovalBroker::new();
        let session = SessionHandle::generate().expect("session");
        let approved_profile = profile("cargo-check", 5_000);
        broker
            .issue(
                spec(session.clone(), approved_profile.clone()),
                MonotonicTime::from_millis(1),
            )
            .expect("issue");

        for now in [2, 3] {
            assert!(
                broker
                    .find_active_for_profile(
                        &session,
                        &approved_profile,
                        7,
                        MonotonicTime::from_millis(now),
                    )
                    .expect("lookup")
                    .is_some()
            );
        }
        assert_eq!(broker.len().expect("length"), 1);

        let widened = profile("cargo-check", 5_001);
        assert!(
            broker
                .find_active_for_profile(
                    &session,
                    &widened,
                    7,
                    MonotonicTime::from_millis(4),
                )
                .expect("lookup")
                .is_none()
        );
        assert!(
            broker
                .find_active_for_profile(
                    &session,
                    &approved_profile,
                    8,
                    MonotonicTime::from_millis(4),
                )
                .expect("lookup")
                .is_none()
        );
    }

    #[test]
    fn expiry_is_reclaimed_and_cannot_match() {
        let limits = HardLimits {
            max_approval_grants: 1,
            max_approval_grants_per_session: 1,
            ..HardLimits::default()
        };
        let broker = ReusableApprovalBroker::from_hard_limits(limits).expect("limits");
        let first_session = SessionHandle::generate().expect("session");
        let mut first = spec(first_session, profile("first", 5_000));
        first.expires_at = MonotonicTime::from_millis(2);
        broker
            .issue(first, MonotonicTime::from_millis(1))
            .expect("first");

        let second_session = SessionHandle::generate().expect("session");
        broker
            .issue(
                spec(second_session, profile("second", 5_000)),
                MonotonicTime::from_millis(2),
            )
            .expect("expired capacity is reclaimed");
        assert_eq!(broker.len().expect("length"), 1);
    }

    #[test]
    fn storage_is_bounded_globally_and_per_session() {
        let limits = HardLimits {
            max_approval_grants: 2,
            max_approval_grants_per_session: 1,
            ..HardLimits::default()
        };
        let broker = ReusableApprovalBroker::from_hard_limits(limits).expect("limits");
        let session_a = SessionHandle::generate().expect("A");
        broker
            .issue(
                spec(session_a.clone(), profile("a1", 5_000)),
                MonotonicTime::from_millis(1),
            )
            .expect("A1");
        assert_eq!(
            broker
                .issue(
                    spec(session_a, profile("a2", 5_000)),
                    MonotonicTime::from_millis(1),
                )
                .expect_err("per-session capacity"),
            ReusableApprovalBrokerError::SessionCapacityExceeded
        );

        let session_b = SessionHandle::generate().expect("B");
        broker
            .issue(
                spec(session_b, profile("b1", 5_000)),
                MonotonicTime::from_millis(1),
            )
            .expect("B1");
        let session_c = SessionHandle::generate().expect("C");
        assert_eq!(
            broker
                .issue(
                    spec(session_c, profile("c1", 5_000)),
                    MonotonicTime::from_millis(1),
                )
                .expect_err("global capacity"),
            ReusableApprovalBrokerError::CapacityExceeded
        );
    }

    #[test]
    fn revoke_is_owner_scoped_and_explicit_revoke_is_idempotent() {
        let broker = ReusableApprovalBroker::new();
        let session_a = SessionHandle::generate().expect("A");
        let session_b = SessionHandle::generate().expect("B");
        let a = broker
            .issue(
                spec(session_a.clone(), profile("a", 5_000)),
                MonotonicTime::from_millis(1),
            )
            .expect("A");
        let b = broker
            .issue(
                spec(session_b.clone(), profile("b", 5_000)),
                MonotonicTime::from_millis(1),
            )
            .expect("B");

        assert_eq!(broker.revoke_session(&session_a).expect("revoke A"), 1);
        assert!(
            broker
                .find_active_for_profile(
                    &session_b,
                    &profile("b", 5_000),
                    7,
                    MonotonicTime::from_millis(2),
                )
                .expect("B lookup")
                .is_some()
        );
        assert!(!broker.revoke(&a.id).expect("A already gone"));
        assert!(broker.revoke(&b.id).expect("revoke B"));
        assert!(!broker.revoke(&b.id).expect("B already gone"));
    }

    #[test]
    fn already_expired_issue_fails_closed() {
        let broker = ReusableApprovalBroker::new();
        let session = SessionHandle::generate().expect("session");
        let mut expired = spec(session, profile("expired", 5_000));
        expired.expires_at = MonotonicTime::from_millis(1);
        assert_eq!(
            broker
                .issue(expired, MonotonicTime::from_millis(1))
                .expect_err("must fail"),
            ReusableApprovalBrokerError::ExpiredAtIssue
        );
    }
}
