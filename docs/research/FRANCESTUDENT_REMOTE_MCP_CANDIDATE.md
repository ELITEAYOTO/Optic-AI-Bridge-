# FranceStudent / Remote MCP candidate

**Status:** active H-01/H-02 research target — transport feasibility and synthetic real-Optic reachability proven; FranceStudent bearer/header carrier and lack of chat id now admin-confirmed; production auth/session implementation still pending.  
**Recorded:** 2026-10-08  
**Last updated:** 2026-10-12

## Motivation

Optic may expose the same application-owned security/runtime core through a remote host adapter so a cloud client such as FranceStudent can use the Optic instance running on the student's own Windows PC.

The intended product property is local-first and zero mandatory maintainer infrastructure: the user's PC keeps the workspace, CPU/RAM, state and Optic authority. A tunnel, if used, transports HTTPS only and never becomes an authorization boundary.

FranceStudent is the priority remote V1 host. ChatGPT remains a target where the user's plan/surface exposes a compatible path. MCP itself remains an adapter rather than an Optic core requirement.

## Verified transport evidence

A real Windows PC has proven Streamable HTTP MCP reachability through a temporary Cloudflare Quick Tunnel.

Earlier H-01 observations:

- FranceStudent displayed the custom MCP server as active;
- real remote POST traffic reached the local probe;
- `probe_ping` succeeded;
- negotiated MCP revision was `2026-07-28`;
- no modern MCP transport session id was observed;
- two same-conversation `probe_session_correlation` calls were corroborated by fresh server traffic;
- no host-owned per-call approval/confirmation UI was observed for those harmless calls.

This proves basic remote Streamable HTTP tool reachability. It does **not** prove production authentication, multi-session mapping or real-project safety.

## Evidence integrity correction

One earlier FranceStudent/Claude response that looked like a new-conversation probe result was later explicitly admitted by the model to have been produced without a real tool call. That sample is discarded.

Separately, H-01 v1 fingerprinted `str(request_id)` even when the MCP Python SDK request id was absent. A stable fingerprint of `None` can therefore masquerade as a stable host signal. The earlier constant request-id fingerprint is **ambiguous** and must not be used as conversation evidence.

The rule is now:

> FranceStudent/model output alone is not proof of tool execution when server-side proof can be collected.

Direct admin statements are recorded as **product-contract input**. Behaviors that matter to interoperability/security should still receive harmless wire-level regression proof where practical.

## H-01 v2 probe

Draft PR #170 upgraded the authority-free probe to `h01-v2`.

Tools:

- `probe_ping`;
- `probe_host_context`;
- `probe_session_correlation(run_id)`;
- `probe_parallel(run_id, delay_ms)`;
- `probe_reset(run_id)`.

Important v2 properties:

- credential-like headers are omitted/redacted;
- other candidate correlation values are process-local one-way fingerprints;
- request-id presence is explicit;
- absent request id yields `request_id_fingerprint=null`;
- every successful call returns a random per-call `probe_invocation` marker;
- every server process has a random instance id;
- the exact same invocation proof is printed locally as `[H01] TOOL_CALL ...`;
- correlation reports appeared/disappeared keys as changes rather than ignoring them;
- parallel overlap is counted only inside an explicit caller-selected parallel test run id;
- correlation state is bounded;
- request body is bounded to 64 KiB;
- normal loopback mode enables DNS-rebinding protection;
- random Quick-Tunnel host acceptance requires an explicit H-01-only switch.

The probe has no filesystem, Git, process, real Optic runtime, mutation or Windows-effect authority.

## FranceStudent service state

FranceStudent is stable again as of 2026-10-12. H-01 is therefore no longer blocked by the previous service incident.

The remaining H-01 checks are now **compatibility evidence**, not blockers for choosing the FranceStudent V1 session strategy. Use the H-01 v2 endpoint/process deliberately; do not confuse it with the separate PR #178 Real Optic Link endpoint used for synthetic real-Optic tests.

