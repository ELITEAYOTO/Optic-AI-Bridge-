# Roadmap

The canonical long-form roadmap remains in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md). The active project state is summarized in [docs/product/CURRENT_STATE_2026-10-12.md](docs/product/CURRENT_STATE_2026-10-12.md), and the detailed engineering checkpoint is [docs/product/PROJECT_HEALTH_2026-10-12.md](docs/product/PROJECT_HEALTH_2026-10-12.md).

**Last reviewed:** 2026-10-12  
**Current phase:** Phase 4 same-repository work continues, with the H-track running in parallel  
**Lifecycle:** executable Rust pre-alpha  
**Release:** no production-supported release

## Current headline

Optic's local Rust security/runtime core remains the source of truth. The remote-host work is evidence-gated: measure FranceStudent/other hosts, keep transport/authentication separate from project authority, and only then introduce production multi-host session resolution/autonomy.

The strongest current remote proof is draft PR #178: FranceStudent can reach a bounded Streamable HTTP compatibility adapter which forwards into the **real Rust `optic-bridge` stdio application server** against a disposable synthetic workspace. Live host calls are server-correlated with adapter markers.

FranceStudent admin answers on 2026-10-12 also remove two major architecture unknowns:

- no per-conversation/chat id is currently sent to MCP servers;
- the authorization field sends exactly `Authorization: Bearer <value>`.

Real project exposure is still deliberately blocked.

## Existing A-08 reusable approval state

- **A-08A complete** — bounded reusable approval domain model tied to one Optic session, exact ToolProfile fingerprint, policy epoch and monotonic expiry.
- **A-08B complete** — bounded broker lifecycle, owner-scoped revocation/cleanup and shared lifecycle integration.
- **A-08C complete** — profiled process execution can reuse a matching current-session grant only after fresh policy/profile/resource/isolation revalidation.
- **A-08D complete** — bounded MCP form choice between `once` and `current_session`; malformed/ambiguous accepted content cannot mint reusable authority.
- **A-08E blocked by host interaction** — direct/server-side reusable approval works, but tested host surfaces do not consistently expose the nested interaction needed for the real-host UX proof.
- **A-08F moved behind H-03** — wording such as “this conversation” is allowed only after a real adapter proves a trustworthy mapping. Until then use “current Optic session”.

## Provider-neutral product invariants

- Optic itself remains free to use without a mandatory paid Optic backend, relay or API-credit path for normal student operation.
- Tailscale, Cloudflare, custom HTTPS and future providers are replaceable connectivity adapters, never authorization authorities.
- MCP is a host adapter, not the Optic core.
- FranceStudent is the priority V1 remote host target; ChatGPT is supported only on compatible surfaces/plans.
- Remote transport and authentication do not mint project capability, leases, session handles or autonomy grants.
- V1 trust/autonomy is scoped to an Optic-owned session mapping and remains locally revocable.
- Trusted development should support realistic `read -> edit -> build -> test -> inspect -> fix -> repeat` loops without per-action approval spam while every action still passes deterministic policy/resource/isolation checks.
- Human authority must have a local Optic path; host elicitation/confirmation UI is optional compatibility behavior, not the only consent boundary.

## H-track

### H-00 — architecture/evidence freeze — COMPLETE

Frozen direction:

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

### H-01 — harmless host characterization — ACTIVE / COMPATIBILITY EVIDENCE REMAINING

Draft PR #170 contains the authority-free `h01-v2` probe.

Earlier verified FranceStudent facts:

- public Streamable HTTP `/mcp` through a temporary Cloudflare Quick Tunnel works;
- real remote requests reached the local Windows probe;
- `probe_ping` succeeded;
- MCP `2026-07-28` negotiated;
- no modern transport session id was observed;
- two same-conversation correlation calls were verified with server activity;
- no host-owned per-call confirmation UI was observed for those harmless calls.

Evidence-integrity hardening:

- one host/model JSON sample admitted to being invented is excluded;
- v1 request-id evidence is ambiguous and excluded;
- `h01-v2` adds explicit request-id presence, `probe_invocation`, `server_instance` and grouped server-side overlap proof.

FranceStudent admin now confirms:

- no per-chat/conversation id is sent to MCP servers today;
- MCP config is account-global;
- secure custom headers are sent on every request and hidden from the model;
- the bearer field maps to `Authorization: Bearer <value>`;
- there is no native per-call confirmation;
- parallel calls are allowed (current host cap reported around 8, planned to change/remove);
- MCP cancellation is not currently supported;
- reconnect/context behavior is unspecified.

