# System Architecture

Status: DECIDED at boundary level; module names may evolve.

## Layers

ChatGPT / MCP client
→ Transport Adapter
→ MCP Adapter
→ Session Router
→ Policy Engine
→ Capability-scoped Services
→ Files/Git/Process services
→ OS Runtime
→ Windows / project workspaces

## Boundary rules

### Transport Adapter
Connectivity only. It must not contain filesystem, Git, process, policy or session business logic.

### MCP Adapter
Protocol schemas, MCP request/response translation, cancellation/progress plumbing. No security decisions.

### Session Router
Maps authenticated connection/session identity to immutable or narrowly mutable SessionContext: project grant, worktree, capabilities, resource budget and owned jobs.

### Policy Engine
Deterministic final authority. Evaluates typed actions and canonical targets. The model may provide intent/context but never the final authorization bit.

### Services
Typed Rust APIs. Files, Git and processes receive an authorized SessionContext rather than global machine access.

### OS Runtime
Windows-specific containment and process lifecycle. Hidden behind traits/interfaces so core logic is testable.

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

This is preferred over coupling the entire application to MCP. Core services should be callable from integration tests and a future local CLI without pretending to be MCP clients.

## Dependency direction

Adapters depend inward. Core/security layers never import a concrete transport. Windows implementation depends on core interfaces, not the reverse.
