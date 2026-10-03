# Open Questions

1. Exact ChatGPT multi-connection/session identity semantics exposed through the selected MCP transport.
2. Authentication binding between a cloud/tunnel connection and local SessionHandle.
3. One daemon with isolated SessionContexts versus worker process per session.
4. Restricted Token versus AppContainer/LPAC compatibility with cargo/npm/Maven/Gradle/Java and native build tools.
5. Same-repo integration strategy: cherry-pick, patch transaction, merge or configurable policy.
6. Which optional MCP capabilities materially improve cancellation/progress/resources without introducing protocol-state coupling.
7. Exact RMCP cache configuration for Optic adapters; default stance is no stale-on-error for freshness/security-sensitive state.
8. Disk-spool ACL/encryption requirements.
9. Safe updater/signing mechanism.
10. Whether a Windows oplock (`FSCTL_REQUEST_OPLOCK`) or handle-based rename experiment materially reduces the residual final path-based commit race enough to justify its break/deadlock/compatibility complexity. Phase 2B already proved that `FILE_ID_INFO` materially detects same-path delete/recreate even when bytes are identical.
11. Whether USN-assisted invalidation reduces revalidation/scanning cost enough to justify its recovery/fallback complexity.
12. ActionId idempotency ledger persistence format, retention window and crash-recovery interaction.
13. Whether content-addressed output dedup justifies its complexity after bounded spool benchmarks.
14. Whether WASM extensions are ever needed; default remains no until a concrete use case exists.
