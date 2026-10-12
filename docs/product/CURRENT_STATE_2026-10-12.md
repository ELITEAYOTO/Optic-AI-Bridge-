# Optic AI Bridge — current state snapshot

**Snapshot date:** 2026-10-12  
**Lifecycle:** pre-alpha / evidence-gated multi-host work  
**Release:** no production-supported release  
**Primary remote-host target:** FranceStudent on Windows, with ChatGPT support only on compatible surfaces

This document is the compact active-state companion to the historical `STATUS.md` and the canonical product roadmap. For the detailed engineering checkpoint (`done / remaining / missing evidence / blockers / next coding candidates`), see [`PROJECT_HEALTH_2026-10-12.md`](PROJECT_HEALTH_2026-10-12.md).

## Core invariant

> **The AI decides what it wants to attempt. The bridge executes only authorized effects. Policy authorizes. Isolation contains.**

Remote transport, tunnels, host metadata and host conversations never become authority by themselves.

## Current production-core state

The Rust core/runtime remains the authoritative execution boundary. Important already-implemented properties include:

- application-owned `SessionHandle` sessions;
- typed `ActionEnvelope` effects;
- scoped capabilities and task leases;
- bounded transport/resource ceilings;
- transactional filesystem mutation with expected-state preconditions and crash-recovery journal;
- bounded Git read and exact-head Git integration paths;
- structured executable + argv process execution;
- Windows Job Object governance;
- Windows AppContainer isolation for the proven high-risk execution path;
- ToolProfile authorization and reusable current-session approval primitives;
- same-repository isolation/integration foundations from earlier phases.

A-08 reusable profile approval remains valid. A-08E is blocked by host interaction behavior, not by failure of the reusable approval core.

## Multi-host H-track

1. **H-00 — complete:** provider-neutral architecture, zero-cost requirement, Optic-owned sessions, local-first authority and no host/tunnel identity as authorization.
2. **H-01 — active / compatibility evidence remaining:** authority-free host characterization. The hardened `h01-v2` probe is implemented in draft PR #170. FranceStudent admin answers now close the chat-id and bearer-carrier discovery questions; reconnect/overlap/header-regression/timeout characterization remains useful.
3. **H-02 — ready for proof implementation:** FranceStudent V1 bearer carrier is known (`Authorization: Bearer <value>`). The authentication/revoke/rotate/redaction/bounds harness is still not implemented.
4. **H-03 — ready for contract implementation:** FranceStudent admin confirms no per-chat id is delivered to MCP. The V1 baseline should therefore be an Optic-minted explicit opaque application handle resolved to an internal Optic-owned `SessionHandle`.
5. **H-04 — pending:** `Review`, `Project Autonomy` and advanced high-authority session modes without approval spam, while every effect remains independently revalidated.
6. **H-05 — pending:** host-independent local approval broker suitable for the later Windows app/widget. FranceStudent has no native per-call confirmation and no MCP cancellation today, reinforcing this requirement.
7. **H-06+ — gated:** production Streamable HTTP adapter, FranceStudent closure, compatible ChatGPT adapter, provider automation and desktop app.

## Verified FranceStudent evidence

Accepted tool-execution evidence must be corroborated by the local probe/adapter where proof instrumentation exists, not merely by model prose. Direct admin answers are recorded separately as host product-contract input and may still receive harmless wire-regression tests.

### Earlier H-01 evidence

Verified on a real Windows PC:

- FranceStudent accepted a public MCP Streamable HTTP `/mcp` endpoint through a temporary Cloudflare Quick Tunnel;
- real remote POST traffic reached the local probe;
- `probe_ping` succeeded;
- negotiated MCP revision was `2026-07-28`;
- no modern MCP transport session id was observed;
- two same-conversation correlation calls were independently corroborated by fresh server traffic;
- no host-owned per-call approval/confirmation UI was observed for those harmless calls.

Evidence integrity corrections remain important:

