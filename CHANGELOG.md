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

### Changed
- Architecture updated for MCP 2026-07-28 stateless protocol semantics.
- Security goal changed from impossible “100% secure” wording to testable invariants plus defense in depth.
- Project lifecycle advanced from documentation-only through executable Phase 0/0.1, completed Phase 1A–1D, and the gated Phase 2 mutation foundation.
- Process network access requires NetworkAccess at both session and task-lease level plus an explicit network lease scope at the policy layer; the Phase 1 runtime itself still refuses network-enabled process starts.
- Workspace prefix authorization is segment-aware (`src` does not authorize `src2`).
- Existing read-only filesystem targets are canonicalized and verified to remain under the canonical workspace root before I/O.
- A session receives `ProcessRun` only when the bridge operator explicitly authorizes at least one executable at startup.
- Process output overflow is classified after stdout/stderr drains complete, closing a Windows race where a fast child could exit before the overflow flag was observed.
- Windows process lifecycle no longer relies on the generic process-wrap Job Object wrapper for Phase 1D resource claims; the runtime uses the Optic-owned Windows adapter so authorized memory/process-count budgets become explicit kernel Job Object limits.
- Phase 2 is split into narrow gates: canonical/bounded observation first, mutation-time OS containment and atomic commit next, durable recovery journal after that, then Git read/integration.

### Security
- Model/repository/process output explicitly treated as untrusted for authorization.
- Transport limits must be enforced by Optic AI Bridge even when an upstream SDK also has limits.
- Initial policy rejects cross-session leases, missing leases for mutating/process actions, stale policy epochs, resource-budget overflow, normal policy mutation and privilege elevation.
- Task leases deny mutations/process/network operations outside their explicit scope.
- Network permission is not inherited from a broad session capability alone for leased process execution.
- Phase 1 runtime state remains application-owned rather than MCP-session-owned.
- Directory listing has both a page bound and an independent deterministic scan ceiling, preventing apparently paginated calls from becoming unbounded directory scans.
- Filesystem reads allocate at most the caller request bounded by the compiled per-call hard ceiling.
- MCP cannot create process capabilities or leases; `process_start` must match an exact canonical executable lease created from operator startup configuration.
- Process environment is empty by default and can inherit only names explicitly allowlisted by the operator.
- Process control is by opaque session-owned JobId; no arbitrary PID kill API exists.
- On Windows, authorized `memory_bytes` and `process_count` now map to Job Object `JOB_OBJECT_LIMIT_JOB_MEMORY` and `JOB_OBJECT_LIMIT_ACTIVE_PROCESS`; native CI proves both limits are enforced.
- Each process job owns its own kill-on-close Job Object and is owned by exactly one application session; Job Objects are containment, not a complete sandbox.
- Phase 2A mutation observation rejects blind overwrite/stale expected state, rejects leaf symlinks, resolves absent-target parents canonically and enforces workspace containment.
- Mutation precondition hashing is both memory-bounded and byte-bounded; oversized existing targets fail closed rather than triggering unbounded scans.
- Phase 2A is not treated as a race-free Windows commit boundary: handle-first reparse/final-path/file-identity revalidation remains mandatory before durable mutation is exposed.
