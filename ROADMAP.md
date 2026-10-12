# Roadmap

The canonical long-form roadmap remains in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md). The active project state is summarized in [docs/product/CURRENT_STATE_2026-10-12.md](docs/product/CURRENT_STATE_2026-10-12.md). The reusable-approval subtrack is in [docs/product/A08_REUSABLE_APPROVAL_TRACK.md](docs/product/A08_REUSABLE_APPROVAL_TRACK.md), and the active multi-host / zero-cost / session-autonomy direction is in [docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md](docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md).

**Last reviewed:** 2026-10-12  
**Current phase:** Phase 4 same-repository work continues, with the H-track running in parallel  
**Lifecycle:** executable Rust pre-alpha  
**Release:** no production-supported release

## Current headline

Optic's local Rust security/runtime core remains the source of truth. The current remote-host work is deliberately evidence-gated: prove what FranceStudent/other hosts actually expose, keep transport/authentication separate from project authority, and only then introduce production multi-host session resolution/autonomy.

The most important new proof is draft PR #178: a bounded Streamable HTTP MCP compatibility adapter can forward into the **real Rust `optic-bridge` stdio application server** against a disposable synthetic workspace. This works on Linux and Windows and does not require a speculative rewrite of the production core merely to continue host testing.

## Existing A-08 reusable approval state

- **A-08A complete** — bounded reusable approval domain model tied to one Optic session, exact ToolProfile fingerprint, policy epoch and monotonic expiry.
- **A-08B complete** — bounded broker lifecycle, owner-scoped revocation/cleanup and shared lifecycle integration.
- **A-08C complete** — profiled process execution can reuse a matching current-session grant only after fresh policy/profile/resource/isolation revalidation.
- **A-08D complete** — bounded MCP form choice between `once` and `current_session`; malformed/ambiguous accepted content cannot mint reusable authority.
- **A-08E blocked by host interaction** — direct/server-side reusable approval works, but tested host surfaces do not consistently expose the nested interaction needed for the real-host UX proof.
- **A-08F moved behind H-03** — wording such as “this conversation” is allowed only after a real adapter proves a trustworthy mapping. Until then use “current Optic session”.

A reusable approval never mints new capability, filesystem/network authority, process eligibility or larger resource ceilings.

## Provider-neutral product invariants

- Optic itself must remain free to use and must not require a paid Optic backend, relay or API-credit path for normal student operation.
- Tailscale, Cloudflare, custom HTTPS and future providers are replaceable connectivity adapters, never authorization authorities.
- MCP is a host adapter, not the Optic core.
- FranceStudent is the priority V1 remote host target; ChatGPT is supported only on surfaces/plans that expose a compatible path.
- Remote transport and authentication do not mint project capability, leases, session handles or autonomy grants.
- V1 trust/autonomy is scoped to an Optic-owned session mapping and remains locally revocable.
- Trusted development should support realistic `read -> edit -> build -> test -> inspect -> fix -> repeat` loops without per-action approval spam, while every action still passes deterministic policy/resource/isolation checks.
- Human authority must have a local Optic path; host elicitation/confirmation UI is optional compatibility behavior, not the only consent boundary.

## H-track

### H-00 — architecture/evidence freeze — COMPLETE

Frozen:

```text
AI host
  -> host adapter
  -> connection authentication + bounded HostContext
  -> SessionResolver
  -> Optic-owned SessionHandle
  -> application services
  -> ActionEnvelope / capabilities / leases / policy
  -> runtime / Windows containment
```

Provider connectivity is outside PolicyEngine.

### H-01 — harmless host characterization — ACTIVE / HOST-BLOCKED

Draft PR #170 contains the authority-free `h01-v2` probe.

Current verified FranceStudent facts:

- public Streamable HTTP `/mcp` through a temporary Cloudflare Quick Tunnel works;
- real remote requests reached the local Windows probe;
- `probe_ping` succeeded;
- negotiated MCP revision is `2026-07-28`;
- no modern transport session id was observed;
- two same-conversation correlation calls were verified with server activity;
- no host-owned per-call confirmation UI was observed for those harmless calls;
- the shown FranceStudent MCP configuration did not visibly expose a custom-header field.

Evidence-integrity correction:

- one host/model JSON sample was later admitted to have been invented without a real tool call and is excluded;
- the v1 constant request-id fingerprint is ambiguous because request ids are optional and absent `None` was fingerprinted;
- `h01-v2` now emits request-id presence explicitly and adds random `probe_invocation` + `server_instance` values mirrored in local `[H01] TOOL_CALL` logs;
- grouped parallel proof is server-side and keyed by an explicit parallel `run_id`.

Remaining real-host checks are currently blocked by a FranceStudent service incident, not failed. When service returns, use a fresh v2 server instance + fresh run ids and rerun new-chat separation, reconnect, grouped parallel dispatch and optional safe cancellation with matching local invocation proof.

No observed H-01 signal is authority. Conversation identity remains unproven.

### H-02 — zero-cost remote authentication — PENDING