Remaining H-01 work is therefore compatibility characterization rather than architecture discovery:

- reconnect behavior;
- true grouped parallel overlap;
- harmless secure-header wire regression;
- dummy bearer header-presence regression;
- host-side timeout characterization if obtainable.

A fresh same/new-chat correlation run may remain useful for diagnostics, but H-03 no longer waits for discovery of a host chat id.

### H-02 — zero-cost remote authentication — READY FOR PROOF IMPLEMENTATION

Issue #172 owns this gate.

FranceStudent V1 carrier is admin-confirmed:

```http
Authorization: Bearer <value>
```

Secure custom JSON headers are also sent on every request and hidden from the model.

This should be treated as a **static bearer compatibility mode**, not OAuth.

H-02 now needs implementation/security proof rather than host-field discovery:

- missing/malformed/wrong credential rejection;
- valid credential permits only the harmless authenticated connection path;
- local revoke + rotation;
- reconnect after revoke cannot resurrect auth;
- secret redaction from logs/diagnostics/tool output;
- constant-time verification where applicable;
- bounded body/header/concurrency/rate behavior;
- authentication result separated from H-03 project/session authority.

The unanswered FranceStudent host timeout is useful operational input, not a blocker, because Optic must enforce its own deadlines.

### H-03 — SessionResolver — READY FOR CONTRACT IMPLEMENTATION / EMPIRICALLY JUSTIFIED

Issue #173 owns the contract.

PR #178 intentionally owns one long-lived upstream stdio Optic application session. In a live test, one FranceStudent conversation produced verified `optic_session_info` `seq=25`; a genuinely new FranceStudent conversation produced `seq=26` without restarting the adapter, both on `adapter_instance=20baecc2eeaf` and the same upstream-visible experimental context.

FranceStudent admin also confirms **no per-conversation/chat id is sent to the MCP server**.

Therefore FranceStudent V1 baseline is no longer “wait for a host correlation signal or use a fallback”. It should be:

```text
Authorization bearer
  -> authenticated connection principal
  -> Optic-minted explicit opaque application handle
  -> SessionResolver
  -> internal Optic-owned SessionHandle
  -> session-owned authority
```

The application handle is not a capability and cannot mint/widen authority. It must be principal-bound, opaque, bounded, expiring/revocable and resolved only through active server-owned state.

H-03 still owns:

- transactional per-session subordinate authority provisioning;
- simultaneous-first-call / duplicate-bootstrap behavior;
- cross-principal rejection;
- revoke/replay/no-resurrection semantics;
- reconnect attach/create rules;
- capacity bounds and cleanup;
- final session lifetime semantics (idle vs absolute lifetime and bounded renewal).

Reconnect measurement remains useful for UX but must not define authority.

### H-04 — bounded session autonomy — PENDING

Issue #174 owns `Review`, `Project Autonomy` and any later advanced high-authority mode.

Autonomy may suppress repeated human prompts but never bypasses per-effect session/lease/policy/profile/resource/path/isolation checks. The AI cannot mint or widen its own authority.

### H-05 — local approval broker — PENDING / HOST CONSTRAINT CONFIRMED

Issue #175 owns a host-independent local authority path suitable for the later desktop app/widget. Pending request ids carry no authority and must be bounded, expiring, owner-bound and replay-safe.

FranceStudent admin confirms there is **no native per-tool-call confirmation** and **no MCP cancellation support today**. Therefore Optic local approval/revoke/deadline/cleanup semantics cannot depend on FranceStudent UI or cancellation propagation.

### H-06 — production Streamable HTTP adapter — GATED

Only after enough H-02/H-03/H-04/H-05 contracts/evidence exist:

- production-bounded Streamable HTTP lives beside stdio, not instead of it;
- per-request activity resolves through `SessionResolver`, not one startup session embedded in the server instance;
- equivalent body/response/concurrency/admission bounds remain enforced;
- tool errors use a host-compatible transport/MCP mapping;
- no host/provider-specific policy logic is pushed into filesystem/Git/process services.

### H-07 — FranceStudent V1

Authenticated connection, local project selection, explicit Optic application/session handle mapping, bounded autonomy, local revoke, reconnect/stale-session negatives and zero-cost setup documentation.

### H-08 — ChatGPT adapter

Only on compatible surfaces/plans. ChatGPT availability is not an Optic product dependency.

### H-09 — provider adapters/setup automation

Tailscale reference path, Cloudflare reference path, custom HTTPS/manual path and future providers behind one product concept.