Useful remaining checks:

1. **Reconnect** — toggle/reconnect the MCP server and observe whether any non-authoritative transport/context fingerprints change.
2. **Parallel dispatch** — two simultaneous `probe_parallel` calls using the same fresh parallel run id and `delay_ms=500`; require server-side `overlap_observed` proof.
3. **Secure-header wire proof** — harmless `X-Optic-Probe` value, verify exact delivery while keeping credential-like values redacted.
4. **Bearer wire regression proof** — dummy non-secret value, verify only scheme/header presence, never log the value.
5. **Timeout characterization** — useful operational evidence if the host publishes or can measure it.

A fresh same-chat/new-chat correlation capture may still be retained for diagnostics, but **FranceStudent admin has explicitly confirmed there is no per-conversation/chat identifier transmitted to MCP servers today**, so H-03 no longer waits on discovery of such an id.

Do not compare process-local HMAC fingerprints across probe restarts.

## FranceStudent admin-confirmed host contract — 2026-10-12

Direct answers from a FranceStudent admin now establish the following current product behavior:

- **No per-conversation/chat identifier is sent to the MCP server.** The admin answered that there is no such value today and noted it could be a future addition.
- MCP configuration is **global to the user's account**, so each conversation can access the configured MCP server.
- FranceStudent has **no native per-tool-call confirmation** once the MCP server/tool is enabled.
- **Custom headers are supported.**
- **Secure JSON headers are sent on every MCP request** and are **hidden from the model**.
- The authorization field sends exactly `Authorization: Bearer <value>`.
- FranceStudent permits parallel tool calls; OpenAI/Claude decides whether to issue them. The admin reported a current host-side limit of roughly **8 calls**, with intent to remove that limit later. Optic must therefore retain its own independent bounds and never depend on the number 8.
- **MCP cancellation is not currently supported** by FranceStudent.
- Reconnect/context behavior is not specified by the admin and remains something to measure.
- Host-side tool-call timeout was asked but is not yet recorded here as answered.

These facts materially narrow H-02/H-03/H-05 and remove the need to speculate about a hidden FranceStudent chat-id contract.

## FranceStudent authentication UI + carrier evidence

The current FranceStudent MCP server editor visibly exposes:

- **Autorisation** labelled `Bearer token ou secret`;
- **Headers sécurisés (JSON)**.

The admin subsequently confirmed the authorization field maps to exactly:

```http
Authorization: Bearer <value>
```

The admin also confirmed secure JSON headers are attached to every request and hidden from the model.

This makes a static-bearer compatibility path concrete for FranceStudent V1. It is still **not OAuth** and must not be described as OAuth unless FranceStudent later implements an actual OAuth flow.

Still worth proving with a harmless test harness:

- bearer header presence/scheme on wire using a dummy value;
- custom-header delivery;
- behavior for missing/malformed/wrong bearer;
- local revoke/rotation/reconnect behavior;
- duplicate/conflicting `Authorization` sources if the UI permits both the bearer field and custom JSON header;
- secret redaction from logs, diagnostics and MCP data.

## Real Optic Link evidence through FranceStudent (2026-10-12)

Today's FranceStudent tests use the **current PR #178 Real Optic Link endpoint**, not the H-01 probe endpoint.

The host discovered exactly:

- `optic_session_info`;
- `optic_fs_list`;
- `optic_fs_read`.

The adapter creates its own disposable synthetic workspace and accepts no caller-selected real workspace root.

### Server-correlated real read

A real `optic_fs_read("public/hello.txt")` returned:

- `adapter_instance=20baecc2eeaf`;
- `sequence=22`;
- `marker=126b81f5327c1b59`.

The exact instance/sequence/marker appeared in the local `[REAL-LINK] TOOL_CALL` log. This is accepted proof that FranceStudent reached the real Rust `optic-bridge` stdio application path and read the synthetic fixture through Optic's real filesystem service.

