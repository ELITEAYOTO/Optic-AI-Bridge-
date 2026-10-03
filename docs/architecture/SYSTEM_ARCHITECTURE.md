# System Architecture

Status: DECIDED at boundary level; module names may evolve.

## Layers

ChatGPT / MCP client
→ Transport Adapter
→ TransportGuard
→ MCP Adapter
→ Session Router
→ ActionEnvelope normalization
→ Policy / Capability Engine
→ Capability-scoped Services
→ Files / Git / Process services
→ OS Runtime
→ Windows / isolated project workspaces

## Boundary rules

### Transport Adapter
Connectivity only. No filesystem, Git, process, policy or session business logic.

### TransportGuard
Optic-owned ceilings for request/frame/body size, concurrency, response size, timeouts and cancellation. SDK defaults are not security invariants.

### MCP Adapter
Protocol schemas and translation. It can expose protocol features but makes no authorization decision.

### Session Router
MCP 2026-07-28 is stateless at protocol level, so Optic owns application sessions explicitly. It maps a server-minted SessionHandle to a validated SessionContext: principal/project grant, worktree, task/capability leases, resource budget, policy epoch and owned jobs.

### ActionEnvelope
Every side-effecting request becomes one typed normalized effect containing canonical targets, expected state, requested resources/network, session/task and policy context. No service can execute directly from raw MCP arguments.

### Policy / Capability Engine
Deterministic final authority. Model intent is advisory input, never an authorization bit.

### Services
Typed Rust APIs receive an authorized envelope/context rather than global machine access.

### OS Runtime
Windows-specific containment/process lifecycle behind narrow interfaces.

## Proposed Cargo workspace

crates/
- optic-bridge-app
- optic-bridge-mcp
- optic-bridge-core
- optic-bridge-session
- optic-bridge-policy
- optic-bridge-fs
- optic-bridge-git
- optic-bridge-process
- optic-bridge-windows
- optic-bridge-journal

Do not split further until dependency boundaries justify it.

## Local core + MCP adapter

Core services must be callable from integration tests and a future local CLI without pretending to be MCP clients.

## Dependency direction

Adapters depend inward. Core/security layers never import a concrete transport. Windows implementation satisfies core interfaces. Raw transport arguments never reach OS execution without normalization/policy.
