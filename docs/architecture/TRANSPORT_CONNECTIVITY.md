# Transport and Connectivity

## Principle

Execution/security core is transport-independent.

## Adapters

- stdio: local development/tests and compatible clients;
- local HTTP/Streamable HTTP: local adapter where required;
- outbound secure tunnel: optional connectivity adapter for cloud ChatGPT when supported.

Tunnel lifecycle/authentication belongs outside core policy and execution.

## Security

Never bind a privileged MCP endpoint publicly by default. Local listeners bind loopback unless explicitly designed otherwise. Session authentication/identity must not rely only on a caller-supplied session string.

## Compatibility

MCP protocol/version features are feature-detected. Do not make core correctness depend on optional client capabilities.

OpenAI/ChatGPT connectivity and MCP feature availability are external dependencies and must be revalidated against current official documentation before releases.
