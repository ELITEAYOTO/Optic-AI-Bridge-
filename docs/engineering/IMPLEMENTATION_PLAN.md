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

Status: **merged through Phase 2C3C3**.

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

Status: **merged through the authorization/adapter gate**.

##### Phase 2C3A — transactional write + deterministic patch

Status: **merged** in PR #20 (`4415a65c`). Exact final head `531f0e38` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

1. `TransactionalFileService` sits above `JournaledMutationService`; no second OS mutation path exists;
2. whole-file write keeps explicit `ExpectedState::{Absent, Content}` and enforces `max_fs_mutation_bytes` before commit;
3. patch is a bounded deterministic single byte range (`offset`, `remove_bytes`, `insert`) and remains authorized as `FileWrite`, not a new capability/effect;
4. patch prepares against an exact `ContentVersion`, reads the canonical base in bounded chunks, caps the assembled snapshot at the mutation ceiling and re-hashes it before deriving output;
5. changed/mixed snapshots are rejected before commit; the Phase 2C2 boundary then performs final expected-state + Windows identity revalidation again at commit time;
6. the derived patch result is bounded before journaled commit;
7. non-Windows durable mutation remains fail-closed;
8. native Windows coverage includes real write+patch, multi-chunk base reads and stale-base rejection without modification;
9. no MCP mutation tool is exposed by this historical tranche.

##### Phase 2C3B — intended-state recovery + delete

Status: **merged**.

###### Phase 2C3B1 — intended-state journal compatibility

Status: **merged** in PR #22 (`f5eafc3a`). Exact final head `fe025f46` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

1. journal record schema v2 represents intended `ExpectedState::{Absent, Content}` explicitly;
2. the stable journal storage directory is unchanged so surviving v1 records remain discoverable;
3. v1 content journals are strictly normalized to explicit content intent;
4. mixed schemas, unknown fields, invalid persisted content hashes and immutable-field changes fail closed while retaining evidence;
5. `committing|ambiguous` recovery compares the observed canonical state against exact intended state first, then exact prior state;
6. tests prove intended-absent committed and not-committed classifications without pretending delete exists yet;
7. write staging cleanup refuses a staging artifact associated with intended `Absent`;
8. no Windows delete primitive or MCP mutation surface is added here.

###### Phase 2C3B2 — Windows transactional delete

Status: **merged** in PR #24 (`b346a7d9`). Exact final head `e74cc0cf` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` before squash merge.

Implemented:

1. `optic-bridge-windows` opens a dedicated delete-capable no-reparse handle with read + `DELETE`, captures final path and exact `FILE_ID_INFO`, and issues `SetFileInformationByHandle(..., FileDispositionInfo, ...)` on that same handle rather than deleting by path;
2. `PreparedDelete` requires an exact existing `ContentVersion`; commit revalidates canonical state, final-path containment, exact file identity and bounded BLAKE3 bytes on the delete handle before the effect;
3. stale content and same-path/same-content delete-recreate identity changes fail closed without deleting the replacement;
4. successful handle deletion closes before returning and the runtime explicitly verifies final `ExpectedState::Absent`, with a committed-but-unverified recovery-required result preserved when proof fails;
5. `JournaledMutationService` wraps delete in the existing durable `prepared → committing → verified|ambiguous` lifecycle with `intended = ExpectedState::Absent` and no write-staging artifact;
6. any atomic failure after durable `committing` remains recovery-required rather than a blind-retry signal;
7. native Windows child-process gates terminate after durable prepared, durable committing-before-delete, atomic delete service return and terminal state before retirement; restart recovery classifies all four outcomes and a second recovery is empty;
8. `TransactionalFileService::delete(path, exact_content_version)` exposes the proven runtime primitive without creating a second OS mutation path;
9. non-Windows durable delete remains fail-closed as unsupported;
10. no MCP write, patch or delete tool is exposed by this historical tranche.

Claim boundary: the forced-crash suite proves process termination/restart semantics at the documented service boundaries, not sudden-power-loss ACID durability. The write/replace path's documented Phase 2B `ReplaceFileW` external-writer window is unchanged; delete itself stays handle-based through its namespace effect.

##### Phase 2C3C — authorization/adapter gate

Status: **merged** through PR #30 (`624e88da`).

###### Phase 2C3C1 — authorized runtime boundary + ActionId binding

Status: **merged** through PR #26 (`ac381002`) and PR #27 (`1b5a3393`). Exact final heads `fcd1a6aa` and `ae05d6a2` each passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

