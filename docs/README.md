# Documentation Map

This directory is the canonical engineering knowledge base for Optic AI Bridge.

## Read this first for the current state

1. [AI Handoff](../AI_HANDOFF.md) — mandatory security/architecture guardrails.
2. [Current State Snapshot — 2026-10-12](product/CURRENT_STATE_2026-10-12.md) — compact active state without rewriting the long historical status ledger.
3. [Project Health Checkpoint — 2026-10-12](product/PROJECT_HEALTH_2026-10-12.md) — what is finished, what remains, blockers, missing evidence and next coding candidates.
4. [Roadmap](../ROADMAP.md) — active phase/gates.
5. [Host Compatibility & Session Autonomy Track](product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md) — H-00 through H-10.
6. [Remote/Autonomy Security Audit Gates](security/REMOTE_AUTONOMY_AUDIT_GATES.md) — cross-cutting blockers before real-project remote exposure.
7. [FranceStudent / Remote MCP Candidate](research/FRANCESTUDENT_REMOTE_MCP_CANDIDATE.md) — target-host evidence, new auth/header UI facts and remaining unknowns.
8. [Real Optic MCP Link Experiment](research/REAL_OPTIC_MCP_LINK_EXPERIMENT.md) — synthetic Streamable HTTP -> real stdio bridge proof, including live FranceStudent validation.
9. [Provider-neutral Remote Host ADR](decisions/ADR-0012-PROVIDER_NEUTRAL_REMOTE_HOST_BOUNDARY.md).
10. [A-08 Reusable Approval Track](product/A08_REUSABLE_APPROVAL_TRACK.md).
11. [Generic Project Continuity Prompt](../PROJECT_CONTINUITY_PROMPT.md) — paste into a fresh AI conversation if context is lost.

## Canonical architecture/security reading order

1. [Product Charter](product/PRODUCT_CHARTER.md)
2. [Scope & Roadmap](product/SCOPE_AND_ROADMAP.md)
3. [System Architecture](architecture/SYSTEM_ARCHITECTURE.md)
4. [AI-Native Safety Architecture](architecture/AI_NATIVE_SAFETY_ARCHITECTURE.md)
5. [Multi-session & Concurrency](architecture/MULTI_SESSION_CONCURRENCY.md)
6. [Threat Model](security/THREAT_MODEL.md)
7. [Security Invariants](security/SECURITY_INVARIANTS.md)
8. [Policy & Capabilities](security/POLICY_CAPABILITIES.md)
9. [MCP Tool Contracts](specs/MCP_TOOL_CONTRACTS.md)
10. [Process Runtime](architecture/PROCESS_RUNTIME.md)
11. [Memory & Output](architecture/MEMORY_OUTPUT.md)
12. [Implementation Plan](engineering/IMPLEMENTATION_PLAN.md)
13. [Test & CI Strategy](engineering/TEST_AND_CI_STRATEGY.md)
14. [ChatGPT Desktop Quick Install](operations/CHATGPT_DESKTOP_QUICK_INSTALL.md)
15. [Windows Manual Smoke Test](operations/WINDOWS_MANUAL_SMOKE.md)

Repository-level living documents: [ROADMAP](../ROADMAP.md), [STATUS](../STATUS.md), [CHANGELOG](../CHANGELOG.md), [SECURITY](../SECURITY.md), [MAINTENANCE](../MAINTENANCE.md), [AI_HANDOFF](../AI_HANDOFF.md), [PROJECT_CONTINUITY_PROMPT](../PROJECT_CONTINUITY_PROMPT.md).

## Current evidence split

Use the documents according to their role:

- `CURRENT_STATE_2026-10-12.md` — concise “where we are now”.
- `PROJECT_HEALTH_2026-10-12.md` — engineering audit: done / remaining / missing / blocked / next coding candidates.
- `FRANCESTUDENT_REMOTE_MCP_CANDIDATE.md` — host-specific evidence and unknowns.
- `REAL_OPTIC_MCP_LINK_EXPERIMENT.md` — real Optic synthetic integration proof and non-claims.
- `REMOTE_AUTONOMY_AUDIT_GATES.md` — security conditions that block real-project remote exposure.
- `RESEARCH_LEDGER.md` — accepted vs rejected research claims.

## Current documentation rule

Do not create a second normative document for a subject already owned here. Update its canonical owner and link to it.

The current-state/project-health documents are intentionally **not** replacements for the long historical `STATUS.md`; they are operational front doors while the historical ledger remains intact.

Status vocabulary:

- **DECIDED** — accepted architecture.
- **PROPOSED** — candidate awaiting ADR/PoC.
- **TARGET** — measurable objective, not a result.
- **RESEARCH** — sourced information awaiting implementation validation.
- **OPEN** — unresolved question.
- **EVIDENCE-PENDING** — implementation/harness exists but the required real-world measurement is still missing.
- **BLOCKED BY HOST** — missing external-host availability/evidence; not a failed implementation.

## Structure

- `product/` — why, scope, roadmap, current product state, project health and autonomy track.
- `architecture/` — boundaries, sessions, processes, transport, memory and recovery.
- `security/` — threats, policy, invariants, filesystem/Windows containment and remote/autonomy audit gates.
- `specs/` — stable external contracts.
- `engineering/` — implementation, tests, dependencies, CI and reuse.
- `operations/` — install, diagnostics, benchmarks and manual smoke testing.
- `decisions/` — Architecture Decision Records (ADRs).
- `research/` — evidence, experiments and open questions.

When a host/transport/security milestone changes, update the current-state snapshot, project-health checkpoint, the owning track/ADR, relevant issue/PR and `ROADMAP.md`. Historical status material should be synchronized surgically rather than replaced wholesale.

## Audit evidence supplements

- [Repository audit](engineering/REPOSITORY_HEALTH_AUDIT_2026-10-12.md) — exact refs/CI, issue disposition and remaining documentation drift.
- [FranceStudent evidence ledger](research/FRANCESTUDENT_REAL_LINK_EVIDENCE_2026-10-12.md) — source-specific proof and missing raw provenance.
