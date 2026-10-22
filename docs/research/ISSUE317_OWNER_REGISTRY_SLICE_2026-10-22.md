# Windows V1 #317 — owner registry first production-shaped slice

Status: **UNWIRED RESEARCH CODE, NO PRODUCTION #317 FIX**.
Date: 2026-10-22. New private `cfg(windows)` module
`git/owner_registry.rs` is compiled but has no Git/MCP call sites.

A service-owned registry now reserves finite concurrent job count, memory,
CPU and output before spawn; mints monotonic never-reused IDs; checks a
real SessionRegistry admission and `watch_active` grant; separates
Session from internal HostStartup; requests stop outside saturated
ordinary admissions; and holds reservations and native-owner payloads
until an internal nonblocking terminal observer explicitly proves cleanup.
Native observation errors and uncertain results retain the owner.
Cross-session access and wrong-domain release are rejected.

Current generic payload tests are **inert**, not Win32 owners; the interface
is a reviewed candidate only. Before production use: bind actual Jobs and
pipes, worker-owned real permits, exact resource/task lease ceilings,
suspend/assign/resume lifecycle, stdin joins, native failure matrices,
out-of-band authenticated stop, owner Drop/host-crash recovery, and gate
`SessionLifecycleManager::seal_quiescent` / `try_reap`.
No early migration of GitRead, no CAS/retry assumption, no V1 readiness
claim. Refer to ADR-0035 and issue #323. This branch must not be bulk merged.