1. `AuthorizedFileMutationService` resolves the active application session and the exact active task lease from application-owned registries before policy evaluation;
2. normalized `FileWrite` / `FileDelete` effects pass through `PolicyEngine`; primitive/effect mismatch is rejected before mutation;
3. write and patch share `FileWrite` authority while delete requires distinct `FileDelete`; structural workspace scopes and resource ceilings remain enforced by existing policy;
4. negative tests cover missing lease, cross-session lease, missing capability, stale policy epoch, scope escape, effect mismatch, stale expected state, symlink escape and mutation byte ceilings;
5. success reaches only `TransactionalFileService`; no second OS mutation route and no MCP mutation adapter are introduced;
6. the normalized `ActionEnvelope.action_id` becomes the same operation identity used by the recovery journal, deterministic write staging, returned commit metadata and restart recovery;
7. an already-active `.prepared.tmp` or journal slot for the same `ActionId` fails closed; this prevents concurrent/recovery collision without pretending to be the Phase 3 replay/idempotency ledger.

###### Phase 2C3C2 — application-owned mutation authority provisioning

Status: **merged** in PR #29 (`75477c3b`).

1. application/operator-owned startup configuration defines mutation authority; MCP cannot create or widen it;
2. `FileWrite` and/or `FileDelete` are added to the session only when corresponding mutation authority is explicitly provisioned;
3. exact task leases carry structural workspace scope (`WorkspaceAll` or `WorkspacePrefix`), canonical bounded resource ceiling, expiry and policy epoch;
4. the authoritative write/delete lease mapping remains application-owned, so a caller cannot submit or freely select an arbitrary lease id;
5. startup parsing rejects unsafe scope syntax and requires an absolute recovery state directory when mutation authority exists;
6. the session capability set is derived from actual provisioned authority rather than client request.

###### Phase 2C3C3 — thin MCP mutation adapter

Status: **merged** in PR #30 (`624e88da`). Exact final head `f70e520b` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

1. raw MCP parameters normalize into typed `Effect::FileWrite` / `Effect::FileDelete` plus explicit `ExpectedState` / exact `ContentVersion`;
2. the server generates the `ActionId` and resolves application-owned mutation authority internally; public schemas expose neither `TaskLeaseId` nor caller-provided `ActionId`;
3. the adapter invokes only `AuthorizedFileMutationService`; it performs no independent authorization or filesystem mutation;
4. `fs_write` / `fs_apply_patch` are registered only with `FileWrite` authority, while `fs_delete` requires the distinct `FileDelete` authority. With no authority the historical read/process surface is unchanged;
5. base64 payloads are framed by the bounded MCP transport and decoded bytes remain under mutation limits; structured responses are checked again against response ceilings;
6. startup recovery runs before MCP service start whenever a mutation state directory is supplied; unresolved recovery conflicts fail startup closed;
7. recovery-required / ambiguous outcomes are never mapped to success or a blind-retry contract;
8. started blocking durable mutations retain their transport execution permit and are awaited to a known transaction/recovery outcome. They are intentionally not wrapped in an adapter timeout that cannot cancel a started `spawn_blocking` task and could otherwise report failure while the filesystem effect still commits;
9. non-Windows durable mutation remains fail-closed as unsupported.

Design constraints across Phase 2C:

- journal entry count and bytes are hard bounded;
- ActionId is the recovery operation key; a general replay/idempotency service remains Phase 3 unless recovery requires a narrower primitive;
- wall-clock timestamps are diagnostic only and never authorization/freshness state;
- recovery never infers success merely from the absence of a temp file;
- an operation that may have committed but cannot be proven remains explicit/ambiguous until reconciled;
- recovery evidence must not be retired before operation-owned staging has been safely handled;
- durability claims must match actual Windows semantics and the tested crash model.

**Phase 2C gate passed.** Transactional mutation runtime, deterministic startup reconciliation, forced process-crash tests, application-owned authorization and the thin conditional MCP adapter are merged. The remaining Phase 2 work is Git.

### Phase 2D — Git read/integration

Status: **in progress; read path merged, integration current**.

#### Phase 2D1 — bounded Git read runtime

Status: **merged** in PR #32 (`1cb3cc03`). Exact final head `ed3b5c43` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