### Real workspace containment negative

- a caller-supplied absolute Windows path targeting the user's real Desktop/repository was rejected;
- a deliberate `../outside.txt` read was rejected with `optic.invalid_workspace_path`, JSON-RPC code `-32602`, surfaced by FranceStudent as HTTP 400.

This is accepted negative evidence for the synthetic fixture. It does not authorize real-project exposure.

### Multiple calls from one assistant turn

FranceStudent issued two distinct real reads from one assistant turn:

- `seq=23`, marker `ff1fc559e79dd183`;
- `seq=24`, marker `e6aeabb6875f6ffb`.

Both matched local logs. This proves two real tool calls in one turn, **not** temporal overlap. True concurrent overlap is still useful to characterize with H-01 v2 `probe_parallel`, even though the admin has confirmed FranceStudent permits parallel calls.

### New FranceStudent conversation on the same live adapter

One conversation produced verified `optic_session_info` `seq=25`, marker `16a1e00238f6c7f9`.

A genuinely new FranceStudent conversation, while keeping the same live adapter/tunnel, produced `seq=26`, marker `5198a949dec91035`, with the same `adapter_instance=20baecc2eeaf` and the same upstream-visible experimental session-info context (`file_read`, `file_search`, `policy_epoch=1`, principal `local-stdio`, project `local-workspace`, monotonic expiry value `1800000`).

Safe conclusion:

> Distinct FranceStudent conversations currently converge on the same long-lived PR #178 adapter/upstream experimental context, and FranceStudent admin confirms no per-chat id is transmitted to the MCP server today.

The current experimental bridge therefore cannot create one Optic application session per FranceStudent conversation from host metadata alone. That is direct evidence for the necessity of H-03 `SessionResolver` + an Optic-minted explicit application handle before real multi-conversation authority.

## What is already safe to conclude

- MCP `2026-07-28` should be treated as stateless at protocol level.
- Missing `Mcp-Session-Id` is expected, not a failure.
- FranceStudent admin confirms **no per-conversation/chat id is currently transmitted to MCP servers**.
- Cloudflare/network tracing headers are not conversation identity.
- MCP client metadata may be stable across calls but is not conversation identity by itself.
- Request id is not authority and the v1 request-id evidence is unusable as a conversation signal.
- FranceStudent has no native per-call confirmation UI.
- FranceStudent does not currently support MCP cancellation.
- Secure custom headers are supported on every request and hidden from the model.
- The authorization field uses `Authorization: Bearer <value>`.
- FranceStudent permits parallel calls, but Optic must keep independent concurrency/admission bounds.
- Host/model-rendered JSON cannot be trusted as invocation proof without corroboration where proof instrumentation exists.
- FranceStudent can reach the real Optic synthetic read/list/session-info path.
- The current PR #178 bridge does not isolate different FranceStudent conversations into separate upstream Optic application sessions.

For FranceStudent V1, H-03 should now treat an **Optic-minted explicit application handle** as the baseline strategy, not merely a fallback waiting on a host chat id.

The real authority remains an independent application-owned `SessionHandle`.

## Authentication constraints

The production remote endpoint must authenticate independently from Cloudflare/Tailscale/provider identity.

Access tokens do not belong in URI query strings. Production Optic will not use `?token=...` URLs.

FranceStudent's V1-compatible carrier is now admin-confirmed:

```http
Authorization: Bearer <credential>
```

Requirements remain:

- high-entropy locally generated connection credential;
- authentication on every request/context before session resolution;
- constant-time verification where applicable;
- local revoke and rotation;
- reconnect must not resurrect revoked auth;
- no secret in logs/diagnostics/tool output;
- independent request/header/concurrency/rate bounds owned by Optic;
- authenticated principal still has zero project/process authority until H-03/application policy maps it into an Optic-owned session and grants separate capability/lease authority.

Do not call this compatibility mode OAuth unless the host actually performs an OAuth flow.

