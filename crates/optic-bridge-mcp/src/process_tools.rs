use std::{collections::BTreeSet, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use optic_bridge_core::{
    ActionEnvelope, ActionId, Capability, Effect, JobId, LeaseScope, NetworkAccess, ResourceBudget,
    TaskLease, TaskLeaseId, ToolInvocation, WorkspacePath,
};
use optic_bridge_policy::{PolicyDecision, PolicyReason};
use optic_bridge_runtime::{
    ProcessError, ProcessStartSpec, ProcessStatus, ProcessStream, SessionLifecycleError,
    SessionLifecycleManager, TaskLeaseRegistryError, ToolProfileRegistryError,
};
use rmcp::{
    ErrorData, Json,
    handler::server::wrapper::Parameters,
    service::{RequestContext, RoleServer},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ApprovalAuthorizationError, ApprovalAuthorizationRuntime, authorize_action_with_human_approval,
    server::ReadonlyMcpServer,
};

const DEFAULT_PROCESS_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_PROCESS_OUTPUT_BYTES: u64 = 1024 * 1024;
const DEFAULT_PROCESS_MEMORY_BYTES: u64 = 512 * 1024 * 1024;
const DEFAULT_PROCESS_COUNT: u32 = 1;
const DEFAULT_PROCESS_READ_BYTES: u64 = 64 * 1024;
const MCP_PROCESS_ENVELOPE_RESERVE_BYTES: u64 = 12 * 1024;

#[tool_router(router = process_tool_router, vis = "pub")]
impl ReadonlyMcpServer {
    #[tool(
        name = "process_start",
        description = "Start one operator-authorized executable with structured arguments. Network access is unavailable in Phase 1C."
    )]
    pub async fn process_start(
        &self,
        params: Parameters<ProcessStartRequest>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ProcessStartResponse>, ErrorData> {
        if params.0.network.unwrap_or(false) {
            return Err(ErrorData::invalid_request(
                "optic.network_runtime_unavailable",
                None,
            ));
        }
        if self.tool_profiles.is_some() {
            return self.process_start_profiled(params.0, context).await;
        }

        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(super::server::map_transport_error)?;
        let admission = self
            .sessions
            .begin_admission(&self.session, now)
            .map_err(super::server::map_session_error)?;
        let grant = admission.grant();
        let executable = self
            .processes
            .canonicalize_executable(&params.0.executable)
            .map_err(map_process_error)?;
        let lease_id = self
            .process_leases
            .get(&executable)
            .cloned()
            .ok_or_else(|| {
                ErrorData::invalid_request("optic.process_executable_not_allowed", None)
            })?;
        let lease = self
            .task_leases
            .get_active(&lease_id, &self.session, now)
            .map_err(map_task_lease_error)?;
        let class = lease.process_execution_class(&executable).ok_or_else(|| {
            ErrorData::internal_error("optic.process_execution_class_unavailable", None)
        })?;
        let resources = requested_budget(&params.0)?;
        let effect = Effect::ProcessRun {
            executable: executable.clone(),
            class,
            network: NetworkAccess::Denied,
        };
        authorize_process(self, grant, &lease.id, &lease, effect, resources, now)?;
        let workspace_read_files =
            process_workspace_read_files(grant.allows(Capability::FileRead), &lease);
        let cwd = params
            .0
            .cwd
            .as_deref()
            .map(parse_workspace_path)
            .transpose()?;
        let job_id = self
            .processes
            .start(ProcessStartSpec {
                session: self.session.clone(),
                class,
                workload_class: lease.workload_class,
                network: NetworkAccess::Denied,
                executable,
                args: params.0.args,
                cwd,
                workspace_read_files,
                env_allowlist: params.0.env_allowlist,
                resources,
            })
            .map_err(map_process_error)?;
        drop(admission);
        let response = ProcessStartResponse {
            job_id: job_id.to_token(),
        };
        self.ensure_structured_payload_fits(&response)?;
        drop(permit);
        Ok(Json(response))
    }

    async fn process_start_profiled(
        &self,
        request: ProcessStartRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ProcessStartResponse>, ErrorData> {
        let profiles = self.tool_profiles.as_ref().ok_or_else(|| {
            ErrorData::internal_error("optic.process_profile_runtime_missing", None)
        })?;
        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(super::server::map_transport_error)?;
        let grant = self.active_grant(now)?;
        let executable = self
            .processes
            .canonicalize_executable(&request.executable)
            .map_err(map_process_error)?;
        let lease_id = self
            .process_leases
            .get(&executable)
            .cloned()
            .ok_or_else(|| {
                ErrorData::invalid_request("optic.process_executable_not_allowed", None)
            })?;
        let lease = self
            .task_leases
            .get_active(&lease_id, &self.session, now)
            .map_err(map_task_lease_error)?;
        let class = lease.process_execution_class(&executable).ok_or_else(|| {
            ErrorData::internal_error("optic.process_execution_class_unavailable", None)
        })?;
        let resources = requested_budget(&request)?;
        let workspace_read_files =
            process_workspace_read_files(grant.allows(Capability::FileRead), &lease);
        let cwd = request
            .cwd
            .as_deref()
            .map(parse_workspace_path)
            .transpose()?;
        let env_allowlist = self
            .processes
            .normalize_environment_allowlist(&request.env_allowlist)
            .map_err(map_process_error)?;
        let invocation = ToolInvocation {
            executable: executable.clone(),
            class,
            workload_class: lease.workload_class,
            args: request.args,
            cwd,
            workspace_read_files: workspace_read_files.into_iter().collect::<BTreeSet<_>>(),
            env_allowlist,
            network: NetworkAccess::Denied,
            resources,
        };
        let profile = profiles
            .resolve_unique(&invocation)
            .map_err(map_tool_profile_registry_error)?;
        let action_id = ActionId::generate()
            .map_err(|_| ErrorData::internal_error("optic.action_id_unavailable", None))?;
        let envelope = ActionEnvelope {
            action_id,
            session: self.session.clone(),
            task_lease: Some(lease.id.clone()),
            effect: Effect::ProfiledProcessRun {
                profile: profile.name().clone(),
                executable: executable.clone(),
                class,
                network: NetworkAccess::Denied,
                approval: profile.approval_requirement(),
            },
            resources,
            policy_epoch: grant.policy_epoch,
        };
        let approval_timeout_ms = permit
            .deadline()
            .as_millis()
            .saturating_sub(now.as_millis())
            .max(1);
        let message = profiled_process_approval_message(&profile, &invocation);
        let admission = authorize_action_with_human_approval(
            &context,
            ApprovalAuthorizationRuntime::new(
                &self.sessions,
                &self.task_leases,
                &self.approvals,
                &self.policy,
                self.clock.as_ref(),
            ),
            &envelope,
            message,
            Duration::from_millis(approval_timeout_ms),
        )
        .await
        .map_err(map_approval_authorization_error)?;

        let revalidated = profiles
            .resolve_unique(&invocation)
            .map_err(map_tool_profile_registry_error)?;
        if revalidated.name() != profile.name() {
            return Err(ErrorData::invalid_request(
                "optic.process_profile_changed",
                None,
            ));
        }

        let job_id = self
            .processes
            .start(ProcessStartSpec {
                session: self.session.clone(),
                class,
                workload_class: invocation.workload_class,
                network: invocation.network,
                executable: invocation.executable,
                args: invocation.args,
                cwd: invocation.cwd,
                workspace_read_files: invocation.workspace_read_files.into_iter().collect(),
                env_allowlist: invocation.env_allowlist.into_iter().collect(),
                resources: invocation.resources,
            })
            .map_err(map_process_error)?;
        drop(admission);
        let response = ProcessStartResponse {
            job_id: job_id.to_token(),
        };
        self.ensure_structured_payload_fits(&response)?;
        drop(permit);
        Ok(Json(response))
    }

    #[tool(
        name = "process_read",
        description = "Read a bounded stdout or stderr chunk from a process job owned by the current application session."
    )]
    pub async fn process_read(
        &self,
        params: Parameters<ProcessReadRequest>,
    ) -> Result<Json<ProcessReadResponse>, ErrorData> {
        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(super::server::map_transport_error)?;
        self.active_grant(now)?;
        let job_id = parse_job_id(&params.0.job_id)?;
        let max_read = self.max_mcp_process_read_bytes();
        let max_bytes = params
            .0
            .max_bytes
            .unwrap_or(DEFAULT_PROCESS_READ_BYTES.min(max_read));
        if max_bytes == 0 || max_bytes > max_read {
            return Err(ErrorData::invalid_params(
                "optic.process_read_limit_exceeded",
                None,
            ));
        }
        let stream = match params.0.stream {
            ProcessStreamRequest::Stdout => ProcessStream::Stdout,
            ProcessStreamRequest::Stderr => ProcessStream::Stderr,
        };
        let chunk = self
            .processes
            .read(
                &self.session,
                &job_id,
                stream,
                params.0.cursor.unwrap_or(0),
                max_bytes,
            )
            .map_err(map_process_error)?;
        let response = ProcessReadResponse {
            encoding: "base64".to_owned(),
            data: STANDARD.encode(chunk.bytes),
            offset: chunk.offset,
            next_offset: chunk.next_offset,
            eof: chunk.eof,
            truncated: chunk.truncated,
        };
        self.ensure_structured_payload_fits(&response)?;
        drop(permit);
        Ok(Json(response))
    }

    #[tool(
        name = "process_stop",
        description = "Request termination of one process job owned by the current application session. Arbitrary PIDs are never accepted."
    )]
    pub async fn process_stop(
        &self,
        params: Parameters<ProcessJobRequest>,
    ) -> Result<Json<ProcessStopResponse>, ErrorData> {
        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(super::server::map_transport_error)?;
        self.active_grant(now)?;
        let job_id = parse_job_id(&params.0.job_id)?;
        let requested = self
            .processes
            .stop(&self.session, &job_id)
            .map_err(map_process_error)?;
        let response = ProcessStopResponse { requested };
        self.ensure_structured_payload_fits(&response)?;
        drop(permit);
        Ok(Json(response))
    }

    #[tool(
        name = "process_result",
        description = "Return bounded lifecycle metadata for one process job owned by the current application session."
    )]
    pub async fn process_result(
        &self,
        params: Parameters<ProcessJobRequest>,
    ) -> Result<Json<ProcessResultResponse>, ErrorData> {
        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(super::server::map_transport_error)?;
        self.active_grant(now)?;
        let job_id = parse_job_id(&params.0.job_id)?;
        let result = self
            .processes
            .result(&self.session, &job_id)
            .map_err(map_process_error)?;
        let response = ProcessResultResponse {
            status: process_status_name(result.status).to_owned(),
            exit_code: result.exit_code,
            output_truncated: result.output_truncated,
        };
        self.ensure_structured_payload_fits(&response)?;
        drop(permit);
        Ok(Json(response))
    }

    #[tool(
        name = "session_cancel",
        description = "Revoke the current application session and its task leases, then request termination of all owned process jobs."
    )]
    pub async fn session_cancel(&self) -> Result<Json<SessionCancelResponse>, ErrorData> {
        let now = self.clock.now();
        let permit = self
            .transport_guard
            .begin_execution(now)
            .map_err(super::server::map_transport_error)?;
        let lifecycle = SessionLifecycleManager::new_with_approval_broker(
            self.sessions.clone(),
            self.task_leases.clone(),
            self.processes.clone(),
            self.approvals.clone(),
        );
        let report = lifecycle
            .revoke(&self.session)
            .map_err(map_session_lifecycle_error)?;
        let response = SessionCancelResponse {
            revoked_leases: u64::try_from(report.revoked_leases).unwrap_or(u64::MAX),
            cancelled_jobs: u64::try_from(report.cancellation_requests).unwrap_or(u64::MAX),
        };
        self.ensure_structured_payload_fits(&response)?;
        drop(permit);
        Ok(Json(response))
    }

    fn max_mcp_process_read_bytes(&self) -> u64 {
        let payload = self
            .limits
            .max_response_bytes
            .saturating_sub(MCP_PROCESS_ENVELOPE_RESERVE_BYTES);
        let binary = payload.saturating_div(4).saturating_mul(3);
        self.limits.max_process_read_bytes.min(binary)
    }
}

