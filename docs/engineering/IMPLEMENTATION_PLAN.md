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

Status: **merged** in PR #4 (`d33a1e5`).

- `optic-bridge-runtime` is the application-owned runtime layer;
- real monotonic `StdClock` adapter;
- revocable/expiring `SessionRegistry`;
- `TransportGuard` with hard request/response byte limits, request lifetime and bounded concurrency;
- transport/filesystem `HardLimits`;
- bounded `fs_read` and deterministic paginated `fs_list` runtime services;
- canonical containment for existing read-only paths;
- hard page and total-directory-scan ceilings;
- runtime independent of MCP so SDK behavior cannot silently become authorization or resource policy.

Gate passed: Linux + Windows formatting/lint/tests and dependency policy green.

### Phase 1B — MCP read-only adapter

Status: **merged** in PR #5 (`681f939`).

- pinned maintained RMCP adapter;
- stdio first transport;
- one Optic-owned application session for initial server lifecycle;
- read-only/system surface: `fs_list`, `fs_read`, `session_info`;
- public paths normalized through `WorkspacePath` before filesystem access;
- typed `ActionEnvelope` plus `PolicyEngine` authorization before runtime I/O;
- MCP handlers do not call filesystem/process APIs directly;
- Optic-owned handler concurrency/deadline/response ceilings;
- custom bounded JSON-line stdio transport caps a line before JSON deserialization and caps the final serialized response before stdout write.

Gate passed: direct negative handler tests, bounded read/list outputs, intended tool-surface test, Linux/Windows CI and dependency policy.

### Phase 1C — structured process lifecycle

Status: **merged** in PR #6 (`adf2e772`).

- opaque session-owned `JobId` with no arbitrary PID operation;
- revocable application-owned `TaskLeaseRegistry`;
- structured executable + `args[]` + cwd + controlled environment; shell strings are not the primitive;
- executable must be absolute, canonical and exactly present in an operator-created task lease;
- operator startup allowlists create process leases through `--allow-executable`; inherited environment names require `--allow-env`;
- MCP never creates capabilities or task leases;
- `process_start`, cursor-based bounded `process_read`, `process_stop`, `process_result` and `session_cancel`;
- `process_start` normalizes to `Effect::ProcessRun` and is authorized by both the application session and exact executable task lease;
- network remains unavailable in the Phase 1 runtime and `network=true` fails closed;
- stdout/stderr capture, active jobs, retained records, output reads and timeout are hard bounded;
- cancellation propagates session → job → owned child tree;
- Windows lifecycle already used Job Object assignment for tree ownership; Unix development builds use process-group lifecycle containment;
- Windows CI found and closed a fast-exit/output-overflow status race.

Gate passed: cross-session rejection, timeout/cancel/output-overflow determinism, no arbitrary PID operation, explicit operator allowlist requirement, Linux/Windows lint/tests and dependency policy.

### Phase 1D — Windows Job Object enforcement and Phase 1 gate

Status: **implemented on PR #7 branch; merge only while final docs + CI remain green**.

- dedicated `optic-bridge-windows` crate contains the narrow Win32 `unsafe` boundary;
- cross-platform core/policy/MCP/runtime remain unsafe-free;
- custom `LimitedJobObject` derives Windows containment limits from the already-authorized `ResourceBudget`;
- force temporary suspended child creation;
- create/configure Job Object before assignment;
- enable `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` and `JOB_OBJECT_LIMIT_JOB_MEMORY`;
- assign the child before resuming its threads;
- retain Job Object ownership until the complete child tree is terminal;
- timeout/explicit stop/output overflow terminate the owned Job Object tree through the wrapped child;
- non-Windows builds retain process-group behavior and make no Windows enforcement claim.

Native Windows gate passed on PR #7 implementation head:

- `process_count = 1` blocks descendant creation;
- a 128 MiB job-memory limit prevents a fixture from reaching a 384 MiB allocation target;
- timeout kills a spawned descendant before it can write a delayed survival marker;
- dropping the live wrapper proves kill-on-close cleanup;
- all existing Phase 1C timeout/stop/output-overflow tests remain green;
- Windows Clippy/tests, Ubuntu format/Clippy/tests and `cargo-deny` all pass.

Phase 1 gate is considered satisfied once the final documentation head passes the same CI and PR #7 is merged. Job Objects provide lifecycle/resource containment, not a complete security sandbox.

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
