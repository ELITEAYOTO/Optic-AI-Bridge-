# Host compatibility, zero-cost connectivity and session autonomy track

**Status:** LIVING SUBTRACK  
**Last reviewed:** 2026-10-12

This subtrack records the product and architecture work required to make Optic AI Bridge usable by large numbers of students across multiple AI clients without coupling the core runtime to one host, one transport provider, one paid service or one host-specific approval UI.

The existing core security/runtime work remains authoritative. This track changes how hosts connect, how host activity is mapped into Optic-owned sessions, and how a human can grant bounded autonomy without being prompted for every normal development action.

For the compact active-state summary see `CURRENT_STATE_2026-10-12.md`. For cross-cutting remote security blockers see `../security/REMOTE_AUTONOMY_AUDIT_GATES.md`.

## Product invariants

1. **Optic itself must remain free to use.** The project must not require an Optic subscription, paid relay, paid hosted backend, OpenAI API credits or another developer-funded server for normal operation.
2. **The student must not need a paid connectivity service.** Free third-party connectivity such as Tailscale or Cloudflare is acceptable. No single provider is mandatory.
3. **MCP is an adapter, not the product boundary.** FranceStudent and ChatGPT are V1 host targets where the actual host surface supports the required path; additional adapters remain possible.
4. **The bridge remains local-first.** Project data and execution stay on the student's machine unless an explicitly authorized action requires otherwise.
5. **Connectivity is never authority.** A stdio connection, HTTP endpoint, tunnel URL, bearer credential, host conversation id, request id or provider account never mints project/process/file authority by itself.
6. **Trust is session-scoped for V1.** A human may grant autonomy for the current Optic session mapping. Longer-lived grants are later work.
7. **Normal trusted development must not become a prompt loop.** Once a bounded autonomy grant is active, repeated reads, edits, builds, tests and approved toolchain use inside its scope should not require repeated human confirmation.
8. **Every effect remains independently checked.** Session, lease, policy epoch, profile, path, resource, network and isolation validation still run for each action.
9. **Revocation must remain local and truthful.** The user must be able to stop/revoke a session from Optic even if the remote host remains connected, and revoke semantics must account for already-admitted work.
10. **Host evidence must be real evidence.** A model's prose/JSON is not accepted as proof that a tool executed when server-side proof can be collected.

## UX direction from the Figma product concept

The current Figma concept already contains the product structure needed by this track:

- onboarding with project/client/connection selection;
- provider-independent connection concept and advanced custom HTTPS path;
- FranceStudent connection management;
- project permissions and security profiles;
- session/activity views and explicit session revocation;
- diagnostics and provider recovery;
- a floating Windows companion capable of surfacing approval-required, connection and project states.

The key product rule remains:

> **The connection transports requests. The profile decides rights.**

The desktop application is not a prerequisite for proving the transport/session architecture. Its implementation remains later work after host/session/auth/autonomy/local-approval service boundaries stabilize.

## Architecture direction

```text
AI host
(ChatGPT / FranceStudent / future client)
        |
        v
Host adapter
(MCP stdio / MCP Streamable HTTP / future adapter)
        |
        v
Connection authentication + bounded HostContext
        |
        v
SessionResolver
(host context -> Optic-owned SessionHandle)
        |
        v
Optic application services
        |
        v
ActionEnvelope + capabilities + leases + policy
        |
        v
Runtime + Windows containment
```

Transport providers sit beside the host adapter, not inside authorization:

```text
MCP Streamable HTTP
        |
        +-- direct/local
        +-- Tailscale
        +-- Cloudflare
        +-- custom HTTPS provider
        +-- future provider
```

A provider makes an endpoint reachable. It cannot decide what that endpoint may do.

## HostContext and session mapping

Every host adapter should normalize only bounded, understood metadata into `HostContext`, for example:

- adapter kind/version;
- authenticated connection principal;
- protocol revision/client capabilities;
- host-provided conversation correlation when actually proven;
- reconnect generation for diagnostics;
- locally selected project reference.

`SessionResolver` then maps that context to an opaque Optic-owned `SessionHandle`.

Host-provided ids are correlation inputs only. They are not capabilities, leases or bearer authority.

Because MCP `2026-07-28` is stateless, H-03 must support two evidence-backed strategies:

1. a proven host conversation signal used only as a mapping key; or
2. an Optic-minted explicit application handle threaded between calls.

The second strategy is the required fallback while conversation identity remains unproven.

## Autonomy model

### Review

Conservative mode. Read-only project activity may be broadly allowed within the selected project. Mutations, execution and sensitive effects may require local review according to policy.

### Project Autonomy

Recommended trusted-development mode. The current session may autonomously perform the normal development loop inside one selected project and within bounded toolchain/policy limits:

- read/search project files;
- write/patch/delete where the project profile permits;
- Git read and approved integration workflows;
- start approved project toolchains;
- build/test/read outputs;
- repeat without per-action confirmation while all deterministic checks continue to pass.