fn authorize_process(
    server: &ReadonlyMcpServer,
    grant: &optic_bridge_core::SessionGrant,
    lease_id: &TaskLeaseId,
    lease: &optic_bridge_core::TaskLease,
    effect: Effect,
    resources: ResourceBudget,
    now: optic_bridge_core::MonotonicTime,
) -> Result<(), ErrorData> {
    let action_id = ActionId::generate()
        .map_err(|_| ErrorData::internal_error("optic.action_id_unavailable", None))?;
    let envelope = ActionEnvelope {
        action_id,
        session: server.session.clone(),
        task_lease: Some(lease_id.clone()),
        effect,
        resources,
        policy_epoch: grant.policy_epoch,
    };
    map_process_policy_decision(server.policy.evaluate(&envelope, grant, Some(lease), now))
}

fn map_process_policy_decision(decision: PolicyDecision) -> Result<(), ErrorData> {
    match decision {
        PolicyDecision::Allow => Ok(()),
        PolicyDecision::Deny(PolicyReason::ProcessIsolationRequired) => Err(
            ErrorData::invalid_request("optic.process_isolation_unavailable", None),
        ),
        PolicyDecision::RequireApproval(_) | PolicyDecision::Deny(_) => {
            Err(ErrorData::invalid_request("optic.policy_denied", None))
        }
    }
}

