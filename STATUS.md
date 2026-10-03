# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / Phase 1 runtime foundation  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Build the Phase 1 vertical slice in narrow, independently tested increments without weakening the Phase 0/0.1 security contracts.

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
- Dual session + task-lease authorization for process network access.
- Monotonic runtime authorization deadline values.
- Regression tests for scope escape and missing network authorization.
- ADR-0009 for typed effects/scoped leases.
- Research pass on handle-first filesystem I/O, Windows file identity, USN-assisted invalidation, ActionId idempotency, RMCP cache freshness, Job Objects and AppContainer/LPAC.

### Phase 1 tranche A — implemented on PR #4 branch
- New `optic-bridge-runtime` crate, keeping runtime ownership outside the MCP adapter.
- Real monotonic `StdClock` adapter for application deadlines.
- Revocable/expiring `SessionRegistry` with fail-closed typed errors.
- `TransportGuard` with Optic-owned request/response byte ceilings, request deadlines and concurrent-request permits.
- Bounded read-only filesystem service for `fs_read` and `fs_list`.
- Canonical containment check for existing read-only paths before I/O.
- Deterministic sorted directory pagination with a hard scan ceiling.
- Executable Phase 1 `HardLimits` for transport/filesystem/process ceilings; zero never means unlimited.
- Unit tests for transport limits, session revocation/expiry, filesystem chunking/pagination and scan ceilings.
- Linux/Windows CI and dependency policy green before documentation finalization.

### Current / next implementation
- Merge Phase 1 tranche A after the final documentation CI remains green.
- Phase 1 tranche B: maintained RMCP stdio adapter with one application-owned session and bounded `fs_list` / `fs_read` routed through normalization, policy and runtime services.
- Make transport/frame/body behavior explicit around RMCP so SDK defaults never become security authority.
- Phase 1 tranche C: structured process lifecycle (`process_start/read/stop/result`) with per-session ownership and bounded output.
- Phase 1 tranche D: Windows Job Object lifecycle/resource enforcement and native cleanup/resource tests.

### Later validated research candidates
- Phase 2: handle-first filesystem service and FILE_ID_INFO identity PoC.
- Phase 3: bounded ActionId idempotency ledger integrated with recovery state.
- Phase 4: worktree resource lifecycle and USN/notification-assisted invalidation with mandatory commit-time revalidation.
- Hardening research: restricted-token vs AppContainer/LPAC compatibility matrix.

### Not implemented yet
- MCP server/adapter.
- Process execution service and Windows Job Object runtime.
- Filesystem mutation services and Git execution services.
- Transaction journal.
- Installer/tunnel integration.
- Public release.

## Main baseline

`main` includes the Phase 0.1 squash merge from PR #2 and the Phase 1 status realignment from PR #3. Phase 1 tranche A is under review in PR #4.

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
