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

Status: **complete** through Phase 2D3 exact-head Git integration and the final ChatGPT Desktop smoke.

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

Status: **complete** through Phase 2D3 and the final real ChatGPT Desktop smoke.

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

Status: **complete**. All narrow runtime/authorization/recovery/MCP/packaging gates and the final ChatGPT Desktop smoke passed.

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

Status: **merged through B3**.

**2D3B1 — internal authorization boundary — merged in PR #41 (`f3d0898f`):**

1. `GitIntegrationAuthoritySet` mints no lease by default and, when enabled internally, exactly one `GitIntegrate` lease with `LeaseScope::Repository`;
2. the lease contains only bounded resource ceiling, expiry and policy epoch owned by the application registry;
3. `AuthorizedGitIntegrationService` resolves active session + exact active lease and applies `PolicyEngine` before the 2D3A runtime is reachable;
4. negative coverage rejects missing lease, wrong scope, missing session capability and cross-session lease;
5. no app startup flag and no MCP integration tool are added by B1. Exact final head `be3ffd97` passed dependency policy plus Ubuntu/Windows CI.

**2D3B2 — bounded recovery — merged in PR #42 (`b74d563e`):**

1. enumerate Git worktrees with machine-readable `--porcelain -z` under byte/record ceilings and separately cap Optic-owned recovery candidates by the concurrent-request ceiling;
2. never use repository-global `worktree prune`; ignore user worktrees outside the Optic integration root;
3. require every Optic-root candidate to be a direct canonically encoded ActionId-named, registered, locked, non-symlink, canonically contained worktree; malformed/missing/unregistered/unlocked state fails closed before cleanup;
4. validate the complete snapshot before the first removal, then revalidate each path immediately before cleanup;
5. remove locked owned worktrees directly with double `--force`, avoiding an unlock/remove race window;
6. forced-process-termination coverage leaves a real locked orphan before target-ref mutation and proves restart cleanup does not move the target;
7. exact final head `0ff95b82` passed dependency policy plus Ubuntu/Windows CI.

**2D3B3 — startup/operator authority wiring — merged in PR #43 (`73ade6e3`):**

1. require `--git-integration-executable`, `--git-integration-root` and `--git-integration-ref` as one complete integration-runtime tuple; partial tuples fail startup closed;
2. run bounded B2 recovery before MCP serve whenever that tuple is present, even without mutation authority;
3. require separate `--allow-git-integrate` before minting the application-owned repository-scoped `GitIntegrate` task lease and session capability;
4. keep `--git-executable` exclusively tied to `GitRead` so integration/recovery does not implicitly grant read authority;
5. retain the recovered `GitIntegrationService` behind `AuthorizedGitIntegrationService` only when integration authority exists; recovery-only startup drops the runtime afterward;
6. reject MCP server construction when integration runtime/authority presence is inconsistent;
7. expose no `git_integrate` router or public mutation schema in B3; exact final head `c82059eb` passed dependency policy plus Ubuntu/Windows CI.

##### Phase 2D3C — thin MCP integration adapter

Status: **merged** in PR #44 (`b9766e4a`). Exact final head `55c4f3f2` passed dependency policy, Ubuntu format/Clippy/tests and Windows installer/Clippy/tests.

1. conditionally register exactly one `git_integrate` tool only when B3 application-owned integration authority is present;
2. accept only `source_head` and `expected_target_head` as full Git object ids and reject unknown public fields;
3. expose no repository/ref/path/raw-argv/lease/ActionId authority to MCP callers;
4. generate ActionId server-side, resolve the authoritative integration lease internally and normalize to `Effect::GitIntegrate`;
5. invoke only `AuthorizedGitIntegrationService`; no second Git mutation path exists in the adapter;
6. retain the transport execution permit until a started blocking integration reaches a known runtime outcome;
7. map stale target to explicit precondition failure and divergence to non-fast-forward; cleanup/recovery-required conditions remain fail-closed;
8. classify any failure to prove state after a successful atomic ref update as `outcome_uncertain`, never as a safe retry;
9. prove integration authority can expose `git_integrate` without implicitly exposing Git read tools;
10. CI validation passed on the exact PR head; PR #45 (`99837439`, exact green head `fa760213`) then passed a native Windows real-binary/stdin MCP smoke for integration-only authority, exact-head fast-forward and stale-target rejection.

##### Phase 2D3D — integration-target observation companion

Status: **merged** in PR #46 (`c44ba567`). Exact final head `f4d04c3` passed dependency policy, Ubuntu/Windows CI and the native real-binary MCP smoke.