fn requested_budget(request: &ProcessStartRequest) -> Result<ResourceBudget, ErrorData> {
    ResourceBudget {
        timeout_ms: request.timeout_ms.unwrap_or(DEFAULT_PROCESS_TIMEOUT_MS),
        output_bytes: request
            .output_budget
            .unwrap_or(DEFAULT_PROCESS_OUTPUT_BYTES),
        memory_bytes: request.memory_bytes.unwrap_or(DEFAULT_PROCESS_MEMORY_BYTES),
        process_count: request.process_count.unwrap_or(DEFAULT_PROCESS_COUNT),
    }
    .validate_nonzero()
    .map_err(|_| ErrorData::invalid_params("optic.invalid_process_budget", None))
}

fn parse_workspace_path(value: &str) -> Result<WorkspacePath, ErrorData> {
    WorkspacePath::parse(value)
        .map_err(|_| ErrorData::invalid_params("optic.invalid_workspace_path", None))
}

fn parse_job_id(value: &str) -> Result<JobId, ErrorData> {
    JobId::from_token(value).map_err(|_| ErrorData::invalid_params("optic.invalid_job_id", None))
}

fn map_task_lease_error(error: TaskLeaseRegistryError) -> ErrorData {
    match error {
        TaskLeaseRegistryError::ProcessIdentityChanged => {
            ErrorData::invalid_request("optic.process_executable_identity_changed", None)
        }
        TaskLeaseRegistryError::ProcessIdentityUnavailable => {
            ErrorData::invalid_request("optic.process_executable_identity_unavailable", None)
        }
        TaskLeaseRegistryError::StateUnavailable
        | TaskLeaseRegistryError::AlreadyRegistered
        | TaskLeaseRegistryError::CapacityExceeded
        | TaskLeaseRegistryError::SessionCapacityExceeded
        | TaskLeaseRegistryError::UnknownLease
        | TaskLeaseRegistryError::Revoked
        | TaskLeaseRegistryError::WrongSession
        | TaskLeaseRegistryError::Expired
        | TaskLeaseRegistryError::LeaseStillActive => {
            ErrorData::invalid_request("optic.task_lease_inactive", None)
        }
    }
}

