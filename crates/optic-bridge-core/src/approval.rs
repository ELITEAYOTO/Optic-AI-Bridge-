use crate::{
    ActionEnvelope, ActionId, ApprovalId, Effect, MonotonicTime, ResourceBudget,
    ReusableApprovalId, SessionHandle, ToolProfile, ToolProfileFingerprint, ToolProfileName,
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

/// Reusable human approval for one immutable ToolProfile inside one application session.
///
/// Unlike `ApprovalGrant`, this value intentionally does not contain an `ActionId`:
/// every covered invocation remains a new action and must independently pass normal
/// session/task/profile/policy/resource/isolation checks. The reusable grant only
/// suppresses repeated human elicitation when the exact approved profile identity is
/// still active under the same session and policy epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReusableApprovalGrant {
    pub id: ReusableApprovalId,
    pub session: SessionHandle,
    pub profile: ToolProfileName,
    pub profile_fingerprint: ToolProfileFingerprint,
    pub expires_at: MonotonicTime,
    pub policy_epoch: u64,
}

impl ReusableApprovalGrant {
    #[must_use]
    pub fn is_expired_at(&self, now: MonotonicTime) -> bool {
        now >= self.expires_at
    }

    #[must_use]
    pub fn matches_profile(
        &self,
        session: &SessionHandle,
        profile: &ToolProfile,
        policy_epoch: u64,
        now: MonotonicTime,
    ) -> bool {
        !self.is_expired_at(now)
            && &self.session == session
            && &self.profile == profile.name()
            && self.profile_fingerprint == profile.fingerprint()
            && self.policy_epoch == policy_epoch
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::{
        NetworkAccess, ProcessExecutionClass, ToolApprovalRequirement, ToolProfileSpec,
        WorkloadClass,
    };

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

    fn profile(timeout_ms: u64) -> ToolProfile {
        ToolProfile::from_spec(ToolProfileSpec {
            name: ToolProfileName::parse("cargo-check").expect("profile name"),
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
                output_bytes: 1024,
                memory_bytes: 1024,
                process_count: 1,
            },
            approval: ToolApprovalRequirement::HumanRequired,
        })
        .expect("profile")
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

    #[test]
    fn reusable_approval_matches_only_same_session_profile_epoch_and_lifetime() {
        let session = SessionHandle::generate().expect("session");
        let approved_profile = profile(5_000);
        let grant = ReusableApprovalGrant {
            id: ReusableApprovalId::generate().expect("reusable approval id"),
            session: session.clone(),
            profile: approved_profile.name().clone(),
            profile_fingerprint: approved_profile.fingerprint(),
            expires_at: MonotonicTime::from_millis(100),
            policy_epoch: 7,
        };

        assert!(grant.matches_profile(
            &session,
            &approved_profile,
            7,
            MonotonicTime::from_millis(99)
        ));
        assert!(!grant.matches_profile(
            &session,
            &approved_profile,
            7,
            MonotonicTime::from_millis(100)
        ));

        let other_session = SessionHandle::generate().expect("other session");
        assert!(!grant.matches_profile(
            &other_session,
            &approved_profile,
            7,
            MonotonicTime::from_millis(1)
        ));
        assert!(!grant.matches_profile(
            &session,
            &approved_profile,
            8,
            MonotonicTime::from_millis(1)
        ));

        let widened_profile = profile(5_001);
        assert_eq!(widened_profile.name(), approved_profile.name());
        assert!(!grant.matches_profile(
            &session,
            &widened_profile,
            7,
            MonotonicTime::from_millis(1)
        ));
    }
}
