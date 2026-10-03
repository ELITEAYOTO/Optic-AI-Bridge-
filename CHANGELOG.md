# Changelog

All notable project changes are recorded here.

## [Unreleased]

### Added
- Canonical product, architecture, security, specification, engineering, operations and research documentation.
- Multi-session and same-repository worktree isolation design.
- Deterministic policy/capability model.
- Bounded output and crash-recovery design.
- Living roadmap, project status, security policy and maintenance policy.
- Security invariants and AI-native safety architecture.
- ADRs for application-owned session handles, action envelopes and source/sink containment.
- Phase 0 Rust workspace with `optic-bridge-core`, `optic-bridge-policy` and `optic-bridge-app`.
- Opaque random SessionHandle, TaskLeaseId, ActionId and JobId primitives with redacted Debug output.
- Strict workspace-relative path primitive and BLAKE3 content versions.
- Explicit ResourceBudget/HardLimits types where zero is never interpreted as unlimited.
- Typed ActionEnvelope, SessionGrant and TaskLease.
- Initial deterministic PolicyEngine with negative security tests.
- Windows/Linux CI and cargo-deny dependency policy.
- Phase 0.1 typed `Effect` model so target/precondition/reversibility semantics are derived from one valid variant rather than independently combinable fields.
- Explicit `ExpectedState::{Absent, Content}` mutation preconditions and validated Git object ids for integration preconditions.
- Scope-bearing task leases for workspace prefixes, repositories, process executables and network destinations.
- Monotonic runtime deadline primitive for session/task expiry checks.
- Security tests for lease-scope escape and process-network authorization.
- Phase 1 `optic-bridge-runtime` crate for application-owned runtime services independent of MCP.
- Real monotonic `StdClock` adapter and revocable/expiring `SessionRegistry`.
- `TransportGuard` enforcing request/response byte ceilings, request deadlines and bounded concurrent requests independently of SDK defaults.
- Bounded read-only filesystem service for chunked `fs_read` and deterministic paginated `fs_list`.
- Hard filesystem read/list/scan ceilings and transport concurrency/request-duration ceilings in `HardLimits`.
- Runtime tests covering session revocation/expiry, transport byte/concurrency/deadline bounds, filesystem chunking/pagination and directory scan ceilings.
- Phase 1B `optic-bridge-mcp` adapter using pinned RMCP 3.4.0.
- Optic-owned bounded JSON-line stdio transport with request framing bounded before JSON deserialization and response size bounded after final JSON serialization.
- Read-only MCP tools `fs_read`, `fs_list` and `session_info`, routed through application session lookup, typed effects, policy and runtime services.
- Revocable `TaskLeaseRegistry` for application-owned process authorization.
- Structured `ProcessManager` with absolute canonical executables, args arrays, workspace-contained cwd, cleared environment and explicit inherited-environment allowlist.
- Operator-only process startup allowlists through `--allow-executable` and `--allow-env`.
- Phase 1C MCP tools `process_start`, `process_read`, `process_stop`, `process_result` and `session_cancel`.
- Bounded active process jobs, retained job history, output RAM, process output reads and execution timeout.
- Session-owned cursor-readable stdout/stderr capture with opaque JobIds instead of arbitrary PIDs.
- Native Windows regression coverage for process output, cross-session rejection, timeout, explicit stop and output overflow.
- Phase 1D `optic-bridge-windows` crate as the narrow Win32/unsafe containment boundary.
- Custom Windows `LimitedJobObject` with suspended creation, configure-before-resume ordering, kill-on-close, active-process and total job-memory limits.
- Native Windows Phase 1D fixtures proving process-count enforcement, memory enforcement, descendant-tree timeout cleanup and kill-on-close behavior.
- Phase 2A bounded streaming BLAKE3 content observation for mutation preconditions.
- `HardLimits::max_fs_mutation_bytes` so mutation observation has an explicit hard byte ceiling.
- Runtime `MutationObservation` / `MutationError` foundation for canonical target observation and exact `ExpectedState` conflict detection without exposing durable writes.
- Regression coverage for stale/blind-overwrite preconditions, oversized mutation targets, leaf symlink denial and parent-symlink workspace escape handling.
- Phase 2B Windows handle-first filesystem primitives using no-reparse opens, final paths from handles and `FILE_ID_INFO` identity.
- `AtomicMutationService` / opaque `PreparedMutation` foundation with double expected-state + identity revalidation around same-directory staging.
- Windows existing-target commit through `ReplaceFileW` and absent-target create-only commit through hard-link semantics.
- Explicit `CommitVerification::{Verified, CommittedButUnverified}` so a post-commit verification failure is not misreported as a pre-commit rollback.
- Native Windows mutation tests proving replacement, create-only semantics, same-content delete/recreate identity rejection and parent-directory recreation rejection.
- Phase 2C1 `MutationRecoveryJournal` with operator-owned recovery state stored outside the canonical workspace.
- Append-only recovery state machine `prepared → committing → verified|ambiguous`, keyed by opaque `ActionId`.
- Dedicated `HardLimits::max_mutation_journal_file_bytes` and `max_mutation_recovery_records` ceilings.
- Bounded deterministic recovery scan and reconciliation against exact prior/intended content state.
- Recovery tests proving prepared-only abort classification, intended/prior-state reconciliation, third-state fail-closed retention, outside-workspace state placement and verified-terminal retirement.

