# Project Status

**Last updated:** 2026-10-04  
**Lifecycle:** pre-alpha / Phase 2C3B2 transactional delete under review  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Phase 2C3B2 is implemented in PR #24. The exact code head `3a5758e3` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`. The runtime now has a Windows handle-based transactional delete requiring an exact existing `ContentVersion`, durable journal intent `ExpectedState::Absent`, recovery-required semantics after durable `committing`, and native forced-process-crash recovery gates. `TransactionalFileService::delete` routes only through that proven journaled boundary. Public MCP file mutation remains intentionally disabled; after #24 merges, the next implementation tranche is Phase 2C3C authorization/adapter gating.

### Completed
- Documentation ownership and living governance.
- Windows-first Rust direction.
- MCP isolated as an adapter.
- Deterministic deny-by-default policy boundary.
- Bounded-output requirement.
- Multi-session/worktree isolation architecture.
- Phase 0 Cargo workspace merged to `main` via PR #1.
- Core opaque SessionHandle/TaskLeaseId/ActionId primitives.
- WorkspacePath and BLAKE3 ContentVersion primitives.
- ResourceBudget/HardLimits baseline.
- Typed ActionEnvelope, SessionGrant and TaskLease domain model.
- Initial deterministic PolicyEngine and negative security tests.
- Windows/Linux CI plus dependency-policy workflow.
- Phase 0.1 contract hardening merged to `main` via PR #2.
- Typed `Effect` variants replacing independently combinable action/target fields.
- Explicit file `ExpectedState` and Git target-head preconditions.
- Scope-bearing task leases for workspace, repository, executable and network boundaries.
- Segment-aware workspace scope checks.
- Dual session + task-lease authorization contract for process network access.
- Monotonic runtime authorization deadline values.
- Regression tests for scope escape and missing network authorization.
- ADR-0009 for typed effects/scoped leases.
- Research pass on handle-first filesystem I/O, Windows file identity, USN-assisted invalidation, ActionId idempotency, RMCP cache freshness, Job Objects and AppContainer/LPAC.

### Phase 1A — merged
- PR #4 merged to `main` as `d33a1e5`.
- `optic-bridge-runtime` keeps application runtime ownership outside the MCP adapter.
- Real monotonic `StdClock`.
- Revocable/expiring `SessionRegistry`.
- Optic-owned `TransportGuard` with request/response byte ceilings, request deadlines and bounded concurrency.
- Bounded `fs_read` and deterministic paginated `fs_list` with canonical workspace containment and hard directory-scan ceilings.

### Phase 1B — merged
- PR #5 merged to `main` as `681f939`.
- Maintained RMCP 3.4.0 used only as the MCP adapter.
- Optic-owned bounded JSON-line stdio transport enforces an input frame ceiling before JSON deserialization and a final serialized response ceiling before stdout write.
- Application-owned session lifecycle is independent of MCP transport state.
- MCP surface includes bounded `fs_read`, `fs_list` and `session_info` routed through normalization, policy and runtime services.

### Phase 1C — merged
- PR #6 merged to `main` as `adf2e772`.
- Opaque session-owned `JobId`; no arbitrary PID operation.
- Revocable application-owned `TaskLeaseRegistry`.
- Structured process API: absolute/canonical executable + `args[]` + workspace-contained cwd + controlled inherited environment.
- Operator-only startup allowlists (`--allow-executable`, `--allow-env`); MCP tools cannot create capabilities or task leases.
- MCP tools: `process_start`, `process_read`, `process_stop`, `process_result`, plus `session_cancel`.
- Network remains unavailable in the Phase 1 runtime; `network=true` fails closed.
- Bounded active jobs, retained records, stdout/stderr RAM, per-call output reads and process timeouts.

### Phase 1D — merged
- PR #7 merged to `main` as `69af07a`.
- `optic-bridge-windows` isolates the narrow Win32/unsafe boundary.
- Windows jobs use configure-before-resume Job Objects with kill-on-close, active-process and total job-memory ceilings.
- Native Windows tests prove process-count, memory, descendant-tree timeout, kill-on-close and fail-closed unwrap behavior.

### Phase 2A — merged
- PR #9 merged to `main` as `71bdf082`.
- Adds bounded streaming BLAKE3 content observation and `HardLimits::max_fs_mutation_bytes`.
- Adds canonical mutation observation and exact `ExpectedState::{Absent, Content}` conflict detection.
- Existing leaf symlinks are rejected; absent targets canonicalize the parent; targets remain workspace-contained.
- This tranche performs no durable mutation and exposes no MCP write/delete/patch tools.

