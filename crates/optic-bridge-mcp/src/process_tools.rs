use base64::{Engine as _, engine::general_purpose::STANDARD};
use optic_bridge_core::{
    ActionEnvelope, ActionId, Effect, JobId, NetworkAccess, ResourceBudget, TaskLeaseId,
    WorkspacePath,
};
use optic_bridge_policy::{PolicyDecision, PolicyReason};
use optic_bridge_runtime::{
    ProcessError, ProcessStartSpec, ProcessStatus, ProcessStream, SessionLifecycleError,
    SessionLifecycleManager, TaskLeaseRegistryError,
};
use rmcp::{ErrorData, Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::server::ReadonlyMcpServer;

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
    ) -> Result<Json<ProcessStartResponse>, ErrorData> {
        if params.0.network.unwrap_or(false) {
            return Err(ErrorData::invalid_request(
                "optic.network_runtime_unavailable",
                None,
            ));
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
                executable,
                args: params.0.args,
                cwd,
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
        let lifecycle = SessionLifecycleManager::new(
            self.sessions.clone(),
            self.task_leases.clone(),
            self.processes.clone(),
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
        SessionLifecycleError::TaskLeaseRegistry(error) => map_task_lease_error(error),
        SessionLifecycleError::Process(error) => map_process_error(error),
        SessionLifecycleError::SessionStillActive => {
            ErrorData::invalid_request("optic.session_still_active", None)
        }
        SessionLifecycleError::ExpiredAtProvision | SessionLifecycleError::HandleGeneration(_) => {
            ErrorData::internal_error("optic.session_lifecycle_error", None)
        }
    }
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
        ProcessError::ProcessRecordLimitExceededForSession => {
            ErrorData::internal_error("optic.process_session_record_limit", None)
        }
        ProcessError::InvalidLimits(_)
        | ProcessError::RootNotDirectory
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
    }
}
