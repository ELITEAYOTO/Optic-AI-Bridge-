# Research Ledger

This file separates research input from accepted architecture.

**Last reviewed:** 2026-10-12

## Inputs consolidated

Research covered Desktop/Remote Commander resource behavior, MCP/OpenAI connectivity, Rust MCP SDK, Windows Job Objects/restricted tokens/AppContainer, OpticCode process/policy/transactions, Windows memory diagnostics, filesystem identity/watchers/USN, Git worktrees, retry/idempotency and multi-session/same-repository parallel work.

The 2026-10-10 through 2026-10-12 batch additionally covers:

- real FranceStudent Streamable HTTP evidence;
- modern MCP `2026-07-28` stateless transport semantics;
- host/session correlation and evidence integrity;
- zero-cost tunnel-provider interchangeability;
- remote endpoint authentication constraints;
- FranceStudent bearer/secret + secure-header UI evidence;
- session-scoped autonomy and host-independent local approval UX;
- a real Optic stdio server behind a bounded Streamable HTTP compatibility adapter;
- live FranceStudent -> real Optic synthetic read/list/session-info evidence;
- cross-capability security auditing for sensitive filesystem/Git content, protected repository metadata, path representation and session revocation semantics.

## Accepted architecture conclusions

Existing accepted conclusions remain:

- native lightweight Rust core;
- MCP as adapter rather than core;
- deterministic local authorization;
- bounded output/history;
- explicit process ownership/cleanup;
- first-class multi-session isolation;
- separate worktrees for same-repository concurrency;
- native Windows security tests;
- typed executable effects;
- explicit mutation preconditions;
- scope-bearing task leases;
- dual session+lease network authorization;
- monotonic runtime authorization deadlines.

For the H-track, the following are accepted constraints:

- connectivity/tunnel identity is never project/runtime authority;
- no mandatory paid Optic backend/relay/API-credit path for normal student operation;
- Tailscale, Cloudflare and future providers remain replaceable connectivity adapters;
- host-provided conversation/session/request values are correlation inputs only; Optic owns the real `SessionHandle`;
- MCP `2026-07-28` must not be treated as a persistent application session;
- V1 trust/autonomy is bounded to an Optic-owned session mapping and locally revocable;
- repeated trusted development actions should not require repeated confirmation while every action still passes policy/resource/isolation checks;
- access tokens are not placed in URI query strings;
- host-specific interaction/auth behavior must be measured before architecture or user-facing wording depends on it;
- model-rendered JSON is not accepted as tool-execution proof when server-side invocation proof is available;
- a compatibility HTTP adapter can sit in front of the real Optic stdio application server for synthetic evidence gathering without rewriting the production core;
- real-project remote exposure remains separately gated by sensitive-content, path, Git-composition and lifecycle/revocation audits;
- a production multi-conversation adapter cannot simply reuse one startup application session and assume host conversations map correctly; H-03 session resolution is an explicit application concern.

## Current research batches

- [Phase 0.1 Optimization & Robustness Research — 2026-10-03](PHASE_0_1_OPTIMIZATION_RESEARCH.md)
- [FranceStudent / Remote MCP candidate](FRANCESTUDENT_REMOTE_MCP_CANDIDATE.md)
- [Real Optic MCP Link Experiment](REAL_OPTIC_MCP_LINK_EXPERIMENT.md)
- [Project Health Checkpoint — 2026-10-12](../product/PROJECT_HEALTH_2026-10-12.md)
- [Host compatibility, zero-cost connectivity and session autonomy track](../product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md)
- [Remote/autonomy security audit gates](../security/REMOTE_AUTONOMY_AUDIT_GATES.md)

## H-01 real-host evidence status

Verified on the target Windows PC from the earlier H-01 runs:

- remote Streamable HTTP `/mcp` through a temporary Cloudflare Quick Tunnel is viable;
- real remote POST traffic reached the local H-01 probe;
- `probe_ping` succeeded;
- protocol `2026-07-28` negotiated;
- no modern protocol transport session was present;
- two same-conversation correlation calls were server-corroborated;
- no host-owned per-call approval UI was observed for those harmless calls.

Evidence rejected or downgraded:

- one supposed new-conversation JSON result was later admitted by the host model to have been invented without a tool invocation and is discarded;
- the v1 stable request-id fingerprint is ambiguous because request ids are optional and absent `None` was fingerprinted.

`h01-v2` adds explicit request-id presence, server-minted per-call markers mirrored in local logs, grouped server-side parallel-overlap proof, union-key correlation comparisons and broader credential-header redaction.

FranceStudent is stable again as of 2026-10-12. The remaining H-01 v2 checks are therefore **evidence-pending**, not blocked by the old service outage. They still require a deliberate run against the H-01 v2 endpoint/process rather than the PR #178 Real Optic Link endpoint.

## FranceStudent auth/header UI evidence

The current FranceStudent MCP editor visibly exposes:

- an authorization input labelled **`Bearer token ou secret`**;
- **`Headers sécurisés (JSON)`**.

