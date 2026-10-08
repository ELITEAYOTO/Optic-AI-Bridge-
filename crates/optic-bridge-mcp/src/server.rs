use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use optic_bridge_core::{
    ActionEnvelope, ActionId, Capability, Effect, HardLimits, LimitError, ResourceBudget,
    SessionGrant, SessionHandle, TaskLeaseId, WorkspacePath,
};
use optic_bridge_policy::{PolicyDecision, PolicyEngine};
use optic_bridge_runtime::{
    ApprovalBroker, AuthorizedFileMutationService, AuthorizedGitIntegrationService,
    BoundedFileSystem, Clock, EntryKind, FileSystemError, GitIntegrationAuthoritySet,
    GitReadService, MutationAuthoritySet, ProcessError, ProcessManager, SessionRegistry,
    SessionRegistryError, TaskLeaseRegistry, ToolProfileRegistry, TransportError, TransportGuard,
    TransportLimits,
};
use rmcp::{
    ErrorData, Json,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_READ_BYTES: u64 = 64 * 1024;
const DEFAULT_LIST_ENTRIES: u32 = 64;
const MCP_ENVELOPE_RESERVE_BYTES: u64 = 8 * 1024;
pub(crate) const STRUCTURED_VALUE_RESERVE_BYTES: u64 = 1024;

#[derive(Clone)]
pub struct ReadonlyMcpServer {
    tool_router: ToolRouter<Self>,
    filesystem: Arc<BoundedFileSystem>,
    pub(crate) sessions: Arc<SessionRegistry>,
    pub(crate) session: SessionHandle,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) policy: Arc<PolicyEngine>,
    pub(crate) transport_guard: TransportGuard,
    pub(crate) limits: HardLimits,
    pub(crate) processes: Arc<ProcessManager>,
    pub(crate) task_leases: Arc<TaskLeaseRegistry>,
    pub(crate) approvals: Arc<ApprovalBroker>,
    pub(crate) tool_profiles: Option<Arc<ToolProfileRegistry>>,
    pub(crate) process_leases: Arc<BTreeMap<String, TaskLeaseId>>,
    pub(crate) mutation_service: Option<Arc<AuthorizedFileMutationService>>,
    pub(crate) mutation_authorities: Arc<MutationAuthoritySet>,
    pub(crate) git_service: Option<Arc<GitReadService>>,
    pub(crate) git_integration_service: Option<Arc<AuthorizedGitIntegrationService>>,
    pub(crate) git_integration_authorities: Arc<GitIntegrationAuthoritySet>,
}

#[tool_handler(router = self.tool_router)]
impl rmcp::ServerHandler for ReadonlyMcpServer {}