### Changed
- Architecture updated for MCP 2026-07-28 stateless protocol semantics.
- Security goal changed from impossible “100% secure” wording to testable invariants plus defense in depth.
- Project lifecycle advanced from documentation-only through executable Phase 0/0.1, completed Phase 1A–1D, Phase 2A/2B mutation foundations and the Phase 2C1 recovery-state foundation.
- Process network access requires NetworkAccess at both session and task-lease level plus an explicit network lease scope at the policy layer; the Phase 1 runtime itself still refuses network-enabled process starts.
- Workspace prefix authorization is segment-aware (`src` does not authorize `src2`).
- Existing read-only filesystem targets are canonicalized and verified to remain under the canonical workspace root before I/O.
- A session receives `ProcessRun` only when the bridge operator explicitly authorizes at least one executable at startup.
- Process output overflow is classified after stdout/stderr drains complete, closing a Windows race where a fast child could exit before the overflow flag was observed.
- Windows process lifecycle uses the Optic-owned Windows adapter so authorized memory/process-count budgets become explicit kernel Job Object limits.
- Phase 2 is split into narrow gates: canonical/bounded observation, Windows mutation-time containment and atomic namespace commit, recovery state machine, journal-wrapped forced-crash gate, transactional file services, then Git read/integration.
- Non-Windows durable file commit remains explicitly unsupported until an equivalent containment/commit boundary is designed and proven.

### Security
- Model/repository/process output explicitly treated as untrusted for authorization.
- Transport limits must be enforced by Optic AI Bridge even when an upstream SDK also has limits.
- Initial policy rejects cross-session leases, missing leases for mutating/process actions, stale policy epochs, resource-budget overflow, normal policy mutation and privilege elevation.
- Task leases deny mutations/process/network operations outside their explicit scope.
- Network permission is not inherited from a broad session capability alone for leased process execution.
- Phase 1 runtime state remains application-owned rather than MCP-session-owned.
- Directory listing has both a page bound and an independent deterministic scan ceiling.
- Filesystem reads allocate at most the caller request bounded by the compiled per-call hard ceiling.
- MCP cannot create process capabilities or leases; `process_start` must match an exact canonical executable lease created from operator startup configuration.
- Process environment is empty by default and can inherit only names explicitly allowlisted by the operator.
- Process control is by opaque session-owned JobId; no arbitrary PID kill API exists.
- On Windows, authorized `memory_bytes` and `process_count` map to Job Object `JOB_OBJECT_LIMIT_JOB_MEMORY` and `JOB_OBJECT_LIMIT_ACTIVE_PROCESS`; native CI proves both limits are enforced.
- Phase 2A mutation observation rejects blind overwrite/stale expected state, rejects leaf symlinks, resolves absent-target parents canonically and enforces workspace containment.
- Mutation precondition hashing is both memory-bounded and byte-bounded.
- Phase 2B binds prepared Windows mutations to `FILE_ID_INFO`, so same-path delete/recreate with identical content is still detected as stale.
- Existing and absent targets are revalidated immediately before the namespace commit; replacement data is staged in the same directory and synced before commit.
- Final-component reparse points are denied in the Windows handle boundary, and opened final paths must remain under the canonical workspace root.
- Phase 2B does not claim compare-and-swap semantics: `ReplaceFileW` still names the final target by path after revalidation, leaving a documented external-writer TOCTOU window.
- Phase 2C1 recovery state is kept outside the project workspace, is byte/count bounded, ignores only a torn trailing transition, and retains unresolved third-state conflicts instead of retrying blindly.
- A `prepared` journal record alone cannot authorize a namespace commit; the durable `committing` transition is the protocol boundary before commit may be attempted.
- Phase 2C1 does not yet prove forced-crash behavior around the real Windows namespace commit or power-loss/ACID durability.
- Public MCP file mutation remains disabled until the journal-wrapped commit and forced-crash recovery gates pass.
