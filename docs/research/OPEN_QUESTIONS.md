# Open Questions

1. Exact ChatGPT multi-connection/session identity semantics exposed through the selected MCP transport.
2. Authentication binding between a cloud/tunnel connection and local SessionId.
3. One daemon with isolated SessionContexts versus worker process per session.
4. Restricted Token compatibility with cargo/npm/Maven/Gradle/Java.
5. Same-repo integration strategy: cherry-pick, patch transaction, merge or configurable policy.
6. Which optional MCP capabilities materially improve cancellation/progress/resources.
7. Disk-spool ACL/encryption requirements.
8. Safe updater/signing mechanism.
9. Whether content-addressed output dedup justifies its complexity.
10. Whether WASM extensions are ever needed; default remains no until a concrete use case exists.