1. add read-only `Effect::GitIntegrationObserve` mapped to `Capability::GitIntegrate`;
2. require the same active application-owned task lease and `LeaseScope::Repository`;
3. expose conditional `git_integration_status()` only alongside integration authority;
4. return only the exact current `target_head`, never the internal ref/repository/path/executable/argv/lease;
5. use the observation as the explicit `expected_target_head` for later integration rather than weakening or auto-filling the precondition;
6. extend the native real-binary smoke to observe target → integrate → observe new target → reject stale old observation.

##### Phase 2D3 ChatGPT packaging closure

Status: **merged** in PR #47 (`1e42b1ff`). Exact final head `4ff720b2` passed dependency policy, Ubuntu/Windows CI, the native real-binary integration smoke and the dedicated installer-profile smoke.

1. keep the default ChatGPT tool profile unchanged and integration authority absent;
2. require an explicit installer opt-in for the complete integration executable/root/ref + `--allow-git-integrate` tuple;
3. reject symbolic internal refs and create the fixed direct ref only when absent, using disabled hooks/prompts/system+global config/replacement objects plus `update-ref --no-deref` create-only old-OID comparison; never reset an existing direct ref;
4. allowlist `git_integration_status` plus prompt-gated `git_integrate` only in the opt-in profile;
5. keep read-only mode incompatible with integration mutation authority;
6. exercise generated configs and doctor startup in Windows CI under the runner workspace, with no ChatGPT user-profile mutation;
7. never remove repository state during normal uninstall; require explicit workspace + cleanup switch and CAS-delete the direct Optic ref when requested;
8. final disposable-repository ChatGPT Desktop smoke completed on 2026-10-05: observe initial target → prompt-gated fast-forward → observe new target → reject stale old target; independent verification confirmed unchanged caller workspace/branch, no orphan worktree and empty recovery state.

Phase 2 gate passed: stale-write tests, path/reparse escape tests, forced-crash recovery tests, Git stale-target/conflict/cleanup rejection, native real-binary smoke, installer-profile smoke and the final real ChatGPT Desktop exact-head integration smoke all passed. Non-fast-forward merge production semantics remain deliberately deferred beyond Phase 2D3.

## Phase 3 — multi-session

Status: **current**. Phase 3A is merged; Phase 3B is split into lifecycle and admission gates before public multi-session orchestration.

### Phase 3A — bounded shared-runtime resource isolation

Status: **merged** in PR #50 (`661c1604`), exact head `3c0c4448`; pull-request and post-merge `main` CI passed Ubuntu, Windows and dependency policy.

1. Compiled hard ceiling for retained application sessions; zero is never unlimited and capacity failure is fail-closed.
2. Bridge-wide process ceilings remain final guards while smaller per-session ceilings bound active jobs, retained records and reserved stdout/stderr RAM.
3. Every `JobId` remains owner-bound and record eviction is owner-only: a session may retire only its own terminal history.
4. Global-limit and per-session-limit errors remain distinct at the MCP adapter boundary.
5. Adversarial A/B tests prove one session's active-job/output exhaustion does not consume the other's quota and record pressure cannot evict the other session's terminal result.

### Phase 3B1 - application-owned session lifecycle and coordinated revoke

Status: **merged** in PR #51 (`78ae66f`), exact head `db20e4f`.

1. `SessionLifecycleManager` owns future session provisioning from `SessionGrantSpec` and generates opaque `SessionHandle`s inside the application boundary.
2. Revoke is fail-closed in order: invalidate the session first, then revoke all owned task leases, then request termination of all owned process jobs.
3. B1 does not physically remove revoked/expired session, lease or process records; capacity reclamation is deferred until admission can be serialized with revoke.
4. Existing MCP `session_cancel` delegates to the lifecycle manager without changing its response schema.
5. No public MCP session creation/minting tool is introduced by B1.

### Phase 3B2 - atomic admission versus revoke and safe reap

Status: **merged** in PR #52 (`2748f688`), exact head `c44c89ec`.

1. Acquire `SessionAdmissionPermit` while the session is active and hold it through sensitive sink entry.
2. Revoke closes new admission first and waits for admitted effects before lease/job cleanup.
3. `try_reap` refuses active/in-flight sessions and active jobs; quiescent cleanup removes owner-scoped terminal process records, then leases, then session.
4. Direct task-lease expired purge is removed and explicit `session_cancel` remains usable after natural expiry.