fn map_session_lifecycle_error(error: SessionLifecycleError) -> ErrorData {
    match error {
        SessionLifecycleError::SessionRegistry(error) => super::server::map_session_error(error),
        SessionLifecycleError::ApprovalBroker(_) => {
            ErrorData::internal_error("optic.approval_broker_unavailable", None)
        }
        SessionLifecycleError::TaskLeaseRegistry(error) => map_task_lease_error(error),
        SessionLifecycleError::Process(error) => map_process_error(error),
        SessionLifecycleError::SessionStillActive => {
            ErrorData::invalid_request("optic.session_still_active", None)
        }
        SessionLifecycleError::ExpiredAtProvision
        | SessionLifecycleError::SessionWorktree(_)
        | SessionLifecycleError::HandleGeneration(_)
        | SessionLifecycleError::RenewalExpiryNotFuture
        | SessionLifecycleError::RenewalMustExtend
        | SessionLifecycleError::RenewalBeyondHorizon
        | SessionLifecycleError::RenewalSessionInactive
        | SessionLifecycleError::RenewalLeaseSetInvalid => {
            ErrorData::internal_error("optic.session_lifecycle_error", None)
        }
    }
}

fn profiled_process_approval_message(
    profile: &optic_bridge_core::ToolProfile,
    invocation: &ToolInvocation,
) -> String {
    let cwd = invocation
        .cwd
        .as_ref()
        .map_or("<workspace-root>", WorkspacePath::as_str);
    format!(
        "Approve exact tool profile '{}'\nExecutable: {}\nClass: {:?}\nWorkload: {:?}\nArgs: {:?}\nCwd: {}\nWorkspace read files: {:?}\nEnvironment allowlist: {:?}\nNetwork: {:?}\nRequested resources: timeout={}ms output={}B memory={}B process_count={}",
        profile.name().as_str(),
        invocation.executable,
        invocation.class,
        invocation.workload_class,
        invocation.args,
        cwd,
        invocation.workspace_read_files,
        invocation.env_allowlist,
        invocation.network,
        invocation.resources.timeout_ms,
        invocation.resources.output_bytes,
        invocation.resources.memory_bytes,
        invocation.resources.process_count,
    )
}

