# Optic AI Bridge

**Status:** pre-alpha — Phase 2B merged, Phase 2C recovery in progress  
**Target:** Windows-first, Rust, local-first, lightweight MCP bridge for AI-assisted development.

> **Core rule:** The AI decides what it needs. The bridge executes. Deterministic policy authorizes. OS isolation contains.

Optic AI Bridge is intended to give ChatGPT (and other MCP-capable clients later) safe access to developer workflows such as project files, code search, Git, builds, tests, and supervised local processes—without embedding an LLM and without requiring an Electron/Node runtime for the bridge itself.

The repository started documentation-first and now contains an executable Rust implementation. Phase 1A through 1D, Phase 2A and Phase 2B are merged. The current baseline includes bounded MCP filesystem reads, structured lease-gated processes, native Windows Job Object containment, bounded mutation observation, and a Windows handle/identity/revalidation boundary for atomic namespace commits. Public durable file mutation is still intentionally not exposed.

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
- native Windows gates for process count, memory, descendant-tree timeout cleanup, kill-on-close and fail-closed containment unwrap.

## Implemented Phase 2 foundations

### Phase 2A — observation / preconditions

- streaming BLAKE3 content observation without loading whole targets into RAM;
- hard mutation-observation byte ceiling through `HardLimits::max_fs_mutation_bytes`;
- canonical mutation target observation for existing and absent targets;
- exact `ExpectedState::Absent` / `ExpectedState::Content(version)` conflict checks;
- leaf-symlink rejection and parent-canonicalization/workspace containment checks.

### Phase 2B — Windows commit containment

- no-reparse file/directory handles and final-path containment validation;
- exact Windows identity through `FILE_ID_INFO`;
- expected-state + identity revalidation before staging and immediately before commit;
- same-directory create-new staging with `sync_all`;
- existing-target `ReplaceFileW` and absent-target create-only hard-link semantics;
- explicit verified vs committed-but-unverified result semantics;
- native Windows tests for same-content delete/recreate detection and parent-directory replacement.

This boundary intentionally does not claim kernel compare-and-swap semantics: the final `ReplaceFileW` call remains path-based and a small external-writer window is documented.

### Phase 2C1 — recovery journal under review

PR #15 adds a bounded recovery-journal state machine before the journal is allowed to control real mutations:

- operator-controlled recovery state outside the project workspace;
- one opaque ActionId per operation;
- `prepared → committing → verified|ambiguous` journal transitions;
- `sync_all` on complete journal states;
- 64 KiB default per-journal ceiling and 256-entry startup recovery ceiling;
- deterministic reconciliation to exact intended state, exact prior state, or fail-closed unresolved conflict;
- torn trailing transition handling without accepting a corrupt initial record.

The clean 2C1 code head passes Linux/Windows CI and dependency policy. The next gate wires this journal around the real Phase 2B Windows commit and runs forced child-process crashes at each transition boundary. Only after that will transactional write/patch/delete runtime services be considered. MCP mutation tools remain disabled throughout these gates.

The implementation remains pre-alpha. Installer/tunnel integration, public mutation/Git tools, multi-session orchestration and stronger restricted-token/AppContainer-style hardening are not complete yet. See [`STATUS.md`](STATUS.md) for the precise implementation state.

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
