# Scope and Roadmap

Status: LIVING DOCUMENT. Last reviewed: 2026-10-03.

## Phase 0 — Architecture freeze (COMPLETED BASELINE)

- Cargo workspace and stable internal interfaces.
- Explicit application SessionHandle independent of MCP transport sessions.
- ActionEnvelope as the single execution authorization boundary.
- Security invariants mapped to tests.
- TransportGuard hard message/body/concurrency limits.
- Capability/task lease model.
- Source/sink containment model.
- Maintenance, status, changelog and release-security lifecycle.

Baseline merged through PR #1 on 2026-10-03.

## Phase 0.1 — Contract hardening (COMPLETED)

- Typed `Effect` variants instead of independently combinable action/target fields.
- Explicit file `ExpectedState` and Git target-head preconditions.
- Scope-bearing task leases for workspace/repository/executable/network boundaries.
- Segment-aware workspace scope checks.
- Dual session + task-lease authorization for process network access.
- Monotonic runtime authorization deadlines.
- Regression tests for scope/network failures.
- Research pass on filesystem identity, handle-first I/O, stale-context invalidation, retry/idempotency and Windows isolation profiles.

Gate passed on 2026-10-03: formatting, Clippy, tests and dependency-policy checks were green on Linux/Windows before PR #2 was squash-merged into `main`.

## Phase 1 — Vertical slice (CURRENT)

- TransportGuard-owned request/frame/body/concurrency/response/time limits.
- MCP stdio adapter using the maintained Rust MCP SDK behind TransportGuard.
- Explicit application SessionHandle mapping and session registry/revocation lifecycle.
- Real monotonic Clock adapter for active session/task deadlines.
- Bounded `fs_list` / `fs_read` path through normalization and policy.
- Structured `process_start/read/stop/result` with Windows Job Object lifecycle/resource enforcement.
- Explicit RMCP cache/freshness policy: stale cached state never satisfies mutation/security preconditions.
- Windows-native tests for process-tree cleanup and resource ceilings.

**Gate:** native Windows integration test proves child-tree cleanup and memory/output bounds; transport/session limits remain enforceable independently of SDK defaults.

## V1 essential after Phase 1

- Transactional patch/write with expected state/version.
- Git status/diff/log and worktree lifecycle.
- Deterministic deny-by-default policy preserved across all services.
- Bounded output, pagination, disk spool quotas and TTL.
- Crash recovery journal.
- Two simultaneous sessions on different projects.
- Same-repo parallel work via isolated Git worktrees.
- Windows-native security/integration tests.

## V1.5 candidates

- Deterministic coordinator for same-repo integration.
- Event-driven stale-context invalidation: TargetHeadChanged/FileVersionChanged/LeaseRevoked, with mandatory commit-time revalidation.
- Bounded ActionId idempotency ledger if retry/recovery tests justify it.
- Richer CLI session/task/resource dashboard.
- Restricted-token and AppContainer/LPAC execution profiles after compatibility testing.
- Signed installer/update path and release provenance.

## Later / experimental

- FILE_ID_INFO-backed durable file identity if Phase 2 adversarial tests show material benefit.
- USN-assisted invalidation if benchmarks justify recovery/fallback complexity.
- Content-addressed dedup if benchmarks justify it.
- WASM/WASI extension boundary only after a concrete extension requirement.
- VM/Windows Sandbox hard-isolation mode for untrusted workloads.
- Linux/macOS adapters after Windows invariants are stable.

## Reject unless evidence changes

Embedded AI security reviewer as authorization dependency; generic shell as core primitive; probabilistic “risk score” controlling permissions; microservice/plugin-per-tool architecture; Electron dashboard; unbounded configurable limits; automatic trust of MCP tool annotations; filesystem watcher/USN silence as freshness authority; stale protocol cache as mutation authority; broad remote-desktop scope.

## Maintenance rule

A phase advances only when acceptance and security gates pass. Every roadmap movement updates STATUS.md. User-visible/relevant engineering changes update CHANGELOG.md. Changed invariants require tests and usually an ADR.
