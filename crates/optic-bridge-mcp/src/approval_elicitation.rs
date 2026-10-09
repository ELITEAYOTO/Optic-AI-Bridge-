use std::time::Duration;

use optic_bridge_core::{ActionEnvelope, MonotonicTime, TaskLease, ToolProfile};
use optic_bridge_policy::{PolicyDecision, PolicyEngine, PolicyReason};
use optic_bridge_runtime::{
    ApprovalBroker, ApprovalBrokerError, ApprovalSpec, Clock, ReusableApprovalBrokerError,
    SessionAdmissionPermit, SessionRegistry, SessionRegistryError, SessionReusableApprovalError,
    SessionReusableApprovalService, TaskLeaseRegistry, TaskLeaseRegistryError,
};
use rmcp::{
    model::{ElicitRequestParams, ElicitationAction, ElicitationSchema, EnumSchema},
    service::{ElicitationMode, RequestContext, RoleServer, ServiceError},
};
use serde_json::Value;
use thiserror::Error;

const PROFILE_APPROVAL_SCOPE_FIELD: &str = "approval_scope";
const PROFILE_APPROVAL_ONCE: &str = "once";
const PROFILE_APPROVAL_CURRENT_SESSION: &str = "current_session";

/// Application interpretation of one MCP elicitation response.
/// Only `Accepted` may be used by a higher layer to mint an ApprovalGrant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HumanApprovalDecision {
    Accepted,
    Declined,
    Cancelled,
    Unsupported,
}

/// Bounded user choice for an exact ToolProfile approval prompt.
///
/// `CurrentSession` is intentionally named after the Optic application session,
/// not a chat/conversation. The stronger user-facing conversation claim remains
/// gated on proving a trustworthy host conversation-to-session lifecycle mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileApprovalDecision {
    Once,
    CurrentSession,
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

/// Ask the MCP host for the approval lifetime of one exact ToolProfile action.
///
/// The requested form contains exactly one required single-select enum. `once` is
/// the least-authority default. A client that accepts but omits, corrupts or adds
/// ambiguous form content degrades to `Once`; malformed content can therefore never
/// mint reusable authority. A client without form elicitation remains `Unsupported`,
/// preserving the pre-existing fail-closed behavior.
pub async fn request_profile_approval_choice(
    context: &RequestContext<RoleServer>,
    message: impl Into<String>,
    timeout: Duration,
) -> Result<ProfileApprovalDecision, ApprovalElicitationError> {
    if !context
        .peer
        .supported_elicitation_modes()
        .contains(&ElicitationMode::Form)
    {
        return Ok(ProfileApprovalDecision::Unsupported);
    }

    let scope_schema = EnumSchema::builder(vec![
        PROFILE_APPROVAL_ONCE.to_owned(),
        PROFILE_APPROVAL_CURRENT_SESSION.to_owned(),
    ])
    .with_default(PROFILE_APPROVAL_ONCE)
    .map_err(|_| ApprovalElicitationError::InvalidSchema)?
    .enum_titles(vec![
        "Allow once".to_owned(),
        "Allow for current Optic session".to_owned(),
    ])
    .map_err(|_| ApprovalElicitationError::InvalidSchema)?
    .description(
        "Reusable approval applies only to this exact tool profile while the current Optic session remains active. Every invocation is revalidated.",
    )
    .build();
    let requested_schema = ElicitationSchema::builder()
        .required_enum_schema(PROFILE_APPROVAL_SCOPE_FIELD, scope_schema)
        .title("Approval scope")
        .description("Choose how long Optic may reuse approval for this exact tool profile.")
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
        ElicitationAction::Accept => Ok(accepted_profile_approval_scope(result.content.as_ref())),
        ElicitationAction::Decline => Ok(ProfileApprovalDecision::Declined),
        ElicitationAction::Cancel => Ok(ProfileApprovalDecision::Cancelled),
        _ => Err(ApprovalElicitationError::UnknownAction),
    }
}

