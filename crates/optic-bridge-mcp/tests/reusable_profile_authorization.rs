use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use optic_bridge_core::{
    ActionEnvelope, ActionId, Capability, Effect, HardLimits, LeaseScope, MonotonicTime,
    NetworkAccess, PrincipalId, ProcessExecutionClass, ProjectId, ResourceBudget, SessionGrant,
    SessionHandle, TaskLease, TaskLeaseId, ToolApprovalRequirement, ToolProfile, ToolProfileName,
    ToolProfileSpec, WorkloadClass,
};
use optic_bridge_mcp::{
    ApprovalAuthorizationError, ApprovalAuthorizationRuntime,
    authorize_profiled_action_with_reusable_or_human_approval,
};
use optic_bridge_policy::PolicyEngine;
use optic_bridge_runtime::{
    ApprovalBroker, Clock, ReusableApprovalBroker, SessionRegistry, SessionReusableApprovalService,
    TaskLeaseRegistry,
};
use rmcp::{
    ClientHandler, ErrorData, Json, ServerHandler, ServiceExt,
    handler::server::router::tool::ToolRouter,
    model::{
        CallToolRequestParams, ClientCapabilities, ClientConfig, ElicitRequestParams, ElicitResult,
        ElicitationAction, Implementation,
    },
    service::{RequestContext, RoleClient, RoleServer},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
struct FixedClock(MonotonicTime);

impl Clock for FixedClock {
    fn now(&self) -> MonotonicTime {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct GateResponse {
    authorized: bool,
}

#[derive(Clone)]
struct ApprovalClient {
    action: ElicitationAction,
    content: Option<Value>,
    request_count: Arc<Mutex<usize>>,
}

impl ApprovalClient {
    fn new(action: ElicitationAction, content: Option<Value>) -> Self {
        Self {
            action,
            content,
            request_count: Arc::new(Mutex::new(0)),
        }
    }

    fn cancel() -> Self {
        Self::new(ElicitationAction::Cancel, None)
    }

    fn accept_scope(scope: &str) -> Self {
        Self::new(
            ElicitationAction::Accept,
            Some(serde_json::json!({"approval_scope":scope})),
        )
    }
}

impl ClientHandler for ApprovalClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            ClientCapabilities::builder().enable_elicitation().build(),
            Implementation::new("optic-reusable-profile-test", "1.0.0"),
        )
    }

    async fn create_elicitation(
        &self,
        _request: ElicitRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<ElicitResult, ErrorData> {
        *self.request_count.lock().expect("request count") += 1;
        let mut result = ElicitResult::new(self.action.clone());
        if let Some(content) = &self.content {
            result = result.with_content(content.clone());
        }
        Ok(result)
    }
}

#[derive(Clone)]
struct ProfileGateServer {
    tool_router: ToolRouter<Self>,
    sessions: Arc<SessionRegistry>,
    leases: Arc<TaskLeaseRegistry>,
    approvals: Arc<ApprovalBroker>,
    policy: Arc<PolicyEngine>,
    clock: Arc<FixedClock>,
    profile: ToolProfile,
    envelope: ActionEnvelope,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ProfileGateServer {}

#[tool_router(router = profile_gate_router)]
impl ProfileGateServer {
    #[tool(
        name = "profile_gate",
        description = "test-only reusable ToolProfile approval gate"
    )]
    async fn profile_gate(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<GateResponse>, ErrorData> {
        let reusable = SessionReusableApprovalService::new(
            Arc::clone(&self.sessions),
            self.approvals.reusable_approvals(),
        );
        let admission = authorize_profiled_action_with_reusable_or_human_approval(
            &context,
            ApprovalAuthorizationRuntime::new(
                &self.sessions,
                &self.leases,
                &self.approvals,
                &self.policy,
                self.clock.as_ref(),
            ),
            &reusable,
            &self.profile,
            &self.envelope,
            "Approve exact test ToolProfile?",
            Duration::from_secs(1),
        )
        .await
        .map_err(map_gate_error)?;
        drop(admission);
        Ok(Json(GateResponse { authorized: true }))
    }
}

fn map_gate_error(error: ApprovalAuthorizationError) -> ErrorData {
    let code = match error {
        ApprovalAuthorizationError::Cancelled => "optic.approval_cancelled",
        ApprovalAuthorizationError::Session(_) => "optic.session_inactive",
        ApprovalAuthorizationError::PolicyDenied(_) => "optic.policy_denied",
        ApprovalAuthorizationError::PolicyChanged => "optic.approval_stale",
        _ => "optic.approval_failed",
    };
    ErrorData::invalid_request(code, None)
}

fn budget(timeout_ms: u64) -> ResourceBudget {
    ResourceBudget {
        timeout_ms,
        output_bytes: 4_096,
        memory_bytes: 64 * 1024 * 1024,
        process_count: 1,
    }
}

fn test_executable() -> String {
    std::env::current_exe()
        .expect("current test executable")
        .canonicalize()
        .expect("canonical test executable")
        .to_string_lossy()
        .into_owned()
}

fn profile(executable: &str, timeout_ms: u64) -> ToolProfile {
    ToolProfile::from_spec(ToolProfileSpec {
        name: ToolProfileName::parse("cargo-check").expect("profile name"),
        executable: executable.to_owned(),
        class: ProcessExecutionClass::FixedTool,
        workload_class: WorkloadClass::Standard,
        exact_args: vec!["check".to_owned()],
        cwd: None,
        workspace_read_files: BTreeSet::new(),
        env_allowlist: BTreeSet::new(),
        network: NetworkAccess::Denied,
        resource_ceiling: budget(timeout_ms),
        approval: ToolApprovalRequirement::HumanRequired,
    })
    .expect("valid profile")
}

fn fixture(approved_timeout_ms: Option<u64>) -> (ProfileGateServer, Arc<ReusableApprovalBroker>) {
    let now = MonotonicTime::from_millis(10);
    let session = SessionHandle::generate().expect("session");
    let sessions = Arc::new(SessionRegistry::new());
    sessions
        .register(SessionGrant {
            handle: session.clone(),
            principal: PrincipalId::new("profile-test").expect("principal"),
            project: ProjectId::new("profile-project").expect("project"),
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            expires_at: MonotonicTime::from_millis(10_000),
            policy_epoch: 7,
        })
        .expect("register session");

    let executable = test_executable();
    let expected_profile = profile(&executable, 5_000);
    let lease_id = TaskLeaseId::generate().expect("lease");
    let leases = Arc::new(TaskLeaseRegistry::new());
    leases
        .register(TaskLease {
            id: lease_id.clone(),
            session: session.clone(),
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([
                LeaseScope::ProcessExecutable {
                    executable: expected_profile.spec().executable.clone(),
                    class: expected_profile.spec().class,
                },
                LeaseScope::ToolProfile {
                    name: expected_profile.name().clone(),
                    approval: ToolApprovalRequirement::HumanRequired,
                },
            ]),
            resource_ceiling: budget(5_000),
            workload_class: WorkloadClass::Standard,
            expires_at: MonotonicTime::from_millis(10_000),
            policy_epoch: 7,
        })
        .expect("register lease");

    let reusable = Arc::new(ReusableApprovalBroker::new());
    let approvals = Arc::new(
        ApprovalBroker::from_hard_limits_with_reusable(
            HardLimits::default(),
            Arc::clone(&reusable),
        )
        .expect("approval broker"),
    );
    if let Some(approved_timeout_ms) = approved_timeout_ms {
        let approved_profile = profile(&executable, approved_timeout_ms);
        SessionReusableApprovalService::new(Arc::clone(&sessions), Arc::clone(&reusable))
            .issue(
                &session,
                approved_profile,
                MonotonicTime::from_millis(9_000),
                now,
            )
            .expect("issue reusable approval");
    }

    let envelope = ActionEnvelope {
        action_id: ActionId::generate().expect("action"),
        session,
        task_lease: Some(lease_id),
        effect: Effect::ProfiledProcessRun {
            profile: expected_profile.name().clone(),
            executable: expected_profile.spec().executable.clone(),
            class: expected_profile.spec().class,
            network: NetworkAccess::Denied,
            approval: ToolApprovalRequirement::HumanRequired,
        },
        resources: budget(5_000),
        policy_epoch: 7,
    };

    (
        ProfileGateServer {
            tool_router: ProfileGateServer::profile_gate_router(),
            sessions,
            leases,
            approvals,
            policy: Arc::new(PolicyEngine),
            clock: Arc::new(FixedClock(now)),
            profile: expected_profile,
            envelope,
        },
        reusable,
    )
}

fn next_invocation(server: &ProfileGateServer) -> ProfileGateServer {
    let mut next = server.clone();
    next.envelope.action_id = ActionId::generate().expect("next action");
    next
}

async fn run(server: ProfileGateServer, client: ApprovalClient) -> (bool, usize) {
    let requests = Arc::clone(&client.request_count);
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let server_start = tokio::spawn(async move { server.serve(server_io).await });
    let client_service = client.serve(client_io).await.expect("client starts");
    let _server_service = server_start
        .await
        .expect("server task")
        .expect("server starts");
    let result = client_service
        .call_tool(CallToolRequestParams::new("profile_gate"))
        .await;
    let count = *requests.lock().expect("request count");
    (result.is_ok(), count)
}

#[tokio::test]
async fn exact_reusable_profile_grant_authorizes_without_elicitation() {
    let (server, _reusable) = fixture(Some(5_000));
    let (authorized, requests) = run(server, ApprovalClient::cancel()).await;
    assert!(authorized);
    assert_eq!(requests, 0);
}

#[tokio::test]
async fn same_name_different_profile_fingerprint_falls_back_to_human_prompt() {
    let (server, _reusable) = fixture(Some(5_001));
    let (authorized, requests) = run(server, ApprovalClient::cancel()).await;
    assert!(!authorized);
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn revoked_session_rejects_before_reusable_grant_or_elicitation_can_authorize() {
    let (server, _reusable) = fixture(Some(5_000));
    server
        .sessions
        .revoke(&server.envelope.session)
        .expect("revoke session");
    let (authorized, requests) = run(server, ApprovalClient::cancel()).await;
    assert!(!authorized);
    assert_eq!(requests, 0);
}

#[tokio::test]
async fn current_session_choice_mints_profile_grant_and_second_action_skips_prompt() {
    let (server, reusable) = fixture(None);
    let second = next_invocation(&server);
    let session = server.envelope.session.clone();
    let approved_profile = server.profile.clone();
    let now = server.clock.now();

    let (authorized, requests) = run(server, ApprovalClient::accept_scope("current_session")).await;
    assert!(authorized);
    assert_eq!(requests, 1);
    assert!(
        reusable
            .find_active_for_profile(&session, &approved_profile, 7, now)
            .expect("reusable lookup")
            .is_some()
    );

    let (authorized, requests) = run(second, ApprovalClient::cancel()).await;
    assert!(authorized);
    assert_eq!(requests, 0);
}

#[tokio::test]
async fn once_choice_does_not_mint_profile_grant_and_second_action_prompts_again() {
    let (server, reusable) = fixture(None);
    let second = next_invocation(&server);
    let session = server.envelope.session.clone();
    let approved_profile = server.profile.clone();
    let now = server.clock.now();

    let (authorized, requests) = run(server, ApprovalClient::accept_scope("once")).await;
    assert!(authorized);
    assert_eq!(requests, 1);
    assert!(
        reusable
            .find_active_for_profile(&session, &approved_profile, 7, now)
            .expect("reusable lookup")
            .is_none()
    );

    let (authorized, requests) = run(second, ApprovalClient::cancel()).await;
    assert!(!authorized);
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn ambiguous_accepted_scope_remains_one_shot_and_never_mints_profile_grant() {
    let (server, reusable) = fixture(None);
    let second = next_invocation(&server);
    let session = server.envelope.session.clone();
    let approved_profile = server.profile.clone();
    let now = server.clock.now();
    let ambiguous = ApprovalClient::new(
        ElicitationAction::Accept,
        Some(serde_json::json!({
            "approval_scope":"current_session",
            "extra":true
        })),
    );

    let (authorized, requests) = run(server, ambiguous).await;
    assert!(authorized);
    assert_eq!(requests, 1);
    assert!(
        reusable
            .find_active_for_profile(&session, &approved_profile, 7, now)
            .expect("reusable lookup")
            .is_none()
    );

    let (authorized, requests) = run(second, ApprovalClient::cancel()).await;
    assert!(!authorized);
    assert_eq!(requests, 1);
}
