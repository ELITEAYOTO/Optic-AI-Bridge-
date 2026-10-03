# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / Phase 0.1 contract hardening  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Close the remaining security/domain contract gaps before adding MCP or OS effects.

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

### In progress — Phase 0.1
- Replace independently combinable action kind/target fields with typed `Effect` variants.
- Make file creation/update preconditions explicit through `ExpectedState`.
- Add scope-bearing task leases for workspace, repository, executable and network boundaries.
- Require network capability and network scope at both session and task-lease level.
- Use monotonic deadline values for runtime lease/session expiry checks.
- Expand executable security-invariant tests for scope/network failures.
- Research file identity, handle-first filesystem operations, event-driven invalidation and idempotent action execution before deciding their implementation phase.

### Next
- TransportGuard contracts and tests.
- Session registry/revocation lifecycle.
- Phase 1 vertical slice: MCP stdio -> one session -> bounded file read/list -> policy -> supervised process lifecycle.

### Not implemented yet
- MCP server/adapter.
- Files/Git/process execution services.
- Windows Job Object runtime.
- Transaction journal.
- Installer/tunnel integration.
- Public release.

## Current branch / PR

`hardening/phase-0-1-contracts` — Phase 0.1 contract hardening.

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