fn map_tool_profile_registry_error(error: ToolProfileRegistryError) -> ErrorData {
    match error {
        ToolProfileRegistryError::NoMatchingProfile
        | ToolProfileRegistryError::UnknownProfile
        | ToolProfileRegistryError::InvocationMismatch => {
            ErrorData::invalid_request("optic.process_profile_not_authorized", None)
        }
        ToolProfileRegistryError::AmbiguousProfile => {
            ErrorData::internal_error("optic.process_profile_ambiguous", None)
        }
        ToolProfileRegistryError::InvalidLimits
        | ToolProfileRegistryError::StateUnavailable
        | ToolProfileRegistryError::AlreadyRegistered
        | ToolProfileRegistryError::CapacityExceeded => {
            ErrorData::internal_error("optic.process_profile_runtime_error", None)
        }
    }
}

fn map_approval_authorization_error(error: ApprovalAuthorizationError) -> ErrorData {
    match error {
        ApprovalAuthorizationError::PolicyDenied(PolicyReason::ProcessIsolationRequired) => {
            ErrorData::invalid_request("optic.process_isolation_unavailable", None)
        }
        ApprovalAuthorizationError::PolicyDenied(_) => {
            ErrorData::invalid_request("optic.policy_denied", None)
        }
        ApprovalAuthorizationError::Session(error) => super::server::map_session_error(error),
        ApprovalAuthorizationError::TaskLease(error) => map_task_lease_error(error),
        ApprovalAuthorizationError::Declined => {
            ErrorData::invalid_request("optic.approval_declined", None)
        }
        ApprovalAuthorizationError::Cancelled => {
            ErrorData::invalid_request("optic.approval_cancelled", None)
        }
        ApprovalAuthorizationError::Unsupported => {
            ErrorData::invalid_request("optic.approval_unsupported", None)
        }
        ApprovalAuthorizationError::PolicyChanged => {
            ErrorData::invalid_request("optic.approval_stale", None)
        }
        ApprovalAuthorizationError::Elicitation(_) => {
            ErrorData::internal_error("optic.approval_transport_error", None)
        }
        ApprovalAuthorizationError::Broker(_) => {
            ErrorData::internal_error("optic.approval_broker_error", None)
        }
    }
}

fn process_workspace_read_files(
    session_allows_file_read: bool,
    lease: &TaskLease,
) -> Vec<WorkspacePath> {
    if !session_allows_file_read || !lease.allows(Capability::FileRead) {
        return Vec::new();
    }
    lease
        .scopes
        .iter()
        .filter_map(|scope| match scope {
            LeaseScope::WorkspacePrefix(path) => Some(path.clone()),
            _ => None,
        })
        .collect()
}