## Real Optic link available for host tests

Draft PR #178 proves:

```text
remote Streamable HTTP MCP client
  -> bounded compatibility adapter
  -> real Rust `optic-bridge` stdio server
  -> disposable synthetic workspace
```

No process, mutation, Git, `session_cancel` or arbitrary upstream forwarding is exposed.

On experiment head `ef5bc84144670135d13f55a59a782611aeae762f`, dedicated workflow #11 is green on Linux + Windows and normal CI #653 is green on dependency policy + Ubuntu + Windows.

Do **not** point PR #178 at a real repository. Real-project remote exposure remains blocked by the security audit gates documented in `../security/REMOTE_AUTONOMY_AUDIT_GATES.md`.

## Architecture direction

```text
FranceStudent / other remote host
        |
        | HTTPS + Streamable HTTP /mcp
        | Authorization: Bearer <connection credential>
        v
Remote host adapter
  - endpoint authentication
  - transport hard limits
  - bounded HostContext
  - explicit Optic application/session handle for FranceStudent V1
        |
        v
SessionResolver
  - principal-bound handle -> Optic-owned SessionHandle
        |
        v
Existing Optic normalization / PolicyEngine / leases / runtime
        |
        +--> filesystem / Git / isolated process paths
```

Rules:

1. Security/execution core stays transport-independent.
2. Local HTTP listener binds loopback by default.
3. Public/tunnel URL is never a secret or authority.
4. Real remote Optic access requires endpoint authentication.
5. Remote client cannot mint sessions, capabilities, task leases, scopes or process grants.
6. FranceStudent V1 has no host chat id today; use an Optic-minted explicit application handle for application continuity.
7. Any future host-provided conversation/session ids remain bounded correlation inputs only; Optic owns the real `SessionHandle`.
8. Session autonomy never removes per-action policy/resource/isolation revalidation.
9. Interpreter/repository-code execution remains subject to existing isolation gates regardless of transport.
10. Tunnel providers are replaceable adapters outside PolicyEngine.
11. No mandatory Optic-owned VPS/relay/database is introduced for the free path.
12. Removing one provider must not require changes to core policy, leases or tool semantics.
13. `session_cancel` is a control-plane operation and is not exposed casually through a shared remote adapter; host cancellation cannot currently be relied upon from FranceStudent.

## Current gates

1. **H-00 complete** — product/architecture freeze.
2. **H-01 active / compatibility evidence remaining** — harness implemented; admin answers close the chat-id/auth-carrier discovery questions, while reconnect/overlap/header wire/timeout characterization remains useful.
3. **H-02 ready for proof implementation** — FranceStudent V1 bearer carrier is known; build the rejection/revoke/rotate/redaction/bounds harness before production transport.
4. **H-03 ready for contract implementation** — FranceStudent V1 baseline is explicit Optic-minted application handle + principal binding; lifetime/renewal/provisioning/revoke contracts still need implementation and tests.
5. **H-04/H-05 pending** — autonomy/local approval before production remote real-project effects. Admin confirmation of no native confirmation/cancellation reinforces H-05.
6. **H-06+ gated** — production transport and host productization.

Remote mutation/process/Git exposure remains a later independent decision and must reuse the existing Optic authority model rather than weakening it.

## Remaining genuinely important host questions

There are no longer any FranceStudent-admin answers that should block choosing the H-02 bearer architecture or the H-03 explicit-handle strategy.

Useful but non-blocking host facts still worth obtaining/measuring:

- maximum tool-call timeout / timeout behavior;
- reconnect behavior after disabling/re-enabling a MCP or transport reconnection;
- exact duplicate-header precedence if `Authorization` can be configured from two UI surfaces;
- future changes to the current ~8 host call cap;
- whether/when cancellation support is added.

Optic must remain correct even if these host behaviors change, by enforcing its own authentication, lifetime, admission, deadlines, cancellation/revoke and resource bounds.
