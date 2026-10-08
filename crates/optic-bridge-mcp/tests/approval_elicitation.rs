use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use optic_bridge_mcp::{HumanApprovalDecision, request_human_approval};
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
