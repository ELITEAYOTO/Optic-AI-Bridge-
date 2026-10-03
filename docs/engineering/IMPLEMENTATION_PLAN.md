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

Phase 1 was deliberately split into narrow mergeable tranches so protocol, runtime and Windows failures remained attributable.

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

Status: **merged** in PR #7 (`69af07a`).

- dedicated `optic-bridge-windows` crate contains the narrow Win32 `unsafe` boundary;
- cross-platform core/policy/MCP/runtime remain unsafe-free;
- custom `LimitedJobObject` derives Windows containment limits from the already-authorized `ResourceBudget`;
- temporary suspended child creation;
- Job Object creation/configuration before assignment;
- `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` and `JOB_OBJECT_LIMIT_JOB_MEMORY`;
- child assignment before thread resume;
- Job Object ownership retained until the complete child tree is terminal;
- timeout/explicit stop/output overflow terminate the owned Job Object tree;
- containment unwrap is fail-closed and does not leak the Job Object handle;
- non-Windows builds retain process-group behavior and make no Windows enforcement claim.

Native Windows gate passed before merge:

- `process_count = 1` blocks descendant creation;
- a 128 MiB job-memory limit prevents a fixture from reaching a 384 MiB allocation target;
- timeout kills a spawned descendant before it can write a delayed survival marker;
- dropping the live wrapper proves kill-on-close cleanup;
- explicitly unwrapping containment does not allow the child to survive;
- all existing Phase 1C timeout/stop/output-overflow tests remain green;
- Windows Clippy/tests, Ubuntu format/Clippy/tests and `cargo-deny` all pass.

**Phase 1 gate passed.** The current pre-alpha vertical slice has an Optic-owned session/policy boundary, bounded stdio MCP transport, bounded read-only filesystem access, structured lease-gated process execution and native Windows lifecycle/process-count/job-memory containment. Job Objects provide containment, not a complete security sandbox.

## Phase 2 — safe mutation/Git

Status: **in progress**.

Start narrow rather than exposing all write/Git tools at once.

### Phase 2A — mutation observation and expected-state foundation

Status: **merged** in PR #9 (`71bdf082`).

- streaming BLAKE3 `ContentVersion` observation for readers;
- observation is byte-bounded by `HardLimits::max_fs_mutation_bytes` as well as memory-bounded;
- canonical mutation observation distinguishes `ExpectedState::Absent` from exact `ExpectedState::Content(version)`;
- existing mutation leaves must be regular files and leaf symlinks are denied;
- absent targets canonicalize their parent before deriving the authorization target;
- canonical targets must remain inside the canonical workspace root;
- exact expected-state mismatch returns a structured mutation conflict;
- mutation-specific errors remain isolated from the existing read-only MCP filesystem error contract;
- negative tests cover stale versions, blind overwrite, bounded observation and Unix symlink/path escape cases;
- no durable write/delete/patch operation and no MCP mutation tool are added in this tranche.

Important boundary: Phase 2A is a planning/revalidation foundation, **not** the final race-free mutation commit path. Windows mutation execution must still converge on validated handles/reparse/final-target checks and revalidate immediately before commit/replace.

Gate passed: the exact final tree `c5bba2be` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`. A temporary CI-only PR #10 validated that exact SHA because PR #9's Actions concurrency group had a cancelled intermediate Ubuntu matrix job stuck without steps; PR #10 was closed without merge before PR #9 was squash-merged.

### Phase 2B — mutation-time OS containment and atomic commit

Status: **implemented / under review** in PR #13.

Implemented:

1. Windows no-reparse handle opens inside the existing `optic-bridge-windows` unsafe boundary;
2. final-path/root validation from the opened file/directory handle;
3. `FILE_ID_INFO` binding for existing-file identity and absent-target parent-directory identity;
4. exact expected-state plus identity revalidation before staging and again immediately before namespace commit;
5. bounded create-new temporary write in the same directory with `sync_all`;
6. existing-target replacement through `ReplaceFileW`;
7. absent-target creation through create-only hard-link semantics, so a concurrently appearing target is not overwritten;
8. explicit post-commit verification with a distinct `CommittedButUnverified` state for cases where the namespace operation may have committed but verification cannot prove it;
9. non-Windows durable commit remains fail-closed as unsupported;
10. no MCP mutation exposure yet.

Native Windows gate on code head `ba24c1eb` proves:

- existing target replacement succeeds and verifies the new content version;
- deleting/recreating the same path with the same bytes is rejected by changed file identity;
- absent-target create succeeds without overwrite semantics;
- recreating the parent directory invalidates the prepared absent-target plan;
- low-level Windows file identity and `ReplaceFileW` adapter tests pass;
- previous Phase 1 Job Object gates remain green.

Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` all passed on the code head before documentation alignment.

Important boundary: this is a strong optimistic-concurrency commit foundation, but **not a kernel compare-and-swap against arbitrary external writers**. `ReplaceFileW` still names the final target by path after immediate handle/content/identity revalidation, leaving a very small external-writer race window. Do not claim that window is eliminated. A later oplock or handle-based rename experiment is justified only if it materially reduces the window without introducing deadlock or compatibility risk.

### Phase 2C — durable recovery journal and file mutation services

Planned after 2B:

1. bounded durable mutation journal with explicit state transitions;
2. startup detection/reconciliation of incomplete mutations;
3. forced-crash tests proving incomplete work cannot be silently reported as committed;
4. transactional file write/patch/delete runtime services built on the 2B prepared/commit boundary;
5. MCP mutation tools only after policy, stale-write, reparse and recovery gates pass.

### Phase 2D — Git read/integration

Planned after mutation recovery is stable:

1. bounded Git read primitives (`status`, `diff`, `log`);
2. Git integration only with exact validated `expected_target_head`;
3. explicit integration/conflict gate;
4. MCP Git exposure only after negative tests pass.

Phase 2 gate: stale-write tests, path/reparse escape tests, forced-crash recovery tests and Git stale-target rejection.

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
