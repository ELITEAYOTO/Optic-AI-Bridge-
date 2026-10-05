# Scope and Roadmap

Status: LIVING DOCUMENT. Last reviewed: 2026-10-05.

## Phase 0 — Architecture freeze (COMPLETED BASELINE)

- Cargo workspace and stable internal interfaces.
- Explicit application SessionHandle independent of MCP transport sessions.
- ActionEnvelope as the single execution authorization boundary.
- Security invariants mapped to tests.
- TransportGuard hard message/body/concurrency limits.
- Capability/task lease model.
- Source/sink containment model.
- Maintenance, status, changelog and release-security lifecycle.

Baseline merged through PR #1 on 2026-10-03.

## Phase 0.1 — Contract hardening (COMPLETED)

- Typed `Effect` variants instead of independently combinable action-kind/target fields.
- Explicit file `ExpectedState` and Git target-head preconditions.
- Scope-bearing task leases for workspace/repository/executable/network boundaries.
- Segment-aware workspace scope checks.
- Dual session + task-lease authorization for process network access.
- Monotonic runtime authorization deadlines.
- Regression tests for scope/network failures.
- Research pass on filesystem identity, handle-first I/O, stale-context invalidation, retry/idempotency and Windows isolation profiles.

Gate passed on 2026-10-03: formatting, Clippy, tests and dependency-policy checks were green on Linux/Windows before PR #2 was squash-merged into `main`.

## Phase 1 — Vertical slice (COMPLETED)

Implemented and merged through PR #7:

- TransportGuard-owned request/frame/body/concurrency/response/time limits.
- MCP stdio adapter using maintained RMCP behind Optic-owned limits.
- Explicit application SessionHandle mapping and session registry/revocation lifecycle.
- Real monotonic Clock adapter for active session/task deadlines.
- Bounded `fs_list` / `fs_read` path through normalization and policy.
- Structured `process_start/read/stop/result` with opaque JobIds.
- Operator-owned executable/environment allowlists.
- Windows Job Object lifecycle/resource enforcement with kill-on-close, process-count and job-memory limits.
- Native Windows regression tests for process-tree cleanup and resource ceilings.

Phase 1 gate passed. Job Objects are containment, not a complete sandbox.

## Phase 2 — Safe mutation and Git (COMPLETED)

Phase 2 is intentionally split into narrow security gates.

### Phase 2A — mutation observation/preconditions (COMPLETED)

Merged in PR #9 (`71bdf082`).

- bounded streaming BLAKE3 content observation;
- explicit `ExpectedState::{Absent, Content}` checks;
- canonical target/workspace containment;
- leaf-symlink denial and stale-state conflicts.

### Phase 2B — Windows mutation containment/atomic commit (COMPLETED)

Merged in PR #13 (`80f3aa9b`).

- no-reparse handles and final-path validation;
- `FILE_ID_INFO` identity binding;
- expected-state + identity revalidation around staging/commit;
- same-directory create-new staging;
- `ReplaceFileW` for existing targets and create-only hard-link semantics for absent targets;
- explicit committed/verified result semantics.

Residual boundary remains documented: existing-target replacement is not a kernel compare-and-swap against arbitrary external writers.

### Phase 2C — durable transactional file mutation (COMPLETED)

Merged through PR #30 (`624e88da`).

- bounded append-only mutation recovery journal outside the workspace;
- process-crash/restart recovery gates;
- transactional whole-file write, deterministic byte-range patch and handle-based delete;
- application-owned `FileWrite` / `FileDelete` task leases with exact workspace scope;
- conditional MCP `fs_write`, `fs_apply_patch`, `fs_delete` exposure only when matching authority exists;
- server-generated ActionIds and startup recovery-before-serve;
- cancellation-safe durable mutation handling that does not report timeout while a non-cancellable filesystem effect may still commit.

This proves the documented process-termination/restart model, not sudden-power-loss ACID durability.

### Phase 2D — Git read/integration (COMPLETED)

#### Phase 2D1 — bounded Git read runtime (COMPLETED)

Merged in PR #32 (`1cb3cc03`).

- exact canonical repository-root binding;
- absolute/canonical Git executable;
- bounded status, literal-path/staged diff and paginated log;
- prompt/pager/external diff/textconv/fsmonitor/untracked-cache constraints;
- pinned/reachable log cursors and hard command deadlines.

