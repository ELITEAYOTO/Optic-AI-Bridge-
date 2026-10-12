# FranceStudent / Remote MCP candidate

**Status:** active H-01 research target — transport feasibility proven; conversation identity and production auth/session semantics still unproven.  
**Recorded:** 2026-10-08  
**Last updated:** 2026-10-12

## Motivation

Optic may expose the same application-owned security/runtime core through a remote host adapter so a cloud client such as FranceStudent can use the Optic instance running on the student's own Windows PC.

The intended product property is local-first and zero mandatory maintainer infrastructure: the user's PC keeps the workspace, CPU/RAM, state and Optic authority. A tunnel, if used, transports HTTPS only and never becomes an authorization boundary.

FranceStudent is the priority remote V1 host. ChatGPT remains a target where the user's plan/surface exposes a compatible path. MCP itself remains an adapter rather than an Optic core requirement.

## Verified transport evidence

A real Windows PC has proven this path:

```text
FranceStudent
  -> HTTPS
  -> MCP Streamable HTTP /mcp
  -> Cloudflare Quick Tunnel
  -> local H-01 probe
```

Accepted observations before the current host incident:

- FranceStudent displayed the custom MCP server as active;
- real remote POST traffic reached the local probe;
- `probe_ping` succeeded;
- negotiated MCP revision was `2026-07-28`;
- no modern MCP transport session id was observed;
- two same-conversation `probe_session_correlation` calls were corroborated by fresh server traffic;
- no host-owned per-call approval/confirmation UI was observed for those harmless calls;
- the shown MCP configuration used `Authentication: None` and did not visibly expose a custom-header field.

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

## Current real-host block

FranceStudent subsequently reported a service-wide technical incident. The remaining checks are therefore **blocked by host**, not failed.

When FranceStudent recovers, use a **fresh v2 server process + fresh run ids** and require matching local invocation proof.

Remaining checks:

1. **New conversation separation** — same correlation run id, genuinely new FranceStudent chat, compare only server-verified calls.
2. **Reconnect** — toggle/reconnect the MCP server in the same visible conversation, then repeat a verified correlation call.
3. **Parallel dispatch** — two simultaneous `probe_parallel` calls using the same fresh parallel run id and `delay_ms=500`; require server-side `overlap_observed` proof.
4. **Optional cancellation/timeout** — only if the host exposes a safe UI action.
5. **Authentication/header shape** — inspect actual FranceStudent auth configuration; test harmless header behavior only if the UI exposes it.

Do not compare process-local HMAC fingerprints across probe restarts.

## What is already safe to conclude

- MCP `2026-07-28` should be treated as stateless at protocol level.
- Missing `Mcp-Session-Id` is expected, not a failure.
- No currently observed FranceStudent request/header/meta value is proven as conversation identity.
- Cloudflare/network tracing headers are not conversation identity.
- MCP client metadata may be stable across calls but is not conversation identity by itself.
- Request id is not authority and the v1 request-id evidence is unusable as a conversation signal.
- Host confirmation UI cannot be assumed.
- Host/model-rendered JSON cannot be trusted as invocation proof without corroboration.

H-03 must therefore keep an **Optic-minted explicit application-handle fallback** unless a later verified FranceStudent signal proves useful as a non-authoritative mapping input.

The real authority remains an independent application-owned `SessionHandle`.

## Authentication constraints

The production remote endpoint must authenticate independently from Cloudflare/Tailscale/provider identity.

Access tokens do not belong in URI query strings. Production Optic will not use `?token=...` URLs.

Preferred direction where supported:

- standard `Authorization` bearer behavior and standards-native OAuth/resource discovery when the host actually supports it.

Compatibility direction where required:

- a locally generated high-entropy static bearer/key mechanism may be considered only if the real FranceStudent configuration supports such a mechanism and its exact request behavior is measured;
- document it honestly as a compatibility credential, not OAuth;
- support local revoke and rotation;
- never log the secret or return it from a tool.

Authentication creates only a connection principal/context. It does not mint project capabilities, leases, autonomy grants or a caller-selected internal session handle.

## Real Optic link available for next-stage host tests

Draft PR #178 now proves a separate synthetic-only end-to-end path:

```text
remote Streamable HTTP MCP client
  -> bounded compatibility adapter
  -> real Rust `optic-bridge` stdio server
  -> disposable synthetic workspace
```

The remote allowlist is exactly:

- `optic_session_info`;
- `optic_fs_list`;
- `optic_fs_read`.

No process, mutation, Git, `session_cancel` or arbitrary upstream forwarding is exposed.

On experiment head `ef5bc84144670135d13f55a59a782611aeae762f`, dedicated workflow #11 is green on Linux + Windows and normal CI #653 is green on dependency policy + Ubuntu + Windows.

This gives the next FranceStudent run a much stronger test target than `probe_ping`: it can exercise the real Optic session/policy/lease/filesystem stack while still touching only adapter-created synthetic data.

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

## Transport candidate

Primary candidate: MCP **Streamable HTTP `/mcp`**.

Do not implement legacy `/sse` merely because an example mentions it. Add compatibility transports only when a real important client requires them.

## Connectivity / zero-cost candidates

Interchangeable options remain the goal:

- Cloudflare Quick Tunnel for authority-free/synthetic experiments only;
- Tailscale/Funnel or equivalent user-owned path subject to real validation;
- named/custom tunnel or user reverse proxy for advanced users;
- future provider adapters without core changes.

No third-party free tier is promised permanent. Durability comes from provider interchangeability and no mandatory maintainer-hosted infrastructure.

## Current gates

1. **H-00 complete** — product/architecture freeze.
2. **H-01 active / host-blocked** — hardened real-host characterization, remaining FranceStudent checks await host recovery.
3. **H-02 pending** — endpoint authentication/revocation/rotation/bounds proof behind a zero-cost provider.
4. **H-03 pending** — evidence-driven `HostContext -> SessionResolver -> SessionHandle` contract with explicit-handle fallback.
5. **H-04/H-05 pending** — autonomy/local approval before production remote real-project effects.
6. **H-06+ gated** — production transport and host productization.

Remote mutation/process/Git exposure remains a later independent decision and must reuse the existing Optic authority model rather than weakening it.