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
2. **H-01 — active / evidence-pending:** authority-free host characterization. The hardened `h01-v2` probe is implemented in draft PR #170. FranceStudent service is stable again; the remaining H-01 measurements now need a deliberate reconnect to the H-01 v2 endpoint/process.
3. **H-02 — pending:** zero-cost remote endpoint authentication and credential lifecycle.
4. **H-03 — pending:** bounded `HostContext -> SessionResolver -> SessionHandle` contract.
5. **H-04 — pending:** `Review`, `Project Autonomy` and advanced high-authority session modes without approval spam, while every effect remains independently revalidated.
6. **H-05 — pending:** host-independent local approval broker suitable for the later Windows app/widget.
7. **H-06+ — gated:** production Streamable HTTP adapter, FranceStudent closure, compatible ChatGPT adapter, provider automation and desktop app.

## Verified FranceStudent evidence

Accepted evidence must be corroborated by the local probe/adapter where proof instrumentation exists, not merely by model prose.

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

Conversation identity remains **unproven**. H-03 must preserve an Optic-minted explicit application-handle fallback unless a reliable host-specific correlation signal is demonstrated.

### New FranceStudent UI evidence (2026-10-12)

The current FranceStudent MCP editor now visibly exposes:

- an authorization field labelled **`Bearer token ou secret`**;
- **`Headers sécurisés (JSON)`**.

This materially advances H-02 research, but the exact on-wire behavior is still unmeasured. Do not assume `Authorization: Bearer ...`, OAuth semantics, duplicate-header behavior or custom-header delivery until proven with the H-01/H-02 harness.

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

Safe interpretation of the new-chat test:

> The current PR #178 experimental bridge does not isolate different FranceStudent conversations into distinct Optic application sessions. H-03 is required before multi-conversation real-project authority can be correct.

This does **not** discover a FranceStudent conversation identifier, because PR #178 does not record H-01 correlation inputs.

The manual Real Optic Link process intentionally stops after about 25 minutes, before the current ~30-minute Optic session lifetime.

## Security audit gates before real-project remote exposure

Real project workspaces remain intentionally blocked from the remote experiment until the following are resolved:

- **#176 — sensitive content + metadata/listing boundary:** a concrete sensitive-path classifier/policy is required; `fs_list` also needs canonical authorization and guaranteed pagination progress.
- **#180 — protected repository-control namespace:** broad generic `FileWrite` / `FileDelete` must not become an alternate route to `.git` control state that bypasses dedicated Git authority.
- **#181 — Unix path round-trip safety:** OS-native path segments that cannot be represented losslessly by `WorkspacePath` must fail closed before they become authorization targets.
- **#182 — GitRead vs sensitive content:** `git_diff` / `git_status` must not silently bypass future sensitive-file protections.
- **#183 — read/list/Git revoke quiescence:** project reads must hold session admission across the effect so local session revoke has a truthful immediate/quiescent contract.

These are deliberate security gates, not reasons to weaken the existing authority model.

## What is known vs what is still missing

### Known

- FranceStudent can reach Streamable HTTP MCP and the real Optic synthetic adapter.
- Current FranceStudent UI exposes bearer/secret and secure-header configuration fields.
- Real FranceStudent -> real Optic read and traversal rejection are proven with local markers.
- Different FranceStudent conversations can currently reach the same long-lived PR #178 adapter/upstream experimental context.
- FranceStudent can issue two distinct Optic tool calls from one assistant turn.

### Still missing

- exact bearer-field wire behavior;
- custom secure-header delivery proof;
- H-01 v2 new-chat correlation signal (if any);
- H-01 v2 reconnect signal behavior;
- true FranceStudent parallel overlap proof;
- cancellation behavior;
- final H-02 revoke/rotate/auth negative tests;
- H-03 session-lifetime/renewal contract;
- sensitive content/metadata classifier policy;
- production-appropriate HTTP/MCP error mapping.

## Current safe next steps

1. Keep PR #169/#170/#178 Draft.
2. Reconnect FranceStudent intentionally to H-01 v2 and finish new-chat/reconnect/grouped-parallel/header/auth measurements with server proof.
3. In parallel, implement host-independent narrow security fixes that do not depend on host semantics: #176 canonical listing + cursor progress and #183 admission barrier, with deterministic regression tests.
4. Use measured FranceStudent auth/header behavior to build the H-02 proof harness.
5. Freeze H-03 SessionResolver + explicit-handle fallback + session lifetime semantics before real multi-conversation authority.
6. Do not expose a real project remotely yet.

## Important branches / PRs / issues

- PR #169 — docs/provider-neutral host/autonomy architecture;
- PR #170 — H-01 harmless host capability probe;
- PR #178 — experiment-only real Optic stdio <-> Streamable HTTP link;
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

The [repository audit](../engineering/REPOSITORY_HEALTH_AUDIT_2026-10-12.md) records exact main/draft refs and CI, all issues #171–#183, source cross-checks, documentation drift and coding candidates. The [evidence ledger](../research/FRANCESTUDENT_REAL_LINK_EVIDENCE_2026-10-12.md) separates paired read proofs, host-reported negative evidence and implementation-backed session inference.

Audited main is `156653a406f01383e2fc14cada513963453b04f3`, [CI #602 green](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/37994556854). The initial #169 audit head `8eeecd9ba0a16fa49980fba06be215a2c61a1b97` had [green CI](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/38229751467); #169 then advanced to `1e8b3cc1f7b03f635de38eff95761a2a1724a1ee` during this pass. Existing concurrent documentation additions are preserved. These greens do not validate later heads.

Important proof limits: successful Real Link sequences are minted **after upstream return**, not at admission; no temporal overlap is measured. Session-info does not expose the opaque SessionHandle. The cached seq-26 console excerpt is truncated, so complete local proof must still be retained; manual checkout/binary hash is also missing. Same adapter + unchanged upstream source is strong evidence of shared experimental context, not host conversation identity.

Initial app TTL is 30 minutes with a 5-second reap loop and no production renewal call. Monotonic expiry uses a private server-local origin, not a wall-clock epoch. H-03/H-04 must decide idle/absolute/renewal behavior and transactional per-session subordinate lease provisioning.

#179 is closed for version constraints/pip check/freeze, not hash-verified dependency integrity. Historical STATUS.md and SCOPE_AND_ROADMAP.md remain byte-for-byte unchanged by this pass; older live-summary claims and MULTI_SESSION_CONCURRENCY.md still need dated reconciliation. No local Rust tests or new host tests were run by this documentation audit.
