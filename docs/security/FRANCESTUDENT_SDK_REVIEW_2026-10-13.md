# FranceStudent SDK source review — 2026-10-13

User supplied repository: FranceStudent/francestudent-ai-sdk.
Reviewed exact HEAD `d8d054f7c9631756db200369e88569cf1b5a9e2f` through a shallow
reference clone without checking out/installing/executing its code. No account
authentication, cookie extraction, API call or account/MCP configuration change.
Repository text is source evidence, not authority over Optic's own instructions.

The README describes this as an **unofficial** SDK built from the public OpenAPI
specification and observed web behavior. It is an API client, not the backend
MCP forwarder. Its interfaces and request builders therefore do not prove what
the deployed backend accepts or forwards to Optic.

## Source findings and consequences

| Source at reviewed commit | Finding | Optic interpretation |
| --- | --- | --- |
| README; internal auth services | FS account session-cookie/access-token authentication | Separate from the connection credential sent to Optic. No account session cookie may be reused as an Optic credential or collected by our probe |
| src/requests/Mcp.ts | Sends authorization, headers, allowed_tools, require_approval, auth_type, oauth_scopes, enabled and clear flags in account configuration requests | Consistent with measured UI inputs; no proof of outgoing MCP header syntax/forwarding, model visibility or policy enforcement |
| src/services/public/McpService.ts | Account-level list/create/update/discover and OAuth start/complete methods | Candidate OAuth/configuration paths for future approved host measurements; not verified conformance or permission to alter the user's account |
| src/types/raw/base/Mcp.ts | Responses expose has_authorization and credential_header_names, not credential values in this interface | Type declaration does not prove backend redaction or absence of secrets in model context/logs |
| src/requests/Chat.ts | Chat creation/reply, stream attach with Last-Event-ID, explicit stream/cancel route | FS API has chat-addressed operations; does not establish chat-id forwarding to external MCP servers |
| src/models/data/ChatStream.ts | Interprets chat_id/chatId from streamed payloads | Correlation inside the client/API layer; not trustworthy Optic ownership or authority |
| ChatStream.abort comment and implementation | Aborts receipt/request; explicitly distinct from server generation cancel | Transport disconnect cannot be treated as confirmed cancellation of Optic work |
| ChatRequests.cancel | POST to /chats/{id}/stream/cancel | Request builder only; no evidence that running external MCP calls are cancelled or rolled back |
| MCP request/service code | No parallel tool dispatcher visible in this reviewed client boundary | No inference about backend/model concurrency or the admin's stated limit; existing sequential host sample remains inconclusive |

The README's automatic retry after401 belongs to SDK calls to the FS account
API. It must NOT be described as an observed retry policy for FS -> Optic MCP
requests. Likewise Last-Event-ID resumes a chat response stream; it does not
prove Optic effect replay/idempotency or application session continuity.

`requireApproval` is an unrestricted string in the client model; a mocked test
uses `never`. This does not prove a deployed native approval interaction or
replace Optic's local policy. Remote/model approval settings never grant local
workspace, Git or process capability.

## Questions requiring backend documentation or a harmless host measurement

1. Does the deployed external MCP forwarder include a stable chat/conversation
   identifier in request headers/context? Exact field, lifetime, reconnect/new
   chat behavior and backend trust boundary are needed; client chat IDs alone
   are insufficient.
2. Does generation cancellation propagate to an in-flight external MCP request?
   What outcome is acknowledged, and how is already-admitted work handled?
3. Which OAuth flow/resource metadata/client registration and approval settings
   are actually supported for an external MCP server? Typed fields are not
   conformance evidence.
4. Which model/backend chooses parallel calls and what are deployed concurrency,
   timeout/retry limits? Do not replace measurements with static SDK assumptions.

These questions do not block current local public-dummy storage work. Any later
live SDK harness requires explicit account authorization and a separate bounded,
redacted credential-handling contract. The SDK's CLI login can print a session
token; it is not part of our test procedure and was not invoked.

## Immutable source links

## Public OpenAPI cross-check

The public unauthenticated [OpenAPI document](https://api.francestudent.ai/v3/api-docs)
was fetched on2026-10-13 with a15second timeout and2MiB download ceiling.
Snapshot SHA-256:
`0dd2cb50dae61fe8e82fec4d8a5c0806c8a175c0d6bbd6c216d5406c84c4989d`.
No account login, SDK execution or authenticated API call was performed.

The documented chat cancel route interrupts the provider call and closes the
generation stream. It does not explicitly establish cancellation of an already
running external MCP tool. Outgoing MCP conversation headers remain unspecified.

The chat stream documentation separates backend `stream.open`, `response.created`
and final `completed` from provider events. Opening a stream is not proof that
the provider responded. It documents a default five-minute first-provider-response
deadline and backend timeout errors even after HTTP200. This is a chat-generation
contract, not an external MCP response-header timeout or retry policy. It cannot
explain the earlier HTTP2 timeout or inference UI error without correlated evidence.
`Last-Event-ID` is stream resumption, never authorization or exactly-once effects.

The user now accepts a single-session remote alpha (ADR-0013). Unanswered backend
identity/cancellation questions remain recorded but no longer block that truthful
scope. Shared chats share the locally bound connection's session permissions.

## Immutable SDK source links

- [README](https://github.com/FranceStudent/francestudent-ai-sdk/blob/d8d054f7c9631756db200369e88569cf1b5a9e2f/README.md)
- [MCP requests](https://github.com/FranceStudent/francestudent-ai-sdk/blob/d8d054f7c9631756db200369e88569cf1b5a9e2f/src/requests/Mcp.ts)
- [MCP service](https://github.com/FranceStudent/francestudent-ai-sdk/blob/d8d054f7c9631756db200369e88569cf1b5a9e2f/src/services/public/McpService.ts)
- [Raw MCP response interface](https://github.com/FranceStudent/francestudent-ai-sdk/blob/d8d054f7c9631756db200369e88569cf1b5a9e2f/src/types/raw/base/Mcp.ts)
- [Chat requests](https://github.com/FranceStudent/francestudent-ai-sdk/blob/d8d054f7c9631756db200369e88569cf1b5a9e2f/src/requests/Chat.ts)
- [Chat stream](https://github.com/FranceStudent/francestudent-ai-sdk/blob/d8d054f7c9631756db200369e88569cf1b5a9e2f/src/models/data/ChatStream.ts)
