# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / architecture and documentation  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Freeze the V1 execution/security model before writing broad functionality.

### Completed
- Documentation ownership map.
- Windows-first Rust direction.
- MCP isolated as an adapter.
- Deterministic deny-by-default policy boundary.
- Bounded-output requirement.
- Multi-session isolation requirement.
- Same-repository parallelism via isolated Git worktrees.
- Crash/recovery and transaction direction.

### In progress
- Explicit application session handles for stateless MCP 2026-07-28.
- AI-native safety/action envelope.
- Executable security invariants.
- Transport input/output hard limits independent of SDK defaults.
- Capability lease and stale-context design.
- Release/supply-chain security lifecycle.

### Not implemented yet
- Cargo workspace/runtime.
- MCP server.
- Files/Git/process tools.
- Windows Job Object runtime.
- Installer/tunnel integration.
- Public release.

## Health rule

This file must describe reality. Never mark a feature implemented because it exists in a design document.
