use std::{collections::HashMap, sync::Mutex};

use optic_bridge_core::{
    ActionEnvelope, ActionId, ApprovalGrant, ApprovalId, Effect, HardLimits, IdError, LimitError,
    MonotonicTime, ResourceBudget, SessionHandle,
};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct ApprovalSpec {
    pub session: SessionHandle,
    pub action_id: ActionId,
    pub effect: Effect,
    pub resources: ResourceBudget,
    pub expires_at: MonotonicTime,
    pub policy_epoch: u64,
}

#[derive(Debug)]
pub struct ApprovalBroker {
    grants: Mutex<HashMap<ApprovalId, ApprovalGrant>>,
    max_grants: u32,
    max_grants_per_session: u32,
}

impl Default for ApprovalBroker {
    fn default() -> Self {
        let limits = HardLimits::default();
        Self {
            grants: Mutex::new(HashMap::new()),
            max_grants: limits.max_approval_grants,
            max_grants_per_session: limits.max_approval_grants_per_session,
        }
    }
}

impl ApprovalBroker {
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
        spec: ApprovalSpec,
        now: MonotonicTime,
    ) -> Result<ApprovalGrant, ApprovalBrokerError> {
        if spec.expires_at <= now {
            return Err(ApprovalBrokerError::ExpiredAtIssue);
        }

        let mut grants = self
            .grants
            .lock()
            .map_err(|_| ApprovalBrokerError::StateUnavailable)?;
        grants.retain(|_, grant| !grant.is_expired_at(now));
        let max_grants =
            usize::try_from(self.max_grants).map_err(|_| ApprovalBrokerError::CapacityExceeded)?;
        if grants.len() >= max_grants {
            return Err(ApprovalBrokerError::CapacityExceeded);
        }
        let owned = grants
            .values()
            .filter(|grant| grant.session == spec.session)
            .count();
        let max_owned = usize::try_from(self.max_grants_per_session)
            .map_err(|_| ApprovalBrokerError::SessionCapacityExceeded)?;
        if owned >= max_owned {
            return Err(ApprovalBrokerError::SessionCapacityExceeded);
        }

        let id = ApprovalId::generate().map_err(ApprovalBrokerError::IdGeneration)?;
        let grant = ApprovalGrant {
            id: id.clone(),
            session: spec.session,
            action_id: spec.action_id,
            effect: spec.effect,
            resources: spec.resources,
            expires_at: spec.expires_at,
            policy_epoch: spec.policy_epoch,
        };
        grants.insert(id, grant.clone());
        Ok(grant)
    }

    /// Atomically validates and consumes one exact approval.
    ///
    /// Mismatches do not consume the grant; successful use always does. This keeps
    /// authority one-shot without allowing unrelated malformed requests to burn it.
    pub fn consume_exact(
        &self,
        id: &ApprovalId,
        envelope: &ActionEnvelope,
        now: MonotonicTime,
    ) -> Result<ApprovalGrant, ApprovalBrokerError> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| ApprovalBrokerError::StateUnavailable)?;
        if grants.get(id).is_some_and(|grant| grant.is_expired_at(now)) {
            grants.remove(id);
            return Err(ApprovalBrokerError::Expired);
        }
        let grant = grants.get(id).ok_or(ApprovalBrokerError::UnknownApproval)?;
        validate_exact(grant, envelope, now)?;
        grants
            .remove(id)
            .ok_or(ApprovalBrokerError::UnknownApproval)
    }

    pub fn revoke_session(&self, session: &SessionHandle) -> Result<usize, ApprovalBrokerError> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| ApprovalBrokerError::StateUnavailable)?;
        let before = grants.len();
        grants.retain(|_, grant| &grant.session != session);
        Ok(before - grants.len())
    }

    #[cfg(test)]
    fn len(&self) -> Result<usize, ApprovalBrokerError> {
        self.grants
            .lock()
            .map(|grants| grants.len())
            .map_err(|_| ApprovalBrokerError::StateUnavailable)
    }
}

