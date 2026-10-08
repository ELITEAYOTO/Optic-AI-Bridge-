use std::time::Duration;

use optic_bridge_core::{ActionEnvelope, MonotonicTime, TaskLease};
use optic_bridge_policy::{PolicyDecision, PolicyEngine, PolicyReason};
use optic_bridge_runtime::{
    ApprovalBroker, ApprovalBrokerError, ApprovalSpec, Clock, SessionAdmissionPermit,
    SessionRegistry, SessionRegistryError, TaskLeaseRegistry, TaskLeaseRegistryError,
};
use rmcp::{
    model::{ElicitRequestParams, ElicitationAction, ElicitationSchema},
    service::{ElicitationMode, RequestContext, RoleServer, ServiceError},
};
use thiserror::Error;

/// Application interpretation of one MCP elicitation response.
/// Only `Accepted` may be used by a higher layer to mint an ApprovalGrant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HumanApprovalDecision {
    Accepted,
    Declined,
    Cancelled,
    Unsupported,
}

#[derive(Debug, Error)]
pub enum ApprovalElicitationError {
    #[error("MCP elicitation failed: {0}")]
    Service(#[from] ServiceError),
    #[error("MCP client returned an unknown elicitation action")]
    UnknownAction,
    #[error("failed to build the bounded approval elicitation schema")]
    InvalidSchema,
}

/// Request one bounded, explicit human confirmation through standard MCP elicitation.
/// This transport never mints authority itself.
pub async fn request_human_approval(
    context: &RequestContext<RoleServer>,
    message: impl Into<String>,
    timeout: Duration,
) -> Result<HumanApprovalDecision, ApprovalElicitationError> {
    if !context
        .peer
        .supported_elicitation_modes()
        .contains(&ElicitationMode::Form)
    {
        return Ok(HumanApprovalDecision::Unsupported);
    }

    let requested_schema = ElicitationSchema::builder()
        .build()
        .map_err(|_| ApprovalElicitationError::InvalidSchema)?;
    let params = ElicitRequestParams::FormElicitationParams {
        meta: None,
        message: message.into(),
        requested_schema,
    };
    let result = context
        .peer
        .create_elicitation_with_timeout(params, Some(timeout))
        .await?;

    #[allow(unreachable_patterns)]
    match result.action {
        ElicitationAction::Accept => Ok(HumanApprovalDecision::Accepted),
        ElicitationAction::Decline => Ok(HumanApprovalDecision::Declined),
        ElicitationAction::Cancel => Ok(HumanApprovalDecision::Cancelled),
        _ => Err(ApprovalElicitationError::UnknownAction),
    }
}

#[derive(Debug, Error)]
pub enum ApprovalAuthorizationError {
    #[error(transparent)]
    Elicitation(#[from] ApprovalElicitationError),
    #[error(transparent)]
    Session(#[from] SessionRegistryError),
    #[error(transparent)]
    TaskLease(#[from] TaskLeaseRegistryError),
    #[error(transparent)]
    Broker(#[from] ApprovalBrokerError),
    #[error("policy denied the exact action: {0:?}")]
    PolicyDenied(PolicyReason),
    #[error("the user declined the exact action")]
    Declined,
    #[error("the approval prompt was cancelled")]
    Cancelled,
    #[error("the connected MCP client does not support form elicitation")]
    Unsupported,
    #[error("the approval-required policy decision changed while awaiting the user")]
    PolicyChanged,
}

#[derive(Clone, Copy)]
pub struct ApprovalAuthorizationRuntime<'a> {
    sessions: &'a SessionRegistry,
    task_leases: &'a TaskLeaseRegistry,
    approvals: &'a ApprovalBroker,
    policy: &'a PolicyEngine,
    clock: &'a dyn Clock,
}

impl<'a> ApprovalAuthorizationRuntime<'a> {
    #[must_use]
    pub const fn new(
        sessions: &'a SessionRegistry,
        task_leases: &'a TaskLeaseRegistry,
        approvals: &'a ApprovalBroker,
        policy: &'a PolicyEngine,
        clock: &'a dyn Clock,
    ) -> Self {
        Self {
            sessions,
            task_leases,
            approvals,
            policy,
            clock,
        }
    }
}

/// Authorize one exact application-owned action, optionally using standard MCP
/// human elicitation when deterministic policy returns `RequireApproval`.
///
/// No session admission is held while the user is deciding. After acceptance,
/// the exact envelope is revalidated under a fresh admission, an exact one-shot
/// ApprovalGrant is issued and immediately consumed, and the admission permit is
/// returned to the caller so revoke cannot race the subsequent effect.
pub async fn authorize_action_with_human_approval<'a>(
    context: &RequestContext<RoleServer>,
    runtime: ApprovalAuthorizationRuntime<'a>,
    envelope: &ActionEnvelope,
    message: impl Into<String>,
    timeout: Duration,
) -> Result<SessionAdmissionPermit<'a>, ApprovalAuthorizationError> {
    let ApprovalAuthorizationRuntime {
        sessions,
        task_leases,
        approvals,
        policy,
        clock,
    } = runtime;
    let now = clock.now();
    let session = sessions.get_active(&envelope.session, now)?;
    let lease = active_task_lease(task_leases, envelope, now)?;
    let approval_reason = match policy.evaluate(envelope, &session, lease.as_ref(), now) {
        PolicyDecision::Allow => {
            return admit_and_revalidate_allow(sessions, task_leases, policy, clock, envelope);
        }
        PolicyDecision::RequireApproval(reason) => reason,
        PolicyDecision::Deny(reason) => {
            return Err(ApprovalAuthorizationError::PolicyDenied(reason));
        }
    };

    match request_human_approval(context, message, timeout).await? {
        HumanApprovalDecision::Accepted => {}
        HumanApprovalDecision::Declined => return Err(ApprovalAuthorizationError::Declined),
        HumanApprovalDecision::Cancelled => return Err(ApprovalAuthorizationError::Cancelled),
        HumanApprovalDecision::Unsupported => return Err(ApprovalAuthorizationError::Unsupported),
    }

    let now = clock.now();
    let admission = sessions.begin_admission(&envelope.session, now)?;
    let lease = active_task_lease(task_leases, envelope, now)?;
    match policy.evaluate(envelope, admission.grant(), lease.as_ref(), now) {
        PolicyDecision::RequireApproval(reason) if reason == approval_reason => {}
        PolicyDecision::Deny(reason) => {
            return Err(ApprovalAuthorizationError::PolicyDenied(reason));
        }
        PolicyDecision::Allow | PolicyDecision::RequireApproval(_) => {
            return Err(ApprovalAuthorizationError::PolicyChanged);
        }
    }

    let grant = approvals.issue(
        ApprovalSpec {
            session: envelope.session.clone(),
            action_id: envelope.action_id.clone(),
            effect: envelope.effect.clone(),
            resources: envelope.resources,
            expires_at: admission.grant().expires_at,
            policy_epoch: envelope.policy_epoch,
        },
        now,
    )?;
    approvals.consume_exact(&grant.id, envelope, now)?;
    Ok(admission)
}

fn admit_and_revalidate_allow<'a>(
    sessions: &'a SessionRegistry,
    task_leases: &TaskLeaseRegistry,
    policy: &PolicyEngine,
    clock: &dyn Clock,
    envelope: &ActionEnvelope,
) -> Result<SessionAdmissionPermit<'a>, ApprovalAuthorizationError> {
    let now = clock.now();
    let admission = sessions.begin_admission(&envelope.session, now)?;
    let lease = active_task_lease(task_leases, envelope, now)?;
    match policy.evaluate(envelope, admission.grant(), lease.as_ref(), now) {
        PolicyDecision::Allow => Ok(admission),
        PolicyDecision::Deny(reason) => Err(ApprovalAuthorizationError::PolicyDenied(reason)),
        PolicyDecision::RequireApproval(_) => Err(ApprovalAuthorizationError::PolicyChanged),
    }
}

fn active_task_lease(
    task_leases: &TaskLeaseRegistry,
    envelope: &ActionEnvelope,
    now: MonotonicTime,
) -> Result<Option<TaskLease>, TaskLeaseRegistryError> {
    envelope
        .task_lease
        .as_ref()
        .map(|id| task_leases.get_active(id, &envelope.session, now))
        .transpose()
}