### H-10 — desktop application

Implement the full visual application after host/session/auth/autonomy/local-approval service contracts stabilize. The existing Figma work remains the UX baseline.

## Real Optic MCP link experiment — PR #178

Current experiment:

```text
FranceStudent / Streamable HTTP client
  -> bounded Python compatibility adapter
  -> real Rust `optic-bridge` stdio server
  -> disposable synthetic workspace
```

Remote tools are exactly:

- `optic_session_info`;
- `optic_fs_list`;
- `optic_fs_read`.

No process, mutation, Git, `session_cancel` or arbitrary upstream forwarding is exposed.

Automated state on head `ef5bc84144670135d13f55a59a782611aeae762f`:

- Real Optic Link workflow #11 green Linux + Windows;
- normal CI #653 green dependency policy + Ubuntu + Windows;
- real session/list/read paths, traversal rejection, bounded successful simultaneous client tasks (actual upstream effect overlap unmeasured), per-call proof and transport-client reconnect continuity proven.

Live FranceStudent evidence on 2026-10-12:

- exact three-tool discovery;
- server-correlated real read at `seq=22`, marker `126b81f5327c1b59`;
- real traversal denial for `../outside.txt`;
- two real read calls in one assistant turn (`seq=23`, `seq=24`), without claiming temporal overlap;
- a new FranceStudent conversation continued to the same live adapter/upstream experimental context (`seq=25 -> 26`, same adapter instance).

See `docs/research/REAL_OPTIC_MCP_LINK_EXPERIMENT.md` and `docs/research/FRANCESTUDENT_REAL_LINK_EVIDENCE_2026-10-12.md` for accepted interpretation and non-claims.

## Security gates before real-project remote exposure

See [docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md](docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md).

Open blockers include:

- **#176:** sensitive-content + metadata/listing classifier/policy, canonical listing authorization and pagination progress;
- **#180:** generic `FileWrite`/`FileDelete` must not modify protected `.git` control state and bypass `GitIntegrate`;
- **#181:** Unix native path segments must round-trip exactly into authority-bearing `WorkspacePath`;
- **#182:** `GitRead` must not bypass sensitive-content policy through diff/status;
- **#183:** `fs_read`, `fs_list` and Git read must hold session admission through the effect so revoke/quiescence semantics are truthful.

Draft PR #184 is an unmerged hardening candidate for #183/#181/#176-partial. It does not close the broader real-project gate until reviewed, tested and merged with the remaining security contracts satisfied.

## Coding candidates that are not host-blocked

Highest-value narrow work that can proceed now:

1. Continue/review #184 for #183 admission, #181 native path safety and #176 pagination partial.
2. Finish #176 canonical/prepared `fs_list` authorization and the sensitive classifier contract.
3. #180 protected Git control namespace for generic mutation, after focused fixture tests.
4. #182 GitRead/sensitive-content policy decision and regressions.
5. Build the H-02 bearer proof harness now that the carrier is known.
6. Freeze H-03 explicit application-handle / principal-binding / provisioning / lifetime contract.
7. Optional synthetic mutation extension of PR #178 only after auditing the real mutation service; keep it disposable/no-Git/no-process/no real workspace and route effects through real Optic transactional mutation.

No further FranceStudent admin answer is required before starting H-02 proof work or H-03 contract work.

## Documentation and evidence rules

- `docs/product/CURRENT_STATE_2026-10-12.md` is the compact active-state snapshot.
- `docs/product/PROJECT_HEALTH_2026-10-12.md` is the detailed engineering health checkpoint.
- Historical `STATUS.md` remains the implementation ledger and should be synchronized surgically, not replaced wholesale.
- `PROJECT_CONTINUITY_PROMPT.md` is the context-recovery prompt for a fresh AI conversation.
- Every milestone/security decision should update the owning doc/ADR/issue/PR body and exact-head CI evidence.
- Host/model prose alone is not accepted as tool-execution proof when local invocation evidence is available.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.

## Remaining host questions / measurements

There are no remaining FranceStudent-admin questions that are architecture-blocking for H-02/H-03.

Useful but non-blocking:

- maximum tool-call timeout / timeout behavior;
- reconnect behavior after disable/re-enable or network reconnection;
- duplicate `Authorization` precedence if both auth surfaces are used;
- future evolution of the current ~8-call host cap;
- future cancellation support.

Optic must remain correct if those host behaviors change by keeping authentication, admission, deadlines, cancellation/revoke and resource limits server-owned.
