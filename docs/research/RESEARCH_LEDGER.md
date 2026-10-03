# Research Ledger

This file separates research input from accepted architecture.

## Inputs consolidated
Research covered Desktop/Remote Commander resource behavior, MCP/OpenAI connectivity, Rust MCP SDK, Windows Job Objects/restricted tokens/AppContainer, OpticCode process/policy/transactions, Windows memory diagnostics, filesystem identity/watchers/USN, Git worktrees, retry/idempotency and multi-session/same-repository parallel work.

## Accepted conclusions
Native lightweight core; MCP as adapter; deterministic local authorization; bounded output/history; explicit process ownership/cleanup; first-class multi-session isolation; separate worktrees for same-repo concurrency; native Windows security tests; typed executable effects; explicit mutation preconditions; scope-bearing task leases; dual session+lease network authorization; monotonic runtime authorization deadlines.

## Current research batches
- [Phase 0.1 Optimization & Robustness Research — 2026-10-03](PHASE_0_1_OPTIMIZATION_RESEARCH.md)

This batch recommends handle-first filesystem validation, strict MCP cache/freshness behavior, Job Object enforcement and locked machine-readable worktree lifecycle at the relevant service boundary. It proposes FILE_ID_INFO identity, USN-assisted invalidation, ActionId idempotency and AppContainer/LPAC as measured prototypes rather than immediate architecture commitments.

## Claims not accepted as facts
Exact RAM savings before benchmarks; Job Objects as complete sandbox; schema validation as prompt-injection protection; AI review as final authorization; permanence of current ChatGPT/MCP connectivity behavior; filesystem watcher/USN silence as proof of freshness; cached MCP data as mutation authority; broad plugin/microservice architecture before demonstrated need.
