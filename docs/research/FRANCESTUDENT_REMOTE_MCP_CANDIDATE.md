# FranceStudent / Remote MCP candidate

**Status:** active H-01/H-02 research target — transport feasibility and synthetic real-Optic reachability proven; production auth/session semantics still unproven.  
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

This proves basic remote Streamable HTTP tool reachability. It does **not** prove production authentication, conversation identity, multi-session mapping or real-project safety.

## Evidence integrity correction

One earlier FranceStudent/Claude response that looked like a new-conversation probe result was later explicitly admitted by the model to have been produced without a real tool call. That sample is discarded.

Separately, H-01 v1 fingerprinted `str(request_id)` even when the MCP Python SDK request id was absent. A stable fingerprint of `None` can therefore masquerade as a stable host signal. The earlier constant request-id fingerprint is **ambiguous** and must not be used as conversation evidence.

The rule is now:

> FranceStudent/model output alone is not proof of tool execution when server-side proof can be collected.

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

The remaining H-01 checks are now **evidence-pending** and require an intentional reconnect to the H-01 v2 endpoint/process. Do not confuse them with the separate PR #178 Real Optic Link endpoint used for today's synthetic real-Optic tests.

Remaining H-01 checks:

1. **New conversation separation** — same correlation run id, genuinely new FranceStudent chat, compare only server-verified calls.
2. **Reconnect** — toggle/reconnect the MCP server in the same visible conversation, then repeat a verified correlation call.
3. **Parallel dispatch** — two simultaneous `probe_parallel` calls using the same fresh parallel run id and `delay_ms=500`; require server-side `overlap_observed` proof.
4. **Optional cancellation/timeout** — only if the host exposes a safe UI action.
5. **Authentication/header shape** — measure the current FranceStudent auth/header UI with harmless values and local proof.

Do not compare process-local HMAC fingerprints across probe restarts.

## New FranceStudent authentication UI evidence

The current FranceStudent MCP server editor now visibly exposes:

- **Autorisation** labelled `Bearer token ou secret`;
- **Headers sécurisés (JSON)**.

This answers part of the earlier admin/host questions: FranceStudent now clearly has a credential field and a secure custom-header surface.

It still does **not** prove:

- that the credential field is sent exactly as `Authorization: Bearer <value>`;
- that it is OAuth;
- whether custom `Authorization` in the JSON field is accepted/rejected/merged;
- whether arbitrary harmless custom headers are delivered exactly;
- missing/wrong/revoked credential behavior.

H-02 must measure those facts rather than assume them.

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

Both matched local logs. This proves two real tool calls in one turn, **not** temporal overlap. True concurrent dispatch remains unproven until H-01 v2 `probe_parallel` or equivalent overlap instrumentation is used.

### New FranceStudent conversation on the same live adapter

One conversation produced verified `optic_session_info` `seq=25`, marker `16a1e00238f6c7f9`.

A genuinely new FranceStudent conversation, while keeping the same live adapter/tunnel, produced `seq=26`, marker `5198a949dec91035`, with the same `adapter_instance=20baecc2eeaf` and the same upstream-visible experimental session-info context (`file_read`, `file_search`, `policy_epoch=1`, principal `local-stdio`, project `local-workspace`, monotonic expiry value `1800000`).

Safe conclusion:

> Distinct FranceStudent conversations currently converge on the same long-lived PR #178 adapter/upstream experimental context.

This does **not** prove that FranceStudent provides no conversation identifier because PR #178 does not instrument H-01 correlation inputs. It does prove the current experimental bridge itself does not create one Optic application session per FranceStudent conversation. That is direct evidence for the necessity of H-03 `SessionResolver` semantics before real multi-conversation authority.

## What is already safe to conclude

- MCP `2026-07-28` should be treated as stateless at protocol level.
- Missing `Mcp-Session-Id` is expected, not a failure.
- No currently observed FranceStudent request/header/meta value is proven as conversation identity.
- Cloudflare/network tracing headers are not conversation identity.
- MCP client metadata may be stable across calls but is not conversation identity by itself.
- Request id is not authority and the v1 request-id evidence is unusable as a conversation signal.
- Host confirmation UI cannot be assumed.
- Host/model-rendered JSON cannot be trusted as invocation proof without corroboration.
- FranceStudent can reach the real Optic synthetic read/list/session-info path.
- The current PR #178 bridge does not isolate different FranceStudent conversations into separate upstream Optic application sessions.

H-03 must therefore keep an **Optic-minted explicit application-handle fallback** unless a later verified FranceStudent signal proves useful as a non-authoritative mapping input.

The real authority remains an independent application-owned `SessionHandle`.

## Authentication constraints

The production remote endpoint must authenticate independently from Cloudflare/Tailscale/provider identity.

Access tokens do not belong in URI query strings. Production Optic will not use `?token=...` URLs.

The current FranceStudent UI now makes a static bearer/key compatibility path plausible, but the exact wire contract must be measured.

Requirements remain:

- high-entropy locally generated connection credential;
- authentication per request/context;
- local revoke and rotation;
- no secret in logs/diagnostics/tool output;
- authenticated principal still has zero project/process authority until H-03/application policy maps it into an Optic-owned session and grants separate capability/lease authority.

Do not call the static compatibility mode OAuth unless the host actually performs an OAuth flow.

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
        v
Remote host adapter
  - endpoint authentication
  - transport hard limits
  - bounded HostContext
  - HostContext -> Optic-owned SessionHandle resolution
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
6. Host-provided conversation/session ids are correlation inputs only; Optic owns the real `SessionHandle`.
7. Session autonomy never removes per-action policy/resource/isolation revalidation.
8. Interpreter/repository-code execution remains subject to existing isolation gates regardless of transport.
9. Tunnel providers are replaceable adapters outside PolicyEngine.
10. No mandatory Optic-owned VPS/relay/database is introduced for the free path.
11. Removing one provider must not require changes to core policy, leases or tool semantics.
12. `session_cancel` is a control-plane operation and is not exposed casually through a shared remote adapter.

## Current gates

1. **H-00 complete** — product/architecture freeze.
2. **H-01 active / evidence-pending** — harness implemented; remaining host-correlation/concurrency/header measurements require a deliberate H-01 v2 run.
3. **H-02 pending** — endpoint authentication/revocation/rotation/bounds proof behind a zero-cost provider; new FranceStudent bearer/header UI is useful input but not yet measured on wire.
4. **H-03 pending** — evidence-driven `HostContext -> SessionResolver -> SessionHandle` contract with explicit-handle fallback.
5. **H-04/H-05 pending** — autonomy/local approval before production remote real-project effects.
6. **H-06+ gated** — production transport and host productization.

Remote mutation/process/Git exposure remains a later independent decision and must reuse the existing Optic authority model rather than weakening it.