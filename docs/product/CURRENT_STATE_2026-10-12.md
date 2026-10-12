# Optic AI Bridge — current state snapshot

**Snapshot date:** 2026-10-12  
**Lifecycle:** pre-alpha / evidence-gated multi-host work  
**Release:** no production-supported release  
**Primary remote-host target:** FranceStudent on Windows, with ChatGPT support only on compatible surfaces

This document is a compact current-state companion to the historical `STATUS.md` and the canonical product roadmap. It exists so a new contributor or AI coding agent can recover the actual active state without rewriting or truncating the long historical status record.

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
- same-repository isolation/integration foundations through the active Phase 4 work.

A-08 reusable profile approval remains valid. A-08E is blocked by host interaction behavior, not by failure of the reusable approval core.

## Multi-host H-track

The active product/architecture direction is the H-track:

1. **H-00 — complete:** provider-neutral architecture, zero-cost requirement, Optic-owned sessions, local-first authority and no host/tunnel identity as authorization.
2. **H-01 — active / externally blocked:** authority-free host characterization. The hardened `h01-v2` probe is implemented in draft PR #170. Remaining FranceStudent checks are blocked by a host-side service incident and must be rerun with server-side invocation proof when the host recovers.
3. **H-02 — pending:** zero-cost remote endpoint authentication and credential lifecycle.
4. **H-03 — pending:** bounded `HostContext -> SessionResolver -> SessionHandle` contract.
5. **H-04 — pending:** `Review`, `Project Autonomy` and advanced high-authority session modes without approval spam, while every effect remains independently revalidated.
6. **H-05 — pending:** host-independent local approval broker suitable for the later Windows app/widget.
7. **H-06+ — gated:** production Streamable HTTP adapter, FranceStudent closure, compatible ChatGPT adapter, provider automation and desktop app.

## Verified FranceStudent evidence so far

Accepted evidence must be corroborated by the local probe/server, not merely by model prose.

Verified on a real Windows PC before the current FranceStudent service incident:

- FranceStudent accepted a public MCP Streamable HTTP `/mcp` endpoint through a temporary Cloudflare Quick Tunnel;
- real remote POST traffic reached the local probe;
- `probe_ping` succeeded;
- negotiated MCP revision was `2026-07-28`;
- no modern MCP transport session id was observed, which is expected under the stateless protocol;
- two same-conversation correlation calls were independently corroborated by fresh server traffic;
- no host-owned per-call approval/confirmation UI was observed for those harmless calls;
- the shown FranceStudent configuration exposed `Authentication: None` and did not visibly expose a custom-header field.

Important evidence correction:

- one earlier host/model JSON response was later admitted by the model to have been invented without a tool call and is excluded from evidence;
- the old constant request-id fingerprint is also excluded as a conversation signal because MCP Python SDK request ids are optional and the v1 probe fingerprinted absent `None` values ambiguously.

`h01-v2` fixes both problems with explicit request-id presence and a server-minted per-call proof mirrored in local logs.

Conversation identity remains **unproven**. H-03 must preserve an Optic-minted explicit application-handle fallback unless a reliable host-specific correlation signal is demonstrated.

## Real Optic MCP link experiment

Draft PR #178 proves an experiment-only end-to-end path:

```text
MCP Streamable HTTP client
        -> bounded Python compatibility adapter
        -> real Rust `optic-bridge` stdio MCP server
        -> disposable synthetic workspace
```

The remote experiment exposes exactly:

- `optic_session_info` -> real upstream `session_info`;
- `optic_fs_list` -> real upstream `fs_list`;
- `optic_fs_read` -> real upstream `fs_read`.

It deliberately excludes process execution, mutations, Git, `session_cancel` and arbitrary upstream tool forwarding.

Dedicated Real Optic MCP Link workflow #11 is green on Linux and Windows, and normal repository CI #653 is green on dependency policy, Ubuntu and Windows. The experiment proves that the existing stdio application server can sit safely behind a replaceable HTTP/MCP compatibility boundary for further host tests without a speculative production-runtime rewrite.

It additionally proves transport reconnect continuity: a new HTTP/MCP client can reconnect to the same adapter process and reach the same long-lived Optic application session. This is **transport continuity only**, not host-conversation identity.

## Experiment reproducibility and admission bounds

The H-01 / real-link Python environment now uses a frozen tested constraints graph rather than only a direct `mcp==2.3.0` pin. Both Windows and Linux runs execute `pip check` and record `pip freeze --all` evidence.

The synthetic real-link adapter is bounded by:

- 64 KiB request body;
- 32 admitted HTTP tasks;
- 8 concurrent upstream forwards;
- 250 ms forward-admission timeout;
- 15 s upstream call timeout;
- 5 s keep-alive;
- DNS-rebinding protection for normal loopback use;
- explicit temporary relaxation only for the disposable Quick-Tunnel experiment.

These are experiment guards, not H-02 production authentication.

## Security audit gates found before real-project remote exposure

Real project workspaces remain intentionally blocked from the remote experiment until the following are resolved:

- **#176 — sensitive content + metadata/listing boundary:** a concrete sensitive-path classifier/policy is required for reads, listing and mutation semantics. `ReadSensitiveScope` alone is not a classifier.
- **#180 — protected repository-control namespace:** broad generic `FileWrite` / `FileDelete` must not become an alternate route to `.git` control state that bypasses dedicated Git authority.
- **#181 — Unix path round-trip safety:** OS-native path segments that cannot be represented losslessly by `WorkspacePath` must fail closed before they become authorization targets.
- **#182 — GitRead vs sensitive content:** `git_diff` / `git_status` must not silently bypass future sensitive-file protections.
- **#183 — read/list/Git revoke quiescence:** project reads must hold session admission across the effect so local session revoke has a truthful immediate/quiescent contract.

These are deliberate security gates, not reasons to weaken the existing authority model.

## Current safe next steps

While FranceStudent is unavailable:

- continue read-only audits and narrow regression design;
- harden experiment reproducibility and proof integrity;
- keep PR #170 and PR #178 draft;
- do not expose a real repository remotely;
- do not implement production H-02/H-03 assumptions from unverified host behavior.

When FranceStudent returns:

1. start a fresh `h01-v2` probe server instance and fresh correlation/parallel run ids;
2. re-run verified new-conversation separation with matching local invocation markers;
3. test grouped parallel dispatch with two calls using the same parallel run id;
4. test reconnect under the same proof standard;
5. inspect the exact authentication options exposed by the host without placing secrets in URLs;
6. then run the synthetic-only PR #178 real-Optic link against FranceStudent before any real workspace is considered.

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
- issues #180–#183 — additional security audit gates described above.

## Documentation rule

Historical `STATUS.md` remains the long implementation ledger. Use this snapshot plus `ROADMAP.md` and `docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md` for the active direction. Update this snapshot when a major host/transport/audit gate changes instead of overwriting historical evidence.