This is not unrestricted machine access.

### Full Access / advanced high-authority mode

A later advanced mode may grant broader authority after explicit local human choice, but the label never means Optic disables hard safety boundaries.

Even there, the AI cannot:

- change Optic's own security policy by itself;
- silently elevate administrator privileges;
- mint or widen its own authority;
- bypass isolation/resource ceilings;
- disable audit/revocation;
- access protected system/credential boundaries without dedicated proven contracts.

A disclaimer is not containment.

## Tool execution: hybrid model

Exact ToolProfiles remain useful without requiring a confirmation maze.

Planned direction:

1. application/operator-owned executable identity remains pinned and classified;
2. known development toolchains can be discovered/configured locally;
3. structured executable + argv remains the primitive; opaque shell strings do not become the fundamental API;
4. session autonomy can authorize repeated invocations of an approved toolchain envelope within project/resource/isolation limits;
5. a genuinely new executable/toolchain can require one local decision before joining the current session's bounded catalog;
6. high-risk interpreters/repository code remain subject to the proven isolation path.

## Approval UX direction

MCP elicitation remains a compatibility option, not the only approval mechanism.

The long-term human authority path must be able to live locally in Optic (desktop app/widget, with a later headless fallback):

```text
remote action requires authority
        -> Optic creates bounded pending request id
        -> local Optic UI displays request
        -> human accepts/rejects / chooses session scope
        -> Optic revalidates session + lease + policy + profile + expiry
        -> grant is issued only after successful revalidation
```

The pending request id carries no authority and must be bounded, expiring, single-owner and replay-safe.

## Evidence — FranceStudent H-01

### Transport proof

FranceStudent has accepted a public MCP Streamable HTTP `/mcp` endpoint through a temporary Cloudflare Quick Tunnel from a real Windows PC. Real remote POST traffic reached the local harmless probe and `probe_ping` succeeded.

Negotiated MCP revision was `2026-07-28`; no modern protocol transport session id was observed.

### Same-conversation evidence

Two `probe_session_correlation` calls in the same FranceStudent conversation were corroborated by fresh local server traffic before the host outage. Core MCP client metadata remained stable while several network/tracing values varied.

That alone is **not** a conversation identifier.

### Evidence-integrity correction

One earlier FranceStudent/Claude response for a supposed new-conversation test was later explicitly admitted by the model to have been generated without a real tool invocation. It is excluded.

The v1 probe also fingerprinted `str(request_id)` even when the MCP SDK request id was absent. The previously stable request-id fingerprint is therefore ambiguous and excluded as a conversation signal.

`h01-v2` corrects this by:

- emitting `request_id_present` explicitly;
- using `request_id_fingerprint=null` when absent;
- returning a random per-call `probe_invocation` marker;
- returning a random per-server-process instance id;
- printing matching `[H01] TOOL_CALL ...` proof locally;
- comparing appeared/disappeared correlation keys rather than only intersections;
- grouping parallel-overlap evidence by explicit parallel `run_id`;
- broadening credential-like header-name redaction.

### Host UI observation

No host-owned per-call approval/confirmation UI was observed for the harmless tool calls tested. The shown FranceStudent MCP configuration used `Authentication: None` and did not visibly expose a custom-header field. This is an observation of the shown UI, not proof that the product can never support headers elsewhere.

### Current external block

FranceStudent then reported a host-side service incident. Remaining checks are **blocked by host**, not failed:

- clean new-conversation separation under v2 proof;
- reconnect comparison under the same proof standard;
- grouped parallel dispatch;
- optional safe cancellation/timeout;
- exact authentication/header behavior.

When service returns, use a fresh v2 server process and fresh correlation/parallel run ids. Old process-local fingerprints are not comparable across restarts.

## Evidence — real Optic MCP link experiment

Draft PR #178 adds an experiment-only bridge:

```text
Streamable HTTP MCP client
  -> bounded Python compatibility adapter
  -> real Rust `optic-bridge` stdio MCP server
  -> disposable synthetic workspace
```

Remote allowlist is exactly:

- `optic_session_info` -> upstream `session_info`;
- `optic_fs_list` -> upstream `fs_list`;
- `optic_fs_read` -> upstream `fs_read`.

The adapter never accepts a caller-selected real workspace root and does not expose process tools, mutations, Git, `session_cancel` or arbitrary upstream method names.

On experiment head `ef5bc84144670135d13f55a59a782611aeae762f`:

- dedicated Real Optic MCP Link workflow #11 is green on Linux + Windows;
- normal CI #653 is green on dependency policy + Ubuntu + Windows;
- real Optic session/list/read behavior is reached;
- real Optic rejects traversal;
- two HTTP reads can be forwarded concurrently through the same long-lived upstream stdio session;
- each successful adapter call has a server-minted invocation proof;
- a new HTTP/MCP client connection can reconnect to the same adapter and observe the same long-lived upstream Optic application-session fingerprint.