fn map_process_error(error: ProcessError) -> ErrorData {
    match error {
        ProcessError::ExecutableMustBeAbsolute => {
            ErrorData::invalid_params("optic.process_executable_must_be_absolute", None)
        }
        ProcessError::ExecutableNotFile | ProcessError::NonUtf8Executable => {
            ErrorData::invalid_params("optic.invalid_process_executable", None)
        }
        ProcessError::CwdOutsideWorkspace
        | ProcessError::CwdNotDirectory
        | ProcessError::NonUtf8WorkingDirectory => {
            ErrorData::invalid_params("optic.invalid_process_cwd", None)
        }
        #[cfg(windows)]
        ProcessError::PinnedWorkingDirectory(_) => {
            ErrorData::invalid_params("optic.invalid_process_cwd", None)
        }
        ProcessError::InvalidEnvironmentName | ProcessError::EnvironmentNotAllowed => {
            ErrorData::invalid_params("optic.process_environment_not_allowed", None)
        }
        ProcessError::ResourceBudgetExceeded
        | ProcessError::OutputMemoryLimitExceeded
        | ProcessError::RequestShapeTooLarge
        | ProcessError::TooManyProcessArguments => {
            ErrorData::invalid_params("optic.process_budget_exceeded", None)
        }
        ProcessError::OutputMemoryLimitExceededForSession => {
            ErrorData::invalid_params("optic.process_session_output_memory_limit", None)
        }
        ProcessError::ProcessMemoryCapacityExceeded => {
            ErrorData::internal_error("optic.process_memory_capacity_exceeded", None)
        }
        ProcessError::ProcessMemoryCapacityExceededForSession => {
            ErrorData::internal_error("optic.process_session_memory_capacity_exceeded", None)
        }
        ProcessError::HostMemoryUnavailable => {
            ErrorData::internal_error("optic.process_host_memory_unavailable", None)
        }
        ProcessError::HostMemoryHeadroomExceeded => {
            ErrorData::internal_error("optic.process_host_memory_headroom_exceeded", None)
        }
        ProcessError::CpuCapacityExceeded => {
            ErrorData::internal_error("optic.process_cpu_capacity_exceeded", None)
        }
        ProcessError::CpuCapacityExceededForSession => {
            ErrorData::internal_error("optic.process_session_cpu_capacity_exceeded", None)
        }
        ProcessError::ReadLimitExceeded => {
            ErrorData::invalid_params("optic.process_read_limit_exceeded", None)
        }
        ProcessError::CursorOutOfRange => {
            ErrorData::invalid_params("optic.process_cursor_out_of_range", None)
        }
        ProcessError::UnknownJob => ErrorData::invalid_params("optic.process_job_not_found", None),
        ProcessError::IsolationUnavailable | ProcessError::IsolationEnvironmentUnavailable(_) => {
            ErrorData::invalid_request("optic.process_isolation_unavailable", None)
        }
        ProcessError::TooManyActiveJobs | ProcessError::ProcessRecordLimitExceeded => {
            ErrorData::internal_error("optic.process_runtime_busy", None)
        }
        ProcessError::TooManyActiveJobsForSession => {
            ErrorData::internal_error("optic.process_session_active_limit", None)
        }
        ProcessError::HeavyWorkloadCapacityExceeded => {
            ErrorData::internal_error("optic.process_heavy_workload_capacity_exceeded", None)
        }
        ProcessError::HeavyWorkloadCapacityExceededForSession => ErrorData::internal_error(
            "optic.process_session_heavy_workload_capacity_exceeded",
            None,
        ),
        ProcessError::ProcessRecordLimitExceededForSession => {
            ErrorData::internal_error("optic.process_session_record_limit", None)
        }
        ProcessError::InvalidLimits(_)
        | ProcessError::ConflictingEnvironmentGrant
        | ProcessError::RootNotDirectory
        | ProcessError::WorkspaceReadGrantsRequireIsolation
        | ProcessError::TooManyWorkspaceReadGrants
        | ProcessError::WorkspaceReadGrantOutsideWorkspace
        | ProcessError::WorkspaceReadGrantNotFile
        | ProcessError::NonUtf8WorkspaceRoot
        | ProcessError::NonUtf8WorkspaceReadGrant
        | ProcessError::DuplicateWorkspaceReadGrant
        | ProcessError::IsolationLauncherMustBeAbsolute
        | ProcessError::IsolationLauncherNotFile
        | ProcessError::IsolationLauncherProtocol(_)
        | ProcessError::RuntimeUnavailable
        | ProcessError::JobIdUnavailable
        | ProcessError::StateUnavailable
        | ProcessError::Io(_) => ErrorData::internal_error("optic.process_runtime_error", None),
    }
}

