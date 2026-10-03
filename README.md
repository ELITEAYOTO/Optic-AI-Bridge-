# Optic AI Bridge

**Status:** pre-alpha — Phase 2C2 recovery gate implemented on PR #18, final review in progress  
**Target:** Windows-first, Rust, local-first, lightweight MCP bridge for AI-assisted development.

> **Core rule:** The AI decides what it needs. The bridge executes. Deterministic policy authorizes. OS isolation contains.

Optic AI Bridge is intended to give ChatGPT (and other MCP-capable clients later) safe access to developer workflows such as project files, code search, Git, builds, tests, and supervised local processes—without embedding an LLM and without requiring an Electron/Node runtime for the bridge itself.

The repository started documentation-first and now contains an executable Rust implementation. Phase 1A through 1D, Phase 2A, Phase 2B and Phase 2C1 are merged. Phase 2C2 is implemented on PR #18 and is undergoing its final full CI/documentation gate. Public durable file mutation is still intentionally not exposed.

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

### Phase 2C1 — bounded recovery journal

Merged in PR #15:

- operator-controlled recovery state outside the project workspace;
- one opaque ActionId per operation;
- `prepared → committing → verified|ambiguous` journal transitions;
- `sync_all` on complete journal states;
- 64 KiB default per-journal ceiling and 256-entry startup recovery ceiling;
- deterministic reconciliation to exact intended state, exact prior state, or fail-closed unresolved conflict;
- torn trailing transition handling without accepting a corrupt initial record.

### Phase 2C2 — journal-wrapped commit / crash recovery

Implemented on PR #18, with public MCP mutation still disabled:

- one ActionId binds the recovery journal and deterministic same-directory staging artifact;
- `prepared` and then `committing` are durable before the Phase 2B atomic mutation service is invoked;
- verified/ambiguous terminal semantics preserve post-commit uncertainty rather than turning it into a blind retry;
- production recovery preserves journal evidence until surviving staging is validated and cleaned;
- staging cleanup requires a regular file, bounded size and exact BLAKE3 match to the journaled intended content;
- native Windows child-process tests terminate after durable prepared, after durable committing, after atomic-service return before terminal state, and after terminal state before retirement, then prove deterministic restart reconciliation;
- tampered staging fails closed and retains journal evidence.

The crash model is deliberately narrow: the post-commit hook is after the atomic service returns (effect + its verification), not between the raw namespace syscall and verification. The project does **not** claim sudden-power-loss ACID durability.

The next gate is Phase 2C3 transactional write/patch/delete runtime services with policy and negative recovery tests. MCP mutation tools remain disabled until that gate passes.

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
