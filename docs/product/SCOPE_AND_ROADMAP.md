# Scope and Roadmap

## V1 essential

- Cargo workspace and stable internal interfaces.
- MCP adapter using the maintained Rust MCP SDK.
- session identity and scoped project grant;
- filesystem list/read/search plus transactional patch/write;
- Git status/diff/log and worktree lifecycle needed for isolation;
- structured process start/read/stop/result;
- per-session process ownership and Windows Job Object cleanup;
- deterministic deny-by-default policy;
- bounded output, pagination, disk spool quotas and TTL;
- crash recovery journal;
- two simultaneous sessions on different projects;
- isolated same-repo parallel work via Git worktrees;
- Windows-native security/integration tests.

## V1.5 candidates

- coordinator/arbiter for same-repo integration;
- event notifications for relevant cross-session changes;
- capability leases with explicit expiry/revocation;
- richer resource accounting and CLI session dashboard;
- signed installer/update path.

## Later / optional

- WASM/WASI extension boundary if real extension demand appears;
- hardened restricted-token execution profile;
- hard isolation via VM/Sandbox for untrusted workloads;
- Linux/macOS adapters after Windows invariants are stable.

## Reject unless evidence changes

Embedded AI security reviewer as an authorization dependency, generic shell as the core primitive, microservice/plugin-per-tool architecture, Electron dashboard, unbounded configurable limits, and broad remote-desktop features.

## Delivery gates

A phase advances only when its invariants have automated tests. Calendar estimates are planning aids, not release criteria.