- exact canonical repository root + absolute canonical Git executable;
- bounded read-only `status`, literal-path/staged `diff`, and paginated `log`;
- disabled prompting/pager/external diff/textconv and bounded command deadline/output;
- log cursors pinned to their original reachable HEAD;
- concurrent readers share no mutable runtime state.

#### Phase 2D2 — operator-owned MCP Git read adapter

Status: **merged** in PR #33 (`aee4f168`). Exact final head `54951225` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

- operator-owned `--git-executable` conditionally provisions `GitRead`;
- MCP exposes `git_status`, `git_diff`, `git_log` only when that application-owned runtime exists;
- callers cannot select repository paths, executable paths, raw Git argv or authority identifiers;
- raw status/diff bytes remain exact via base64 within structured-response ceilings;
- log cursors are validated as current HEAD or reachable ancestors.

#### Phase 2D3 — exact-head Git integration

Status: **current**, split into narrow gates.

##### Phase 2D3A — exact-head integration runtime foundation

Status: **merged** in PR #40 (`22b34149`). Exact final head `96b30f94` passed dependency policy, Ubuntu format/Clippy/tests and Windows Clippy/tests.

- `Effect::GitIntegrate` binds exact `source_head` + `expected_target_head` object ids;
- runtime target is restricted to an operator-owned **direct** ref under `refs/optic/integration/`; ordinary branch refs and symbolic refs fail closed;
- first integration primitive is intentionally fast-forward-only; divergence returns deterministic `NonFastForward` instead of invoking merge machinery;
- preparation uses an ActionId-owned detached, locked `--no-checkout` worktree under an integration root outside the repository checkout;
- the worktree is validated against the exact expected head and must be removed successfully before any target ref update;
- the caller workspace HEAD/files remain untouched;
- the target is revalidated immediately before `git update-ref --no-deref <ref> <source> <expected>`, which repeats the exact-head comparison atomically;
- Git prompting/pagers/system+global config/replacement objects are disabled and an empty hooks path is revalidated before mutation-capable Git calls;
- no MCP Git mutation tool is exposed by 2D3A.

##### Phase 2D3B — application-owned integration authority + recovery

Status: **current**, split into B1/B2.

**2D3B1 — internal authorization boundary — merged in PR #41 (`f3d0898f`):**

1. `GitIntegrationAuthoritySet` mints no lease by default and, when enabled internally, exactly one `GitIntegrate` lease with `LeaseScope::Repository`;
2. the lease contains only bounded resource ceiling, expiry and policy epoch owned by the application registry;
3. `AuthorizedGitIntegrationService` resolves active session + exact active lease and applies `PolicyEngine` before the 2D3A runtime is reachable;
4. negative coverage rejects missing lease, wrong scope, missing session capability and cross-session lease;
5. no app startup flag and no MCP integration tool are added by B1. Exact final head `be3ffd97` passed dependency policy plus Ubuntu/Windows CI.

**2D3B2 — bounded recovery — current branch:**

1. enumerate Git worktrees with machine-readable `--porcelain -z` under byte/record ceilings and separately cap Optic-owned recovery candidates by the concurrent-request ceiling;
2. never use repository-global `worktree prune`; ignore user worktrees outside the Optic integration root;
3. require every Optic-root candidate to be a direct ActionId-named, registered, locked, non-symlink, canonically contained worktree; malformed/missing/unregistered/unlocked state fails closed before cleanup;
4. validate the complete snapshot before the first removal, then revalidate each path immediately before cleanup;
5. remove locked owned worktrees directly with double `--force`, avoiding an unlock/remove race window;
6. forced-process-termination coverage leaves a real locked orphan before target-ref mutation and proves restart cleanup does not move the target;
7. startup/operator invocation remains a following sub-gate after this runtime recovery passes CI.

##### Phase 2D3C — thin MCP integration adapter

Remaining gate:

1. expose no repository/ref/path/raw-argv/lease/ActionId authority to MCP callers;
2. generate ActionId server-side and resolve the application-owned integration lease internally;
3. expose the tool only when `GitIntegrate` authority is provisioned;
4. keep exact stale-target/non-fast-forward/cleanup outcomes structured and fail-closed;
5. validate the final adapter in a disposable repository before adding it to the ChatGPT plugin tool allowlist.

Phase 2 gate: stale-write tests, path/reparse escape tests, forced-crash recovery tests and Git stale-target/conflict/cleanup rejection. Non-fast-forward merge production semantics remain deferred until the fast-forward exact-head boundary and its recovery gate are proven.

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
