use crate::{
    ActionEnvelope, ActionId, ApprovalId, Effect, MonotonicTime, ResourceBudget, SessionHandle,
};

/// One exact, application-owned approval for one normalized action.
///
/// Approval never widens session or task-lease authority. It is only meaningful
/// after normal policy checks have reached `RequireApproval`, and the runtime
/// broker enforces one-shot consumption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalGrant {
    pub id: ApprovalId,
    pub session: SessionHandle,
    pub action_id: ActionId,
    pub effect: Effect,
    pub resources: ResourceBudget,
    pub expires_at: MonotonicTime,
    pub policy_epoch: u64,
}

impl ApprovalGrant {
    #[must_use]
    pub fn is_expired_at(&self, now: MonotonicTime) -> bool {
        now >= self.expires_at
    }

    #[must_use]
    pub fn exactly_matches(&self, envelope: &ActionEnvelope, now: MonotonicTime) -> bool {
        !self.is_expired_at(now)
            && self.session == envelope.session
            && self.action_id == envelope.action_id
            && self.effect == envelope.effect
            && self.resources == envelope.resources
            && self.policy_epoch == envelope.policy_epoch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NetworkAccess, ProcessExecutionClass};

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
            policy_epoch: 7,
        }
    }

    #[test]
    fn approval_is_exact_to_action_effect_resources_epoch_and_expiry() {
        let action_id = ActionId::generate().expect("action id");
        let session = SessionHandle::generate().expect("session");
        let envelope = envelope(action_id.clone(), session.clone());
        let grant = ApprovalGrant {
            id: ApprovalId::generate().expect("approval id"),
            session,
            action_id,
            effect: envelope.effect.clone(),
            resources: envelope.resources,
            expires_at: MonotonicTime::from_millis(100),
            policy_epoch: envelope.policy_epoch,
        };

        assert!(grant.exactly_matches(&envelope, MonotonicTime::from_millis(99)));
        assert!(!grant.exactly_matches(&envelope, MonotonicTime::from_millis(100)));

        let mut changed = envelope.clone();
        changed.action_id = ActionId::generate().expect("other action");
        assert!(!grant.exactly_matches(&changed, MonotonicTime::from_millis(1)));

        let mut changed = envelope.clone();
        changed.resources.output_bytes += 1;
        assert!(!grant.exactly_matches(&changed, MonotonicTime::from_millis(1)));

        let mut changed = envelope.clone();
        changed.policy_epoch += 1;
        assert!(!grant.exactly_matches(&changed, MonotonicTime::from_millis(1)));
    }
}
