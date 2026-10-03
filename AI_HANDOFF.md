# AI Coding Handoff — Mandatory Guardrails

Read this file before changing architecture or implementing features.

## Non-negotiable invariant
**The AI decides. The bridge executes. Policy authorizes. Isolation contains.**

## MUST
Preserve transport/core separation; enforce session ownership on every resource; use typed actions; bound all long-lived state; use expected hashes/preconditions for mutations; keep same-repo sessions in separate worktrees; make policy deterministic and deny-by-default; test Windows process cleanup/security boundaries natively; document changed invariants in ADRs.

## MUST NOT
Add Electron; require Node/Python/Docker for core; embed a required LLM; let AI grant/approve permissions; expose arbitrary PID kill; use opaque shell strings as the fundamental process API; allow unbounded output/history/queues/caches/responses; call Job Objects a full sandbox; silently overwrite stale files; share mutable cwd/environment across sessions; let normal tools mutate policy; couple core execution to one ChatGPT/OpenAI transport; import OpticCode Java/RAG/editor layers for convenience; expand scope without Product Charter/ADR updates.

## Before coding
Identify the canonical doc, security boundary, session ownership, resource bounds, cancellation/cleanup, crash recovery, tests and ADR requirement.

## Definition of done
Formatting/lint/tests pass; relevant security tests exist; no unbounded state was introduced; docs/contracts match code; Windows-specific behavior is tested on Windows; benchmark claims remain TARGET until measured.
