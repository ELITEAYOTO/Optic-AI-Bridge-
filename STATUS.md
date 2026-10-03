# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / Phase 2C2 journal-wrapped commit and crash recovery  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Phase 2C1 is merged on `main` via PR #15 (`85aec4c6`): Optic now has a bounded recovery-journal state machine and deterministic reconciliation foundation. The current implementation tranche is Phase 2C2: wire that journal around the proven Windows mutation commit, bind the operation ActionId to its staging artifact, and prove process-crash recovery at the actual namespace boundary. Public MCP file mutation remains intentionally disabled until those recovery and policy gates pass.

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
- Six recovery-specific tests pass on native Windows alongside the existing Phase 2B mutation and Phase 1 Job Object gates.
- Scope boundary: 2C1 proves the recovery state machine, not yet the journal-wrapped real commit, forced crash behavior or power-loss/ACID durability.

### Current / next implementation
- Phase 2C2: bind one journal ActionId to the actual Windows mutation lifecycle and staging artifact.
- Persist `committing` before any namespace commit can be attempted; map post-commit uncertainty to explicit recovery-required state rather than blind retry.
- Add child-process crash fixtures after durable `prepared`, after durable `committing` before namespace commit, immediately after namespace commit before terminal journal state, and after terminal state before retirement.
- Startup recovery must deterministically reconcile every surviving bounded journal entry before public mutation is enabled.
- Only after that gate, Phase 2C3 builds transactional file write/patch/delete runtime services; MCP `fs_write` / patch / delete remain deferred.
- Evaluate an oplock/handle-based rename PoC only if it materially reduces the documented residual external-writer window without creating deadlock/compatibility complexity.

### Later validated research candidates
- Phase 2C hardening: Windows oplock / handle-based rename experiment for the remaining path-based final-commit race.
- Phase 3: bounded ActionId idempotency ledger integrated with recovery state.
- Phase 4: worktree resource lifecycle and USN/notification-assisted invalidation with mandatory commit-time revalidation.
- Hardening research: restricted-token vs AppContainer/LPAC compatibility matrix.

### Not implemented yet
- Journal-wrapped production mutation commit and forced-crash recovery gate.
- Public filesystem mutation MCP services and Git execution services.
- Cross-platform durable mutation primitive equivalent to the Windows 2B boundary.
- Multi-session public runtime orchestration and same-repository worktree execution.
- Installer/tunnel integration.
- Restricted-token/AppContainer hardening profile.
- Public release.

## Main baseline

`main` includes Phase 1A (`d33a1e5`), Phase 1B (`681f939`), Phase 1C (`adf2e772`), Phase 1D (`69af07a`), Phase 2A (`71bdf082`), Phase 2B (`80f3aa9b`), Phase 2B closure docs (`c9590f5c`) and Phase 2C1 (`85aec4c6`). Phase 2C2 is the current implementation tranche.

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