### Phase 2B — merged
- PR #13 merged to `main` as `80f3aa9b`; closure docs merged in PR #14 as `c9590f5c`.
- `optic-bridge-windows` owns the Win32 filesystem boundary in addition to Job Objects.
- Existing files and parent directories are inspected through no-reparse handles; final-component reparse points are denied.
- `GetFinalPathNameByHandleW` verifies containment and `FILE_ID_INFO` binds prepared mutations to exact file or parent-directory identity.
- Prepared writes revalidate expected state + identity before staging and immediately before namespace commit.
- Replacement bytes are create-new staged in the same directory and `sync_all`'d.
- Existing targets commit through `ReplaceFileW`; absent targets use create-only hard-link semantics.
- Post-commit verification is explicit via `CommitVerification::{Verified, CommittedButUnverified}`.
- Native Windows CI proves replacement, create-only creation, same-content delete/recreate rejection and parent-directory recreation rejection.
- Residual boundary: `ReplaceFileW` is still path-based at the final call, so this is not claimed to be a kernel compare-and-swap against arbitrary external writers.

### Phase 2C1 — merged
- PR #15 merged to `main` as `85aec4c6` after the exact final SHA `90a6b646` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` via temporary validation PR #16; #16 was closed without merge.
- Adds `MutationRecoveryJournal` outside the canonical workspace so project-scoped MCP paths cannot address recovery state.
- Adds dedicated hard ceilings: `max_mutation_journal_file_bytes` (64 KiB default) and `max_mutation_recovery_records` (256 default); zero never means unlimited.
- One opaque `ActionId` keys each transaction journal.
- Complete journal states are append-only: `prepared → committing → verified|ambiguous`.
- The initial `prepared` record is create-new staged, `sync_all`'d and published by same-directory rename; later transitions append a complete JSON line and `sync_all`.
- A torn trailing state line falls back to the last complete state; corruption before the first complete `prepared` record fails closed.
- Recovery is deterministic and bounded: exact intended state = committed, exact prior state = not committed, third state/canonical mismatch = unresolved conflict retained fail-closed.
- `verified` is terminal and is not reinterpreted from later external edits.
- Well-formed unpublished `*.prepared.tmp` records are removable because namespace commit is forbidden before durable `committing` under the protocol.

### Phase 2C2 — merged
- PR #18 merged to `main` as `0297406c`; exact final head `fdfc3ace` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` before squash merge.
- One opaque operation `ActionId` owns both the durable journal and deterministic same-directory `.optic-<ActionId>.staged` artifact.
- `prepared` is durable first; `committing` is appended and synced before the atomic mutation service may be called.
- Atomic errors after durable `committing` become recovery-required and are best-effort marked `ambiguous`; blind retry is not treated as safe.
- Verified commits append durable `verified` before journal retirement. `CommittedButUnverified` remains explicit recovery-required state.
- Production recovery uses a preserving inspection path: journal evidence is not retired until staging validation/cleanup has succeeded.
- Any surviving staging artifact must be a regular file, must not exist for `PreparedNotCommitted`, must remain under the mutation byte ceiling, and its bounded BLAKE3 must match the journaled intended content before removal. Mismatch/unsafe cleanup fails closed while retaining journal evidence.
- Native Windows child-process tests terminate the process after durable `prepared`, after durable `committing` before the atomic service call, after the atomic commit service returns but before terminal journal state, and after terminal state before retirement. Restart recovery deterministically classifies each case and a second recovery is empty.
- Important precision: the post-commit crash hook is after `AtomicMutationService` returns (namespace effect and its post-commit verification have completed), not an instrumentation point between the raw `ReplaceFileW`/hard-link instruction and verification.
- No power-loss/ACID durability claim is made; the tested model is process termination/restart.

### Phase 2C3A — transactional write + patch merged
- PR #20 merged to `main` as `4415a65c`; exact final head `531f0e38` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.
- `TransactionalFileService` sits above the proven `JournaledMutationService`; no direct OS mutation path is introduced.
- Whole-file write requires explicit `ExpectedState::{Absent, Content}` and enforces the mutation byte ceiling before journaled commit.
- Patch is intentionally a deterministic single byte range (`offset`, `remove_bytes`, `insert`) rather than a new authorization effect; it uses the existing `FileWrite` authority model.
- Patch planning requires an exact base `ContentVersion`. The canonical target is read in bounded chunks, the assembled snapshot remains under `max_fs_mutation_bytes`, and its BLAKE3 must still equal the expected base before the patch result is built.
- The patched result is byte-bounded and commits through the same Phase 2C2 journaled Windows path, which performs its own final expected-state and identity revalidation.
- Native Windows tests include real journaled write+patch, multi-chunk snapshot assembly, stale patch rejection without modification, and empty recovery after successful operations.
- Non-Windows durable mutation remains fail-closed as unsupported.
- Public MCP write/patch/delete tools remain disabled.

