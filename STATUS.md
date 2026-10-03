# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / Phase 1 vertical slice  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Finish the Windows-specific Phase 1D resource-containment gate without weakening the application-owned session, policy, transport and process boundaries already implemented.

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

### Phase 1C — implemented on PR #6 branch
- Opaque session-owned `JobId`; no arbitrary PID operation.
- Revocable application-owned `TaskLeaseRegistry`.
- Structured process API: absolute/canonical executable + `args[]` + workspace-contained cwd + controlled inherited environment.
- Operator-only startup allowlists (`--allow-executable`, `--allow-env`); MCP tools cannot create capabilities or task leases.
- Session receives `ProcessRun` only when at least one executable is explicitly operator-authorized.
- MCP tools: `process_start`, `process_read`, `process_stop`, `process_result`, plus `session_cancel`.
- `process_start` normalizes to `Effect::ProcessRun` and requires both the active session grant and the exact executable task lease before runtime execution.
- Network remains unavailable in the Phase 1C runtime; `network=true` fails closed.
- Bounded active jobs, retained records, stdout/stderr RAM, per-call output reads and process timeouts.
- Environment is cleared by default; only operator-allowlisted variables may be inherited.
- Windows lifecycle uses `process-wrap` Job Object assignment plus kill-on-drop; Unix uses a process group plus kill-on-drop.
- Native Windows CI caught and closed an output-overflow terminal-status race; timeout, explicit stop, output overflow and cross-session ownership tests pass on Windows and Linux.

### Current / next implementation
- Finalize PR #6 documentation and merge only while Linux/Windows/dependency gates remain green.
- Phase 1D: narrow audited Windows Job Object adapter for kernel-enforced process-count and memory limits, deterministic handle cleanup and native child-tree/resource gates.
- Do not claim `memory_bytes` or `process_count` as Windows-kernel-enforced until Phase 1D exists and its native tests pass.

### Later validated research candidates
- Phase 2: handle-first filesystem service and FILE_ID_INFO identity PoC.
- Phase 3: bounded ActionId idempotency ledger integrated with recovery state.
- Phase 4: worktree resource lifecycle and USN/notification-assisted invalidation with mandatory commit-time revalidation.
- Hardening research: restricted-token vs AppContainer/LPAC compatibility matrix.

### Not implemented yet
- Windows Job Object memory/process-count hard-limit adapter and Phase 1D native resource gate.
- Filesystem mutation services and Git execution services.
- Transaction journal.
- Installer/tunnel integration.
- Public release.

## Main baseline

`main` includes Phase 1A (`d33a1e5`) and Phase 1B (`681f939`). Phase 1C is under review in PR #6 and must remain green before merge.

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
