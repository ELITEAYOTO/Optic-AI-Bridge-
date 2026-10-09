use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use optic_bridge_mcp::{ProfileApprovalDecision, request_profile_approval_choice};
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct ProbeResponse {
    ok: bool,
}

#[derive(Clone)]
struct ChoiceServer {
    tool_router: ToolRouter<Self>,
    observed: Arc<Mutex<Vec<ProfileApprovalDecision>>>,
}

impl ChoiceServer {
    fn new(observed: Arc<Mutex<Vec<ProfileApprovalDecision>>>) -> Self {
        Self {
            tool_router: Self::choice_tool_router(),
            observed,
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ChoiceServer {}

#[tool_router(router = choice_tool_router)]
impl ChoiceServer {
    #[tool(
        name = "profile_approval_choice_probe",
        description = "test-only profile approval scope elicitation"
    )]
    async fn profile_approval_choice_probe(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ProbeResponse>, ErrorData> {
        let decision = request_profile_approval_choice(
            &context,
            "Approve this exact test ToolProfile?",
            Duration::from_secs(1),
        )
        .await
        .map_err(|_| ErrorData::internal_error("choice probe failed", None))?;
        self.observed
            .lock()
            .expect("observed decisions")
            .push(decision);
        Ok(Json(ProbeResponse { ok: true }))
    }
}

#[derive(Clone)]
struct ChoiceClient {
    action: ElicitationAction,
    content: Option<Value>,
    advertise_elicitation: bool,
    request_count: Arc<Mutex<usize>>,
    schemas: Arc<Mutex<Vec<Value>>>,
}

impl ChoiceClient {
    fn new(
        action: ElicitationAction,
        content: Option<Value>,
        advertise_elicitation: bool,
    ) -> Self {
        Self {
            action,
            content,
            advertise_elicitation,
            request_count: Arc::new(Mutex::new(0)),
            schemas: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ClientHandler for ChoiceClient {
    fn get_info(&self) -> ClientConfig {
        let capabilities = if self.advertise_elicitation {
            ClientCapabilities::builder().enable_elicitation().build()
        } else {
            ClientCapabilities::builder().build()
        };
        ClientConfig::new(
            capabilities,
            Implementation::new("optic-profile-choice-test", "1.0.0"),
        )
    }

    async fn create_elicitation(
        &self,
        request: ElicitRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<ElicitResult, ErrorData> {
        *self.request_count.lock().expect("request count") += 1;
        if let ElicitRequestParams::FormElicitationParams {
            requested_schema, ..
        } = &request
        {
            self.schemas
                .lock()
                .expect("schemas")
                .push(serde_json::to_value(requested_schema).expect("serialize requested schema"));
        }

        let mut result = ElicitResult::new(self.action.clone());
        if let Some(content) = &self.content {
            result = result.with_content(content.clone());
        }
        Ok(result)
    }
}

async fn run_choice(
    client: ChoiceClient,
) -> (
    Result<rmcp::model::CallToolResult, rmcp::service::ServiceError>,
    Vec<ProfileApprovalDecision>,
    usize,
    Vec<Value>,
) {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let server = ChoiceServer::new(Arc::clone(&observed));
    let request_count = Arc::clone(&client.request_count);
    let schemas = Arc::clone(&client.schemas);
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);

    let server_start = tokio::spawn(async move { server.serve(server_io).await });
    let client_service = client.serve(client_io).await.expect("client starts");
    let _server_service = server_start
        .await
        .expect("server task")
        .expect("server starts");
    let result = client_service
        .call_tool(CallToolRequestParams::new("profile_approval_choice_probe"))
        .await;

    let decisions = observed.lock().expect("observed decisions").clone();
    let requests = *request_count.lock().expect("request count");
    let schemas = schemas.lock().expect("schemas").clone();
    (result, decisions, requests, schemas)
}

#[tokio::test]
async fn exact_current_session_choice_is_distinct_and_schema_defaults_to_once() {
    let (result, decisions, requests, schemas) = run_choice(ChoiceClient::new(
        ElicitationAction::Accept,
        Some(serde_json::json!({"approval_scope":"current_session"})),
        true,
    ))
    .await;

    assert!(result.is_ok());
    assert_eq!(decisions, vec![ProfileApprovalDecision::CurrentSession]);
    assert_eq!(requests, 1);
    assert_eq!(schemas.len(), 1);

    let schema = &schemas[0];
    assert_eq!(schema["required"], serde_json::json!(["approval_scope"]));
    let scope = &schema["properties"]["approval_scope"];
    assert_eq!(scope["type"], "string");
    assert_eq!(scope["default"], "once");
    assert_eq!(
        scope["oneOf"],
        serde_json::json!([
            {"const":"once","title":"Allow once"},
            {"const":"current_session","title":"Allow for current Optic session"}
        ])
    );
}

#[tokio::test]
async fn explicit_once_choice_stays_one_shot() {
    let (result, decisions, requests, _) = run_choice(ChoiceClient::new(
        ElicitationAction::Accept,
        Some(serde_json::json!({"approval_scope":"once"})),
        true,
    ))
    .await;
    assert!(result.is_ok());
    assert_eq!(decisions, vec![ProfileApprovalDecision::Once]);
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn accepted_missing_or_malformed_scope_degrades_to_once_never_reusable() {
    let malformed = [
        None,
        Some(serde_json::json!({})),
        Some(serde_json::json!({"approval_scope":"unexpected"})),
        Some(serde_json::json!({"approval_scope":7})),
        Some(serde_json::json!({
            "approval_scope":"current_session",
            "extra":true
        })),
    ];

    for content in malformed {
        let (result, decisions, requests, _) = run_choice(ChoiceClient::new(
            ElicitationAction::Accept,
            content,
            true,
        ))
        .await;
        assert!(result.is_ok());
        assert_eq!(decisions, vec![ProfileApprovalDecision::Once]);
        assert_eq!(requests, 1);
    }
}

#[tokio::test]
async fn decline_cancel_and_unsupported_remain_distinct() {
    for (action, expected) in [
        (ElicitationAction::Decline, ProfileApprovalDecision::Declined),
        (ElicitationAction::Cancel, ProfileApprovalDecision::Cancelled),
    ] {
        let (result, decisions, requests, _) =
            run_choice(ChoiceClient::new(action, None, true)).await;
        assert!(result.is_ok());
        assert_eq!(decisions, vec![expected]);
        assert_eq!(requests, 1);
    }

    let (result, decisions, requests, schemas) = run_choice(ChoiceClient::new(
        ElicitationAction::Accept,
        Some(serde_json::json!({"approval_scope":"current_session"})),
        false,
    ))
    .await;
    assert!(result.is_ok());
    assert_eq!(decisions, vec![ProfileApprovalDecision::Unsupported]);
    assert_eq!(requests, 0);
    assert!(schemas.is_empty());
}