Issue #172 owns this gate.

Requirements include:

- endpoint authentication independent from tunnel/provider identity;
- missing/wrong credentials rejected;
- valid credential produces only a connection principal;
- immediate local revoke and rotation;
- no credentials in URI query strings, ordinary logs or tool output;
- rate/size/concurrency/reconnect negative tests;
- standards-native OAuth/Authorization path where the host supports it, otherwise an explicitly documented compatibility mechanism measured from the real host UI.

Do not invent FranceStudent header/token semantics before measurement.

### H-03 — SessionResolver — PENDING

Issue #173 owns the contract.

`HostContext` remains bounded/non-authoritative. `SessionResolver` creates or resolves the opaque application-owned `SessionHandle`.

Because MCP `2026-07-28` is stateless, H-03 must either:

1. use a proven host conversation-correlation signal only as a mapping input; or
2. use an Optic-minted explicit application handle threaded across calls.

The second path is the required fallback until the first is proven.

### H-04 — bounded session autonomy — PENDING

Issue #174 owns `Review`, `Project Autonomy` and any later advanced high-authority mode.

Autonomy may suppress repeated human prompts but never bypasses per-effect session/lease/policy/profile/resource/path/isolation checks. The AI cannot mint or widen its own authority.

### H-05 — local approval broker — PENDING

Issue #175 owns a host-independent local authority path suitable for the later desktop app/widget. Pending request ids carry no authority and must be bounded, expiring, owner-bound and replay-safe.

### H-06 — production Streamable HTTP adapter — GATED

Only after H-01 through H-05 supply enough evidence:

- production-bounded Streamable HTTP lives beside stdio, not instead of it;
- per-request activity resolves through `SessionResolver`, not a single startup session embedded in the server instance;
- equivalent body/response/concurrency/admission bounds remain enforced;
- no host/provider-specific policy logic is pushed into filesystem/Git/process services.

### H-07 — FranceStudent V1

Authenticated connection, local project selection, session mapping, bounded autonomy, local revoke, reconnect/stale-session negatives and zero-cost setup documentation.

### H-08 — ChatGPT adapter

Only on compatible surfaces/plans. ChatGPT availability is not an Optic product dependency.

### H-09 — provider adapters/setup automation

Tailscale reference path, Cloudflare reference path, custom HTTPS/manual path and future providers behind one product concept.

### H-10 — desktop application

Implement the full visual application after host/session/auth/autonomy/local-approval service contracts stabilize. The existing Figma work remains the UX baseline.

## Real Optic MCP link experiment — PR #178

The current experiment proves:

```text
Streamable HTTP client
  -> bounded Python compatibility adapter
  -> real Rust `optic-bridge` stdio server
  -> disposable synthetic workspace
```

Remote tools are exactly:

- `optic_session_info`;
- `optic_fs_list`;
- `optic_fs_read`.

No process, mutation, Git, `session_cancel` or arbitrary upstream forwarding is exposed.

On experiment head `ef5bc84144670135d13f55a59a782611aeae762f`:

- dedicated Real Optic MCP Link workflow #11 is green on Linux + Windows;
- normal repository CI #653 is green on dependency policy + Ubuntu + Windows;
- real session/list/read paths are reached;
- traversal rejection comes from real Optic;
- concurrent forwarding is proven;
- per-call adapter invocation proof is present;
- a new HTTP/MCP client can reconnect to the same long-lived upstream Optic application session.

Reconnect continuity is transport evidence only, not conversation identity.

The shared Python experiment dependency graph is version-frozen and validated with `pip check` + `pip freeze --all`.

## Security gates before real-project remote exposure

See [docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md](docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md).

Open blockers include:

- **#176:** explicit sensitive-content + metadata/listing classifier/policy and canonical listing boundary;
- **#180:** generic `FileWrite`/`FileDelete` must not modify protected `.git` control state and bypass `GitIntegrate`;
- **#181:** Unix native path segments must round-trip exactly into authority-bearing `WorkspacePath`;
- **#182:** `GitRead` must not bypass sensitive-content policy through diff/status;
- **#183:** `fs_read`, `fs_list` and Git read must hold session admission through the effect so revoke/quiescence semantics are truthful.

The synthetic PR #178 experiment is intentionally safe relative to these gates because it uses adapter-created disposable data, zero mutation/process/Git authority and no remote session-cancel path.

## Documentation and evidence rules

- [docs/product/CURRENT_STATE_2026-10-12.md](docs/product/CURRENT_STATE_2026-10-12.md) is the compact active-state snapshot.
- Historical `STATUS.md` remains the implementation ledger and should be synchronized surgically, not replaced wholesale.
- [PROJECT_CONTINUITY_PROMPT.md](PROJECT_CONTINUITY_PROMPT.md) is the generic context-recovery prompt for a fresh AI conversation.
- Every milestone/security decision should update the owning doc/ADR/issue/PR body and exact-head CI evidence.
- Host/model prose alone is not accepted as tool-execution proof when local invocation evidence is available.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.