- one earlier host/model JSON response was later admitted by the model to have been invented without a tool call and is excluded;
- the old v1 request-id fingerprint is excluded as a conversation signal because request ids are optional and the v1 probe fingerprinted absent `None` values ambiguously;
- `h01-v2` now emits explicit request-id presence plus server-minted per-call proof mirrored in local logs.

### FranceStudent admin-confirmed contract — 2026-10-12

Direct answers from a FranceStudent admin establish the following current behavior:

- **FranceStudent does not send a per-conversation/chat identifier to MCP servers today.** The admin noted this could be a future addition.
- MCP configuration is **global to the user's account**, so each conversation can access the configured MCP server.
- Secure custom JSON headers are supported, sent on **every request**, and are **hidden from the model**.
- The authorization field sends exactly:

```http
Authorization: Bearer <value>
```

- FranceStudent has **no native per-tool-call confirmation**.
- FranceStudent permits parallel tool calls; OpenAI/Claude decides whether to issue them. Admin reported a current host cap of roughly **8 calls**, with intent to remove it later. Optic must keep independent concurrency/admission bounds.
- MCP **cancellation is not currently supported** by FranceStudent.
- reconnect/context behavior is not specified by the admin and should be measured;
- the host-side maximum tool-call timeout remains unanswered/unmeasured.

These answers remove two major architectural uncertainties: FranceStudent V1 need not wait for a chat id, and the compatibility bearer carrier is known.

## Real Optic MCP link experiment — live FranceStudent proof

Draft PR #178 proves an experiment-only end-to-end path:

```text
FranceStudent / Streamable HTTP MCP client
        -> bounded Python compatibility adapter
        -> real Rust `optic-bridge` stdio MCP server
        -> disposable synthetic workspace
```

The remote experiment exposes exactly:

- `optic_session_info` -> real upstream `session_info`;
- `optic_fs_list` -> real upstream `fs_list`;
- `optic_fs_read` -> real upstream `fs_read`.

It deliberately excludes process execution, mutations, Git, `session_cancel` and arbitrary upstream tool forwarding.

Automated state on experiment head `ef5bc84144670135d13f55a59a782611aeae762f`:

- Real Optic MCP Link workflow #11: green Linux + Windows;
- normal CI #653: green dependency policy + Ubuntu + Windows;
- real traversal rejection, bounded concurrent forwarding, per-call proof and transport-client reconnect continuity are proven.

### Live server-correlated FranceStudent proof on 2026-10-12

The current FranceStudent `Optic-Test` connector is a new/current PR #178 Real Optic Link endpoint, not the old H-01 probe endpoint.

Verified with matching host JSON + local `[REAL-LINK] TOOL_CALL` evidence on adapter instance `20baecc2eeaf`:

- FranceStudent discovered only `optic_session_info`, `optic_fs_list`, `optic_fs_read`;
- an absolute Windows path to the real Desktop/repository was rejected and the host observed only the adapter-owned synthetic fixture;
- `optic_fs_read("public/hello.txt")` was proven end-to-end at `seq=22`, marker `126b81f5327c1b59`;
- a deliberate `../outside.txt` read was rejected with `optic.invalid_workspace_path` / JSON-RPC `-32602` (surfaced by FranceStudent as HTTP 400);
- two distinct reads were issued in one FranceStudent assistant turn (`seq=23` and `seq=24`, both matching local markers). This proves two real calls in one turn, **not** temporal overlap;
- one conversation produced verified `optic_session_info` `seq=25`, and a genuinely new FranceStudent conversation produced `seq=26` without restarting the live adapter; both reached the same `adapter_instance=20baecc2eeaf` and the same upstream-visible experimental session-info context.

Safe interpretation of the new-chat test + admin contract:

> The current PR #178 experimental bridge does not isolate different FranceStudent conversations into distinct Optic application sessions, and FranceStudent currently provides no chat id to the MCP server from which such isolation could be derived automatically.