#### Phase 2D2 — operator-owned MCP Git read (COMPLETED)

Merged in PR #33 (`aee4f168`).

- `GitRead` exists only when the operator supplies `--git-executable`;
- conditional MCP `git_status`, `git_diff`, `git_log`;
- callers cannot select repository roots, executable paths, raw argv or authority IDs;
- exact raw status/diff bytes are returned as base64 within response ceilings.

The first real Windows/ChatGPT Desktop integration smoke passed on 2026-10-04 against this Phase 2D2 surface. It proved MCP stdio initialization, file reads, Git status/diff/log, scoped transactional write/patch/delete, stale-version rejection, scope denial, unallowlisted-process denial and clean recovery retirement on a disposable repository. This is machine/integration validation only, not a production-readiness claim.

#### Phase 2D3 — exact-head Git integration (COMPLETED)

Phase 2D3 is intentionally split so no Git mutation MCP surface appears before each lower boundary is proven.

**2D3A runtime foundation (COMPLETED — PR #40, `22b34149`):**

- `GitIntegrate` effects bind exact `source_head` and `expected_target_head` commit ids;
- the first primitive is fast-forward-only and targets only a direct operator-owned ref under `refs/optic/integration/`;
- symbolic refs and ordinary user branch refs are rejected;
- preparation uses an ActionId-owned detached/locked `--no-checkout` worktree outside the repository checkout;
- the worktree must validate and clean up before the target can move;
- the caller workspace is not checked out/reset/updated;
- exact target state is revalidated immediately before an atomic `update-ref --no-deref <new> <expected>` old-value comparison;
- Git prompting/pagers/system/global config/replacement refs are disabled and the forced-empty hooks directory is revalidated before mutation-capable calls;
- stale target, divergent history, invalid source, path collision, overlap and hook/ref redirection cases have negative regression coverage.

**2D3B1 internal authorization (COMPLETED — PR #41, `f3d0898f`):**

- application-owned authority primitive mints no lease by default and, when enabled, exactly one `GitIntegrate` + `LeaseScope::Repository` task lease;
- a transport-agnostic authorized integration service resolves active session + exact active lease and applies `PolicyEngine` before 2D3A runtime execution;
- missing lease, wrong scope, missing session capability and cross-session authority fail before target update;
- B1 adds neither startup flags nor MCP Git mutation tools.

**2D3B2 bounded recovery (COMPLETED — PR #42, `b74d563e`):**

- bounded `worktree list --porcelain -z` snapshot plus bounded integration-root scan, with Optic-owned candidates separately capped by the concurrent-request ceiling;
- no global prune; user worktrees outside the Optic root are untouched;
- only direct ActionId-named, registered, locked, non-symlink and canonically contained worktrees are cleanup candidates;
- the full snapshot must validate before any removal and each candidate is revalidated immediately before removal;
- locked worktrees are removed without an unlock gap using double `--force`;
- process-termination recovery coverage proves an orphan left before ref mutation can be removed without moving the target;
- exact final head `0ff95b82` passed dependency policy plus Ubuntu/Windows CI.

**2D3B3 startup/operator authority (COMPLETED — PR #43, `73ade6e3`):**

- integration recovery/runtime configuration is a complete tuple of separate `--git-integration-executable`, absolute integration root and operator-owned internal ref;
- that tuple without `--allow-git-integrate` is recovery-only and grants no integration capability;
- `--allow-git-integrate` explicitly provisions the application-owned repository-scoped `GitIntegrate` lease and session capability after recovery succeeds;
- `--git-executable` remains the independent Git-read opt-in, so integration/recovery does not silently grant `GitRead`;
- runtime/authority mismatch fails MCP server construction and B3 itself registers no `git_integrate` tool;
- exact final head `c82059eb` passed dependency policy plus Ubuntu/Windows CI.

**2D3C thin MCP integration adapter (COMPLETED — PR #44, `b9766e4a`):**

- conditionally exposes `git_integrate(source_head, expected_target_head)` only with application-owned `GitIntegrate` authority;
- rejects unknown public fields and exposes no repository/ref/path/executable/raw-argv/lease/ActionId authority;
- generates ActionId and resolves the repository lease server-side, then calls only the authorized runtime;
- preserves exact stale-target and non-fast-forward failures; post-ref-update verification uncertainty is explicit and never a safe-retry response;
- proves Git integration authority can exist without exposing Git read tools;
- exact final head `55c4f3f2` passed dependency policy plus Ubuntu/Windows CI.

**2D3 closure smoke (COMPLETED — PR #45, `99837439`):**

- native Windows CI builds the real `optic-bridge.exe`, starts it over stdio MCP with integration-only authority, performs an exact-head fast-forward and proves reuse of the stale precondition is rejected without moving the target;
- exact final head `fa760213` passed Ubuntu/Windows/dependency policy and the Windows binary smoke.

**2D3D integration-target observation (COMPLETED — PR #46, `c44ba567`):**

- conditional `git_integration_status()` returns only the exact current target commit under existing repository-scoped `GitIntegrate` authority;
- the observation is typed read-only and does not grant or depend on generic `GitRead`;
- clients use the returned commit as the explicit `expected_target_head`, preserving exact-head optimistic concurrency across reconnects without exposing the internal ref;
- exact final head `f4d04c3` passed dependency policy, Ubuntu/Windows CI and the native real-binary status/integrate/status/stale smoke.

**ChatGPT integration packaging (COMPLETED — PR #47, `1e42b1ff`):**

- keeps the default profile unchanged and Git integration off by default;
- requires explicit `-EnableGitIntegration` on an exact repository root with `HEAD`;
- generates only the fixed internal-ref/root/executable authority and allowlists `git_integration_status` + prompt-gated `git_integrate`;
- Windows CI proves default denial, incompatible read-only mode, symbolic-ref rejection, create-only ref bootstrap, existing-ref preservation, generated tool/approval config, doctor startup, guarded install-root ownership and explicit uninstall cleanup without touching a real ChatGPT profile;
- exact final head `4ff720b2` passed dependency policy, Ubuntu/Windows CI, native real-binary integration smoke and the installer-profile smoke.

**Final Phase 2D3 desktop gate (COMPLETED — 2026-10-05):**

- real ChatGPT Desktop loaded the installed opt-in profile from a disposable repository;
- `git_integration_status` observed the exact initial internal target `a76941d0545ce0b1a4359e808feb31d6a97362a0`;
- prompt-gated `git_integrate` fast-forwarded to descendant `2e7462c0150858d18b31fc9b46cc26ea1a25304f`;
- a second status observed the new target;
- reuse of the stale old target was rejected with `optic.precondition_failed`;
- independent Git verification proved caller `main`/files were unchanged, no operation worktree remained, and recovery state was empty.

Non-fast-forward merge production semantics remain deliberately deferred; Phase 2D3 proves the fast-forward exact-head boundary and recovery model.

## Phase 3 — Multi-session runtime (CURRENT)

Independent sessions/projects, per-session jobs/resources/capabilities, with adversarial cross-session tests.

Evaluate a bounded ActionId idempotency/replay ledger once stateful retries justify it.

## Phase 4 — Same-repository parallelism

Git worktrees, deterministic coordinator and integration/conflict gates for concurrent same-repository sessions.

Event-driven invalidation may be evaluated only as an optimization on top of mandatory commit-time revalidation.

## Phase 5 — Connectivity/install

Status: future phase, with developer-preview groundwork already pulled forward.

Already implemented ahead of this phase:

- user-scoped Windows PowerShell install/update path;
- local ChatGPT Desktop compatibility plugin packaging;
- MCP handshake/tool doctor self-test;
- uninstaller;
- tag-driven Windows prerelease bundle + SHA-256 workflow.

Still part of the later Phase 5 gate:

- first tagged/public prerelease and repeatable clean-machine timing validation;
- broader autoconfiguration/upgrade compatibility as the host evolves;
- additional transport/tunnel integration only if required.

## Phase 6 — Hardening

Restricted-token/AppContainer/LPAC compatibility experiments, fuzz/property tests, long soak and signed release pipeline.

## Reject unless evidence changes

Embedded AI security reviewer as authorization dependency; generic shell as core primitive; probabilistic “risk score” controlling permissions; microservice/plugin-per-tool architecture; Electron dashboard; unbounded configurable limits; automatic trust of MCP tool annotations; filesystem watcher/USN silence as freshness authority; stale protocol cache as mutation authority; broad remote-desktop scope.

## Maintenance rule

A phase advances only when acceptance and security gates pass. Every roadmap movement updates `STATUS.md`. User-visible/relevant engineering changes update `CHANGELOG.md`. Changed invariants require tests and usually an ADR.
