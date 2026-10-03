# Scope and Roadmap

Status: LIVING DOCUMENT. Last reviewed: 2026-10-03.

## Phase 0 — Architecture freeze (CURRENT)

- Cargo workspace and stable internal interfaces.
- Explicit application SessionHandle independent of MCP transport sessions.
- ActionEnvelope as the single execution authorization boundary.
- Security invariants mapped to tests.
- TransportGuard hard message/body/concurrency limits.
- Capability/task lease model.
- Source/sink containment model.
- Maintenance, status, changelog and release-security lifecycle.

**Gate:** no unresolved contradiction between tool contracts, policy, session ownership and resource bounds.

## V1 essential

- MCP adapter using the maintained Rust MCP SDK behind TransportGuard.
- Session identity, scoped project grant and revocable task/capability leases.
- Filesystem list/read/search plus transactional patch/write with expected versions.
- Git status/diff/log and worktree lifecycle.
- Structured process start/read/stop/result.
- Per-session process ownership and Windows Job Object cleanup.
- Deterministic deny-by-default policy.
- Bounded output, pagination, disk spool quotas and TTL.
- Crash recovery journal.
- Two simultaneous sessions on different projects.
- Same-repo parallel work via isolated Git worktrees.
- Windows-native security/integration tests.

## V1.5 candidates

- Deterministic coordinator for same-repo integration.
- Event-driven stale-context invalidation: TargetHeadChanged/FileVersionChanged/LeaseRevoked.
- Richer CLI session/task/resource dashboard.
- Restricted-token execution profile after compatibility testing.
- Signed installer/update path and release provenance.

## Later / experimental

- Content-addressed dedup if benchmarks justify it.
- WASM/WASI extension boundary only after a concrete extension requirement.
- VM/Windows Sandbox hard-isolation mode for untrusted workloads.
- Linux/macOS adapters after Windows invariants are stable.

## Reject unless evidence changes

Embedded AI security reviewer as authorization dependency; generic shell as core primitive; probabilistic “risk score” controlling permissions; microservice/plugin-per-tool architecture; Electron dashboard; unbounded configurable limits; automatic trust of MCP tool annotations; broad remote-desktop scope.

## Maintenance rule

A phase advances only when acceptance and security gates pass. Every roadmap movement updates STATUS.md. User-visible/relevant engineering changes update CHANGELOG.md. Changed invariants require tests and usually an ADR.