fn accepted_profile_approval_scope(content: Option<&Value>) -> ProfileApprovalDecision {
    let Some(object) = content.and_then(Value::as_object) else {
        return ProfileApprovalDecision::Once;
    };
    if object.len() != 1 {
        return ProfileApprovalDecision::Once;
    }
    match object
        .get(PROFILE_APPROVAL_SCOPE_FIELD)
        .and_then(Value::as_str)
    {
        Some(PROFILE_APPROVAL_CURRENT_SESSION) => ProfileApprovalDecision::CurrentSession,
        Some(PROFILE_APPROVAL_ONCE) => ProfileApprovalDecision::Once,
        Some(_) | None => ProfileApprovalDecision::Once,
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
    #[error(transparent)]
    ReusableBroker(#[from] ReusableApprovalBrokerError),
    #[error("policy denied the exact action: {0:?}")]
    PolicyDenied(PolicyReason),
    #[error("the user declined the exact action")]
    Declined,
    #[error("the approval prompt was cancelled")]
    Cancelled,
    #[error("the connected MCP client does not support form elicitation")]
    Unsupported,
    #[error("the approval-required policy decision changed while awaiting authorization")]
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

/// Authorize an exact profiled action by reusing a matching active session/profile
/// approval when possible, otherwise falling back to the unchanged one-shot human
/// approval flow.
///
/// Reusable approval is consulted only after deterministic policy already returned
/// `RequireApproval` for the exact envelope. A successful reusable lookup transfers a
/// live session admission permit, then the exact task lease and policy decision are
/// revalidated again under that permit before it is returned to the caller. The grant
/// therefore cannot bypass profile, capability, scope, resource, isolation, policy-epoch
/// or session-lifecycle checks.
pub async fn authorize_profiled_action_with_reusable_or_human_approval<'a>(
    context: &RequestContext<RoleServer>,
    runtime: ApprovalAuthorizationRuntime<'a>,
    reusable_approvals: &'a SessionReusableApprovalService,
    profile: &ToolProfile,
    envelope: &ActionEnvelope,
    message: impl Into<String>,
    timeout: Duration,
) -> Result<SessionAdmissionPermit<'a>, ApprovalAuthorizationError> {
    let now = runtime.clock.now();
    let session = runtime.sessions.get_active(&envelope.session, now)?;
    let lease = active_task_lease(runtime.task_leases, envelope, now)?;
    let approval_reason = match runtime
        .policy
        .evaluate(envelope, &session, lease.as_ref(), now)
    {
        PolicyDecision::Allow => {
            return admit_and_revalidate_allow(
                runtime.sessions,
                runtime.task_leases,
                runtime.policy,
                runtime.clock,
                envelope,
            );
        }
        PolicyDecision::RequireApproval(reason) => reason,
        PolicyDecision::Deny(reason) => {
            return Err(ApprovalAuthorizationError::PolicyDenied(reason));
        }
    };

    let reusable = reusable_approvals
        .admit_active_for_profile(&envelope.session, profile, now)
        .map_err(map_reusable_approval_error)?;
    if let Some((grant, admission)) = reusable {
        let revalidate_now = runtime.clock.now();
        if grant.matches_profile(
            &envelope.session,
            profile,
            admission.grant().policy_epoch,
            revalidate_now,
        ) {
            let lease = active_task_lease(runtime.task_leases, envelope, revalidate_now)?;
            match runtime.policy.evaluate(
                envelope,
                admission.grant(),
                lease.as_ref(),
                revalidate_now,
            ) {
                PolicyDecision::RequireApproval(reason) if reason == approval_reason => {
                    return Ok(admission);
                }
                PolicyDecision::Deny(reason) => {
                    return Err(ApprovalAuthorizationError::PolicyDenied(reason));
                }
                PolicyDecision::Allow | PolicyDecision::RequireApproval(_) => {
                    return Err(ApprovalAuthorizationError::PolicyChanged);
                }
            }
        }
        drop(admission);
    }

    authorize_action_with_human_approval(context, runtime, envelope, message, timeout).await
}

fn map_reusable_approval_error(error: SessionReusableApprovalError) -> ApprovalAuthorizationError {
    match error {
        SessionReusableApprovalError::Session(error) => ApprovalAuthorizationError::Session(error),
        SessionReusableApprovalError::Broker(error) => {
            ApprovalAuthorizationError::ReusableBroker(error)
        }
        SessionReusableApprovalError::ExpiryNotFuture
        | SessionReusableApprovalError::ExpiryBeyondSession => {
            ApprovalAuthorizationError::PolicyChanged
        }
    }
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