Accepted conclusion: FranceStudent now exposes both a credential field and a secure custom-header configuration surface.

Not yet accepted as facts:

- exact `Authorization: Bearer ...` wire mapping;
- OAuth behavior;
- custom `Authorization` conflict/merge behavior;
- arbitrary harmless custom-header delivery;
- missing/wrong/revoked credential semantics.

Those remain H-01/H-02 measurements.

## Real Optic MCP link evidence

Draft PR #178 proves an experiment-only path:

```text
Streamable HTTP MCP client
  -> bounded Python compatibility adapter
  -> real Rust `optic-bridge` stdio MCP server
  -> disposable synthetic workspace
```

The experiment exposes only session-info/list/read wrappers, uses real upstream Optic session/policy/lease/filesystem code, proves real traversal rejection, concurrent forwarding, per-call invocation proof and transport-client reconnect continuity to the same long-lived upstream Optic session.

Dedicated workflow #11 is green on Linux + Windows at experiment head `ef5bc84144670135d13f55a59a782611aeae762f`; normal CI #653 is green on dependency policy + Ubuntu + Windows.

### Live FranceStudent proof accepted on 2026-10-12

The current FranceStudent tests use the PR #178 endpoint, not the H-01 probe.

Accepted server-correlated facts:

- FranceStudent discovered exactly `optic_session_info`, `optic_fs_list`, `optic_fs_read`;
- a real `optic_fs_read(public/hello.txt)` returned `adapter_instance=20baecc2eeaf`, `seq=22`, marker `126b81f5327c1b59`, and the exact proof appeared in local `[REAL-LINK] TOOL_CALL` output;
- the read payload contained the expected synthetic per-start fixture marker;
- an absolute Windows path to the real Desktop/repository was rejected;
- `../outside.txt` was rejected with `optic.invalid_workspace_path` / JSON-RPC `-32602` (host surfaced HTTP 400);
- two real read calls were emitted from one FranceStudent assistant turn (`seq=23` and `seq=24`, both with matching local markers);
- one conversation emitted `optic_session_info` `seq=25`, and a genuinely new FranceStudent conversation emitted `seq=26` against the same live `adapter_instance=20baecc2eeaf` and the same upstream-visible experimental session-info context.

Accepted interpretation:

- FranceStudent can drive the real Optic synthetic path;
- the current PR #178 bridge itself does not isolate separate FranceStudent conversations into separate upstream Optic application sessions;
- H-03 session resolution is therefore a real requirement, not only a theoretical design preference.

Not accepted from this evidence:

- a FranceStudent conversation id has been discovered;
- FranceStudent true temporal parallel dispatch is proven;
- production authentication is proven;
- real-project remote safety is proven.

## Security research findings elevated to implementation gates

- **#176:** `ReadSensitiveScope` is not a sensitive-path classifier; content, metadata/listing and protected mutation semantics need an explicit canonical policy boundary. `fs_list` also has a canonical-authorization issue and a non-progressing-cursor bug.
- **#180:** generic broad file mutation must not become an alternate route to `.git` repository-control state around dedicated Git authority.
- **#181:** Unix native path segments must round-trip exactly into authority-bearing `WorkspacePath`; lossy normalization must fail closed.
- **#182:** `GitRead` needs an explicit relationship with sensitive-content policy so status/diff cannot silently bypass filesystem protections.
- **#183:** read/list/Git effects need lifecycle admission held through the bounded effect for truthful immediate revoke/quiescence semantics.

These findings do not invalidate the synthetic PR #178 experiment because that experiment provisions no mutation/process/Git authority and no real project data.

## Claims not accepted as facts

The following remain unaccepted unless future evidence proves them:

- exact RAM savings before benchmarks;
- Job Objects as a complete sandbox;
- schema validation as prompt-injection protection;
- AI review as final authorization;
- permanence of current ChatGPT/MCP connectivity behavior;
- filesystem watcher/USN silence as proof of freshness;
- cached MCP data as mutation authority;
- broad plugin/microservice architecture before demonstrated need;
- stability/uniqueness of any host-provided conversation/session/request metadata as conversation identity;
- FranceStudent bearer field mapping to a specific HTTP Authorization shape before real measurement;
- FranceStudent secure-header delivery before real measurement;
- tunnel identity as authentication;
- a disclaimer as a substitute for containment in advanced high-authority mode;
- HTTP reconnect continuity as proof of host-conversation identity;
- same `adapter_instance` across two FranceStudent conversations as proof that FranceStudent exposes no conversation id;
- two calls in one assistant turn as proof of true temporal overlap;
- a scope type as proof that sensitive paths are classified;
- `GitRead` as automatically safe merely because Git command execution is hardened;
- green transport CI as proof that real-project remote exposure is safe.

## Research discipline

When a host behavior is unknown, record the missing evidence instead of encoding an assumption into production code.

When external model output can be fabricated or summarized, pair it with local/server evidence.

When a security finding crosses capabilities, record it at the shared authority boundary rather than fixing only one adapter surface.

When an experiment deliberately owns one long-lived upstream session, do not infer production conversation/session semantics from that design.