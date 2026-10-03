# Optic AI Bridge

**Status:** pre-alpha — Phase 1 vertical slice at final native gate  
**Target:** Windows-first, Rust, local-first, lightweight MCP bridge for AI-assisted development.

> **Core rule:** The AI decides what it needs. The bridge executes. Deterministic policy authorizes. OS isolation contains.

Optic AI Bridge is intended to give ChatGPT (and other MCP-capable clients later) safe access to developer workflows such as project files, code search, Git, builds, tests, and supervised local processes—without embedding an LLM and without requiring an Electron/Node runtime for the bridge itself.

The repository started documentation-first and now contains an executable Rust implementation. Phase 1A, 1B and 1C are merged. Phase 1D adds the remaining Windows-native process-count, job-memory and descendant-tree Job Object gate and is under final review.

## Implemented Phase 1 surface

- application-owned sessions, monotonic expiry and revocation;
- independent hard transport/body/concurrency ceilings;
- bounded MCP stdio framing and responses;
- project-scoped bounded `fs_list` / `fs_read`;
- structured process lifecycle with opaque JobIds, bounded output and timeout/stop handling;
- operator-created executable/environment allowlists for process authority;
- deterministic session + exact task-lease policy before process execution;
- dedicated Windows containment crate with a narrow Win32/unsafe boundary;
- per-job Windows Job Objects with suspended create → configure → assign → resume ordering;
- Windows kill-on-close, active-process and total job-memory limits derived from the authorized `ResourceBudget`;
- native Windows gates for process count, memory, descendant-tree timeout cleanup and kill-on-close.

The implementation remains pre-alpha. Filesystem mutation, Git execution, mutation recovery, installer/tunnel integration and stronger restricted-token/AppContainer-style hardening are not complete yet. See [`STATUS.md`](STATUS.md) for the precise implementation state.

## Canonical documentation

Start with [`docs/README.md`](docs/README.md).

The canonical documents are grouped by purpose:

- `docs/product/` — product charter, scope and roadmap.
- `docs/architecture/` — system, sessions, concurrency, transport, memory and recovery.
- `docs/security/` — threat model, policy, Windows isolation and filesystem safety.
- `docs/specs/` — MCP tool contracts, config and errors.
- `docs/engineering/` — implementation, tests, CI/security gates, dependencies and OpticCode reuse.
- `docs/operations/` — install, observability and benchmarks.
- `docs/decisions/` — Architecture Decision Records (ADRs).
- `docs/research/` — source register, research ledger and open questions.

`AI_HANDOFF.md` is the mandatory guardrail for AI coding agents.

## Current V1 direction

V1 stays deliberately narrow: Rust daemon/core, MCP adapter, deterministic local policy, project-scoped filesystem/search/edit primitives, Git primitives, supervised process execution, bounded output, crash cleanup, and strong session isolation.

Multi-session support is a first-class architecture requirement. Two chats must be able to work on different projects safely; parallel work on the same repository must use isolated Git worktrees and explicit integration/conflict gates rather than uncontrolled shared writes.

## Status labels used in the docs

- **DECIDED** — accepted architecture direction.
- **PROPOSED** — strong candidate, still needs an ADR/PoC.
- **TARGET** — benchmark or resource objective; not a measured fact.
- **RESEARCH** — externally sourced information to validate during implementation.
- **OPEN** — unresolved question.

Research baseline: **2026-10-03**.
