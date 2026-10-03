# Install and Autoconfiguration

## Goal
One lightweight user-scoped install with self-test; no mandatory Node, Python, Docker or administrator service for the core.

## Proposed flow
1. Install signed executable(s) under a user-scoped application directory.
2. Create data/cache/config directories with restrictive permissions.
3. Generate local identity/secrets where required.
4. Detect Git and common toolchains without modifying them.
5. Configure the selected MCP transport.
6. Run self-tests for policy, temporary workspace, process-tree cleanup and connectivity.
7. Print concise pairing/project-grant instructions.

## UX candidates
PROPOSED CLI operations: pair, grant, sessions, revoke, kill-all and doctor. Exact names are not yet contractual.
