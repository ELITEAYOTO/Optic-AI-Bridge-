# Transport and Connectivity

## Principle

Execution/security core is transport-independent.

## Adapters

- stdio: local development/tests and compatible clients;
- local HTTP/Streamable HTTP: local adapter where required;
- outbound secure tunnel: optional connectivity adapter for cloud ChatGPT or another MCP client when supported.

Tunnel lifecycle/authentication belongs outside core policy and execution.

## Remote MCP research candidate

A separate harmless prototype supplied by the project owner has shown that the tested FranceStudent custom-MCP flow can discover and call a tool through MCP Streamable HTTP at `/mcp`. This is compatibility evidence for a future transport candidate, not production Optic support and not a decision to implement remote access now.

The candidate architecture, unproven authentication questions, zero-mandatory-backend goal and future acceptance gates are recorded in [FranceStudent / Remote MCP candidate](../research/FRANCESTUDENT_REMOTE_MCP_CANDIDATE.md).

Any future remote adapter must reuse the same application-owned sessions, policy, capabilities, task leases, resource ceilings and runtime boundaries as stdio. A tunnel/provider must remain replaceable transport infrastructure rather than authority.

## Security

Never bind a privileged MCP endpoint publicly by default. Local listeners bind loopback unless explicitly designed otherwise. Session authentication/identity must not rely only on a caller-supplied session string. A public or tunnel URL is not an authentication secret.

## Compatibility

MCP protocol/version features are feature-detected. Do not make core correctness depend on optional client capabilities.

OpenAI/ChatGPT connectivity, FranceStudent compatibility and MCP feature availability are external dependencies and must be revalidated against current official/client behavior before releases or support claims.