Therefore H-03 FranceStudent V1 should use a **principal-bound Optic-minted explicit application handle** as the baseline application-continuity mechanism. That handle remains non-authoritative by itself; the real authority is the internal Optic-owned `SessionHandle` and application-owned grants/leases/policy state.

The manual Real Optic Link process intentionally stops after about 25 minutes, before the current ~30-minute Optic session lifetime.

## H-02 FranceStudent V1 direction

The carrier question is now closed at the host-contract level:

```http
Authorization: Bearer <connection credential>
```

plus optional secure custom headers sent on every request and hidden from the model.

This should be implemented/proven as a **static bearer compatibility mode**, not called OAuth.

H-02 still needs to prove:

- missing/malformed/wrong credential rejection;
- valid credential allows only the harmless authenticated connection path;
- immediate local revoke;
- rotation invalidates the old credential and accepts the new one;
- reconnect cannot resurrect revoked auth;
- no secret leakage in logs/diagnostics/tool output;
- constant-time validation where applicable;
- independent Optic request/header/concurrency/rate bounds;
- authentication remains separate from H-03 project/session authority.

A harmless wire-level dummy-bearer/custom-header regression is still worthwhile, but there is no longer an admin-answer blocker before designing the H-02 harness.

## H-03 FranceStudent V1 direction

Because FranceStudent does not provide a chat id to MCP servers, the baseline becomes:

```text
request
  -> validate Authorization bearer
  -> authenticated connection principal
  -> resolve caller-supplied opaque Optic application handle
     or create one through an explicit bounded bootstrap contract
  -> SessionResolver
  -> internal Optic-owned SessionHandle
  -> session-owned capabilities / leases / policy
```

Still required before H-03 closes:

- opaque/unguessable/bounded handle format and storage;
- principal binding and cross-principal rejection;
- transactional provisioning of all subordinate session-owned authority;
- simultaneous-first-call / duplicate-bootstrap behavior;
- revoke/replay/no-resurrection semantics;
- exact attach/create rules after reconnect;
- idle-vs-absolute session lifetime and bounded renewal;
- deterministic cleanup/reaping/capacity exhaustion.

Reconnect measurements remain useful for UX/adapter behavior, but H-03 correctness must not depend on transport continuity.

## Security audit gates before real-project remote exposure

Real project workspaces remain intentionally blocked from the remote experiment until the following are resolved:

- **#176 — sensitive content + metadata/listing boundary:** a concrete sensitive-path classifier/policy is required; `fs_list` also needs canonical authorization and guaranteed pagination progress.
- **#180 — protected repository-control namespace:** broad generic `FileWrite` / `FileDelete` must not become an alternate route to `.git` control state that bypasses dedicated Git authority.
- **#181 — Unix path round-trip safety:** OS-native path segments that cannot be represented losslessly by `WorkspacePath` must fail closed before they become authorization targets.
- **#182 — GitRead vs sensitive content:** `git_diff` / `git_status` must not silently bypass future sensitive-file protections.
- **#183 — read/list/Git revoke quiescence:** project reads must hold session admission across the effect so local session revoke has a truthful immediate/quiescent contract.

These are deliberate security gates, not reasons to weaken the existing authority model.

Draft PR #184 is a separate current hardening candidate for #183/#181/#176-partial; it is unmerged and does not change the real-project remote gate by itself.

## What is known vs what is still missing

### Known

- FranceStudent can reach Streamable HTTP MCP and the real Optic synthetic adapter.
- FranceStudent admin confirms no per-chat id is sent to MCP servers today.
- MCP configuration is account-global.
- The bearer field maps to `Authorization: Bearer <value>`.
- Secure custom headers are sent on every request and hidden from the model.
- FranceStudent has no native per-call confirmation and no MCP cancellation today.
- FranceStudent permits model-driven parallel tool calls; Optic must impose its own independent limits.
- Real FranceStudent -> real Optic read and traversal rejection are proven with local markers.
- Different FranceStudent conversations can currently reach the same long-lived PR #178 adapter/upstream experimental context.
- FranceStudent can issue two distinct Optic tool calls from one assistant turn.

