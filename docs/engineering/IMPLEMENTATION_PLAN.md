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

Gate passed: PR #2 merged after green Linux/Windows formatting, lint, tests and dependency policy.

## Phase 1 — vertical slice

Target flow:

`stdio MCP → application session → fs_list/read → policy → process_start/read/stop/result → Windows Job Object → bounded output`

Phase 1 was deliberately split into narrow mergeable tranches so protocol, runtime and Windows failures remained attributable.

### Phase 1A — runtime foundation

Status: **merged** in PR #4 (`d33a1e5`).

- application-owned runtime layer;
- real monotonic clock;
- revocable/expiring session registry;
- bounded transport requests/responses, deadlines and concurrency;
- bounded read-only filesystem read/list services.

### Phase 1B — MCP read-only adapter

Status: **merged** in PR #5 (`681f939`).

- pinned RMCP adapter;
- bounded stdio framing before JSON parsing and after response serialization;
- MCP `fs_list`, `fs_read`, `session_info` routed through normalization, policy and runtime.

### Phase 1C — structured process lifecycle

Status: **merged** in PR #6 (`adf2e772`).

- opaque session-owned JobId;
- revocable task-lease registry;
- structured process start/read/stop/result;
- operator-only executable/environment allowlists;
- bounded output/history/timeouts;
- network remains unavailable and fails closed.

### Phase 1D — Windows Job Object enforcement

Status: **merged** in PR #7 (`69af07a`).

- narrow `optic-bridge-windows` unsafe boundary;
- suspended child creation, Job Object configuration before resume;
- kill-on-close, active-process and total job-memory limits;
- native Windows tests for descendant creation, memory, timeout tree cleanup, kill-on-close and fail-closed unwrap.

**Phase 1 gate passed.** Job Objects provide containment, not a complete security sandbox.

## Phase 2 — safe mutation/Git

Status: **in progress**.

Start narrow rather than exposing all write/Git tools at once.

### Phase 2A — mutation observation and expected-state foundation

Status: **merged** in PR #9 (`71bdf082`).

- streaming BLAKE3 `ContentVersion` observation;
- byte-bounded mutation observation via `HardLimits::max_fs_mutation_bytes`;
- canonical target observation with `ExpectedState::{Absent, Content}`;
- leaf symlink denial and canonical parent handling for absent targets;
- structured stale/blind-overwrite conflicts;
- no durable mutation or MCP write tools.

Gate passed on the exact final tree with Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

### Phase 2B — mutation-time OS containment and atomic commit

Status: **merged** in PR #13 (`80f3aa9b`), closure docs in PR #14 (`c9590f5c`).

Implemented:

1. Windows no-reparse handle opens inside `optic-bridge-windows`;
2. final-path/root validation from opened file/directory handles;
3. `FILE_ID_INFO` binding for existing-file identity and absent-target parent-directory identity;
4. exact expected-state plus identity revalidation before staging and immediately before namespace commit;
5. bounded create-new temporary write in the same directory with `sync_all`;
6. existing-target replacement through `ReplaceFileW`;
7. absent-target creation through create-only hard-link semantics;
8. explicit `CommitVerification::{Verified, CommittedButUnverified}` semantics;
9. non-Windows durable commit remains fail-closed as unsupported;
10. no MCP mutation exposure yet.

Native Windows gates prove replacement, same-content delete/recreate identity rejection, create-only absence semantics, parent-directory identity rejection and the low-level Windows adapter behavior.

Important boundary: this is a strong optimistic-concurrency commit foundation, but **not a kernel compare-and-swap against arbitrary external writers**. `ReplaceFileW` still names the final target by path after immediate handle/content/identity revalidation.

### Phase 2C — durable recovery journal and file mutation services

Status: **current**, split into narrow gates.

#### Phase 2C1 — bounded recovery journal state machine

Status: **merged** in PR #15 (`85aec4c6`).

Implemented:

1. recovery state lives under an operator-controlled state directory outside the canonical workspace;
2. one opaque `ActionId` identifies each per-operation journal;
3. state model `prepared → committing → verified|ambiguous`;
4. initial `prepared` record is create-new staged, `sync_all`'d and published by same-directory rename;
5. later complete JSON-line transitions are appended and `sync_all`'d;
6. hard limits `max_mutation_journal_file_bytes` (64 KiB default) and `max_mutation_recovery_records` (256 default);
7. deterministic bounded startup scan;
8. torn trailing transition lines fall back to the last complete state; corruption before the first complete state fails closed;
9. `prepared` means the journal protocol has not permitted namespace commit yet;
10. `committing` / `ambiguous` reconcile by observing the real bounded target state: exact intended = committed, exact prior = not committed, any third state = unresolved conflict retained for operator/recovery handling;
11. `verified` is terminal and can be retired without reinterpreting later third-party edits;
12. no MCP mutation exposure.

Final validation: exact PR #15 head `90a6b646` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` through temporary validation PR #16 after the main PR concurrency group was blocked by a cancelled predecessor. PR #16 was closed without merge.

What 2C1 does **not** prove:

- the journal alone does not wire itself around the Phase 2B namespace commit;
- it does not by itself prove forced process-crash behavior around that real lifecycle;
- no power-loss / ACID claim is made; file flush and namespace/directory-metadata persistence must not be overstated;
- public write/patch/delete services remain disabled.

