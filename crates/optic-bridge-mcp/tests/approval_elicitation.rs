use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use optic_bridge_core::{
    ActionEnvelope, ActionId, Capability, Effect, HardLimits, LeaseScope, MonotonicTime,
    PrincipalId, ProjectId, ResourceBudget, SessionGrant, SessionHandle, TaskLease, TaskLeaseId,
};
use optic_bridge_mcp::{
    ApprovalAuthorizationError, ApprovalAuthorizationRuntime, HumanApprovalDecision,
    authorize_action_with_human_approval, request_human_approval,
};
use optic_bridge_policy::PolicyEngine;
use optic_bridge_runtime::{
    ApprovalBroker, ApprovalSpec, Clock, SessionRegistry, TaskLeaseRegistry,
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct ProbeResponse {
    ok: bool,
}

#[derive(Clone)]
struct ProbeServer {
    tool_router: ToolRouter<Self>,
    observed: Arc<Mutex<Vec<HumanApprovalDecision>>>,
    timeout: Duration,
}

impl ProbeServer {
    fn new(observed: Arc<Mutex<Vec<HumanApprovalDecision>>>, timeout: Duration) -> Self {
        Self {
            tool_router: Self::probe_tool_router(),
            observed,
            timeout,
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ProbeServer {}

#[tool_router(router = probe_tool_router)]
impl ProbeServer {
    #[tool(
        name = "approval_probe",
        description = "test-only approval elicitation probe"
    )]
    async fn approval_probe(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ProbeResponse>, ErrorData> {
        let decision =
            request_human_approval(&context, "Approve this exact test action?", self.timeout)
                .await
                .map_err(|_| ErrorData::internal_error("probe elicitation failed", None))?;
        self.observed
            .lock()
            .expect("observed decisions")
            .push(decision);
        Ok(Json(ProbeResponse { ok: true }))
    }
}

#[derive(Clone)]
struct ProbeClient {
    action: ElicitationAction,
    advertise_elicitation: bool,
    delay: Duration,
    request_count: Arc<Mutex<usize>>,
}

impl ProbeClient {
    fn new(action: ElicitationAction, advertise_elicitation: bool, delay: Duration) -> Self {
        Self {
            action,
            advertise_elicitation,
            delay,
            request_count: Arc::new(Mutex::new(0)),
        }
    }
}

impl ClientHandler for ProbeClient {
    fn get_info(&self) -> ClientConfig {
        let capabilities = if self.advertise_elicitation {
            ClientCapabilities::builder().enable_elicitation().build()
        } else {
            ClientCapabilities::builder().build()
        };
        ClientConfig::new(
            capabilities,
            Implementation::new("optic-approval-test-client", "1.0.0"),
        )
    }

    async fn create_elicitation(
        &self,
        _request: ElicitRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<ElicitResult, ErrorData> {
        *self.request_count.lock().expect("request count") += 1;
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        Ok(ElicitResult::new(self.action.clone()))
    }
}

async fn run_probe(
    client: ProbeClient,
    timeout: Duration,
) -> (
    Result<rmcp::model::CallToolResult, rmcp::service::ServiceError>,
    Vec<HumanApprovalDecision>,
    usize,
) {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let server = ProbeServer::new(Arc::clone(&observed), timeout);
    let count = Arc::clone(&client.request_count);
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);

    let server_start = tokio::spawn(async move { server.serve(server_io).await });
    let client_service = client
        .serve(client_io)
        .await
        .expect("client service starts");
    let _server_service = server_start
        .await
        .expect("server start task")
        .expect("server service starts");

    let result = client_service
        .call_tool(CallToolRequestParams::new("approval_probe"))
        .await;
    let decisions = observed.lock().expect("observed decisions").clone();
    let requests = *count.lock().expect("request count");
    (result, decisions, requests)
}

#[tokio::test]
async fn accept_is_reported_only_when_client_advertises_and_accepts() {
    let (result, decisions, requests) = run_probe(
        ProbeClient::new(ElicitationAction::Accept, true, Duration::ZERO),
        Duration::from_secs(1),
    )
    .await;
    assert!(result.is_ok());
    assert_eq!(decisions, vec![HumanApprovalDecision::Accepted]);
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn decline_and_cancel_remain_distinct_fail_closed_decisions() {
    for (action, expected) in [
        (ElicitationAction::Decline, HumanApprovalDecision::Declined),
        (ElicitationAction::Cancel, HumanApprovalDecision::Cancelled),
    ] {
        let (result, decisions, requests) = run_probe(
            ProbeClient::new(action, true, Duration::ZERO),
            Duration::from_secs(1),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(decisions, vec![expected]);
        assert_eq!(requests, 1);
    }
}

#[tokio::test]
async fn unsupported_client_fails_closed_without_sending_elicitation() {
    let (result, decisions, requests) = run_probe(
        ProbeClient::new(ElicitationAction::Accept, false, Duration::ZERO),
        Duration::from_secs(1),
    )
    .await;
    assert!(result.is_ok());
    assert_eq!(decisions, vec![HumanApprovalDecision::Unsupported]);
    assert_eq!(requests, 0);
}

#[tokio::test]
async fn timeout_never_becomes_an_acceptance() {
    let (result, decisions, requests) = run_probe(
        ProbeClient::new(ElicitationAction::Accept, true, Duration::from_millis(200)),
        Duration::from_millis(20),
    )
    .await;
    assert!(result.is_err());
    assert!(decisions.is_empty());
    assert_eq!(requests, 1);
}

#[derive(Debug)]
struct FixedClock(MonotonicTime);
impl Clock for FixedClock {
    fn now(&self) -> MonotonicTime {
        self.0
    }
}

#[derive(Clone)]
struct GateServer {
    tool_router: ToolRouter<Self>,
    sessions: Arc<SessionRegistry>,
    leases: Arc<TaskLeaseRegistry>,
    approvals: Arc<ApprovalBroker>,
    policy: Arc<PolicyEngine>,
    clock: Arc<FixedClock>,
    envelope: ActionEnvelope,
    timeout: Duration,
    authorized: Arc<Mutex<usize>>,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for GateServer {}

#[tool_router(router = gate_tool_router)]
impl GateServer {
    #[tool(name = "approval_gate", description = "test-only exact approval gate")]
    async fn approval_gate(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ProbeResponse>, ErrorData> {
        let admission = authorize_action_with_human_approval(
            &context,
            ApprovalAuthorizationRuntime::new(
                &self.sessions,
                &self.leases,
                &self.approvals,
                &self.policy,
                self.clock.as_ref(),
            ),
            &self.envelope,
            "Approve this exact network endpoint?",
            self.timeout,
        )
        .await
        .map_err(map_gate_error)?;
        *self.authorized.lock().expect("authorized count") += 1;
        drop(admission);
        Ok(Json(ProbeResponse { ok: true }))
    }
}

fn map_gate_error(error: ApprovalAuthorizationError) -> ErrorData {
    let code = match error {
        ApprovalAuthorizationError::Declined => "optic.approval_declined",
        ApprovalAuthorizationError::Cancelled => "optic.approval_cancelled",
        ApprovalAuthorizationError::Unsupported => "optic.approval_unsupported",
        ApprovalAuthorizationError::PolicyDenied(_) => "optic.policy_denied",
        ApprovalAuthorizationError::PolicyChanged => "optic.approval_stale",
        _ => "optic.approval_failed",
    };
    ErrorData::invalid_request(code, None)
}

fn gate_fixture(timeout: Duration) -> (GateServer, Arc<Mutex<usize>>) {
    let session = SessionHandle::generate().expect("session");
    let sessions = Arc::new(SessionRegistry::new());
    sessions
        .register(SessionGrant {
            handle: session.clone(),
            principal: PrincipalId::new("approval-test").expect("principal"),
            project: ProjectId::new("approval-project").expect("project"),
            capabilities: std::collections::BTreeSet::from([Capability::NetworkAccess]),
            expires_at: MonotonicTime::from_millis(10_000),
            policy_epoch: 7,
        })
        .expect("register session");
    let lease_id = TaskLeaseId::generate().expect("lease");
    let leases = Arc::new(TaskLeaseRegistry::new());
    let budget = ResourceBudget {
        timeout_ms: 1_000,
        output_bytes: 1_024,
        memory_bytes: 1_024,
        process_count: 1,
    };
    leases
        .register(TaskLease {
            id: lease_id.clone(),
            session: session.clone(),
            capabilities: std::collections::BTreeSet::from([Capability::NetworkAccess]),
            scopes: std::collections::BTreeSet::from([LeaseScope::NetworkEndpoint(
                "tcp://127.0.0.1:443".to_owned(),
            )]),
            resource_ceiling: budget,
            workload_class: optic_bridge_core::WorkloadClass::Standard,
            expires_at: MonotonicTime::from_millis(10_000),
            policy_epoch: 7,
        })
        .expect("register lease");
    let envelope = ActionEnvelope {
        action_id: ActionId::generate().expect("action"),
        session,
        task_lease: Some(lease_id),
        effect: Effect::NetworkAccess {
            endpoint: "tcp://127.0.0.1:443".to_owned(),
        },
        resources: budget,
        policy_epoch: 7,
    };
    let authorized = Arc::new(Mutex::new(0));
    (
        GateServer {
            tool_router: GateServer::gate_tool_router(),
            sessions,
            leases,
            approvals: Arc::new(
                ApprovalBroker::from_hard_limits(HardLimits {
                    max_approval_grants: 1,
                    max_approval_grants_per_session: 1,
                    ..HardLimits::default()
                })
                .expect("approval limits"),
            ),
            policy: Arc::new(PolicyEngine),
            clock: Arc::new(FixedClock(MonotonicTime::from_millis(10))),
            envelope,
            timeout,
            authorized: Arc::clone(&authorized),
        },
        authorized,
    )
}

async fn run_gate(
    client: ProbeClient,
    timeout: Duration,
) -> (
    Result<rmcp::model::CallToolResult, rmcp::service::ServiceError>,
    GateServer,
    usize,
) {
    let (server, authorized) = gate_fixture(timeout);
    let server_copy = server.clone();
    let requests = Arc::clone(&client.request_count);
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let server_start = tokio::spawn(async move { server.serve(server_io).await });
    let client_service = client.serve(client_io).await.expect("client starts");
    let _server_service = server_start
        .await
        .expect("server task")
        .expect("server starts");
    let result = client_service
        .call_tool(CallToolRequestParams::new("approval_gate"))
        .await;
    let count = *requests.lock().expect("request count");
    assert_eq!(
        *authorized.lock().expect("authorized count"),
        usize::from(result.is_ok())
    );
    (result, server_copy, count)
}

#[tokio::test]
async fn accepted_policy_approval_is_exact_consumed_and_returns_admission() {
    let (result, server, requests) = run_gate(
        ProbeClient::new(ElicitationAction::Accept, true, Duration::ZERO),
        Duration::from_secs(1),
    )
    .await;
    assert!(result.is_ok());
    assert_eq!(requests, 1);
    let now = MonotonicTime::from_millis(20);
    server
        .approvals
        .issue(
            ApprovalSpec {
                session: server.envelope.session.clone(),
                action_id: server.envelope.action_id.clone(),
                effect: server.envelope.effect.clone(),
                resources: server.envelope.resources,
                expires_at: MonotonicTime::from_millis(100),
                policy_epoch: server.envelope.policy_epoch,
            },
            now,
        )
        .expect("consumed approval must not retain capacity");
}

#[tokio::test]
async fn decline_cancel_and_unsupported_never_authorize_policy_action() {
    for (action, advertised) in [
        (ElicitationAction::Decline, true),
        (ElicitationAction::Cancel, true),
        (ElicitationAction::Accept, false),
    ] {
        let (result, _, requests) = run_gate(
            ProbeClient::new(action, advertised, Duration::ZERO),
            Duration::from_secs(1),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(requests, usize::from(advertised));
    }
}

#[tokio::test]
async fn revoke_during_elicitation_invalidates_late_acceptance_without_blocking() {
    let client = ProbeClient::new(ElicitationAction::Accept, true, Duration::from_millis(80));
    let requests = Arc::clone(&client.request_count);
    let (server, authorized) = gate_fixture(Duration::from_secs(1));
    let sessions = Arc::clone(&server.sessions);
    let session = server.envelope.session.clone();
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let server_start = tokio::spawn(async move { server.serve(server_io).await });
    let client_service = client.serve(client_io).await.expect("client starts");
    let _server_service = server_start
        .await
        .expect("server task")
        .expect("server starts");
    let call = tokio::spawn(async move {
        client_service
            .call_tool(CallToolRequestParams::new("approval_gate"))
            .await
    });
    for _ in 0..50 {
        if *requests.lock().expect("request count") == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert_eq!(*requests.lock().expect("request count"), 1);
    assert!(
        sessions
            .revoke(&session)
            .expect("revoke while prompt waits")
    );
    let result = call.await.expect("call task");
    assert!(result.is_err());
    assert_eq!(*authorized.lock().expect("authorized count"), 0);
}