### Still missing / useful to measure

- harmless on-wire regression proof for custom secure header delivery and bearer scheme/presence;
- H-01 v2 true parallel overlap proof;
- reconnect/context behavior;
- host-side maximum tool-call timeout;
- duplicate/conflicting `Authorization` precedence if both UI surfaces can set it;
- final H-02 revoke/rotate/auth negative tests;
- H-03 session-lifetime/renewal contract;
- sensitive content/metadata classifier policy;
- production-appropriate HTTP/MCP error mapping.

## Current safe next steps

1. Keep PR #169/#170/#178 Draft; treat PR #184 separately as an unmerged hardening candidate.
2. Run the small remaining H-01 compatibility tests that have value: harmless secure header, dummy bearer presence, grouped parallel overlap, reconnect and timeout characterization if possible.
3. Start the H-02 bearer proof harness now; no further FranceStudent admin answer is required to choose the carrier.
4. Start freezing H-03 around the explicit Optic application-handle baseline; no host chat-id discovery is required for FranceStudent V1.
5. Continue host-independent security hardening (#176/#180/#181/#182/#183) and exact regression tests.
6. Do not expose a real project remotely yet.

## Important branches / PRs / issues

- PR #169 — docs/provider-neutral host/autonomy architecture;
- PR #170 — H-01 harmless host capability probe;
- PR #178 — experiment-only real Optic stdio <-> Streamable HTTP link;
- PR #184 — unmerged read/path/lifecycle hardening candidate (#183/#181/#176 partial);
- issue #171 — real FranceStudent H-01 evidence;
- issue #172 — H-02 remote authentication;
- issue #173 — H-03 SessionResolver;
- issue #174 — H-04 autonomy grants;
- issue #175 — H-05 local approval broker;
- issue #176 — read/list/sensitive-path audit;
- issues #180–#183 — additional security audit gates.

## Documentation rule

Historical `STATUS.md` remains the long implementation ledger. Use this snapshot, `PROJECT_HEALTH_2026-10-12.md`, `ROADMAP.md` and the owning ADR/research/security docs for current direction. Update historical ledgers surgically when production milestones merge rather than overwriting their history.

## Audit supplement — exact provenance and remaining gaps

The [repository audit](../engineering/REPOSITORY_HEALTH_AUDIT_2026-10-12.md) records exact main/draft refs and CI, all issues #171–#183, source cross-checks, documentation drift and coding candidates. The [evidence ledger](../research/FRANCESTUDENT_REAL_LINK_EVIDENCE_2026-10-12.md) now separates server-correlated host proofs, admin product-contract statements and implementation-backed inference.

Important proof limits remain: successful Real Link sequences are minted **after upstream return**, not at admission; no temporal overlap is measured. Session-info does not expose the opaque SessionHandle. Manual checkout/binary hash for the Real Link run is still missing.

Initial app TTL is 30 minutes with a 5-second reap loop and no production renewal call. Monotonic expiry uses a private server-local origin, not a wall-clock epoch. H-03/H-04 must decide idle/absolute/renewal behavior and transactional per-session subordinate lease provisioning.

#179 is closed for version constraints/pip check/freeze, not hash-verified dependency integrity. Historical STATUS.md and SCOPE_AND_ROADMAP.md remain historical sources; older live-summary claims and MULTI_SESSION_CONCURRENCY.md still need dated reconciliation. No new production release is implied by these documentation updates.

## Follow-up implementation candidate — PR #184 (2026-10-12)

[Draft #184](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/184), head `1e7695f12d214c504f84870dfe1f861f5d7d6792`, is based on unchanged main `156653a406f01383e2fc14cada513963453b04f3`. It proposes effect-spanning blocking-worker admission/transport retention for file read/list/Git read (#183), lossless observed native path conversion (#181), and pagination progress/final-cursor fitting (#176 partial). It is an unmerged candidate, not a shipped milestone; exact-head CI should be checked before promotion.
