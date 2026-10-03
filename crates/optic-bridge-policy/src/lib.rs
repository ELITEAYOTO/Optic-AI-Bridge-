#![forbid(unsafe_code)]

//! Deterministic authorization for normalized Optic AI Bridge actions.

use std::time::SystemTime;

use optic_bridge_core::{
    ActionEnvelope, ActionKind, Capability, NetworkAccess, SessionGrant, TaskLease,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    RequireApproval(PolicyReason),
    Deny(PolicyReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyReason {
    WrongSession,
    SessionExpired,
    LeaseRequired,
    LeaseExpired,
    LeaseWrongSession,
    StalePolicyEpoch,
    MissingCapability,
    ResourceBudgetExceeded,
    NetworkNotAuthorized,
    SecurityPolicyImmutable,
    PrivilegeElevationDenied,
}

#[derive(Default)]
pub struct PolicyEngine;

impl PolicyEngine {
    #[must_use]
    pub fn evaluate(
        &self,
        envelope: &ActionEnvelope,
        session: &SessionGrant,
        task_lease: Option<&TaskLease>,
        now: SystemTime,
    ) -> PolicyDecision {
        if envelope.session != session.handle {
            return PolicyDecision::Deny(PolicyReason::WrongSession);
        }
        if session.is_expired_at(now) {
            return PolicyDecision::Deny(PolicyReason::SessionExpired);
        }
        if envelope.policy_epoch != session.policy_epoch {
            return PolicyDecision::Deny(PolicyReason::StalePolicyEpoch);
        }

        match envelope.kind {
            ActionKind::PolicyChange => {
                return PolicyDecision::Deny(PolicyReason::SecurityPolicyImmutable);
            }
            ActionKind::PrivilegeElevation => {
                return PolicyDecision::Deny(PolicyReason::PrivilegeElevationDenied);
            }
            _ => {}
        }

        let Some(required) = envelope.required_capability() else {
            return PolicyDecision::Deny(PolicyReason::MissingCapability);
        };

        if !session.allows(required) {
            return PolicyDecision::Deny(PolicyReason::MissingCapability);
        }

        if envelope.kind.requires_task_lease() {
            let Some(lease) = task_lease else {
                return PolicyDecision::Deny(PolicyReason::LeaseRequired);
            };
            if envelope.task_lease.as_ref() != Some(&lease.id) || lease.session != session.handle {
                return PolicyDecision::Deny(PolicyReason::LeaseWrongSession);
            }
            if lease.is_expired_at(now) {
                return PolicyDecision::Deny(PolicyReason::LeaseExpired);
            }
            if lease.policy_epoch != session.policy_epoch {
                return PolicyDecision::Deny(PolicyReason::StalePolicyEpoch);
            }
            if !lease.allows(required) {
                return PolicyDecision::Deny(PolicyReason::MissingCapability);
            }
            if !envelope.resources.fits_within(lease.resource_ceiling) {
                return PolicyDecision::Deny(PolicyReason::ResourceBudgetExceeded);
            }
        }

        if envelope.network == NetworkAccess::Allowed
            && required != Capability::NetworkAccess
            && !session.allows(Capability::NetworkAccess)
        {
            return PolicyDecision::Deny(PolicyReason::NetworkNotAuthorized);
        }

        if envelope.kind == ActionKind::NetworkAccess {
            return PolicyDecision::RequireApproval(PolicyReason::NetworkNotAuthorized);
        }

        PolicyDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        time::{Duration, SystemTime},
    };

    use optic_bridge_core::{
        ActionEnvelope, ActionId, ActionKind, Capability, NetworkAccess, PrincipalId, ProjectId,
        ResourceBudget, Reversibility, SessionGrant, SessionHandle, Target, TaskLease, TaskLeaseId,
        WorkspacePath,
    };

    use super::*;

    fn budget() -> ResourceBudget {
        ResourceBudget {
            timeout_ms: 10_000,
            output_bytes: 64 * 1024,
            memory_bytes: 256 * 1024 * 1024,
            process_count: 4,
        }
    }

    fn session(now: SystemTime, capabilities: &[Capability]) -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().expect("test entropy"),
            principal: PrincipalId::new("test-principal").expect("valid principal"),
            project: ProjectId::new("project-a").expect("valid project"),
            capabilities: capabilities.iter().copied().collect::<BTreeSet<_>>(),
            expires_at: now + Duration::from_secs(300),
            policy_epoch: 7,
        }
    }

    fn envelope(
        session: &SessionGrant,
        kind: ActionKind,
        lease: Option<&TaskLease>,
    ) -> ActionEnvelope {
        ActionEnvelope {
            action_id: ActionId::generate().expect("test entropy"),
            session: session.handle.clone(),
            task_lease: lease.map(|value| value.id.clone()),
            kind,
            target: Target::WorkspacePath(
                WorkspacePath::parse("src/lib.rs").expect("safe test path"),
            ),
            expected_content: None,
            resources: budget(),
            network: NetworkAccess::Denied,
            reversibility: Reversibility::Transactional,
            policy_epoch: session.policy_epoch,
        }
    }

    fn lease(session: &SessionGrant, now: SystemTime, capabilities: &[Capability]) -> TaskLease {
        TaskLease {
            id: TaskLeaseId::generate().expect("test entropy"),
            session: session.handle.clone(),
            capabilities: capabilities.iter().copied().collect::<BTreeSet<_>>(),
            resource_ceiling: budget(),
            expires_at: now + Duration::from_secs(60),
            policy_epoch: session.policy_epoch,
        }
    }

    #[test]
    fn read_can_be_allowed_without_task_lease() {
        let now = SystemTime::now();
        let session = session(now, &[Capability::FileRead]);
        let action = envelope(&session, ActionKind::FileRead, None);

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn mutation_requires_narrow_task_lease() {
        let now = SystemTime::now();
        let session = session(now, &[Capability::FileWrite]);
        let action = envelope(&session, ActionKind::FileWrite, None);

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now),
            PolicyDecision::Deny(PolicyReason::LeaseRequired)
        );
    }

    #[test]
    fn mutation_with_matching_lease_is_allowed() {
        let now = SystemTime::now();
        let session = session(now, &[Capability::FileWrite]);
        let lease = lease(&session, now, &[Capability::FileWrite]);
        let action = envelope(&session, ActionKind::FileWrite, Some(&lease));

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn cross_session_lease_is_denied() {
        let now = SystemTime::now();
        let first = session(now, &[Capability::FileWrite]);
        let second = session(now, &[Capability::FileWrite]);
        let lease = lease(&second, now, &[Capability::FileWrite]);
        let action = envelope(&first, ActionKind::FileWrite, Some(&lease));

        assert_eq!(
            PolicyEngine.evaluate(&action, &first, Some(&lease), now),
            PolicyDecision::Deny(PolicyReason::LeaseWrongSession)
        );
    }

    #[test]
    fn stale_policy_epoch_is_denied() {
        let now = SystemTime::now();
        let session = session(now, &[Capability::FileRead]);
        let mut action = envelope(&session, ActionKind::FileRead, None);
        action.policy_epoch += 1;

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now),
            PolicyDecision::Deny(PolicyReason::StalePolicyEpoch)
        );
    }

    #[test]
    fn policy_change_is_always_denied() {
        let now = SystemTime::now();
        let session = session(now, &[]);
        let action = envelope(&session, ActionKind::PolicyChange, None);

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now),
            PolicyDecision::Deny(PolicyReason::SecurityPolicyImmutable)
        );
    }

    #[test]
    fn lease_budget_cannot_be_exceeded() {
        let now = SystemTime::now();
        let session = session(now, &[Capability::ProcessRun]);
        let mut lease = lease(&session, now, &[Capability::ProcessRun]);
        lease.resource_ceiling.output_bytes = 1024;
        let action = envelope(&session, ActionKind::ProcessRun, Some(&lease));

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now),
            PolicyDecision::Deny(PolicyReason::ResourceBudgetExceeded)
        );
    }
}