#[tool_router(router = readonly_tool_router)]
impl ReadonlyMcpServer {
    pub fn new(
        root: impl AsRef<Path>,
        sessions: Arc<SessionRegistry>,
        session: SessionHandle,
        clock: Arc<dyn Clock>,
        limits: HardLimits,
    ) -> Result<Self, ServerBuildError> {
        let limits = limits.validate_nonzero()?;
        let processes = Arc::new(ProcessManager::new(root.as_ref(), limits, Vec::new())?);
        Self::new_with_process_runtime(
            root,
            sessions,
            session,
            clock,
            limits,
            processes,
            Arc::new(TaskLeaseRegistry::new()),
            BTreeMap::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_process_runtime(
        root: impl AsRef<Path>,
        sessions: Arc<SessionRegistry>,
        session: SessionHandle,
        clock: Arc<dyn Clock>,
        limits: HardLimits,
        processes: Arc<ProcessManager>,
        task_leases: Arc<TaskLeaseRegistry>,
        process_leases: BTreeMap<String, TaskLeaseId>,
    ) -> Result<Self, ServerBuildError> {
        Self::new_with_mutation_runtime(
            root,
            sessions,
            session,
            clock,
            limits,
            processes,
            task_leases,
            process_leases,
            None,
            MutationAuthoritySet::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_mutation_runtime(
        root: impl AsRef<Path>,
        sessions: Arc<SessionRegistry>,
        session: SessionHandle,
        clock: Arc<dyn Clock>,
        limits: HardLimits,
        processes: Arc<ProcessManager>,
        task_leases: Arc<TaskLeaseRegistry>,
        process_leases: BTreeMap<String, TaskLeaseId>,
        mutation_service: Option<Arc<AuthorizedFileMutationService>>,
        mutation_authorities: MutationAuthoritySet,
    ) -> Result<Self, ServerBuildError> {
        Self::new_with_git_runtime(
            root,
            sessions,
            session,
            clock,
            limits,
            processes,
            task_leases,
            process_leases,
            mutation_service,
            mutation_authorities,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_git_runtime(
        root: impl AsRef<Path>,
        sessions: Arc<SessionRegistry>,
        session: SessionHandle,
        clock: Arc<dyn Clock>,
        limits: HardLimits,
        processes: Arc<ProcessManager>,
        task_leases: Arc<TaskLeaseRegistry>,
        process_leases: BTreeMap<String, TaskLeaseId>,
        mutation_service: Option<Arc<AuthorizedFileMutationService>>,
        mutation_authorities: MutationAuthoritySet,
        git_service: Option<Arc<GitReadService>>,
    ) -> Result<Self, ServerBuildError> {
        Self::new_with_git_integration_runtime(
            root,
            sessions,
            session,
            clock,
            limits,
            processes,
            task_leases,
            process_leases,
            mutation_service,
            mutation_authorities,
            git_service,
            None,
            GitIntegrationAuthoritySet::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_git_integration_runtime(
        root: impl AsRef<Path>,
        sessions: Arc<SessionRegistry>,
        session: SessionHandle,
        clock: Arc<dyn Clock>,
        limits: HardLimits,
        processes: Arc<ProcessManager>,
        task_leases: Arc<TaskLeaseRegistry>,
        process_leases: BTreeMap<String, TaskLeaseId>,
        mutation_service: Option<Arc<AuthorizedFileMutationService>>,
        mutation_authorities: MutationAuthoritySet,
        git_service: Option<Arc<GitReadService>>,
        git_integration_service: Option<Arc<AuthorizedGitIntegrationService>>,
        git_integration_authorities: GitIntegrationAuthoritySet,
    ) -> Result<Self, ServerBuildError> {
        let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits)?);
        Self::new_with_git_integration_runtime_and_approvals(
            root,
            sessions,
            session,
            clock,
            limits,
            processes,
            task_leases,
            approvals,
            process_leases,
            mutation_service,
            mutation_authorities,
            git_service,
            git_integration_service,
            git_integration_authorities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_git_integration_runtime_and_approvals(
        root: impl AsRef<Path>,
        sessions: Arc<SessionRegistry>,
        session: SessionHandle,
        clock: Arc<dyn Clock>,
        limits: HardLimits,
        processes: Arc<ProcessManager>,
        task_leases: Arc<TaskLeaseRegistry>,
        approvals: Arc<ApprovalBroker>,
        process_leases: BTreeMap<String, TaskLeaseId>,
        mutation_service: Option<Arc<AuthorizedFileMutationService>>,
        mutation_authorities: MutationAuthoritySet,
        git_service: Option<Arc<GitReadService>>,
        git_integration_service: Option<Arc<AuthorizedGitIntegrationService>>,
        git_integration_authorities: GitIntegrationAuthoritySet,
    ) -> Result<Self, ServerBuildError> {
        Self::new_with_git_integration_runtime_and_approvals_and_profiles(
            root,
            sessions,
            session,
            clock,
            limits,
            processes,
            task_leases,
            approvals,
            None,
            process_leases,
            mutation_service,
            mutation_authorities,
            git_service,
            git_integration_service,
            git_integration_authorities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_git_integration_runtime_and_approvals_and_profiles(
        root: impl AsRef<Path>,
        sessions: Arc<SessionRegistry>,
        session: SessionHandle,
        clock: Arc<dyn Clock>,
        limits: HardLimits,
        processes: Arc<ProcessManager>,
        task_leases: Arc<TaskLeaseRegistry>,
        approvals: Arc<ApprovalBroker>,
        tool_profiles: Option<Arc<ToolProfileRegistry>>,
        process_leases: BTreeMap<String, TaskLeaseId>,
        mutation_service: Option<Arc<AuthorizedFileMutationService>>,
        mutation_authorities: MutationAuthoritySet,
        git_service: Option<Arc<GitReadService>>,
        git_integration_service: Option<Arc<AuthorizedGitIntegrationService>>,
        git_integration_authorities: GitIntegrationAuthoritySet,
    ) -> Result<Self, ServerBuildError> {
        let limits = limits.validate_nonzero()?;
        if limits.max_response_bytes <= MCP_ENVELOPE_RESERVE_BYTES + STRUCTURED_VALUE_RESERVE_BYTES
        {
            return Err(ServerBuildError::ResponseLimitTooSmall);
        }
        if (mutation_authorities.has_write() || mutation_authorities.has_delete())
            && mutation_service.is_none()
        {
            return Err(ServerBuildError::MutationRuntimeMissing);
        }
        if git_integration_authorities.has_integrate() != git_integration_service.is_some() {
            return Err(ServerBuildError::GitIntegrationRuntimeAuthorityMismatch);
        }

        let filesystem = Arc::new(BoundedFileSystem::from_hard_limits(root, limits)?);
        let transport_guard = TransportGuard::new(TransportLimits::from(limits))?;
        let mut tool_router = Self::readonly_tool_router();
        tool_router.merge(Self::process_tool_router());
        if mutation_authorities.has_write() {
            tool_router.merge(Self::mutation_write_tool_router());
        }
        if mutation_authorities.has_delete() {
            tool_router.merge(Self::mutation_delete_tool_router());
        }
        if git_service.is_some() {
            tool_router.merge(Self::git_read_tool_router());
        }
        if git_integration_authorities.has_integrate() {
            tool_router.merge(Self::git_integrate_tool_router());
        }

        Ok(Self {
            tool_router,
            filesystem,
            sessions,
            session,
            clock,
            policy: Arc::new(PolicyEngine),
            transport_guard,
            limits,
            processes,
            task_leases,
            approvals,
            tool_profiles,
            process_leases: Arc::new(process_leases),
            mutation_service,
            mutation_authorities: Arc::new(mutation_authorities),
            git_service,
            git_integration_service,
            git_integration_authorities: Arc::new(git_integration_authorities),
        })
    }

    #[must_use]
    pub const fn limits(&self) -> HardLimits {
        self.limits
    }

    /// Reports only whether application-owned Git integration authority and its
    /// authorized runtime are both provisioned. No lease, ref, repository path,
    /// executable path, or other authority-bearing detail is exposed.
    #[must_use]
    pub fn git_integration_authority_ready(&self) -> bool {
        self.git_integration_service.is_some() && self.git_integration_authorities.has_integrate()
    }

    #[cfg(test)]
    pub(crate) fn registered_tool_names(&self) -> Vec<String> {
        let mut names = self
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[tool(
        name = "fs_read",
        description = "Read a bounded chunk of one project-relative file. Binary data is returned as base64."
    )]
    pub async fn fs_read(
        &self,
        params: Parameters<FsReadRequest>,
    ) -> Result<Json<FsReadResponse>, ErrorData> {
        let path = parse_workspace_path(&params.0.path)?;
        let offset = params.0.offset.unwrap_or(0);
        let max_mcp_read = self.max_mcp_read_bytes();
        let max_bytes = params
            .0
            .max_bytes
            .unwrap_or(DEFAULT_READ_BYTES.min(max_mcp_read));
        if max_bytes == 0 || max_bytes > max_mcp_read {
            return Err(ErrorData::invalid_params(
                "optic.fs_read_limit_exceeded",
                None,
            ));
        }

        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(map_transport_error)?;
        let grant = self.active_grant(now)?;
        self.authorize(&grant, Effect::FileRead { path: path.clone() }, now)?;

        let filesystem = Arc::clone(&self.filesystem);
        let task_path = path.clone();
        let timeout_ms = permit
            .deadline()
            .as_millis()
            .saturating_sub(now.as_millis())
            .max(1);
        let chunk = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            tokio::task::spawn_blocking(move || {
                filesystem.read(&task_path, offset, Some(max_bytes))
            }),
        )
        .await
        .map_err(|_| ErrorData::internal_error("optic.request_timeout", None))?
        .map_err(|_| ErrorData::internal_error("optic.runtime_join_failed", None))?
        .map_err(map_filesystem_error)?;

        let response = FsReadResponse {
            path: path.as_str().to_owned(),
            encoding: "base64".to_owned(),
            data: STANDARD.encode(chunk.bytes),
            offset: chunk.offset,
            next_offset: chunk.next_offset,
            eof: chunk.eof,
        };
        self.ensure_structured_payload_fits(&response)?;
        drop(permit);
        Ok(Json(response))
    }

    #[tool(
        name = "fs_list",
        description = "List a bounded, deterministic page of one project-relative directory."
    )]
    pub async fn fs_list(
        &self,
        params: Parameters<FsListRequest>,
    ) -> Result<Json<FsListResponse>, ErrorData> {
        let root = params
            .0
            .path
            .as_deref()
            .map(parse_workspace_path)
            .transpose()?;
        let cursor_u64 = params.0.cursor.unwrap_or(0);
        let cursor = usize::try_from(cursor_u64)
            .map_err(|_| ErrorData::invalid_params("optic.invalid_cursor", None))?;
        let limit = params.0.limit.unwrap_or(DEFAULT_LIST_ENTRIES);
        if limit == 0 || limit > self.limits.max_fs_list_page_entries {
            return Err(ErrorData::invalid_params(
                "optic.fs_list_limit_exceeded",
                None,
            ));
        }

        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(map_transport_error)?;
        let grant = self.active_grant(now)?;
        self.authorize(&grant, Effect::FileSearch { root: root.clone() }, now)?;

        let filesystem = Arc::clone(&self.filesystem);
        let task_root = root.clone();
        let timeout_ms = permit
            .deadline()
            .as_millis()
            .saturating_sub(now.as_millis())
            .max(1);
        let page = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            tokio::task::spawn_blocking(move || filesystem.list(task_root.as_ref(), cursor, limit)),
        )
        .await
        .map_err(|_| ErrorData::internal_error("optic.request_timeout", None))?
        .map_err(|_| ErrorData::internal_error("optic.runtime_join_failed", None))?
        .map_err(map_filesystem_error)?;

        let original_len = page.entries.len();
        let mut response = FsListResponse {
            entries: page
                .entries
                .into_iter()
                .map(|entry| FsListEntry {
                    name: entry.name,
                    path: entry.path.as_str().to_owned(),
                    kind: entry_kind_name(entry.kind).to_owned(),
                })
                .collect(),
            next_cursor: page
                .next_cursor
                .map(|value| u64::try_from(value).unwrap_or(u64::MAX)),
        };
        self.fit_list_response(cursor_u64, original_len, &mut response)?;
        drop(permit);
        Ok(Json(response))
    }

    #[tool(
        name = "session_info",
        description = "Return the current Optic application-session metadata without exposing its bearer handle."
    )]
    pub async fn session_info(&self) -> Result<Json<SessionInfoResponse>, ErrorData> {
        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(map_transport_error)?;
        let grant = self.active_grant(now)?;
        let response = SessionInfoResponse {
            principal: grant.principal.as_str().to_owned(),
            project: grant.project.as_str().to_owned(),
            capabilities: grant
                .capabilities
                .iter()
                .map(|capability| capability_name(*capability).to_owned())
                .collect(),
            expires_at_monotonic_ms: grant.expires_at.as_millis(),
            policy_epoch: grant.policy_epoch,
        };
        self.ensure_structured_payload_fits(&response)?;
        drop(permit);
        Ok(Json(response))
    }

    pub(crate) fn active_grant(
        &self,
        now: optic_bridge_core::MonotonicTime,
    ) -> Result<SessionGrant, ErrorData> {
        self.sessions
            .get_active(&self.session, now)
            .map_err(map_session_error)
    }

    fn authorize(
        &self,
        grant: &SessionGrant,
        effect: Effect,
        now: optic_bridge_core::MonotonicTime,
    ) -> Result<(), ErrorData> {
        let action_id = ActionId::generate()
            .map_err(|_| ErrorData::internal_error("optic.action_id_unavailable", None))?;
        let envelope = ActionEnvelope {
            action_id,
            session: self.session.clone(),
            task_lease: None,
            effect,
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

    pub(crate) fn structured_payload_budget(&self) -> u64 {
        self.limits
            .max_response_bytes
            .saturating_sub(MCP_ENVELOPE_RESERVE_BYTES)
    }

    fn max_mcp_read_bytes(&self) -> u64 {
        let encoded_budget = self
            .structured_payload_budget()
            .saturating_sub(STRUCTURED_VALUE_RESERVE_BYTES);
        let binary_budget = encoded_budget.saturating_div(4).saturating_mul(3);
        self.limits.max_fs_read_bytes.min(binary_budget)
    }

    pub(crate) fn ensure_structured_payload_fits<T: Serialize>(
        &self,
        value: &T,
    ) -> Result<(), ErrorData> {
        let bytes = serde_json::to_vec(value)
            .map_err(|_| ErrorData::internal_error("optic.response_serialization_failed", None))?;
        let bytes = u64::try_from(bytes.len())
            .map_err(|_| ErrorData::internal_error("optic.response_too_large", None))?;
        if bytes > self.structured_payload_budget() {
            return Err(ErrorData::internal_error("optic.response_too_large", None));
        }
        Ok(())
    }

    fn fit_list_response(
        &self,
        cursor: u64,
        original_len: usize,
        response: &mut FsListResponse,
    ) -> Result<(), ErrorData> {
        while self.ensure_structured_payload_fits(response).is_err() && !response.entries.is_empty()
        {
            response.entries.pop();
        }
        self.ensure_structured_payload_fits(response)?;

        if response.entries.len() < original_len {
            let kept = u64::try_from(response.entries.len())
                .map_err(|_| ErrorData::internal_error("optic.response_too_large", None))?;
            response.next_cursor = Some(cursor.saturating_add(kept));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FsReadRequest {
    pub path: String,
    pub offset: Option<u64>,
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FsReadResponse {
    pub path: String,
    pub encoding: String,
    pub data: String,
    pub offset: u64,
    pub next_offset: u64,
    pub eof: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FsListRequest {
    pub path: Option<String>,
    pub cursor: Option<u64>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FsListResponse {
    pub entries: Vec<FsListEntry>,
    pub next_cursor: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FsListEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SessionInfoResponse {
    pub principal: String,
    pub project: String,
    pub capabilities: Vec<String>,
    pub expires_at_monotonic_ms: u64,
    pub policy_epoch: u64,
}

#[derive(Debug, Error)]
pub enum ServerBuildError {
    #[error("invalid hard limits: {0}")]
    Limits(#[from] LimitError),
    #[error("filesystem initialization failed: {0}")]
    FileSystem(#[from] FileSystemError),
    #[error("transport guard initialization failed: {0}")]
    Transport(#[from] TransportError),
    #[error("process runtime initialization failed: {0}")]
    Process(#[from] ProcessError),
    #[error("mutation authority requires an initialized authorized mutation runtime")]
    MutationRuntimeMissing,
    #[error("Git integration runtime and application-owned authority must be provisioned together")]
    GitIntegrationRuntimeAuthorityMismatch,
    #[error("response hard limit is too small for the MCP envelope reserve")]
    ResponseLimitTooSmall,
}

fn parse_workspace_path(value: &str) -> Result<WorkspacePath, ErrorData> {
    WorkspacePath::parse(value)
        .map_err(|_| ErrorData::invalid_params("optic.invalid_workspace_path", None))
}

pub(crate) fn map_session_error(_error: SessionRegistryError) -> ErrorData {
    ErrorData::invalid_request("optic.session_inactive", None)
}

pub(crate) fn map_transport_error(error: TransportError) -> ErrorData {
    match error {
        TransportError::TooManyConcurrentRequests => {
            ErrorData::internal_error("optic.server_busy", None)
        }
        TransportError::RequestTooLarge => {
            ErrorData::invalid_request("optic.request_too_large", None)
        }
        TransportError::ResponseTooLarge => {
            ErrorData::internal_error("optic.response_too_large", None)
        }
        TransportError::InvalidLimits => ErrorData::internal_error("optic.invalid_limits", None),
    }
}

fn map_filesystem_error(error: FileSystemError) -> ErrorData {
    match error {
        FileSystemError::OutsideWorkspace => {
            ErrorData::invalid_params("optic.path_outside_workspace", None)
        }
        FileSystemError::NotFile => ErrorData::invalid_params("optic.not_file", None),
        FileSystemError::NotDirectory => ErrorData::invalid_params("optic.not_directory", None),
        FileSystemError::ReadLimitExceeded => {
            ErrorData::invalid_params("optic.fs_read_limit_exceeded", None)
        }
        FileSystemError::ListLimitExceeded => {
            ErrorData::invalid_params("optic.fs_list_limit_exceeded", None)
        }
        FileSystemError::DirectoryScanLimitExceeded => {
            ErrorData::invalid_params("optic.directory_scan_limit_exceeded", None)
        }
        FileSystemError::OffsetOutOfRange => {
            ErrorData::invalid_params("optic.offset_out_of_range", None)
        }
        FileSystemError::Io(io_error) if io_error.kind() == std::io::ErrorKind::NotFound => {
            ErrorData::invalid_params("optic.path_not_found", None)
        }
        FileSystemError::InvalidLimits
        | FileSystemError::RootNotDirectory
        | FileSystemError::Path(_)
        | FileSystemError::NonUtf8Name
        | FileSystemError::Io(_) => ErrorData::internal_error("optic.filesystem_error", None),
    }
}

fn entry_kind_name(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::File => "file",
        EntryKind::Directory => "directory",
        EntryKind::Symlink => "symlink",
        EntryKind::Other => "other",
    }
}

fn capability_name(capability: Capability) -> &'static str {
    match capability {
        Capability::FileRead => "file_read",
        Capability::FileSearch => "file_search",
        Capability::FileWrite => "file_write",
        Capability::FileDelete => "file_delete",
        Capability::GitRead => "git_read",
        Capability::GitIntegrate => "git_integrate",
        Capability::ProcessRun => "process_run",
        Capability::NetworkAccess => "network_access",
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, env, fs, path::PathBuf};

    use optic_bridge_core::{MonotonicTime, PrincipalId, ProjectId};
    use optic_bridge_runtime::{
        ApprovalBrokerError, ApprovalSpec, GitIntegrationAuthoritySpec,
        git_integration_resource_budget,
    };

    use super::*;

    #[derive(Debug)]
    struct FixedClock(MonotonicTime);

    impl Clock for FixedClock {
        fn now(&self) -> MonotonicTime {
            self.0
        }
    }

    fn workspace(label: &str) -> PathBuf {
        let token = ActionId::generate().expect("test entropy").to_token();
        let root = env::temp_dir().join(format!("optic-mcp-{label}-{token}"));
        fs::create_dir_all(&root).expect("create temp workspace");
        root
    }

    fn server(root: &Path, capabilities: &[Capability]) -> ReadonlyMcpServer {
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(MonotonicTime::from_millis(10)));
        let session = SessionHandle::generate().expect("test entropy");
        let grant = SessionGrant {
            handle: session.clone(),
            principal: PrincipalId::new("test-principal").expect("principal"),
            project: ProjectId::new("test-project").expect("project"),
            capabilities: capabilities.iter().copied().collect::<BTreeSet<_>>(),
            expires_at: MonotonicTime::from_millis(1_000),
            policy_epoch: 1,
        };
        let sessions = Arc::new(SessionRegistry::new());
        sessions.register(grant).expect("register session");
        ReadonlyMcpServer::new(root, sessions, session, clock, HardLimits::default())
            .expect("build server")
    }

    #[tokio::test]
    async fn session_cancel_revokes_the_servers_shared_approval_broker() {
        let root = workspace("cancel-approval");
        let server = server(&root, &[Capability::FileRead]);
        let now = server.clock.now();
        let grant = server
            .sessions
            .get_active(&server.session, now)
            .expect("active session");
        let envelope = ActionEnvelope {
            action_id: ActionId::generate().expect("action id"),
            session: server.session.clone(),
            task_lease: None,
            effect: Effect::FileRead {
                path: WorkspacePath::parse("fixture.txt").expect("workspace path"),
            },
            resources: ResourceBudget {
                timeout_ms: 1_000,
                output_bytes: 1_024,
                memory_bytes: 1_024,
                process_count: 1,
            },
            policy_epoch: grant.policy_epoch,
        };
        let approval = server
            .approvals
            .issue(
                ApprovalSpec {
                    session: envelope.session.clone(),
                    action_id: envelope.action_id.clone(),
                    effect: envelope.effect.clone(),
                    resources: envelope.resources,
                    expires_at: now.saturating_add_millis(1_000),
                    policy_epoch: envelope.policy_epoch,
                },
                now,
            )
            .expect("issue approval");

        server.session_cancel().await.expect("cancel session");
        assert_eq!(
            server
                .approvals
                .consume_exact(&approval.id, &envelope, now)
                .expect_err("session cancellation must remove its approval"),
            ApprovalBrokerError::UnknownApproval
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn fs_read_routes_through_runtime_and_returns_base64() {
        let root = workspace("read");
        fs::write(root.join("hello.bin"), b"hello").expect("write fixture");
        let server = server(&root, &[Capability::FileRead]);

        let response = server
            .fs_read(Parameters(FsReadRequest {
                path: "hello.bin".to_owned(),
                offset: None,
                max_bytes: Some(5),
            }))
            .await
            .expect("read should pass")
            .0;
        assert_eq!(response.data, STANDARD.encode(b"hello"));
        assert!(response.eof);

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn fs_list_is_denied_without_file_search_capability() {
        let root = workspace("deny-list");
        let server = server(&root, &[Capability::FileRead]);

        assert!(
            server
                .fs_list(Parameters(FsListRequest {
                    path: None,
                    cursor: None,
                    limit: Some(8),
                }))
                .await
                .is_err()
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn invalid_public_path_fails_before_filesystem_access() {
        let root = workspace("bad-path");
        let server = server(&root, &[Capability::FileRead]);

        assert!(
            server
                .fs_read(Parameters(FsReadRequest {
                    path: "../secret".to_owned(),
                    offset: None,
                    max_bytes: Some(8),
                }))
                .await
                .is_err()
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn git_integration_authority_without_runtime_fails_server_build() {
        let root = workspace("git-integrate-mismatch");
        let limits = HardLimits::default();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(MonotonicTime::from_millis(10)));
        let session = SessionHandle::generate().expect("test entropy");
        let grant = SessionGrant {
            handle: session.clone(),
            principal: PrincipalId::new("test-principal").expect("principal"),
            project: ProjectId::new("test-project").expect("project"),
            capabilities: BTreeSet::from([Capability::GitIntegrate]),
            expires_at: MonotonicTime::from_millis(1_000),
            policy_epoch: 1,
        };
        let sessions = Arc::new(SessionRegistry::new());
        sessions.register(grant).expect("register session");
        let task_leases = Arc::new(TaskLeaseRegistry::new());
        let authority = GitIntegrationAuthoritySet::provision(
            &task_leases,
            &session,
            GitIntegrationAuthoritySpec { enabled: true },
            git_integration_resource_budget(limits),
            MonotonicTime::from_millis(1_000),
            1,
        )
        .expect("provision authority");
        let processes =
            Arc::new(ProcessManager::new(&root, limits, Vec::new()).expect("process runtime"));

        let result = ReadonlyMcpServer::new_with_git_integration_runtime(
            &root,
            sessions,
            session,
            clock,
            limits,
            processes,
            task_leases,
            BTreeMap::new(),
            None,
            MutationAuthoritySet::default(),
            None,
            None,
            authority,
        );

        assert!(matches!(
            result,
            Err(ServerBuildError::GitIntegrationRuntimeAuthorityMismatch)
        ));
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn tool_surface_matches_phase1c_contract() {
        let root = workspace("tools");
        let server = server(&root, &[Capability::FileRead, Capability::FileSearch]);
        assert!(!server.git_integration_authority_ready());
        let mut names = server
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(
            names,
            vec![
                "fs_list",
                "fs_read",
                "process_read",
                "process_result",
                "process_start",
                "process_stop",
                "session_cancel",
                "session_info",
            ]
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
