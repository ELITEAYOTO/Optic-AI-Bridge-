# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / Phase 0 executable foundation  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Make the security/domain foundation compile cleanly before adding MCP or OS effects.

### Completed
- Documentation ownership and living governance.
- Windows-first Rust direction.
- MCP isolated as an adapter.
- Deterministic deny-by-default policy boundary.
- Bounded-output requirement.
- Multi-session/worktree isolation architecture.
- Initial Cargo workspace on `bootstrap/phase-0-foundation`.
- Core opaque SessionHandle/TaskLeaseId/ActionId primitives.
- WorkspacePath and BLAKE3 ContentVersion primitives.
- ResourceBudget/HardLimits baseline.
- ActionEnvelope, SessionGrant and TaskLease domain model.
- Initial deterministic PolicyEngine and negative security tests.
- Initial Windows/Linux CI plus dependency-policy workflow.

### In progress
- Get the bootstrap PR CI green and merge the Phase 0 baseline.
- TransportGuard contracts.
- Session registry/revocation lifecycle.
- More executable security-invariant tests.
- Release/supply-chain hardening.

### Not implemented yet
- MCP server/adapter.
- Files/Git/process execution services.
- Windows Job Object runtime.
- Transaction journal.
- Installer/tunnel integration.
- Public release.

## Current branch / PR

`bootstrap/phase-0-foundation` — PR #1.

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