Reconnect continuity is transport evidence only. It is not FranceStudent conversation identity and not H-03.

The Python experiment graph is frozen through shared constraints, with `pip check` and `pip freeze --all` in CI.

Experiment admission bounds currently include 64 KiB request body, 32 admitted HTTP tasks, 8 concurrent upstream forwards, 250 ms admission wait, 15 s upstream timeout and 5 s keep-alive.

## Remote security gates before real-project exposure

The synthetic real-link experiment must remain synthetic while these cross-capability findings are open:

- **#176** — sensitive content + metadata/listing classifier/policy and canonical list authorization;
- **#180** — generic file mutation must not alter protected `.git` repository-control state and bypass dedicated Git authority;
- **#181** — Unix native path segments must round-trip losslessly into authority-bearing `WorkspacePath`;
- **#182** — `GitRead` must not bypass future sensitive-content policy through `git_diff` / `git_status`;
- **#183** — `fs_read`, `fs_list` and Git read must hold session admission through the effect so revoke/quiescence semantics are truthful.

See `../security/REMOTE_AUTONOMY_AUDIT_GATES.md` for the consolidated contract.

## Remote authentication constraints

H-02 implementation is still pending, but these constraints are already fixed:

1. provider/tunnel identity never substitutes for Optic endpoint authentication;
2. access tokens never go in URI query strings;
3. successful authentication creates only a connection principal/context, not project capability/session authority;
4. credentials must be locally revocable/rotatable;
5. reconnect must not resurrect revoked credentials or sessions.

Prefer standards-compatible `Authorization`/OAuth behavior where the host actually supports it. If FranceStudent exposes only a different documented token/key mechanism, measure its exact behavior before implementing a compatibility adapter. Do not call a static compatibility key “OAuth”.

## Gates

### H-00 — COMPLETE

Architecture/product freeze recorded by PR #169 / ADR-0012.

### H-01 — ACTIVE / HOST-BLOCKED

The hardened authority-free v2 probe is implemented in PR #170. Automated validation is green; remaining primary-host comparisons depend on FranceStudent service recovery.

H-01 is not complete until enough real-host evidence exists to inform H-02/H-03 honestly.

### H-02 — PENDING

Prove zero-cost remote authentication/revocation/rotation, bounds and reconnect behavior. No credentials in URLs/logs. Authentication remains separate from project authority.

### H-03 — PENDING

Define bounded `HostContext`, opaque Optic session creation/resolution, same/new chat rules where provable, reconnect behavior, cross-host separation, local revoke/reap and no authority from caller ids.

Also resolve the current single-startup-session production shape transactionally: read/process/mutation/Git-integration authorities are session-owned and cannot be turned multi-session by swapping one field.

The current 30-minute initial session lifetime also needs an explicit H-03/H-04 idle/absolute renewal design rather than accidental expiry semantics.

### H-04 — PENDING

Define transport-agnostic `Review`, `Project Autonomy` and advanced high-authority grant semantics. Every effect still revalidates. Project/toolchain/policy/resource drift cannot silently inherit authority. The AI cannot widen its own grant.

### H-05 — PENDING

Prove a minimal host-independent local approval service contract. A full desktop app is not required for the proof, but the contract must fit the later app/widget.

### H-06 — GATED

Production-bounded Streamable HTTP beside stdio. Per-request host activity resolves through `SessionResolver`; transport-specific logic stays out of policy/runtime services.

PR #178 proves that extra host tests can already use the real core through a compatibility adapter, so H-06 does not need to be rushed merely to obtain evidence.

### H-07 — FranceStudent V1

Authenticated connection, local project selection, session mapping, read/mutation/process profile, autonomy, local revoke, reconnect/stale-session negatives and zero-cost setup docs.

### H-08 — ChatGPT adapter

Support only compatible surfaces/plans; never weaken Optic because one ChatGPT surface lacks the required path.

### H-09 — provider setup adapters

Tailscale, Cloudflare, custom HTTPS/manual and future providers behind one product concept.

### H-10 — desktop application

Implement the visual app after contracts stabilize, using the existing Figma concept as the baseline.

## Explicit non-goals for the current gate

Do not yet:

- replace stdio with HTTP in production;
- delete or weaken A-08 reusable approval;
- weaken `ActionEnvelope`, lease, policy, ToolProfile or isolation checks;
- add unrestricted shell execution;
- bind authority to tunnel/transport/request/conversation ids;
- expose a real repository through the synthetic PR #178 adapter;
- expose `session_cancel` as a generic shared remote tool;
- begin the full desktop UI implementation;
- promise unsupported ChatGPT plan/surface compatibility;
- require one tunnel provider;
- invent FranceStudent authentication/header semantics.

## Definition of done for this track

This track is complete only when at least one supported remote student host can connect through a zero-cost setup, authenticate, map into an Optic-owned session, receive a bounded human-selected autonomy profile, perform a realistic development loop without repeated approval spam, and be locally revoked without weakening existing policy/isolation/security boundaries.