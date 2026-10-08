use std::time::Duration;

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
