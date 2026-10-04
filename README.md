# Optic AI Bridge

**Status:** pre-alpha — Phase 2D2 operator-owned MCP Git read merged through PR #33; Phase 2D3 Git integration current
**Target:** Windows-first, Rust, local-first, lightweight MCP bridge for AI-assisted development.

> **Core rule:** The AI decides what it needs. The bridge executes. Deterministic policy authorizes. OS isolation contains.

Optic AI Bridge is intended to give ChatGPT (and other MCP-capable clients later) safe access to developer workflows such as project files, code search, Git, builds, tests, and supervised local processes—without embedding an LLM and without requiring an Electron/Node runtime for the bridge itself.

The repository started documentation-first and now contains an executable Rust implementation. Phase 1A through 1D, Phase 2A, Phase 2B, Phase 2C1, Phase 2C2, Phase 2C3A, Phase 2C3B1 and Phase 2C3B2 are merged. Phase 2C3C is now merged as well: PR #26 added the transport-agnostic authorized mutation service as `ac381002`; PR #27 bound the authorized `ActionEnvelope.action_id` to the durable journal/staging/recovery operation as `1b5a3393`; PR #29 provisioned application/operator-owned `FileWrite` / `FileDelete` capabilities and exact workspace-scoped task leases as `75477c3b`; and PR #30 exposed the thin conditional MCP mutation adapter as `624e88da` after exact final head `f70e520b` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`. Durable MCP file mutation is therefore available only when the operator explicitly provisions corresponding authority and a recovery state directory; without that authority the historical read/process tool surface remains unchanged. Phase 2D1 bounded Git read runtime is merged as `1cb3cc03` (validated head `ed3b5c43`), and Phase 2D2 operator-owned MCP `git_status` / `git_diff` / `git_log` is merged as `aee4f168` (validated head `54951225`). Phase 2D3 exact-head Git integration and worktree/conflict isolation is current.

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

Merged in PR #15 (`85aec4c6`):

- operator-controlled recovery state outside the project workspace;
- one opaque ActionId per operation;
- `prepared → committing → verified|ambiguous` journal transitions;
- `sync_all` on complete journal states;
- 64 KiB default per-journal ceiling and 256-entry startup recovery ceiling;
- deterministic reconciliation to exact intended state, exact prior state, or fail-closed unresolved conflict;
- torn trailing transition handling without accepting a corrupt initial record.

### Phase 2C2 — journal-wrapped commit / crash recovery

Merged in PR #18 (`0297406c`), with public MCP mutation still disabled:

- one ActionId binds the recovery journal and deterministic same-directory staging artifact;
- `prepared` and then `committing` are durable before the Phase 2B atomic mutation service is invoked;
- verified/ambiguous terminal semantics preserve post-commit uncertainty rather than turning it into a blind retry;
- production recovery preserves journal evidence until surviving staging is validated and cleaned;
- staging cleanup requires a regular file, bounded size and exact BLAKE3 match to the journaled intended content;
- native Windows child-process tests terminate after durable prepared, after durable committing, after atomic-service return before terminal state, and after terminal state before retirement, then prove deterministic restart reconciliation;
- tampered staging fails closed and retains journal evidence;
- exact pre-merge head `fdfc3ace` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

The crash model is deliberately narrow: the post-commit hook is after the atomic service returns (effect + its verification), not between the raw namespace syscall and verification. The project does **not** claim sudden-power-loss ACID durability.

### Phase 2C3 — transactional runtime services

Phase 2C3A whole-file write + deterministic byte patch is merged. Phase 2C3B1 generalized recovery intent to exact `ExpectedState::{Absent, Content}` with strict v1 journal compatibility. Phase 2C3B2 merged in PR #24 (`b346a7d9`):

- `TransactionalFileService::delete` requires the exact existing `ContentVersion`; there is no blind-delete/absent-plan input;
- Windows opens a dedicated no-reparse read+DELETE handle, captures final path and `FILE_ID_INFO`, re-hashes the exact target on that same handle and performs `FileDispositionInfo` on that handle rather than deleting by path;
- stale content and same-content delete/recreate identity changes fail closed;
- delete uses the existing durable journal with `intended = ExpectedState::Absent`, no write-staging artifact, and recovery-required semantics after durable `committing`;
- native Windows child-process gates cover durable prepared, durable committing-before-delete, post-delete-service return and terminal-before-retirement, with deterministic restart reconciliation and an empty second recovery;
- non-Windows durable delete remains fail-closed as unsupported;
- exact final head `e74cc0cf` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` before squash merge.

### Phase 2C3C — authorization / adapter gate

Phase 2C3C is merged through PRs #26, #27, #29 and #30:

- `AuthorizedFileMutationService` is a transport-agnostic runtime boundary that resolves the active application session and exact active task lease, evaluates the normalized `ActionEnvelope` through `PolicyEngine`, rejects primitive/effect mismatch, and only then reaches `TransactionalFileService`;
- write and patch require `FileWrite`; delete requires the distinct `FileDelete` capability; workspace scope, session ownership, policy epoch, resource budget and lease state remain deterministic policy inputs;
- negative tests cover missing lease, cross-session lease, missing capability, stale policy epoch, scope escape, effect mismatch, stale expected state, mutation byte ceiling and symlink containment;
- the already-normalized `ActionEnvelope.action_id` is reused as the durable transaction identity: journal key, deterministic write-staging owner, commit result ID and recovery operation key;
- an already-active journal/temp slot for the same `ActionId` fails closed instead of being reused; the general replay/idempotency ledger remains a later Phase 3 concern;
- PR #29 (`75477c3b`) provisions mutation authority only from application/operator-owned startup configuration. Write and delete receive distinct task leases containing only explicitly configured `WorkspaceAll` / structural `WorkspacePrefix` scopes, and the session receives each capability only when corresponding authority exists;
- PR #30 (`624e88da`) conditionally registers `fs_write` / `fs_apply_patch` only for provisioned `FileWrite` authority and `fs_delete` only for provisioned `FileDelete` authority; callers cannot supply `TaskLeaseId` or `ActionId`;
- `--mutation-state-dir` is required when mutation authority is configured and may also be supplied alone for recovery-only startup; bounded deterministic recovery runs before MCP is served and unresolved recovery fails startup closed;
- started blocking durable mutations are intentionally followed to a known transaction/recovery outcome while retaining their transport execution permit. The adapter does not return a timeout while a non-cancellable `spawn_blocking` filesystem effect may still commit in the background;
- exact final heads `fcd1a6aa` (#26), `ae05d6a2` (#27) and `f70e520b` (#30) passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.

The implementation remains pre-alpha. Phase 2D Git read/integration is the current implementation tranche. Installer/tunnel integration, multi-session public orchestration and stronger restricted-token/AppContainer-style hardening are not complete yet. See [`STATUS.md`](STATUS.md) for the precise implementation state.

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
