use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use optic_bridge_core::{
    ActionEnvelope, ActionId, Effect, GitObjectId, ResourceBudget, SessionGrant, WorkspacePath,
};
use optic_bridge_policy::PolicyDecision;
use optic_bridge_runtime::{GitLogCursor, GitReadError, GitReadService};
use rmcp::{ErrorData, Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::server::ReadonlyMcpServer;

const DEFAULT_GIT_DIFF_BYTES: u64 = 64 * 1024;
const DEFAULT_GIT_LOG_ENTRIES: u32 = 32;

#[tool_router(router = git_read_tool_router, vis = "pub")]
impl ReadonlyMcpServer {
    #[tool(
        name = "git_status",
        description = "Return bounded Git porcelain-v2 status for the operator-owned repository. Raw porcelain bytes are base64; the caller cannot choose a repository or Git executable."
    )]
    pub async fn git_status(&self) -> Result<Json<GitStatusResponse>, ErrorData> {
        let max_bytes = self.max_mcp_git_read_bytes();
        let snapshot = self
            .run_git_read(move |service| service.status_with_max_bytes(max_bytes))
            .await?;
        let response = GitStatusResponse {
            head: snapshot.head.map(|value| value.as_str().to_owned()),
            encoding: "base64".to_owned(),
            porcelain_v2_base64: STANDARD.encode(snapshot.porcelain_v2),
        };
        self.ensure_structured_payload_fits(&response)?;
        Ok(Json(response))
    }

    #[tool(
        name = "git_diff",
        description = "Return a bounded literal-path Git diff from the operator-owned repository. External diff/textconv and submodule traversal are disabled."
    )]
    pub async fn git_diff(
        &self,
        params: Parameters<GitDiffRequest>,
    ) -> Result<Json<GitDiffResponse>, ErrorData> {
        let path = params
            .0
            .path
            .as_deref()
            .map(parse_workspace_path)
            .transpose()?;
        let staged = params.0.staged.unwrap_or(false);
        let max_mcp_bytes = self.max_mcp_git_read_bytes();
        let max_bytes = params
            .0
            .max_bytes
            .unwrap_or(DEFAULT_GIT_DIFF_BYTES.min(max_mcp_bytes));
        if max_bytes == 0 || max_bytes > max_mcp_bytes {
            return Err(ErrorData::invalid_params(
                "optic.git_read_limit_exceeded",
                None,
            ));
        }

        let diff = self
            .run_git_read(move |service| service.diff(path.as_ref(), staged, Some(max_bytes)))
            .await?;
        let response = GitDiffResponse {
            encoding: "base64".to_owned(),
            data_base64: STANDARD.encode(diff.bytes),
        };
        self.ensure_structured_payload_fits(&response)?;
        Ok(Json(response))
    }

    #[tool(
        name = "git_log",
        description = "Return a bounded page of commit metadata. Cursors remain pinned to their original reachable HEAD and fail closed after incompatible history rewrites."
    )]
    pub async fn git_log(
        &self,
        params: Parameters<GitLogRequest>,
    ) -> Result<Json<GitLogResponse>, ErrorData> {
        let cursor = params.0.cursor.map(parse_log_cursor).transpose()?;
        let start_offset = cursor.as_ref().map(|value| value.offset).unwrap_or(0);
        let limit = params.0.limit.unwrap_or(DEFAULT_GIT_LOG_ENTRIES);
        if limit == 0 || limit > self.limits.max_git_log_entries {
            return Err(ErrorData::invalid_params(
                "optic.git_log_limit_exceeded",
                None,
            ));
        }

        let page = self
            .run_git_read(move |service| service.log(cursor.as_ref(), limit))
            .await?;
        let mut response = GitLogResponse {
            snapshot_head: page
                .snapshot_head
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            entries: page
                .entries
                .into_iter()
                .map(|entry| GitLogEntryResponse {
                    id: entry.id.as_str().to_owned(),
                    parents: entry
                        .parents
                        .into_iter()
                        .map(|parent| parent.as_str().to_owned())
                        .collect(),
                    commit_time_unix_seconds: entry.commit_time_unix_seconds,
                })
                .collect(),
            next_cursor: page.next_cursor.map(cursor_response),
        };
        self.fit_git_log_response(start_offset, &mut response)?;
        Ok(Json(response))
    }

    async fn run_git_read<T, F>(&self, operation: F) -> Result<T, ErrorData>
    where
        T: Send + 'static,
        F: FnOnce(Arc<GitReadService>) -> Result<T, GitReadError> + Send + 'static,
    {
        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(super::server::map_transport_error)?;
        let grant = self.active_grant(now)?;
        self.authorize_git_read(&grant, now)?;
        let service = self.git_service.as_ref().cloned().ok_or_else(|| {
            ErrorData::invalid_request("optic.git_read_authority_unavailable", None)
        })?;
        let result = tokio::task::spawn_blocking(move || operation(service))
            .await
            .map_err(|_| ErrorData::internal_error("optic.runtime_join_failed", None))?
            .map_err(map_git_read_error)?;
        drop(permit);
        Ok(result)
    }

    fn authorize_git_read(
        &self,
        grant: &SessionGrant,
        now: optic_bridge_core::MonotonicTime,
    ) -> Result<(), ErrorData> {
        let action_id = ActionId::generate()
            .map_err(|_| ErrorData::internal_error("optic.action_id_unavailable", None))?;
        let envelope = ActionEnvelope {
            action_id,
            session: self.session.clone(),
            task_lease: None,
            effect: Effect::GitRead,
            resources: ResourceBudget {
                timeout_ms: self.limits.max_request_duration_ms,
                output_bytes: self.limits.max_response_bytes,
                memory_bytes: self.limits.max_active_output_ram_bytes,
                process_count: 1,
            },
            policy_epoch: grant.policy_epoch,
        };
        match self.policy.evaluate(&envelope, grant, None, now) {
            PolicyDecision::Allow => Ok(()),
            PolicyDecision::RequireApproval(_) | PolicyDecision::Deny(_) => {
                Err(ErrorData::invalid_request("optic.policy_denied", None))
            }
        }
    }

    fn max_mcp_git_read_bytes(&self) -> u64 {
        let encoded_budget = self
            .structured_payload_budget()
            .saturating_sub(super::server::STRUCTURED_VALUE_RESERVE_BYTES);
        let binary_budget = encoded_budget.saturating_div(4).saturating_mul(3);
        self.limits.max_git_read_bytes.min(binary_budget)
    }

    fn fit_git_log_response(
        &self,
        start_offset: u64,
        response: &mut GitLogResponse,
    ) -> Result<(), ErrorData> {
        let original_len = response.entries.len();
        while self.ensure_structured_payload_fits(response).is_err() && !response.entries.is_empty()
        {
            response.entries.pop();
        }
        self.ensure_structured_payload_fits(response)?;

        if response.entries.len() < original_len {
            let head = response.snapshot_head.clone().ok_or_else(|| {
                ErrorData::internal_error("optic.git_log_cursor_missing_head", None)
            })?;
            let kept = u64::try_from(response.entries.len())
                .map_err(|_| ErrorData::internal_error("optic.response_too_large", None))?;
            response.next_cursor = Some(GitLogCursorResponse {
                head,
                offset: start_offset.saturating_add(kept),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitDiffRequest {
    pub path: Option<String>,
    pub staged: Option<bool>,
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitStatusResponse {
    pub head: Option<String>,
    pub encoding: String,
    pub porcelain_v2_base64: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitDiffResponse {
    pub encoding: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitLogRequest {
    pub cursor: Option<GitLogCursorRequest>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitLogCursorRequest {
    pub head: String,
    pub offset: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitLogResponse {
    pub snapshot_head: Option<String>,
    pub entries: Vec<GitLogEntryResponse>,
    pub next_cursor: Option<GitLogCursorResponse>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitLogEntryResponse {
    pub id: String,
    pub parents: Vec<String>,
    pub commit_time_unix_seconds: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitLogCursorResponse {
    pub head: String,
    pub offset: u64,
}

fn parse_workspace_path(value: &str) -> Result<WorkspacePath, ErrorData> {
    WorkspacePath::parse(value)
        .map_err(|_| ErrorData::invalid_params("optic.invalid_workspace_path", None))
}

fn parse_log_cursor(value: GitLogCursorRequest) -> Result<GitLogCursor, ErrorData> {
    let head = GitObjectId::parse(value.head)
        .map_err(|_| ErrorData::invalid_params("optic.git_log_cursor_invalid", None))?;
    Ok(GitLogCursor {
        head,
        offset: value.offset,
    })
}

fn cursor_response(value: GitLogCursor) -> GitLogCursorResponse {
    GitLogCursorResponse {
        head: value.head.as_str().to_owned(),
        offset: value.offset,
    }
}

fn map_git_read_error(error: GitReadError) -> ErrorData {
    match error {
        GitReadError::ZeroRequestLimit
        | GitReadError::ReadByteLimitExceeded { .. }
        | GitReadError::LogEntryLimitExceeded { .. } => {
            ErrorData::invalid_params("optic.git_read_limit_exceeded", None)
        }
        GitReadError::LogCursorNotReachable | GitReadError::ObjectId(_) => {
            ErrorData::invalid_params("optic.git_log_cursor_invalid", None)
        }
        GitReadError::OutputLimitExceeded { .. } => {
            ErrorData::internal_error("optic.git_output_too_large", None)
        }
        GitReadError::CommandTimedOut => {
            ErrorData::internal_error("optic.request_timeout", None)
        }
        GitReadError::InvalidLimits
        | GitReadError::GitExecutableMustBeAbsolute
        | GitReadError::GitExecutableNotFile
        | GitReadError::RepositoryRootNotDirectory
        | GitReadError::RepositoryRootMismatch
        | GitReadError::NonUtf8RepositoryRoot
        | GitReadError::CommandFailed { .. }
        | GitReadError::MissingChildPipe
        | GitReadError::ReaderThreadPanicked
        | GitReadError::InvalidObjectId
        | GitReadError::InvalidLogRecord
        | GitReadError::Io(_) => ErrorData::internal_error("optic.git_read_error", None),
    }
}
