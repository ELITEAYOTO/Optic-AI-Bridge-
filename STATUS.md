# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / Phase 1 preparation  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Start the Phase 1 vertical slice without weakening the Phase 0/0.1 security contracts: TransportGuard, session registry/revocation, bounded filesystem reads and supervised Windows process execution.

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

### Current / next implementation
- TransportGuard contracts and tests.
- Session registry/revocation lifecycle and real monotonic Clock adapter.
- Phase 1 vertical slice: MCP stdio -> one session -> bounded fs_list/fs_read -> policy -> process_start/read/stop -> Windows Job Object -> bounded output.
- Explicit RMCP cache/freshness configuration so stale protocol cache never satisfies mutation/security preconditions.

### Later validated research candidates
- Phase 2: handle-first filesystem service and FILE_ID_INFO identity PoC.
- Phase 3: bounded ActionId idempotency ledger integrated with recovery state.
- Phase 4: worktree resource lifecycle and USN/notification-assisted invalidation with mandatory commit-time revalidation.
- Hardening research: restricted-token vs AppContainer/LPAC compatibility matrix.

### Not implemented yet
- MCP server/adapter.
- Files/Git/process execution services.
- Windows Job Object runtime.
- Transaction journal.
- Installer/tunnel integration.
- Public release.

## Main baseline

`main` includes the Phase 0.1 squash merge from PR #2 (`487bb3c`).

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