### Phase 3B3 - lifecycle closure and bounded leases

Status: **merged and post-merge validated** in PR #53 (`25a40e7`), exact green head `2337651`; `main` CI run #260 passed Ubuntu, Windows and dependency policy.

1. Add `HardLimits::max_task_leases` and `max_task_leases_per_session`, with fail-closed global/per-session registration.
2. Keep revoked leases counted until owner-scoped physical cleanup; allow individual physical removal only for unpublished revoked rollback state.
3. Prove partial multi-lease authority failure does not leak capacity.
4. Add bounded `inactive_handles(now)` and `SessionLifecycleManager::reap_inactive(now)` using the existing quiescent reap contract.
5. Route the initial stdio session through `SessionLifecycleManager::provision()` and derive its capabilities only from validated operator startup configuration.
6. Run a five-second internal expiry supervisor through the same lifecycle path using a bounded scan and blocking cleanup off the async executor.
7. Keep public session creation and renewal out of this gate.

Gate passed: exact-head Ubuntu/Windows/dependency-policy CI, exact-head merge, and post-merge `main` CI.

### Phase 3C - process/resource safety before wider autonomy

Status: **current**. Split into narrow security tranches.

#### Phase 3C1 - bounded process termination confirmation

Status: **merged and post-merge validated** in PR #55 (`bacac76`), exact green head `bfb38a5`; `main` CI run #265 passed Ubuntu, Windows and dependency policy.

1. Stop, timeout, output overflow and process-observation failures all request owned-tree termination and then require OS exit confirmation.
2. Confirmation is separately bounded to two seconds instead of awaiting `child.wait()` without a deadline.
3. Failure/error/timeout while proving death becomes `ProcessStatus::TerminationUncertain`; the MCP result string is `termination_uncertain`.
4. Uncertain jobs retain active-job/output reservations and session ownership, cannot be evicted as terminal history and block physical session reap.
5. Output drain tasks are aborted/closed on uncertainty so inherited pipe handles cannot restore an unbounded monitor wait; output is explicitly marked truncated.
6. No CPU governor, executable identity, network sandbox, public session surface or new MCP tool is introduced by 3C1.

Gate passed: exact-head Ubuntu/Windows/dependency-policy CI, exact-head merge, and post-merge `main` CI.

#### Phase 3C2 - bounded process CPU governor

Status: **implemented in current PR #57**. Exact code/test head `480de5c` passed CI #269 on Ubuntu, Windows and dependency policy, including native Windows hard-cap verification and the real MCP smoke; final documentation-head CI/merge remains pending.

1. `HardLimits` owns non-zero CPU ceilings with current defaults of 25% per active job, 75% aggregate Optic process CPU and 50% per session.
2. `ProcessManager` admits CPU reservation atomically with existing process/output capacity; one session cannot consume the entire Optic CPU reservation.
3. `TerminationUncertain` keeps its CPU reservation because process ownership was not proven released; retained proven-terminal history releases CPU reservation.
4. Native Windows Job Objects configure `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` before the suspended root process is assigned/resumed. Failure to configure the cap fails closed.
5. The MCP caller has no CPU-rate field. CPU governance is application/operator-owned rather than caller-selected.
6. The 75% aggregate value is an Optic admission ceiling, not whole-machine utilization telemetry or a guarantee against unrelated host load.

Final gate: documentation-head Ubuntu/Windows/dependency-policy CI, exact-head merge, then post-merge `main` CI.

#### Later Phase 3C gates

Still prioritize ToolProfile/ToolIdentity, truthful network semantics, environment/repository-code execution classification and Windows sandbox compatibility before exposing wider autonomous or public multi-session process orchestration.

Evaluate a bounded ActionId idempotency ledger/replay service only when a current retry/recovery contract needs it.

Phase 3 gate: adversarial cross-session access, lifecycle/admission races, bounded registry/resource pressure and process/tool containment.

## Phase 4 — same-repo parallelism

Git worktrees, deterministic coordinator, integration/conflict gate.

Evaluate event-driven target/file invalidation as an optimization on top of mandatory commit-time revalidation.

Gate: two concurrent sessions cannot silently overwrite or integrate stale changes.

## Phase 5 — connectivity/install

Local HTTP/tunnel adapter as required, installer/autoconfig/self-test.

## Phase 6 — hardening

Restricted-token compatibility experiments, fuzz/property tests, long soak, signed release pipeline.

Avoid implementing later-phase abstractions early unless a current interface genuinely needs the boundary.
