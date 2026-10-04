#![forbid(unsafe_code)]

//! Deterministic authorization for normalized Optic AI Bridge actions.

use optic_bridge_core::{
    ActionEnvelope, Capability, Effect, LeaseScope, MonotonicTime, SessionGrant, TaskLease,
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
    ScopeNotAuthorized,
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
        now: MonotonicTime,
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

        match &envelope.effect {
            Effect::PolicyChange => {
                return PolicyDecision::Deny(PolicyReason::SecurityPolicyImmutable);
            }
            Effect::PrivilegeElevation => {
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

        let lease = if envelope.effect.requires_task_lease() {
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
            if !lease_covers_effect(lease, &envelope.effect) {
                return PolicyDecision::Deny(PolicyReason::ScopeNotAuthorized);
            }
            Some(lease)
        } else {
            None
        };

        if envelope.effect.requires_network_capability() {
            if !session.allows(Capability::NetworkAccess) {
                return PolicyDecision::Deny(PolicyReason::NetworkNotAuthorized);
            }
            if let Some(lease) = lease
                && (!lease.allows(Capability::NetworkAccess)
                    || !lease_covers_network(lease, &envelope.effect))
            {
                return PolicyDecision::Deny(PolicyReason::NetworkNotAuthorized);
            }
        }

        if matches!(&envelope.effect, Effect::NetworkAccess { .. }) {
            return PolicyDecision::RequireApproval(PolicyReason::NetworkNotAuthorized);
        }

        PolicyDecision::Allow
    }
}

fn lease_covers_effect(lease: &TaskLease, effect: &Effect) -> bool {
    match effect {
        Effect::FileWrite { path, .. } | Effect::FileDelete { path, .. } => {
            lease.has_scope(&LeaseScope::WorkspaceAll)
                || lease.scopes.iter().any(|scope| {
                    matches!(scope, LeaseScope::WorkspacePrefix(prefix) if path.is_within(prefix))
                })
        }
        Effect::GitIntegrate { .. } => lease.has_scope(&LeaseScope::Repository),
        Effect::ProcessRun { executable, .. } => {
            lease.has_scope(&LeaseScope::ProcessExecutable(executable.clone()))
        }
        Effect::NetworkAccess { endpoint } => {
            lease.has_scope(&LeaseScope::NetworkEndpoint(endpoint.clone()))
        }
        Effect::FileRead { .. } | Effect::FileSearch { .. } | Effect::GitRead => true,
        Effect::PolicyChange | Effect::PrivilegeElevation => false,
    }
}

fn lease_covers_network(lease: &TaskLease, effect: &Effect) -> bool {
    match effect {
        Effect::ProcessRun { .. } => lease.has_scope(&LeaseScope::NetworkAny),
        Effect::NetworkAccess { endpoint } => {
            lease.has_scope(&LeaseScope::NetworkEndpoint(endpoint.clone()))
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use optic_bridge_core::{
        ActionEnvelope, ActionId, Capability, ContentVersion, Effect, ExpectedState, GitObjectId,
        LeaseScope, MonotonicTime, NetworkAccess, PrincipalId, ProjectId, ResourceBudget,
        SessionGrant, SessionHandle, TaskLease, TaskLeaseId, WorkspacePath,
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

    fn now() -> MonotonicTime {
        MonotonicTime::from_millis(10_000)
    }

    fn session(capabilities: &[Capability]) -> SessionGrant {
        let now = now();
        SessionGrant {
            handle: SessionHandle::generate().expect("test entropy"),
            principal: PrincipalId::new("test-principal").expect("valid principal"),
            project: ProjectId::new("project-a").expect("valid project"),
            capabilities: capabilities.iter().copied().collect::<BTreeSet<_>>(),
            expires_at: now.saturating_add_millis(300_000),
            policy_epoch: 7,
        }
    }

    fn envelope(
        session: &SessionGrant,
        effect: Effect,
        lease: Option<&TaskLease>,
    ) -> ActionEnvelope {
        ActionEnvelope {
            action_id: ActionId::generate().expect("test entropy"),
            session: session.handle.clone(),
            task_lease: lease.map(|value| value.id.clone()),
            effect,
            resources: budget(),
            policy_epoch: session.policy_epoch,
        }
    }

    fn lease(
        session: &SessionGrant,
        capabilities: &[Capability],
        scopes: &[LeaseScope],
    ) -> TaskLease {
        TaskLease {
            id: TaskLeaseId::generate().expect("test entropy"),
            session: session.handle.clone(),
            capabilities: capabilities.iter().copied().collect::<BTreeSet<_>>(),
            scopes: scopes.iter().cloned().collect::<BTreeSet<_>>(),
            resource_ceiling: budget(),
            expires_at: now().saturating_add_millis(60_000),
            policy_epoch: session.policy_epoch,
        }
    }

    fn path(value: &str) -> WorkspacePath {
        WorkspacePath::parse(value).expect("safe test path")
    }

    fn oid(byte: char) -> GitObjectId {
        GitObjectId::parse(std::iter::repeat_n(byte, 40).collect::<String>()).expect("test oid")
    }

    #[test]
    fn git_integrate_requires_repository_scoped_lease() {
        let session = session(&[Capability::GitIntegrate]);
        let action = envelope(
            &session,
            Effect::GitIntegrate {
                source_head: oid('b'),
                expected_target_head: oid('a'),
            },
            None,
        );
        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now()),
            PolicyDecision::Deny(PolicyReason::LeaseRequired)
        );

        let wrong_scope = lease(
            &session,
            &[Capability::GitIntegrate],
            &[LeaseScope::WorkspaceAll],
        );
        let action = envelope(
            &session,
            Effect::GitIntegrate {
                source_head: oid('b'),
                expected_target_head: oid('a'),
            },
            Some(&wrong_scope),
        );
        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&wrong_scope), now()),
            PolicyDecision::Deny(PolicyReason::ScopeNotAuthorized)
        );
    }

    #[test]
    fn git_integrate_with_repository_scope_is_allowed() {
        let session = session(&[Capability::GitIntegrate]);
        let lease = lease(
            &session,
            &[Capability::GitIntegrate],
            &[LeaseScope::Repository],
        );
        let action = envelope(
            &session,
            Effect::GitIntegrate {
                source_head: oid('b'),
                expected_target_head: oid('a'),
            },
            Some(&lease),
        );
        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now()),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn read_can_be_allowed_without_task_lease() {
        let session = session(&[Capability::FileRead]);
        let action = envelope(
            &session,
            Effect::FileRead {
                path: path("src/lib.rs"),
            },
            None,
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now()),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn mutation_requires_narrow_task_lease() {
        let session = session(&[Capability::FileWrite]);
        let action = envelope(
            &session,
            Effect::FileWrite {
                path: path("src/lib.rs"),
                expected: ExpectedState::Absent,
            },
            None,
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now()),
            PolicyDecision::Deny(PolicyReason::LeaseRequired)
        );
    }

    #[test]
    fn mutation_with_matching_scope_is_allowed() {
        let session = session(&[Capability::FileWrite]);
        let lease = lease(
            &session,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspacePrefix(path("src"))],
        );
        let action = envelope(
            &session,
            Effect::FileWrite {
                path: path("src/lib.rs"),
                expected: ExpectedState::Content(ContentVersion::from_bytes(b"old")),
            },
            Some(&lease),
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now()),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn mutation_outside_lease_scope_is_denied() {
        let session = session(&[Capability::FileWrite]);
        let lease = lease(
            &session,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspacePrefix(path("src"))],
        );
        let action = envelope(
            &session,
            Effect::FileWrite {
                path: path("docs/README.md"),
                expected: ExpectedState::Absent,
            },
            Some(&lease),
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now()),
            PolicyDecision::Deny(PolicyReason::ScopeNotAuthorized)
        );
    }

    #[test]
    fn cross_session_lease_is_denied() {
        let first = session(&[Capability::FileWrite]);
        let second = session(&[Capability::FileWrite]);
        let lease = lease(
            &second,
            &[Capability::FileWrite],
            &[LeaseScope::WorkspaceAll],
        );
        let action = envelope(
            &first,
            Effect::FileWrite {
                path: path("src/lib.rs"),
                expected: ExpectedState::Absent,
            },
            Some(&lease),
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &first, Some(&lease), now()),
            PolicyDecision::Deny(PolicyReason::LeaseWrongSession)
        );
    }

    #[test]
    fn stale_policy_epoch_is_denied() {
        let session = session(&[Capability::FileRead]);
        let mut action = envelope(
            &session,
            Effect::FileRead {
                path: path("src/lib.rs"),
            },
            None,
        );
        action.policy_epoch += 1;

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now()),
            PolicyDecision::Deny(PolicyReason::StalePolicyEpoch)
        );
    }

    #[test]
    fn policy_change_is_always_denied() {
        let session = session(&[]);
        let action = envelope(&session, Effect::PolicyChange, None);

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, None, now()),
            PolicyDecision::Deny(PolicyReason::SecurityPolicyImmutable)
        );
    }

    #[test]
    fn lease_budget_cannot_be_exceeded() {
        let session = session(&[Capability::ProcessRun]);
        let mut lease = lease(
            &session,
            &[Capability::ProcessRun],
            &[LeaseScope::ProcessExecutable("cargo".to_owned())],
        );
        lease.resource_ceiling.output_bytes = 1024;
        let action = envelope(
            &session,
            Effect::ProcessRun {
                executable: "cargo".to_owned(),
                network: NetworkAccess::Denied,
            },
            Some(&lease),
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now()),
            PolicyDecision::Deny(PolicyReason::ResourceBudgetExceeded)
        );
    }

    #[test]
    fn process_network_requires_network_capability_in_lease() {
        let session = session(&[Capability::ProcessRun, Capability::NetworkAccess]);
        let lease = lease(
            &session,
            &[Capability::ProcessRun],
            &[
                LeaseScope::ProcessExecutable("cargo".to_owned()),
                LeaseScope::NetworkAny,
            ],
        );
        let action = envelope(
            &session,
            Effect::ProcessRun {
                executable: "cargo".to_owned(),
                network: NetworkAccess::Allowed,
            },
            Some(&lease),
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now()),
            PolicyDecision::Deny(PolicyReason::NetworkNotAuthorized)
        );
    }

    #[test]
    fn process_network_requires_explicit_network_scope() {
        let session = session(&[Capability::ProcessRun, Capability::NetworkAccess]);
        let lease = lease(
            &session,
            &[Capability::ProcessRun, Capability::NetworkAccess],
            &[LeaseScope::ProcessExecutable("cargo".to_owned())],
        );
        let action = envelope(
            &session,
            Effect::ProcessRun {
                executable: "cargo".to_owned(),
                network: NetworkAccess::Allowed,
            },
            Some(&lease),
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now()),
            PolicyDecision::Deny(PolicyReason::NetworkNotAuthorized)
        );
    }

    #[test]
    fn process_network_with_capability_and_scope_is_allowed() {
        let session = session(&[Capability::ProcessRun, Capability::NetworkAccess]);
        let lease = lease(
            &session,
            &[Capability::ProcessRun, Capability::NetworkAccess],
            &[
                LeaseScope::ProcessExecutable("cargo".to_owned()),
                LeaseScope::NetworkAny,
            ],
        );
        let action = envelope(
            &session,
            Effect::ProcessRun {
                executable: "cargo".to_owned(),
                network: NetworkAccess::Allowed,
            },
            Some(&lease),
        );

        assert_eq!(
            PolicyEngine.evaluate(&action, &session, Some(&lease), now()),
            PolicyDecision::Allow
        );
    }
}
