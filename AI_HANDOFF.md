# AI Coding Handoff — Mandatory Guardrails

Read this file before changing architecture or implementing features.

## Non-negotiable invariant

**The AI decides what it wants to attempt. The bridge executes only authorized effects. Policy authorizes. Isolation contains.**

## MUST

- Preserve transport/core separation and the TransportGuard.
- Treat MCP 2026-07-28 transport as stateless; use Optic-owned application SessionHandles for stateful resources.
- Route every executable effect through a typed ActionEnvelope.
- Enforce session/task ownership on every resource.
- Keep capabilities narrow, expiring and revocable.
- Bound every long-lived collection, protocol body, queue, output and spool.
- Use expected hashes/version preconditions for mutations.
- Keep same-repo write sessions in separate worktrees.
- Treat repository/log/network content as untrusted sources.
- Gate sensitive sinks (network, secrets, external writes, policy/privilege) deterministically.
- Test Windows process cleanup/security boundaries natively.
- Add/update an ADR when an architectural/security invariant changes.

## MUST NOT

- Add Electron or require Node/Python/Docker for core.
- Embed a required LLM.
- Let AI grant/approve permissions or use AI risk scores as authorization.
- Trust a client-supplied session ID merely because it is syntactically valid.
- Expose arbitrary PID kill.
- Use opaque shell strings as the fundamental process API.
- Allow unbounded request bodies, stdout/stderr, histories, queues, caches or responses.
- Call Job Objects a full sandbox.
- Silently overwrite stale files.
- Share mutable cwd/environment across sessions.
- Give child processes ambient secrets/network without explicit capability.
- Let normal tools mutate security policy.
- Couple core execution to one ChatGPT/OpenAI transport.
- Assume upstream SDK limits are sufficient.
- Import OpticCode Java/RAG/editor layers for convenience.
- Expand scope without Product Charter/ADR updates.

## Before coding

Identify canonical doc, security invariants touched, ActionEnvelope shape, session/task ownership, resource ceilings, cancellation/cleanup, crash recovery, tests and ADR requirement.

## Definition of done

Formatting/lint/tests pass; security regression tests exist; no unbounded state was introduced; docs/contracts match code; Windows-specific behavior is tested on Windows; STATUS/CHANGELOG are accurate; benchmark claims remain TARGET until measured.