fn validate_exact(
    grant: &ApprovalGrant,
    envelope: &ActionEnvelope,
    now: MonotonicTime,
) -> Result<(), ApprovalBrokerError> {
    if grant.is_expired_at(now) {
        return Err(ApprovalBrokerError::Expired);
    }
    if grant.session != envelope.session {
        return Err(ApprovalBrokerError::WrongSession);
    }
    if grant.action_id != envelope.action_id {
        return Err(ApprovalBrokerError::WrongAction);
    }
    if grant.effect != envelope.effect {
        return Err(ApprovalBrokerError::EffectMismatch);
    }
    if grant.resources != envelope.resources {
        return Err(ApprovalBrokerError::ResourceMismatch);
    }
    if grant.policy_epoch != envelope.policy_epoch {
        return Err(ApprovalBrokerError::StalePolicyEpoch);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalBrokerError {
    #[error("approval broker state is unavailable")]
    StateUnavailable,
    #[error("approval expiry must be later than issuance")]
    ExpiredAtIssue,
    #[error("approval broker reached its hard global capacity")]
    CapacityExceeded,
    #[error("session reached its hard approval capacity")]
    SessionCapacityExceeded,
    #[error("approval is unknown or already consumed")]
    UnknownApproval,
    #[error("approval belongs to another session")]
    WrongSession,
    #[error("approval belongs to another action")]
    WrongAction,
    #[error("approval effect does not exactly match the action")]
    EffectMismatch,
    #[error("approval resource budget does not exactly match the action")]
    ResourceMismatch,
    #[error("approval policy epoch is stale")]
    StalePolicyEpoch,
    #[error("approval has expired")]
    Expired,
    #[error("failed to generate application-owned approval id: {0}")]
    IdGeneration(IdError),
}

#[cfg(test)]
mod tests {
    use optic_bridge_core::{NetworkAccess, ProcessExecutionClass};

    use super::*;

    fn envelope(action_id: ActionId, session: SessionHandle) -> ActionEnvelope {
        ActionEnvelope {
            action_id,
            session,
            task_lease: None,
            effect: Effect::ProcessRun {
                executable: "tool".to_owned(),
                class: ProcessExecutionClass::FixedTool,
                network: NetworkAccess::Denied,
            },
            resources: ResourceBudget {
                timeout_ms: 1_000,
                output_bytes: 1_024,
                memory_bytes: 1_024,
                process_count: 1,
            },
            policy_epoch: 3,
        }
    }

    fn spec(envelope: &ActionEnvelope, expires_at: u64) -> ApprovalSpec {
        ApprovalSpec {
            session: envelope.session.clone(),
            action_id: envelope.action_id.clone(),
            effect: envelope.effect.clone(),
            resources: envelope.resources,
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: envelope.policy_epoch,
        }
    }

    #[test]
    fn successful_consume_is_exact_and_one_shot() {
        let broker = ApprovalBroker::new();
        let envelope = envelope(
            ActionId::generate().expect("action"),
            SessionHandle::generate().expect("session"),
        );
        let grant = broker
            .issue(spec(&envelope, 100), MonotonicTime::from_millis(1))
            .expect("issue approval");
        let consumed = broker
            .consume_exact(&grant.id, &envelope, MonotonicTime::from_millis(2))
            .expect("consume approval");
        assert_eq!(consumed.id, grant.id);
        assert_eq!(broker.len().expect("length"), 0);
        assert_eq!(
            broker
                .consume_exact(&grant.id, &envelope, MonotonicTime::from_millis(3))
                .expect_err("reuse must fail"),
            ApprovalBrokerError::UnknownApproval
        );
    }

    #[test]
    fn mismatch_does_not_burn_unrelated_exact_grant() {
        let broker = ApprovalBroker::new();
        let envelope = envelope(
            ActionId::generate().expect("action"),
            SessionHandle::generate().expect("session"),
        );
        let grant = broker
            .issue(spec(&envelope, 100), MonotonicTime::from_millis(1))
            .expect("issue approval");
        let mut wrong = envelope.clone();
        wrong.action_id = ActionId::generate().expect("wrong action");
        assert_eq!(
            broker
                .consume_exact(&grant.id, &wrong, MonotonicTime::from_millis(2))
                .expect_err("wrong action must fail"),
            ApprovalBrokerError::WrongAction
        );
        broker
            .consume_exact(&grant.id, &envelope, MonotonicTime::from_millis(2))
            .expect("exact action can still consume");
    }

    #[test]
    fn issue_and_storage_are_bounded_globally_and_per_session() {
        let limits = HardLimits {
            max_approval_grants: 2,
            max_approval_grants_per_session: 1,
            ..HardLimits::default()
        };
        let broker = ApprovalBroker::from_hard_limits(limits).expect("valid limits");
        let session_a = SessionHandle::generate().expect("A");
        let session_b = SessionHandle::generate().expect("B");
        let a1 = envelope(ActionId::generate().expect("A1"), session_a.clone());
        broker
            .issue(spec(&a1, 100), MonotonicTime::from_millis(1))
            .expect("A1");
        let a2 = envelope(ActionId::generate().expect("A2"), session_a);
        assert_eq!(
            broker
                .issue(spec(&a2, 100), MonotonicTime::from_millis(1))
                .expect_err("A capacity"),
            ApprovalBrokerError::SessionCapacityExceeded
        );
        let b1 = envelope(ActionId::generate().expect("B1"), session_b);
        broker
            .issue(spec(&b1, 100), MonotonicTime::from_millis(1))
            .expect("B1");
        let session_c = SessionHandle::generate().expect("C");
        let c1 = envelope(ActionId::generate().expect("C1"), session_c);
        assert_eq!(
            broker
                .issue(spec(&c1, 100), MonotonicTime::from_millis(1))
                .expect_err("global capacity"),
            ApprovalBrokerError::CapacityExceeded
        );
    }

    #[test]
    fn session_revocation_removes_only_owned_approvals() {
        let broker = ApprovalBroker::new();
        let session_a = SessionHandle::generate().expect("A");
        let session_b = SessionHandle::generate().expect("B");
        let a = envelope(ActionId::generate().expect("A action"), session_a.clone());
        let b = envelope(ActionId::generate().expect("B action"), session_b.clone());
        let a_grant = broker
            .issue(spec(&a, 100), MonotonicTime::from_millis(1))
            .expect("A grant");
        let b_grant = broker
            .issue(spec(&b, 100), MonotonicTime::from_millis(1))
            .expect("B grant");
        assert_eq!(broker.revoke_session(&session_a).expect("revoke A"), 1);
        assert_eq!(
            broker
                .consume_exact(&a_grant.id, &a, MonotonicTime::from_millis(2))
                .expect_err("A removed"),
            ApprovalBrokerError::UnknownApproval
        );
        broker
            .consume_exact(&b_grant.id, &b, MonotonicTime::from_millis(2))
            .expect("B remains");
    }

    #[test]
    fn expired_grants_are_reclaimed_before_new_issuance() {
        let limits = HardLimits {
            max_approval_grants: 1,
            max_approval_grants_per_session: 1,
            ..HardLimits::default()
        };
        let broker = ApprovalBroker::from_hard_limits(limits).expect("valid limits");
        let first = envelope(
            ActionId::generate().expect("first action"),
            SessionHandle::generate().expect("first session"),
        );
        broker
            .issue(spec(&first, 2), MonotonicTime::from_millis(1))
            .expect("first grant");
        let second = envelope(
            ActionId::generate().expect("second action"),
            SessionHandle::generate().expect("second session"),
        );
        broker
            .issue(spec(&second, 10), MonotonicTime::from_millis(2))
            .expect("expired capacity must be reclaimed");
        assert_eq!(broker.len().expect("length"), 1);
    }
}
