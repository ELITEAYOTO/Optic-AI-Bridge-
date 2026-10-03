# Implementation Plan

## Phase 0 — freeze contracts

Create Cargo workspace, core traits/types, SessionContext, typed errors, configuration ceilings and ADRs. No broad feature work before these compile.

Completed baseline: PR #1 merged 2026-10-03.

## Phase 0.1 — contract hardening

Before adding MCP or OS effects, close inconsistencies that are cheap to fix at the domain layer:

- typed `Effect` variants instead of independently combinable action-kind/target fields;
- explicit file `ExpectedState` and Git target-head preconditions;
- scope-bearing task leases for workspace/repository/executable/network boundaries;
- dual session+lease authorization for process network access;
- monotonic runtime expiry values;
- regression tests for scope/network failures;
- update canonical docs and ADRs.

Research during this phase may produce later ADR candidates, but handle-first filesystem operations, Windows file identity, event-driven stale-context invalidation and action idempotency are not implemented until the relevant service boundary exists.

Gate passed: PR #2 merged after green Linux/Windows formatting, lint, tests and dependency policy.

## Phase 1 — vertical slice

Target flow:

`stdio MCP → application session → fs_list/read → policy → process_start/read/stop/result → Windows Job Object → bounded output`

Phase 1 is deliberately split into narrow mergeable tranches so protocol, runtime and Windows failures remain attributable.

### Phase 1A — runtime foundation

Status: implemented on PR #4 branch; merge only while all CI gates remain green.

- add `optic-bridge-runtime` as an application-owned runtime layer;
- add a real monotonic `StdClock` adapter;
- add revocable/expiring `SessionRegistry`;
- add `TransportGuard` with hard request/response byte limits, request lifetime and bounded concurrency;
- extend `HardLimits` with transport/filesystem ceilings;
- add bounded `fs_read` and deterministic paginated `fs_list` runtime services;
- canonicalize existing read-only paths and reject resolution outside the canonical workspace root;
- bound both returned directory pages and total directory entries scanned per call;
- keep the runtime crate independent of MCP so SDK behavior cannot silently become authorization or resource policy.

Acceptance:

- Linux + Windows formatting/lint/tests green;
- dependency policy green;
- transport/session/filesystem negative tests exercise limits and revocation;
- docs describe only behavior that exists.

### Phase 1B — MCP read-only adapter

- introduce the maintained Rust MCP SDK as an adapter dependency, pinned/controlled through the workspace;
- use stdio transport for the first vertical slice;
- create/map exactly one Optic-owned application session for the initial server lifecycle;
- expose only the Phase 1 read-only/system surface initially (`fs_list`, `fs_read`, `session_info`, `session_cancel`, minimal `system_info` if justified);
- parse all public paths through `WorkspacePath` before filesystem access;
- construct typed `ActionEnvelope` values and authorize them with `PolicyEngine` before calling runtime services;
- ensure MCP handlers never call `std::fs` or process APIs directly;
- apply Optic-owned handler concurrency/deadline/response ceilings and investigate raw stdio frame/body bounding so SDK defaults are never the sole protection;
- define explicit RMCP cache/freshness behavior: protocol cache cannot satisfy authorization or mutation/security preconditions.

Acceptance:

- direct handler tests prove invalid path/session/policy requests fail closed;
- read/list outputs remain bounded independently of caller arguments;
- stdio server starts and exposes only intended tools in CI-compatible tests;
- any raw-frame limitation not yet enforceable is documented rather than implied solved.

### Phase 1C — structured process lifecycle

- add process job identifiers owned by one application session;
- implement structured executable + args + cwd + controlled environment; never shell-string execution as the primitive;
- implement `process_start`, cursor-based bounded `process_read`, `process_stop`, `process_result`;
- deny network by default and preserve dual session+task-lease authorization when network is requested;
- bound stdout/stderr capture, process count, timeout and memory request before platform execution;
- cancellation propagates session → job → child tree.

Acceptance:

- cross-session job access is rejected;
- timeout/cancel/output-overflow paths are deterministic and tested;
- no arbitrary PID operation is exposed.

### Phase 1D — Windows Job Object enforcement and Phase 1 gate

- implement a narrow audited Windows containment adapter using Job Objects;
- configure kill-on-close and approved resource limits;
- assign the child immediately enough that no intended child tree escapes lifecycle ownership;
- close/terminate handles deterministically on normal completion, cancellation, timeout and bridge shutdown;
- retain non-Windows compile/test support without pretending it proves Windows containment.

Phase 1 gate: native Windows integration tests prove owned child-tree cleanup plus memory/process/output/time resource behavior. Transport/session limits remain enforceable independently of MCP SDK defaults.

## Phase 2 — safe mutation/Git

Transactional patch/write, expected states/hashes, git status/diff/log, crash journal.

Evaluate handle-first filesystem operations and durable file identity at this phase because a real mutation-capable filesystem service then exists.

Gate: forced-crash recovery and stale-write tests.

## Phase 3 — multi-session

Independent sessions/projects, per-session jobs/spools/capabilities.

Evaluate ActionId idempotency ledger/replay handling once stateful resources and retries exist.

Gate: adversarial cross-session access tests.

## Phase 4 — same-repo parallelism

Git worktrees, deterministic coordinator, integration/conflict gate.

Evaluate event-driven target/file invalidation as an optimization on top of mandatory commit-time revalidation.

Gate: two concurrent sessions cannot silently overwrite or integrate stale changes.

## Phase 5 — connectivity/install

Local HTTP/tunnel adapter as required, installer/autoconfig/self-test.

## Phase 6 — hardening

Restricted-token compatibility experiments, fuzz/property tests, long soak, signed release pipeline.

Avoid implementing later-phase abstractions early unless a current interface genuinely needs the boundary.