### Phase 2C3B1 — intended-state journal merged
- PR #22 merged to `main` as `f5eafc3a`; exact final head `fe025f46` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.
- New journal records use schema v2 with explicit intended `ExpectedState::{Absent, Content}` while the storage directory remains stable so surviving v1 journals are still discovered.
- V1 content journals are parsed and normalized strictly; mixed v1/v2 transitions, unknown fields, invalid persisted content hashes and immutable-field changes fail closed while retaining evidence.
- Recovery records now carry explicit intended state and classify `committing|ambiguous` against exact intended state first, then exact prior state.
- Tests prove both delete-shaped outcomes before delete exists: observed `Absent` is committed; unchanged exact prior content is not committed.
- Write staging cleanup accepts only `intended = Content`; a surviving write staging artifact associated with `intended = Absent` fails closed.

### Phase 2C3B2 — Windows transactional delete under review
- PR #24 implements the complete runtime delete slice; exact code head `3a5758e3` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` before documentation alignment.
- `optic-bridge-windows` adds a distinct delete-capable no-reparse file handle opened with read + `DELETE` access and captures final path + `FILE_ID_INFO` before mutation.
- The exact handle whose identity and bounded BLAKE3 content are validated receives `SetFileInformationByHandle(..., FileDispositionInfo, ...)`; delete is not reissued by path.
- `PreparedDelete` requires an exact existing `ContentVersion`. Stale bytes and same-path/same-content file recreation fail closed before deletion.
- The handle closes before the atomic primitive returns, and the runtime explicitly verifies the final canonical target is `ExpectedState::Absent`; an unverifiable effect remains recovery-required.
- Journaled delete writes `intended = ExpectedState::Absent`, uses no write-staging artifact, persists `committing` before delete, and preserves the same verified/ambiguous recovery semantics as write.
- Native Windows forced-process-crash tests cover durable prepared, durable committing-before-delete, post-delete-service return and terminal-before-retirement. Restart reconciliation produces the expected committed/not-committed/terminal classifications and the second recovery is empty.
- `TransactionalFileService::delete(path, expected_content_version)` reuses the journaled service; no second OS mutation path exists.
- Non-Windows durable delete remains fail-closed as unsupported.
- Public MCP write/patch/delete remains disabled.

### Current / next implementation
- Finalize PR #24 documentation and require the complete documentation-aligned head to pass Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` before merge.
- After #24 merges, Phase 2C3C becomes current: authorize canonical `FileWrite`/`FileDelete` through existing session capability + exact task lease + structural workspace scope policy.
- Add negative tests for missing/wrong leases, cross-session access, stale policy epoch, scope escape, stale expected state, symlink/reparse escape, oversized input/result and recovery-required outcomes.
- Keep patch normalized to `FileWrite` authority and keep MCP `fs_write`/patch/delete disabled until those runtime/policy/recovery gates pass.
- Expose MCP mutation only as a thin adapter over the proven application/runtime contract once 2C3C is green.
- Evaluate an oplock/handle-based rename PoC only if it materially reduces the documented residual write/replace external-writer window without creating deadlock/compatibility complexity.

### Later validated research candidates
- Phase 2C hardening: Windows oplock / handle-based rename experiment for the remaining path-based final-commit race.
- Phase 3: bounded ActionId idempotency ledger integrated with recovery state.
- Phase 4: worktree resource lifecycle and USN/notification-assisted invalidation with mandatory commit-time revalidation.
- Hardening research: restricted-token vs AppContainer/LPAC compatibility matrix.

### Not implemented yet
- Public filesystem mutation MCP services and their Phase 2C3C authorization/adapter negative gates.
- Git execution services.
- Cross-platform durable mutation primitive equivalent to the Windows 2B/2C boundary.
- Multi-session public runtime orchestration and same-repository worktree execution.
- Installer/tunnel integration.
- Restricted-token/AppContainer hardening profile.
- Power-loss/ACID durability proof.
- Public release.

## Main baseline

`main` includes Phase 1A (`d33a1e5`), Phase 1B (`681f939`), Phase 1C (`adf2e772`), Phase 1D (`69af07a`), Phase 2A (`71bdf082`), Phase 2B (`80f3aa9b`), Phase 2B closure docs (`c9590f5c`), Phase 2C1 (`85aec4c6`), Phase 2C2 (`0297406c`), Phase 2C3A (`4415a65c`) and Phase 2C3B1 (`f5eafc3a`). Phase 2C3B2 is under review in PR #24 and is not yet part of `main`.

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