fn process_status_name(status: ProcessStatus) -> &'static str {
    match status {
        ProcessStatus::Running => "running",
        ProcessStatus::Exited => "exited",
        ProcessStatus::Stopped => "stopped",
        ProcessStatus::TimedOut => "timed_out",
        ProcessStatus::OutputLimitExceeded => "output_limit_exceeded",
        ProcessStatus::TerminationUncertain => "termination_uncertain",
        ProcessStatus::Failed => "failed",
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessStartRequest {
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env_allowlist: Vec<String>,
    pub network: Option<bool>,
    pub timeout_ms: Option<u64>,
    pub output_budget: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub process_count: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ProcessStartResponse {
    pub job_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStreamRequest {
    Stdout,
    Stderr,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessReadRequest {
    pub job_id: String,
    pub stream: ProcessStreamRequest,
    pub cursor: Option<u64>,
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ProcessReadResponse {
    pub encoding: String,
    pub data: String,
    pub offset: u64,
    pub next_offset: u64,
    pub eof: bool,
    pub truncated: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessJobRequest {
    pub job_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ProcessStopResponse {
    pub requested: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ProcessResultResponse {
    pub status: String,
    pub exit_code: Option<i32>,
    pub output_truncated: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SessionCancelResponse {
    pub revoked_leases: u64,
    pub cancelled_jobs: u64,
}

#[cfg(test)]
mod tests {
    use optic_bridge_core::HardLimits;

    use super::*;

    #[test]
    fn process_workspace_read_grants_require_file_read_and_exact_prefix_scopes() {
        let exact = WorkspacePath::parse("input.txt").expect("workspace path");
        let mut lease = TaskLease {
            id: TaskLeaseId::generate().expect("lease id"),
            session: optic_bridge_core::SessionHandle::generate().expect("session"),
            capabilities: std::collections::BTreeSet::from([Capability::ProcessRun]),
            scopes: std::collections::BTreeSet::from([
                LeaseScope::WorkspaceAll,
                LeaseScope::WorkspacePrefix(exact.clone()),
            ]),
            resource_ceiling: HardLimits::default().max_process_budget,
            workload_class: optic_bridge_core::WorkloadClass::Standard,
            expires_at: optic_bridge_core::MonotonicTime::from_millis(1_000),
            policy_epoch: 1,
        };
        assert!(process_workspace_read_files(true, &lease).is_empty());

        lease.capabilities.insert(Capability::FileRead);
        assert!(process_workspace_read_files(false, &lease).is_empty());
        assert_eq!(process_workspace_read_files(true, &lease), vec![exact]);
    }

    #[test]
    fn network_requests_are_not_silently_enabled() {
        let request = ProcessStartRequest {
            executable: "x".to_owned(),
            args: Vec::new(),
            cwd: None,
            env_allowlist: Vec::new(),
            network: Some(true),
            timeout_ms: None,
            output_budget: None,
            memory_bytes: None,
            process_count: None,
        };
        assert!(request.network.unwrap_or(false));
    }

    #[test]
    fn default_budget_is_nonzero_and_bounded_shape() {
        let request = ProcessStartRequest {
            executable: "x".to_owned(),
            args: Vec::new(),
            cwd: None,
            env_allowlist: Vec::new(),
            network: None,
            timeout_ms: None,
            output_budget: None,
            memory_bytes: None,
            process_count: None,
        };
        let budget = requested_budget(&request).expect("default budget");
        assert!(budget.timeout_ms > 0);
        assert!(budget.output_bytes > 0);
        assert!(budget.memory_bytes > 0);
        assert!(budget.process_count > 0);
    }

    #[test]
    fn default_budget_fits_default_session_output_ceiling() {
        let request = ProcessStartRequest {
            executable: "x".to_owned(),
            args: Vec::new(),
            cwd: None,
            env_allowlist: Vec::new(),
            network: None,
            timeout_ms: None,
            output_budget: None,
            memory_bytes: None,
            process_count: None,
        };
        let budget = requested_budget(&request).expect("default budget");
        let limits = HardLimits::default()
            .validate_nonzero()
            .expect("default limits");
        assert!(budget.fits_within(limits.max_process_budget));
        assert!(budget.output_bytes <= limits.max_active_output_ram_bytes_per_session);
    }

    #[test]
    fn lifecycle_session_errors_preserve_inactive_mcp_code() {
        let error = map_session_lifecycle_error(SessionLifecycleError::SessionRegistry(
            optic_bridge_runtime::SessionRegistryError::Revoked,
        ));
        assert_eq!(error.message, "optic.session_inactive");
    }

    #[test]
    fn process_identity_errors_have_distinct_mcp_codes() {
        assert_eq!(
            map_task_lease_error(TaskLeaseRegistryError::ProcessIdentityChanged).message,
            "optic.process_executable_identity_changed"
        );
        assert_eq!(
            map_task_lease_error(TaskLeaseRegistryError::ProcessIdentityUnavailable).message,
            "optic.process_executable_identity_unavailable"
        );
        assert_eq!(
            map_task_lease_error(TaskLeaseRegistryError::Revoked).message,
            "optic.task_lease_inactive"
        );
    }

    #[test]
    fn process_isolation_policy_has_distinct_mcp_code() {
        let error = map_process_policy_decision(PolicyDecision::Deny(
            PolicyReason::ProcessIsolationRequired,
        ))
        .expect_err("high-risk process should fail closed");
        assert_eq!(error.message, "optic.process_isolation_unavailable");
        assert_eq!(
            map_process_error(ProcessError::IsolationUnavailable).message,
            "optic.process_isolation_unavailable"
        );
        assert_eq!(
            map_process_error(ProcessError::IsolationEnvironmentUnavailable("SystemRoot")).message,
            "optic.process_isolation_unavailable"
        );

        let generic =
            map_process_policy_decision(PolicyDecision::Deny(PolicyReason::ScopeNotAuthorized))
                .expect_err("generic denial");
        assert_eq!(generic.message, "optic.policy_denied");
    }

    #[test]
    fn aggregate_memory_capacity_has_distinct_mcp_code() {
        assert_eq!(
            map_process_error(ProcessError::ProcessMemoryCapacityExceeded).message,
            "optic.process_memory_capacity_exceeded"
        );
    }

    #[test]
    fn host_memory_headroom_errors_have_distinct_mcp_codes() {
        assert_eq!(
            map_process_error(ProcessError::HostMemoryUnavailable).message,
            "optic.process_host_memory_unavailable"
        );
        assert_eq!(
            map_process_error(ProcessError::HostMemoryHeadroomExceeded).message,
            "optic.process_host_memory_headroom_exceeded"
        );
    }

    #[test]
    fn heavy_workload_capacity_errors_have_distinct_mcp_codes() {
        assert_eq!(
            map_process_error(ProcessError::HeavyWorkloadCapacityExceeded).message,
            "optic.process_heavy_workload_capacity_exceeded"
        );
        assert_eq!(
            map_process_error(ProcessError::HeavyWorkloadCapacityExceededForSession).message,
            "optic.process_session_heavy_workload_capacity_exceeded"
        );
    }

    #[test]
    fn session_resource_errors_have_distinct_mcp_codes() {
        assert_eq!(
            map_process_error(ProcessError::TooManyActiveJobsForSession).message,
            "optic.process_session_active_limit"
        );
        assert_eq!(
            map_process_error(ProcessError::ProcessRecordLimitExceededForSession).message,
            "optic.process_session_record_limit"
        );
        assert_eq!(
            map_process_error(ProcessError::OutputMemoryLimitExceededForSession).message,
            "optic.process_session_output_memory_limit"
        );
        assert_eq!(
            map_process_error(ProcessError::ProcessMemoryCapacityExceededForSession).message,
            "optic.process_session_memory_capacity_exceeded"
        );
    }

    #[test]
    fn profile_resolution_errors_have_fail_closed_mcp_codes() {
        assert_eq!(
            map_tool_profile_registry_error(ToolProfileRegistryError::NoMatchingProfile).message,
            "optic.process_profile_not_authorized"
        );
        assert_eq!(
            map_tool_profile_registry_error(ToolProfileRegistryError::AmbiguousProfile).message,
            "optic.process_profile_ambiguous"
        );
    }

    #[test]
    fn profiled_approval_message_contains_the_complete_invocation_contract() {
        let profile =
            optic_bridge_core::ToolProfile::from_spec(optic_bridge_core::ToolProfileSpec {
                name: optic_bridge_core::ToolProfileName::parse("node-version").expect("profile"),
                executable: "C:/Tools/node.exe".to_owned(),
                class: optic_bridge_core::ProcessExecutionClass::Interpreter,
                workload_class: optic_bridge_core::WorkloadClass::Heavy,
                exact_args: vec!["--version".to_owned()],
                cwd: Some(WorkspacePath::parse("scratch").expect("cwd")),
                workspace_read_files: BTreeSet::from([
                    WorkspacePath::parse("package.json").expect("read file")
                ]),
                env_allowlist: BTreeSet::from(["PATH".to_owned()]),
                network: NetworkAccess::Denied,
                resource_ceiling: ResourceBudget {
                    timeout_ms: 5_000,
                    output_bytes: 1_024,
                    memory_bytes: 64 * 1024 * 1024,
                    process_count: 1,
                },
                approval: optic_bridge_core::ToolApprovalRequirement::HumanRequired,
            })
            .expect("profile");
        let invocation = ToolInvocation {
            executable: "C:/Tools/node.exe".to_owned(),
            class: optic_bridge_core::ProcessExecutionClass::Interpreter,
            workload_class: optic_bridge_core::WorkloadClass::Heavy,
            args: vec!["--version".to_owned()],
            cwd: Some(WorkspacePath::parse("scratch").expect("cwd")),
            workspace_read_files: BTreeSet::from([
                WorkspacePath::parse("package.json").expect("read file")
            ]),
            env_allowlist: BTreeSet::from(["PATH".to_owned()]),
            network: NetworkAccess::Denied,
            resources: ResourceBudget {
                timeout_ms: 2_000,
                output_bytes: 512,
                memory_bytes: 32 * 1024 * 1024,
                process_count: 1,
            },
        };
        let message = profiled_process_approval_message(&profile, &invocation);
        for expected in [
            "node-version",
            "C:/Tools/node.exe",
            "--version",
            "scratch",
            "package.json",
            "PATH",
            "Denied",
            "timeout=2000ms",
            "output=512B",
        ] {
            assert!(
                message.contains(expected),
                "missing {expected:?}: {message}"
            );
        }
    }
}
