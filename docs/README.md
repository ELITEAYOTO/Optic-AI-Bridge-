# Documentation Map

This directory is the canonical engineering knowledge base for Optic AI Bridge.

## Reading order

1. [Product Charter](product/PRODUCT_CHARTER.md)
2. [System Architecture](architecture/SYSTEM_ARCHITECTURE.md)
3. [Multi-session & Concurrency](architecture/MULTI_SESSION_CONCURRENCY.md)
4. [Threat Model](security/THREAT_MODEL.md)
5. [Policy & Capabilities](security/POLICY_CAPABILITIES.md)
6. [MCP Tool Contracts](specs/MCP_TOOL_CONTRACTS.md)
7. [Process Runtime](architecture/PROCESS_RUNTIME.md)
8. [Memory & Output](architecture/MEMORY_OUTPUT.md)
9. [Implementation Plan](engineering/IMPLEMENTATION_PLAN.md)
10. [AI Handoff](../AI_HANDOFF.md)

## Canonical rule

Do not create a second document for a subject already owned here. Update its canonical owner instead.

Status vocabulary:
- DECIDED: accepted architecture.
- PROPOSED: candidate awaiting ADR/PoC.
- TARGET: measurable objective, not a result.
- RESEARCH: sourced information awaiting implementation validation.
- OPEN: unresolved question.

## Structure

- product: why, scope, non-goals, roadmap.
- architecture: boundaries, sessions, processes, transport, memory, recovery.
- security: threats, policy/capabilities, filesystem safety, Windows isolation.
- specs: stable external contracts.
- engineering: implementation, tests, dependencies, CI, reuse.
- operations: install, diagnostics, benchmarks.
- decisions: ADRs.
- research: sources, evidence and unresolved questions.
