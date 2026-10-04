# Scope and Roadmap

Status: LIVING DOCUMENT. Last reviewed: 2026-10-04.

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

## Phase 1 — Vertical slice (COMPLETED)

Implemented and merged through PR #7:

- TransportGuard-owned request/frame/body/concurrency/response/time limits.
- MCP stdio adapter using maintained RMCP behind Optic-owned limits.
- Explicit application SessionHandle mapping and session registry/revocation lifecycle.
- Real monotonic Clock adapter for active session/task deadlines.
- Bounded `fs_list` / `fs_read` path through normalization and policy.
- Structured `process_start/read/stop/result` with opaque JobIds.
- Operator-owned executable/environment allowlists.
- Windows Job Object lifecycle/resource enforcement with kill-on-close, process-count and job-memory limits.
- Native Windows regression tests for process-tree cleanup and resource ceilings.

Phase 1 gate passed. Job Objects are containment, not a complete sandbox.

## Phase 2 — Safe mutation and Git (CURRENT)

Phase 2 is intentionally split into narrow security gates.

### Phase 2A — mutation observation/preconditions (COMPLETED)

Merged in PR #9 (`71bdf082`).

- bounded streaming BLAKE3 content observation;
- explicit `ExpectedState::{Absent, Content}` checks;
- canonical target/workspace containment;
- leaf-symlink denial and stale-state conflicts.

### Phase 2B — Windows mutation containment/atomic commit (COMPLETED)

Merged in PR #13 (`80f3aa9b`).

- no-reparse handles and final-path validation;
- `FILE_ID_INFO` identity binding;
- expected-state + identity revalidation around staging/commit;
- same-directory create-new staging;
- `ReplaceFileW` for existing targets and create-only hard-link semantics for absent targets;
- explicit committed/verified result semantics.

Residual boundary remains documented: existing-target replacement is not a kernel compare-and-swap against arbitrary external writers.

### Phase 2C — durable transactional file mutation (COMPLETED)

Merged through PR #30 (`624e88da`).

- bounded append-only mutation recovery journal outside the workspace;
- process-crash/restart recovery gates;
- transactional whole-file write, deterministic byte-range patch and handle-based delete;
- application-owned `FileWrite` / `FileDelete` task leases with exact workspace scope;
- conditional MCP `fs_write`, `fs_apply_patch`, `fs_delete` exposure only when matching authority exists;
- server-generated ActionIds and startup recovery-before-serve;
- cancellation-safe durable mutation handling that does not report timeout while a non-cancellable filesystem effect may still commit.

This proves the documented process-termination/restart model, not sudden-power-loss ACID durability.

### Phase 2D — Git read/integration (CURRENT)

#### Phase 2D1 — bounded Git read runtime (COMPLETED)

Merged in PR #32 (`1cb3cc03`).

- exact canonical repository-root binding;
- absolute/canonical Git executable;
- bounded status, literal-path/staged diff and paginated log;
- prompt/pager/external diff/textconv/fsmonitor/untracked-cache constraints;
- pinned/reachable log cursors and hard command deadlines.

#### Phase 2D2 — operator-owned MCP Git read (COMPLETED)

Merged in PR #33 (`aee4f168`).

- `GitRead` exists only when the operator supplies `--git-executable`;
- conditional MCP `git_status`, `git_diff`, `git_log`;
- callers cannot select repository roots, executable paths, raw argv or authority IDs;
- exact raw status/diff bytes are returned as base64 within response ceilings.

#### Phase 2D3 — exact-head Git integration (CURRENT GATE)

Before any Git mutation MCP tool is exposed:

- add a separate `GitIntegrate` authority and repository-scoped task lease;
- validate an exact `expected_target_head` immediately before integration;
- use isolated worktree/integration ownership instead of mutating the caller workspace directly;
- make stale-target, conflict and cleanup outcomes deterministic and fail-closed;
- add negative tests for stale target heads, conflicts, ownership/cleanup and authority separation.

A first manual Windows developer smoke is useful now on a disposable repository, but it validates the current Phase 2D2 machine/integration surface only. It is not a substitute for the Phase 2D3 gate and is not a production-readiness claim.

## Phase 3 — Multi-session runtime

Independent sessions/projects, per-session jobs/resources/capabilities, with adversarial cross-session tests.

Evaluate a bounded ActionId idempotency/replay ledger once stateful retries justify it.

## Phase 4 — Same-repository parallelism

Git worktrees, deterministic coordinator and integration/conflict gates for concurrent same-repository sessions.

Event-driven invalidation may be evaluated only as an optimization on top of mandatory commit-time revalidation.

## Phase 5 — Connectivity/install

Installer/autoconfiguration/self-test and additional transport/tunnel integration as required.

## Phase 6 — Hardening

Restricted-token/AppContainer/LPAC compatibility experiments, fuzz/property tests, long soak and signed release pipeline.

## Reject unless evidence changes

Embedded AI security reviewer as authorization dependency; generic shell as core primitive; probabilistic “risk score” controlling permissions; microservice/plugin-per-tool architecture; Electron dashboard; unbounded configurable limits; automatic trust of MCP tool annotations; filesystem watcher/USN silence as freshness authority; stale protocol cache as mutation authority; broad remote-desktop scope.

## Maintenance rule

A phase advances only when acceptance and security gates pass. Every roadmap movement updates `STATUS.md`. User-visible/relevant engineering changes update `CHANGELOG.md`. Changed invariants require tests and usually an ADR.
