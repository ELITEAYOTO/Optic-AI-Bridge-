# Documentation Map

This directory is the canonical engineering knowledge base for Optic AI Bridge.

## Reading order

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
16. [AI Handoff](../AI_HANDOFF.md)

Repository-level living documents: [ROADMAP](../ROADMAP.md), [STATUS](../STATUS.md), [CHANGELOG](../CHANGELOG.md), [SECURITY](../SECURITY.md), [MAINTENANCE](../MAINTENANCE.md).

## Canonical rule

Do not create a second normative document for a subject already owned here. Update its canonical owner and link to it.

Status vocabulary:
- DECIDED: accepted architecture.
- PROPOSED: candidate awaiting ADR/PoC.
- TARGET: measurable objective, not a result.
- RESEARCH: sourced information awaiting implementation validation.
- OPEN: unresolved question.

## Structure

product = why/scope/roadmap; architecture = boundaries/sessions/processes/transport/memory/recovery; security = threats/policy/invariants/filesystem/Windows; specs = stable external contracts; engineering = implementation/tests/dependencies/CI/reuse; operations = install/diagnostics/benchmarks/manual smoke testing; decisions = ADRs; research = evidence/open questions.
