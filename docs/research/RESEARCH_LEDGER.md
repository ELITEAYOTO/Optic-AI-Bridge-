# Research Ledger

This file separates research input from accepted architecture.

## Inputs consolidated
Research covered Desktop/Remote Commander resource behavior, MCP/OpenAI connectivity, Rust MCP SDK, Windows Job Objects/restricted tokens/AppContainer, OpticCode process/policy/transactions, Windows memory diagnostics, filesystem identity/watchers/USN, Git worktrees, retry/idempotency and multi-session/same-repository parallel work. The 2026-10-10 batch additionally covers FranceStudent Streamable HTTP evidence, host/session correlation, zero-cost tunnel-provider interchangeability, remote endpoint authentication constraints, session-scoped autonomy and host-independent local approval UX.

## Accepted conclusions
Native lightweight core; MCP as adapter; deterministic local authorization; bounded output/history; explicit process ownership/cleanup; first-class multi-session isolation; separate worktrees for same-repo concurrency; native Windows security tests; typed executable effects; explicit mutation preconditions; scope-bearing task leases; dual session+lease network authorization; monotonic runtime authorization deadlines.

For the H-track, the following are also accepted architectural constraints:

- connectivity/tunnel identity is never project/runtime authority;
- no mandatory paid Optic backend/relay/API credits for the student path;
- Tailscale, Cloudflare and future providers remain replaceable transport adapters;
- host-provided conversation/session values are correlation inputs only; Optic owns the real `SessionHandle`;
- V1 trust/autonomy is bounded to an Optic session/chat mapping and locally revocable;
- repeated trusted development actions should not require repeated confirmation while every action still passes policy/resource/isolation checks;
- access tokens are not placed in URI query strings;
- host-specific interaction behavior must be measured before architecture or user-facing wording depends on it.

## Current research batches
- [Phase 0.1 Optimization & Robustness Research — 2026-10-03](PHASE_0_1_OPTIMIZATION_RESEARCH.md)
- [FranceStudent / Remote MCP candidate](FRANCESTUDENT_REMOTE_MCP_CANDIDATE.md)
- [Host compatibility, zero-cost connectivity and session autonomy track](../product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md)

The Phase 0.1 batch recommends handle-first filesystem validation, strict MCP cache/freshness behavior, Job Object enforcement and locked machine-readable worktree lifecycle at the relevant service boundary. It proposes FILE_ID_INFO identity, USN-assisted invalidation, ActionId idempotency and AppContainer/LPAC as measured prototypes rather than immediate architecture commitments.

The host/autonomy batch currently has an automated authority-free H-01 probe in draft PR #170 and tracks the real FranceStudent evidence in issue #171. H-02/H-03 implementation must consume those measured results rather than assume a stable conversation id, custom-header capability or authentication shape.

## Claims not accepted as facts
Exact RAM savings before benchmarks; Job Objects as complete sandbox; schema validation as prompt-injection protection; AI review as final authorization; permanence of current ChatGPT/MCP connectivity behavior; filesystem watcher/USN silence as proof of freshness; cached MCP data as mutation authority; broad plugin/microservice architecture before demonstrated need; stability/uniqueness of any host-provided conversation/session metadata before H-01 evidence; FranceStudent custom-header or bearer-token behavior before real measurement; tunnel identity as authentication; a disclaimer as a substitute for containment in an advanced high-authority mode.
