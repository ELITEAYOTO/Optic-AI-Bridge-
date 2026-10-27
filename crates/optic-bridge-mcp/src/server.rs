use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use optic_bridge_core::{
    ActionEnvelope, ActionId, Capability, Effect, HardLimits, LimitError, ResourceBudget,
    SessionGrant, SessionHandle, TaskLeaseId, WorkspacePath,
};
use optic_bridge_policy::{PolicyDecision, PolicyEngine};
use optic_bridge_runtime::{
    ApprovalBroker, AuthorizedFileMutationService, AuthorizedGitIntegrationService,
    BlockingIoError, BlockingIoGovernor, BoundedFileSystem, Clock, EntryKind, FileSystemError,
    GitIntegrationAuthoritySet, GitReadService, MutationAuthoritySet, ProcessError, ProcessManager,
    ReadAuthoritySet, RequestPermit, SessionRegistry, SessionRegistryError, TaskLeaseRegistry,
    ToolProfileRegistry, TransportError, TransportGuard, TransportLimits,
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
    blocking_io: BlockingIoGovernor,
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
    pub(crate) read_authorities: Arc<ReadAuthoritySet>,
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
        if (mutation_authorities.has_write()
            || mutation_authorities.has_delete()
            || mutation_authorities.has_directory_create())
            && mutation_service.is_none()
        {
            return Err(ServerBuildError::MutationRuntimeMissing);
        }
        if git_integration_authorities.has_integrate() != git_integration_service.is_some() {
            return Err(ServerBuildError::GitIntegrationRuntimeAuthorityMismatch);
        }

        let filesystem = Arc::new(BoundedFileSystem::from_hard_limits(root, limits)?);
        let blocking_io = BlockingIoGovernor::new(limits.max_concurrent_requests)?;
        let transport_guard = TransportGuard::new(TransportLimits::from(limits))?;
        let mut tool_router = Self::readonly_tool_router();
        tool_router.merge(Self::process_tool_router());
        if mutation_authorities.has_write() {
            tool_router.merge(Self::mutation_write_tool_router());
        }
        if mutation_authorities.has_delete() {
            tool_router.merge(Self::mutation_delete_tool_router());
        }
        if mutation_authorities.has_directory_create() {
            tool_router.merge(Self::directory_create_tool_router());
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
            blocking_io,
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
            read_authorities: Arc::new(ReadAuthoritySet::default()),
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

    /// Installs application-owned read/search leases before the server is exposed.
    /// This consumes the server so authority cannot be swapped on a live instance.
    #[must_use]
    pub fn with_read_authorities(mut self, read_authorities: ReadAuthoritySet) -> Self {
        self.read_authorities = Arc::new(read_authorities);
        self
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

        let (canonical_path, chunk) = self
            .run_admitted_read(move |server, grant, permit| {
                let prepared = server
                    .filesystem
                    .prepare_read(&path, offset, Some(max_bytes))
                    .map_err(map_filesystem_error)?;
                let canonical_path = prepared.observation().canonical_path.clone();
                let has_multiple_links = prepared.has_multiple_links();
                server.ensure_read_deadline(permit)?;
                server.authorize(
                    grant,
                    Effect::FileRead {
                        path: canonical_path.clone(),
                    },
                    server.clock.now(),
                )?;
                // A hard link can alias a sensitive file through an ordinary
                // name. The lexical classifier alone cannot prove its origin.
                if has_multiple_links {
                    server.require_sensitive_read_scope(
                        grant,
                        &canonical_path,
                        server.clock.now(),
                    )?;
                }
                let chunk = prepared.read().map_err(map_filesystem_error)?;
                Ok((canonical_path, chunk))
            })
            .await?;

        let response = FsReadResponse {
            path: canonical_path.as_str().to_owned(),
            encoding: "base64".to_owned(),
            data: STANDARD.encode(chunk.bytes),
            offset: chunk.offset,
            next_offset: chunk.next_offset,
            eof: chunk.eof,
        };
        self.ensure_structured_payload_fits(&response)?;
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

        let page = self
            .run_admitted_read(move |server, grant, permit| {
                // Authorize the resolved directory, not an untrusted alias.
                // A symlink within a permitted prefix may target an entirely
                // different workspace subtree. List the canonical target so
                // returned entry paths also name the authorized directory.
                let canonical_root = server
                    .filesystem
                    .canonical_list_root(root.as_ref())
                    .map_err(map_filesystem_error)?;
                server.ensure_read_deadline(permit)?;
                server.authorize(
                    grant,
                    Effect::FileSearch {
                        root: canonical_root.clone(),
                    },
                    server.clock.now(),
                )?;
                if let Some(root) = canonical_root.as_ref() {
                    server.ensure_sensitive_metadata_authorized(grant, root, server.clock.now())?;
                }
                let page = server
                    .filesystem
                    .list(canonical_root.as_ref(), cursor, limit)
                    .map_err(map_filesystem_error)?;
                for entry in &page.entries {
                    server.ensure_sensitive_metadata_authorized(
                        grant,
                        &entry.path,
                        server.clock.now(),
                    )?;
                }
                Ok(page)
            })
            .await?;

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

    /// Admission and transport capacity live in the worker, not in its waiter.
    /// A timeout/cancel cannot cancel spawn_blocking: revoke must still drain
    /// that worker before claiming the session is quiescent.
    pub(crate) async fn run_admitted_read<T, F>(&self, operation: F) -> Result<T, ErrorData>
    where
        T: Send + 'static,
        F: FnOnce(&Self, &SessionGrant, &RequestPermit) -> Result<T, ErrorData> + Send + 'static,
    {
        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(map_transport_error)?;
        let timeout_ms = permit
            .deadline()
            .as_millis()
            .saturating_sub(now.as_millis())
            .max(1);
        let server = self.clone();
        tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            self.blocking_io.run(move || {
                let permit = permit;
                server.ensure_read_deadline(&permit)?;
                let admission = server
                    .sessions
                    .begin_admission(&server.session, server.clock.now())
                    .map_err(map_session_error)?;
                operation(&server, admission.grant(), &permit)
            }),
        )
        .await
        .map_err(|_| ErrorData::internal_error("optic.request_timeout", None))?
        .map_err(map_blocking_io_error)?
    }

    pub(crate) fn ensure_read_deadline(&self, permit: &RequestPermit) -> Result<(), ErrorData> {
        if permit.is_expired_at(self.clock.now()) {
            Err(ErrorData::internal_error("optic.request_timeout", None))
        } else {
            Ok(())
        }
    }

    /// FileSearch permission alone does not disclose protected filenames.
    /// For now this denies the entire affected page (no unsafe cursor skipping).
    pub(crate) fn ensure_sensitive_metadata_authorized(
        &self,
        grant: &SessionGrant,
        path: &WorkspacePath,
        now: optic_bridge_core::MonotonicTime,
    ) -> Result<(), ErrorData> {
        if !optic_bridge_core::requires_sensitive_authority(path) {
            return Ok(());
        }
        self.require_sensitive_read_scope(grant, path, now)
    }

    /// Explicit additional FileRead scope for a potentially sensitive native
    /// alias, even if its observed workspace name looks ordinary.
    fn require_sensitive_read_scope(
        &self,
        grant: &SessionGrant,
        path: &WorkspacePath,
        now: optic_bridge_core::MonotonicTime,
    ) -> Result<(), ErrorData> {
        let denied = || ErrorData::invalid_request("optic.policy_denied", None);
        if !grant.allows(Capability::FileRead) {
            return Err(denied());
        }
        let lease_id = self
            .read_authorities
            .lease_for(Capability::FileRead)
            .ok_or_else(denied)?;
        let lease = self
            .task_leases
            .get_active(lease_id, &self.session, now)
            .map_err(|_| denied())?;
        if !lease.allows(Capability::FileRead)
            || !lease.covers_read_sensitive(path)
            || lease.policy_epoch != grant.policy_epoch
        {
            return Err(denied());
        }
        Ok(())
    }

    pub(crate) fn authorize(
        &self,
        grant: &SessionGrant,
        effect: Effect,
        now: optic_bridge_core::MonotonicTime,
    ) -> Result<(), ErrorData> {
        let capability = effect
            .required_capability()
            .ok_or_else(|| ErrorData::invalid_request("optic.policy_denied", None))?;
        let lease_id = self
            .read_authorities
            .lease_for(capability)
            .ok_or_else(|| ErrorData::invalid_request("optic.policy_denied", None))?;
        let lease = self
            .task_leases
            .get_active(lease_id, &self.session, now)
            .map_err(|_| ErrorData::invalid_request("optic.policy_denied", None))?;
        let action_id = ActionId::generate()
            .map_err(|_| ErrorData::internal_error("optic.action_id_unavailable", None))?;
        let envelope = ActionEnvelope {
            action_id,
            session: self.session.clone(),
            task_lease: Some(lease.id.clone()),
            effect,
            resources: ResourceBudget {
                timeout_ms: self.limits.max_request_duration_ms,
                output_bytes: self.limits.max_response_bytes,
                memory_bytes: self.limits.max_active_output_ram_bytes,
                process_count: 1,
            },
            policy_epoch: grant.policy_epoch,
        };

        match self.policy.evaluate(&envelope, grant, Some(&lease), now) {
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
            if response.entries.is_empty() {
                return Err(ErrorData::internal_error("optic.response_too_large", None));
            }
            let kept = u64::try_from(response.entries.len())
                .map_err(|_| ErrorData::internal_error("optic.response_too_large", None))?;
            response.next_cursor = Some(cursor.saturating_add(kept));
        }
        // Changing the cursor can itself grow the serialized response. Refit
        // with the final cursor and require at least one entry of progress.
        while self.ensure_structured_payload_fits(response).is_err() {
            response.entries.pop();
            if response.entries.is_empty() {
                return Err(ErrorData::internal_error("optic.response_too_large", None));
            }
            let kept = u64::try_from(response.entries.len())
                .map_err(|_| ErrorData::internal_error("optic.response_too_large", None))?;
            response.next_cursor = Some(cursor.saturating_add(kept));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    #[error("blocking I/O governor initialization failed: {0}")]
    BlockingIo(#[from] BlockingIoError),
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

fn map_blocking_io_error(_error: BlockingIoError) -> ErrorData {
    ErrorData::internal_error("optic.blocking_io_unavailable", None)
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
        #[cfg(windows)]
        FileSystemError::Windows(_) => ErrorData::internal_error("optic.filesystem_error", None),
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
        Capability::DirectoryCreate => "directory_create",
        Capability::GitRead => "git_read",
        Capability::GitIntegrate => "git_integrate",
        Capability::ProcessRun => "process_run",
        Capability::NetworkAccess => "network_access",
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, env, fs, path::PathBuf};

    use optic_bridge_core::{MonotonicTime, PrincipalId, ProjectId, SearchMetadataScope};
    use optic_bridge_runtime::{
        ApprovalBrokerError, ApprovalSpec, GitIntegrationAuthoritySpec, ReadAuthoritySet,
        ReadAuthoritySpec, git_integration_resource_budget, read_authority_resource_budget,
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
        server_with_search_scope(root, capabilities, None)
    }

    fn server_with_search_scope(
        root: &Path,
        capabilities: &[Capability],
        search_scope: Option<SearchMetadataScope>,
    ) -> ReadonlyMcpServer {
        server_with_scopes(root, capabilities, search_scope, None)
    }

    fn server_with_scopes(
        root: &Path,
        capabilities: &[Capability],
        search_scope: Option<SearchMetadataScope>,
        sensitive_scope: Option<optic_bridge_core::ReadSensitiveScope>,
    ) -> ReadonlyMcpServer {
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
        let limits = HardLimits::default();
        let server = ReadonlyMcpServer::new(root, sessions, session.clone(), clock, limits)
            .expect("build server");
        let spec = ReadAuthoritySpec {
            search_metadata: if capabilities.contains(&Capability::FileSearch) {
                BTreeSet::from([search_scope.unwrap_or_else(SearchMetadataScope::all)])
            } else {
                BTreeSet::new()
            },
            read_content: if capabilities.contains(&Capability::FileRead) {
                BTreeSet::from([optic_bridge_core::ReadContentScope::all()])
            } else {
                BTreeSet::new()
            },
            read_sensitive: sensitive_scope.into_iter().collect(),
        };
        let read_authorities = ReadAuthoritySet::provision(
            &server.task_leases,
            &session,
            &spec,
            read_authority_resource_budget(limits),
            MonotonicTime::from_millis(1_000),
            1,
        )
        .expect("provision read authority");
        server.with_read_authorities(read_authorities)
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
    async fn file_read_capability_without_application_owned_read_lease_is_denied() {
        let root = workspace("read-no-lease");
        fs::write(root.join("hello.bin"), b"hello").expect("write fixture");
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(MonotonicTime::from_millis(10)));
        let session = SessionHandle::generate().expect("test entropy");
        let grant = SessionGrant {
            handle: session.clone(),
            principal: PrincipalId::new("test-principal").expect("principal"),
            project: ProjectId::new("test-project").expect("project"),
            capabilities: BTreeSet::from([Capability::FileRead]),
            expires_at: MonotonicTime::from_millis(1_000),
            policy_epoch: 1,
        };
        let sessions = Arc::new(SessionRegistry::new());
        sessions.register(grant).expect("register session");
        let server = ReadonlyMcpServer::new(&root, sessions, session, clock, HardLimits::default())
            .expect("build server");

        assert!(
            server
                .fs_read(Parameters(FsReadRequest {
                    path: "hello.bin".to_owned(),
                    offset: None,
                    max_bytes: Some(5),
                }))
                .await
                .is_err()
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
    async fn sensitive_fs_read_is_denied_without_extra_scope_and_works_with_explicit_scope() {
        let root = workspace("sensitive-read");
        fs::write(root.join(".env"), b"fixture-only-secret").expect("fixture");
        fs::write(root.join(".envrc.private"), b"direnv-fixture-only").expect("fixture");
        fs::write(root.join("README.md"), b"ordinary").expect("fixture");
        let read = |path: &str| FsReadRequest {
            path: path.to_owned(),
            offset: None,
            max_bytes: Some(64),
        };
        let ordinary = server(&root, &[Capability::FileRead]);
        assert_eq!(
            ordinary
                .fs_read(Parameters(read(".env")))
                .await
                .err()
                .expect("sensitive read requires extra scope")
                .message,
            "optic.policy_denied"
        );
        assert_eq!(
            ordinary
                .fs_read(Parameters(read(".envrc.private")))
                .await
                .err()
                .expect("direnv private content requires extra scope")
                .message,
            "optic.policy_denied"
        );
        ordinary
            .fs_read(Parameters(read("README.md")))
            .await
            .expect("ordinary content allowed");
        let elevated = server_with_scopes(
            &root,
            &[Capability::FileRead],
            None,
            Some(optic_bridge_core::ReadSensitiveScope::all()),
        );
        let response = elevated
            .fs_read(Parameters(read(".env")))
            .await
            .expect("explicit sensitive authority")
            .0;
        assert_eq!(
            STANDARD.decode(response.data).expect("base64"),
            b"fixture-only-secret"
        );
        let direnv = elevated
            .fs_read(Parameters(read(".envrc.private")))
            .await
            .expect("explicitly authorized direnv content")
            .0;
        assert_eq!(
            STANDARD.decode(direnv.data).expect("base64"),
            b"direnv-fixture-only"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn hard_link_alias_cannot_downgrade_sensitive_file_read_authority() {
        let root = workspace("hardlink-sensitive-read");
        fs::write(root.join(".env"), b"synthetic-secret").expect("fixture");
        fs::hard_link(root.join(".env"), root.join("README.txt")).expect("alias");
        fs::write(root.join("ordinary.txt"), b"ordinary fixture").expect("fixture");
        let read = |name: &str| FsReadRequest {
            path: name.to_owned(),
            offset: None,
            max_bytes: Some(64),
        };
        let ordinary = server(&root, &[Capability::FileRead]);
        assert_eq!(
            ordinary
                .fs_read(Parameters(read("README.txt")))
                .await
                .err()
                .expect("ordinary name cannot bypass protected hardlink")
                .message,
            "optic.policy_denied"
        );
        ordinary
            .fs_read(Parameters(read("ordinary.txt")))
            .await
            .expect("independent regular file is still readable");
        let elevated = server_with_scopes(
            &root,
            &[Capability::FileRead],
            None,
            Some(optic_bridge_core::ReadSensitiveScope::all()),
        );
        let result = elevated
            .fs_read(Parameters(read("README.txt")))
            .await
            .expect("explicitly scoped sensitive read")
            .0;
        assert_eq!(
            STANDARD.decode(result.data).expect("base64"),
            b"synthetic-secret"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[tokio::test]
    async fn fs_list_does_not_expose_sensitive_names_without_extra_scope() {
        let root = workspace("sensitive-list");
        fs::write(root.join("README.md"), b"ordinary").expect("fixture");
        fs::write(root.join(".env"), b"fixture-only-secret").expect("fixture");
        let list = || FsListRequest {
            path: None,
            cursor: None,
            limit: Some(64),
        };
        let ordinary = server(&root, &[Capability::FileSearch, Capability::FileRead]);
        assert_eq!(
            ordinary
                .fs_list(Parameters(list()))
                .await
                .err()
                .expect("sensitive filename must not be returned")
                .message,
            "optic.policy_denied"
        );
        let elevated = server_with_scopes(
            &root,
            &[Capability::FileSearch, Capability::FileRead],
            None,
            Some(optic_bridge_core::ReadSensitiveScope::all()),
        );
        let response = elevated
            .fs_list(Parameters(list()))
            .await
            .expect("authorized listing")
            .0;
        assert!(response.entries.iter().any(|entry| entry.name == ".env"));
        let no_read = server(&root, &[Capability::FileSearch]);
        assert_eq!(
            no_read
                .fs_list(Parameters(list()))
                .await
                .err()
                .expect("no read lease")
                .message,
            "optic.policy_denied"
        );
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

    #[cfg(unix)]
    #[tokio::test]
    async fn fs_list_cannot_follow_a_permitted_alias_outside_its_search_scope() {
        use std::os::unix::fs::symlink;

        let root = workspace("list-scoped-alias");
        fs::create_dir(root.join("allowed")).expect("allowed");
        fs::create_dir(root.join("private")).expect("private");
        fs::write(root.join("private").join("names-only.txt"), b"private")
            .expect("private fixture");
        symlink("../private", root.join("allowed").join("portal")).expect("internal alias");

        let scoped_server = server_with_search_scope(
            &root,
            &[Capability::FileSearch],
            Some(SearchMetadataScope::prefix(
                WorkspacePath::parse("allowed").expect("prefix"),
            )),
        );
        let denied = scoped_server
            .fs_list(Parameters(FsListRequest {
                path: Some("allowed/portal".to_owned()),
                cursor: None,
                limit: Some(8),
            }))
            .await
            .err()
            .expect("alias must not list a directory outside its search scope");
        assert_eq!(denied.message, "optic.policy_denied");

        let permitted = scoped_server
            .fs_list(Parameters(FsListRequest {
                path: Some("allowed".to_owned()),
                cursor: None,
                limit: Some(8),
            }))
            .await
            .expect("authorized directory remains listable");
        assert_eq!(permitted.0.entries.len(), 1);
        assert_eq!(permitted.0.entries[0].name, "portal");
        drop(scoped_server);

        // A whole-workspace lease can follow the same alias, but the returned
        // paths must describe the resolved target rather than the alias.
        let all = server(&root, &[Capability::FileSearch]);
        let response = all
            .fs_list(Parameters(FsListRequest {
                path: Some("allowed/portal".to_owned()),
                cursor: None,
                limit: Some(8),
            }))
            .await
            .expect("whole-workspace search may list canonical target");
        assert_eq!(response.0.entries.len(), 1);
        assert_eq!(response.0.entries[0].path, "private/names-only.txt");

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
    fn list_fitting_rejects_zero_progress_and_refits_the_final_cursor() {
        let root = workspace("list-fit");
        let mut server = server(&root, &[Capability::FileSearch]);
        let entry = || FsListEntry {
            name: "x".to_owned(),
            path: "x".to_owned(),
            kind: "file".to_owned(),
        };
        let mut oversized = FsListResponse {
            entries: vec![FsListEntry {
                name: "x".repeat(200),
                path: "x".repeat(200),
                kind: "file".to_owned(),
            }],
            next_cursor: None,
        };
        server.limits.max_response_bytes = MCP_ENVELOPE_RESERVE_BYTES + 100;
        assert_eq!(
            server
                .fit_list_response(7, 1, &mut oversized)
                .expect_err("must not return cursor 7 again")
                .message,
            "optic.response_too_large"
        );

        // A final cursor with many digits is bigger than the initial null.
        let one = FsListResponse {
            entries: vec![entry()],
            next_cursor: Some(1_000_000_002),
        };
        let budget = serde_json::to_vec(&one).expect("json").len() as u64;
        server.limits.max_response_bytes = MCP_ENVELOPE_RESERVE_BYTES + budget;
        let mut response = FsListResponse {
            entries: vec![entry(), entry(), entry()],
            next_cursor: None,
        };
        server
            .fit_list_response(1_000_000_000, 3, &mut response)
            .expect("fit with progress");
        assert!(!response.entries.is_empty());
        assert_eq!(
            response.next_cursor,
            Some(1_000_000_000 + response.entries.len() as u64)
        );
        server
            .ensure_structured_payload_fits(&response)
            .expect("final cursor also fits");
        let mut empty = FsListResponse {
            entries: vec![],
            next_cursor: None,
        };
        server
            .fit_list_response(0, 0, &mut empty)
            .expect("empty directory is valid");
        assert_eq!(empty.next_cursor, None);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[tokio::test]
    async fn admitted_read_workers_drain_on_revoke_after_cancel_timeout_success_or_error() {
        use std::sync::mpsc;

        for listing in [false, true] {
            for interruption in ["cancel", "timeout", "success", "error"] {
                let root = workspace("read-drain");
                fs::write(root.join("fixture.txt"), b"fixture").expect("fixture");
                let mut server = server(&root, &[Capability::FileRead, Capability::FileSearch]);
                server.transport_guard = TransportGuard::new(TransportLimits {
                    max_request_duration_ms: if interruption == "timeout" { 50 } else { 5_000 },
                    ..TransportLimits::from(server.limits)
                })
                .expect("transport");
                let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
                let (release_tx, release_rx) = mpsc::channel();
                let reader = server.clone();
                let request = tokio::spawn(async move {
                    reader
                        .run_admitted_read(move |server, grant, _permit| {
                            let effect = if listing {
                                Effect::FileSearch { root: None }
                            } else {
                                Effect::FileRead {
                                    path: WorkspacePath::parse("fixture.txt").expect("path"),
                                }
                            };
                            server.authorize(grant, effect, server.clock.now())?;
                            entered_tx.send(()).expect("reader entered");
                            release_rx
                                .recv_timeout(Duration::from_secs(5))
                                .expect("release worker");
                            if interruption == "error" {
                                return Err(ErrorData::internal_error("test.effect_failed", None));
                            }
                            if listing {
                                server
                                    .filesystem
                                    .list(None, 0, 8)
                                    .map_err(map_filesystem_error)?;
                            } else {
                                server
                                    .filesystem
                                    .read(
                                        &WorkspacePath::parse("fixture.txt").expect("path"),
                                        0,
                                        Some(8),
                                    )
                                    .map_err(map_filesystem_error)?;
                            }
                            Ok(())
                        })
                        .await
                });
                tokio::time::timeout(Duration::from_secs(5), entered_rx)
                    .await
                    .expect("start deadline")
                    .expect("start");
                let mut request = Some(request);
                if interruption == "cancel" {
                    let request = request.take().expect("request");
                    request.abort();
                    assert!(request.await.expect_err("cancelled").is_cancelled());
                } else if interruption == "timeout" {
                    let result = request
                        .take()
                        .expect("request")
                        .await
                        .expect("request task");
                    assert_eq!(
                        result.expect_err("timed out").message,
                        "optic.request_timeout"
                    );
                }
                let sessions = Arc::clone(&server.sessions);
                let session = server.session.clone();
                let revoke = tokio::task::spawn_blocking(move || sessions.revoke(&session));
                tokio::time::timeout(Duration::from_secs(5), async {
                    while server
                        .sessions
                        .get_active(&server.session, server.clock.now())
                        .is_ok()
                    {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .expect("revoke marks inactive");
                assert!(
                    !revoke.is_finished(),
                    "revoke must wait for the authorized effect"
                );
                assert_eq!(
                    server.transport_guard.active_requests(),
                    1,
                    "worker owns capacity after waiter cancellation"
                );
                release_tx.send(()).expect("release");
                if let Some(request) = request {
                    let result = request.await.expect("request task");
                    if interruption == "error" {
                        assert_eq!(
                            result.expect_err("effect failed").message,
                            "test.effect_failed"
                        );
                    } else {
                        result.expect("admitted effect completes");
                    }
                }
                tokio::time::timeout(Duration::from_secs(5), revoke)
                    .await
                    .expect("revoke deadline")
                    .expect("revoke task")
                    .expect("revoke");
                tokio::time::timeout(Duration::from_secs(5), async {
                    while server.transport_guard.active_requests() != 0 {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .expect("worker releases capacity");
                assert_eq!(
                    server
                        .run_admitted_read(|_, _, _| Ok(()))
                        .await
                        .expect_err("inactive")
                        .message,
                    "optic.session_inactive"
                );
                fs::remove_dir_all(root).expect("cleanup");
            }
        }
    }

    #[tokio::test]
    async fn owned_read_worker_transfers_original_capacity_beyond_worker_completion() {
        use optic_bridge_runtime::NativeGitReadOwnedPermits;

        let root = workspace("owned-read-transfer");
        let server = server(&root, &[Capability::GitRead]);
        let owned = server
            .run_owned_admitted_read(|_, admission, permit| {
                Ok(NativeGitReadOwnedPermits::new(admission, permit))
            })
            .await
            .expect("transfer without a second admission");
        assert_eq!(server.transport_guard.active_requests(), 1);
        let sessions = Arc::clone(&server.sessions);
        let session = server.session.clone();
        let revoke = tokio::task::spawn_blocking(move || sessions.revoke(&session));
        tokio::time::timeout(Duration::from_secs(5), async {
            while server
                .sessions
                .get_active(&server.session, server.clock.now())
                .is_ok()
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("revoke published");
        assert!(!revoke.is_finished(), "transferred admission still counted");
        assert_eq!(server.transport_guard.active_requests(), 1);
        // No native process exists in this fixture: dropping is safe here.
        drop(owned);
        tokio::time::timeout(Duration::from_secs(5), revoke)
            .await
            .expect("drain deadline")
            .expect("revoke task")
            .expect("revoke");
        assert_eq!(server.transport_guard.active_requests(), 0);
        assert_eq!(
            server
                .run_owned_admitted_read(|_, _, _| Ok(()))
                .await
                .expect_err("revoked session cannot reach operation")
                .message,
            "optic.session_inactive"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[tokio::test]
    async fn owned_read_worker_cannot_upgrade_legacy_git_authority_to_native() {
        use optic_bridge_runtime::{
            NativeGitReadAdmissionError, NativeGitReadLeaseError, NativeGitReadOwnedPermits,
            NativeGitReadTrustedContext, finish_owned_native_git_read_preflight,
        };

        let root = workspace("owned-read-deny-native");
        let server = server(&root, &[Capability::GitRead]);
        server
            .run_owned_admitted_read(|server, admission, permit| {
                let envelope = ActionEnvelope {
                    action_id: ActionId::generate().expect("entropy"),
                    session: server.session.clone(),
                    task_lease: None,
                    effect: Effect::GitRead,
                    resources: ResourceBudget {
                        timeout_ms: 100,
                        output_bytes: 1024,
                        memory_bytes: 4096,
                        process_count: 1,
                    },
                    policy_epoch: admission.grant().policy_epoch,
                };
                let trusted = NativeGitReadTrustedContext {
                    sessions: &server.sessions,
                    trusted_session: &server.session,
                    task_leases: &server.task_leases,
                    policy: &server.policy,
                };
                let result = finish_owned_native_git_read_preflight(
                    &trusted,
                    NativeGitReadOwnedPermits::new(admission, permit),
                    &envelope,
                    "unused-no-lease",
                    "unused-no-lease",
                    server.clock.now(),
                );
                assert!(matches!(
                    result,
                    Err(NativeGitReadAdmissionError::Preflight(
                        NativeGitReadLeaseError::LeaseRequired
                    ))
                ));
                Ok(())
            })
            .await
            .expect("negative preflight fixture");
        assert_eq!(server.transport_guard.active_requests(), 0);
        server
            .sessions
            .revoke(&server.session)
            .expect("no leaked admission");
        fs::remove_dir_all(root).expect("cleanup");
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

    #[cfg(windows)]
    #[tokio::test]
    async fn directory_only_mcp_authority_creates_scoped_leaves_and_stops_on_revocation() {
        use crate::mutation_tools::{DirectoryExpectedState, FsCreateDirectoryRequest};
        use optic_bridge_runtime::{MutationAuthoritySpec, mutation_resource_budget};
        let root = workspace("directory-only");
        let state = workspace("directory-state");
        let base = server(&root, &[Capability::DirectoryCreate]);
        assert!(
            !base
                .registered_tool_names()
                .contains(&"fs_create_directory".into())
        );
        let authority = MutationAuthoritySet::provision(
            &base.task_leases,
            &base.session,
            &MutationAuthoritySpec {
                directory_scopes: BTreeSet::from([optic_bridge_core::LeaseScope::WorkspacePrefix(
                    WorkspacePath::parse("src").unwrap(),
                )]),
                ..Default::default()
            },
            mutation_resource_budget(base.limits),
            MonotonicTime::from_millis(1000),
            1,
        )
        .unwrap();
        let lease = authority
            .lease_for(Capability::DirectoryCreate)
            .unwrap()
            .clone();
        let mutations = AuthorizedFileMutationService::from_hard_limits(
            &root,
            &state,
            base.limits,
            base.sessions.clone(),
            base.task_leases.clone(),
            base.clock.clone(),
        )
        .unwrap();
        let server = ReadonlyMcpServer::new_with_mutation_runtime(
            &root,
            base.sessions.clone(),
            base.session.clone(),
            base.clock.clone(),
            base.limits,
            base.processes.clone(),
            base.task_leases.clone(),
            BTreeMap::new(),
            Some(Arc::new(mutations)),
            authority,
        )
        .unwrap();
        let names = server.registered_tool_names();
        assert!(names.contains(&"fs_create_directory".into()));
        for name in ["fs_write", "fs_apply_patch", "fs_delete"] {
            assert!(!names.contains(&name.into()));
        }
        let request = |path: &str| {
            Parameters(FsCreateDirectoryRequest {
                path: path.into(),
                expected: DirectoryExpectedState::Absent,
            })
        };
        assert!(base.fs_create_directory(request("src")).await.is_err());
        assert!(
            server
                .fs_create_directory(request("src-other"))
                .await
                .is_err()
        );
        assert!(!root.join("src-other").exists());
        assert!(
            server
                .fs_create_directory(request("src"))
                .await
                .unwrap()
                .0
                .created
        );
        assert!(server.fs_create_directory(request("src")).await.is_err());
        assert!(
            server
                .fs_create_directory(request("src/日本語"))
                .await
                .unwrap()
                .0
                .created
        );
        assert!(root.join("src/日本語").is_dir());
        server.task_leases.revoke(&lease).unwrap();
        assert!(
            server
                .fs_create_directory(request("src/after-revoke"))
                .await
                .is_err()
        );
        assert!(!root.join("src/after-revoke").exists());
        drop(server);
        drop(base);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }
}