#### Phase 2C2 — journal-wrapped Windows commit and forced-crash gate

Status: **merged** in PR #18 (`0297406c`).

Implemented in order:

1. bind one operation `ActionId` to the durable journal and deterministic same-directory staging artifact;
2. persist `prepared`, then persist and sync `committing` before the Phase 2B atomic service can be called;
3. execute the Phase 2B commit using that exact ActionId;
4. persist `verified` when the result is proven, or retain/mark recovery-required `ambiguous` semantics when a namespace effect may have happened but cannot be proven;
5. retire a verified journal only after its terminal transition is durable under the tested process-crash model;
6. preserve recovery evidence until operation-owned staging validation/cleanup succeeds;
7. require any surviving staging artifact to be regular, bounded, and BLAKE3-equal to the journaled intended content before deletion; unexpected/tampered staging fails closed and keeps the journal;
8. child-process termination gates cover durable `prepared`, durable `committing` before the atomic call, return from the atomic commit service before terminal journal state, and terminal journal state before retirement;
9. restart reconciliation classifies every tested crash fixture without blind retry and subsequent recovery is empty;
10. unresolved third-state conflicts remain fail-closed and retained.

Crash-model precision:

- the post-commit hook is after `AtomicMutationService` returns, so the namespace effect and its post-commit verification have completed; it is **not** an instrumentation point between the raw `ReplaceFileW`/hard-link operation and verification;
- the gate proves deterministic **process termination + restart** recovery, not sudden-power-loss ACID durability;
- Phase 2B's residual path-based external-writer race remains documented and unchanged.

Gate passed: exact final PR #18 head `fdfc3ace` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` before squash merge.

#### Phase 2C3 — transactional file services

Status: **current**, split into narrow gates.

##### Phase 2C3A — transactional write + deterministic patch

Status: **under review in PR #20**.

1. add `TransactionalFileService` above `JournaledMutationService`; do not introduce another OS mutation path;
2. whole-file write keeps explicit `ExpectedState::{Absent, Content}` and enforces `max_fs_mutation_bytes` before commit;
3. patch is a bounded deterministic single byte range (`offset`, `remove_bytes`, `insert`) and remains authorized as `FileWrite`, not a new capability/effect;
4. prepare against an exact `ContentVersion`, read the canonical base in bounded chunks, cap the assembled snapshot at the mutation ceiling and re-hash it before deriving output;
5. reject a changed/mixed snapshot before commit, then rely on the Phase 2C2 final expected-state + Windows identity revalidation again at commit time;
6. bound the derived patch result before journaled commit;
7. retain non-Windows fail-closed behavior;
8. expose no MCP mutation tool in this tranche.

##### Phase 2C3B — intended-state journal generalization + delete

Required before delete can be called transactional/recoverable:

1. evolve the journal contract from content-only intended success to explicit intended `ExpectedState::{Absent, Content}`;
2. preserve deterministic parsing/recovery of already-written v1 content journals or provide a strictly validated compatible migration path;
3. classify a post-delete crash as committed when the exact intended final state is `Absent` and as not committed when the exact prior content remains;
4. add the Windows delete primitive with the same canonical/no-reparse/file-identity revalidation discipline as write;
5. add forced-crash delete recovery gates before exposing delete above runtime.

##### Phase 2C3C — authorization/adapter gate

Only after the runtime mutation set is complete:

1. authorize canonical `FileWrite`/`FileDelete` effects through existing session capability + exact task lease + structural workspace scope policy;
2. add negative tests for missing/wrong/cross-session leases, stale policy epoch, scope escape, stale expected state, symlink/reparse escape, oversized input/result and recovery-required outcomes;
3. keep patch normalized to `FileWrite` authority;
4. keep MCP `fs_write`/patch/delete disabled until all runtime/policy/recovery gates pass;
5. expose MCP mutation only as a thin adapter over the proven application/runtime contract.

Design constraints across Phase 2C:

- journal entry count and bytes are hard bounded;
- ActionId is the recovery operation key; a general replay/idempotency service remains Phase 3 unless recovery requires a narrower primitive;
- wall-clock timestamps are diagnostic only and never authorization/freshness state;
- recovery never infers success merely from the absence of a temp file;
- an operation that may have committed but cannot be proven remains explicit/ambiguous until reconciled;
- recovery evidence must not be retired before operation-owned staging has been safely handled;
- durability claims must match actual Windows semantics and the tested crash model.

Phase 2C gate: transactional mutation runtime + deterministic startup reconciliation + forced process-crash tests + policy/recovery negative tests before any public mutation surface is enabled.

### Phase 2D — Git read/integration

Planned after mutation recovery is stable:

1. bounded Git read primitives (`status`, `diff`, `log`);
2. Git integration only with exact validated `expected_target_head`;
3. explicit integration/conflict gate;
4. MCP Git exposure only after negative tests pass.

Phase 2 gate: stale-write tests, path/reparse escape tests, forced-crash recovery tests and Git stale-target rejection.

## Phase 3 — multi-session

Independent sessions/projects, per-session jobs/spools/capabilities.

Evaluate a bounded ActionId idempotency ledger/replay service once stateful resources and retries exist.

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
