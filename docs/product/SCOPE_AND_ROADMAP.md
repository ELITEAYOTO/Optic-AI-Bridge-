# Scope and Roadmap

Status: LIVING DOCUMENT. Last reviewed: 2026-10-08.

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

Independent sessions/projects, per-session jobs/resources/capabilities, with adversarial cross-session and lifecycle-race tests before orchestration expands.

### Phase 3A — bounded shared-runtime isolation (COMPLETED)

Merged in PR #50 (`661c1604`, exact green head `3c0c4448`). The first executable gate keeps bridge-wide ceilings while adding smaller per-session ceilings. The session registry has a hard capacity. Process execution keeps opaque session-owned `JobId`s and enforces both global and per-session active-job, retained-record and reserved-output-RAM limits. Under record/output pressure a start request may retire only terminal history owned by the requesting session; another session's retained process result is never evicted to make room. Adversarial A/B tests prove a session exhausting its own active-job or output reservation does not consume the other session's corresponding quota, and owner-only record eviction preserves the other session's result.

### Phase 3B1 - application-owned lifecycle and coordinated revoke (COMPLETED)

Merged in PR #51 (`78ae66f`, exact green head `db20e4f`). `SessionLifecycleManager` is the internal lifecycle boundary for future multi-session orchestration. It generates opaque application-owned `SessionHandle`s from `SessionGrantSpec`, rejects already-expired provisioning, invalidates the session before revoking its task leases, and requests termination of all owned process jobs. Existing MCP `session_cancel` delegates to this lifecycle boundary and no public MCP session-creation tool exists.

### Phase 3B2 - admission/revocation serialization and safe reap (COMPLETED)

Merged in PR #52 (`2748f688`, exact green head `c44c89ec`). Sensitive effect admission is serialized with revoke through `SessionAdmissionPermit`; revoke blocks new admissions, drains already-admitted effects, then revokes leases/cancels owned jobs. Physical reap is owner-scoped and allowed only when no admission/job remains active.

### Phase 3B3 - lifecycle closure and bounded leases (COMPLETED)

Implemented and code/test-validated on exact head `5d39f5b`:

- `TaskLeaseRegistry` has hard global and per-session capacity; zero is never unlimited;
- revoked leases keep their storage slot until owner-scoped physical reap, so revocation cannot masquerade as reclamation;
- partial unpublished authority provisioning removes its rollback lease rather than leaking bounded registry capacity;
- application startup uses `SessionLifecycleManager::provision()` so the initial `SessionHandle` is server-generated through the canonical lifecycle path;
- an internal five-second supervisor scans only the bounded inactive-session set and reuses the existing admission/revoke/quiescent-reap boundary;
- no public MCP `session_create`, arbitrary orchestration, or renewal protocol is added.

PR #53 merged as `25a40e7` from exact green head `2337651`; post-merge `main` CI run #260 passed Ubuntu, Windows and dependency policy.

### Phase 3C - process/resource safety before wider autonomy

**3C1 bounded termination (COMPLETED).** PR #55 merged as `bacac76` from exact green head `bfb38a5`; post-merge `main` CI run #265 passed Ubuntu, Windows and dependency policy. Stop/timeout/output-overflow and process-observation failures request owned-tree termination and wait within a separate two-second confirmation bound. If process-tree death cannot be proven, the record becomes `termination_uncertain`: output is marked truncated, the record retains ownership/quota, and physical session reap/terminal-history eviction remains blocked. This is a quarantine state, not proof that the process is alive or dead.

**3C2 bounded CPU governance (COMPLETED).** PR #57 merged as `fda0f24` from exact green final head `444dab2`; final exact-head CI #270 and post-merge `main` CI #271 passed Ubuntu, Windows and dependency policy. Optic reserves a fixed operator-owned CPU share of 25% per active process job, with 75% aggregate and 50% per-session admission ceilings. On Windows the whole owned process tree is additionally hard-capped with Job Object CPU rate control before resume. Uncertain termination retains CPU reservation; retained proven-terminal history does not. The MCP caller cannot request a larger CPU share, and unsupported/failing Windows hard-cap setup fails closed.

**3C3A pinned executable identity (COMPLETED).** PR #59 merged as `050a4244` from exact green final head `85a7d92c`; exact-head CI #282 and post-merge `main` CI #283 passed Ubuntu, Windows and dependency policy, including native Windows pin/launch coverage and the real MCP smoke. Absolute `ProcessExecutable` task-lease scopes are bound at registration to a canonical, byte-bounded BLAKE3 content identity. On Windows the lease record retains a read handle with read sharing only, preventing concurrent write/delete/rename replacement until physical lease removal while still allowing normal executable launch. Active lease resolution revalidates the identity before process authorization. Non-Windows currently relies on bounded content revalidation and does not claim the same persistent-handle guarantee. No new MCP authority field, network containment or sandbox is added by 3C3A.

**3C3B explicit process execution classification (COMPLETED).** PR #61 merged as `3a907f50` from exact green final head `506e1f7e`; exact-head CI #287 and post-merge `main` CI #288 passed Ubuntu, Windows and dependency policy, including the native Windows suite, real MCP smoke and installer-profile validation. Operator-owned executable authority is classified as `FixedTool`, `Interpreter` or `RepositoryCode`. The class is bound into the executable lease and policy effect; authorization requires exact canonical path + class agreement, while MCP has no class field and cannot downgrade or override the operator decision. Startup requires the classified `--allow-executable=<class>:<absolute-path>` form, rejects legacy unclassified authority, and rejects duplicate canonical executable paths. Classification is an operator assertion rather than automatic executable inspection.

**3C3C1 fail-closed high-risk execution gate (COMPLETED).** PR #63 merged as `148f4720` from exact green final head `3b63e05d`; exact-head CI #291 and post-merge `main` CI #292 passed Ubuntu, Windows and dependency policy, including native tests, real MCP smoke and installer-profile validation. Otherwise-valid `Interpreter` and `RepositoryCode` execution is denied with deterministic `ProcessIsolationRequired` until stronger isolation is implemented and proven. The MCP adapter exposes stable `optic.process_isolation_unavailable` for this case. `FixedTool` remains available under the existing bounded process runtime and Windows Job Object limits. This gate deliberately adds no restricted-token, alternate-desktop or AppContainer claim; it closes the unsafe classification-only interim state by failing closed.

**3C3C2A AppContainer isolation foundation (COMPLETED).** PR #65 merged as `75cc9c62` from exact green final head `4f5e3f7a`; exact-head CI #297 and post-merge `main` CI #298 passed Ubuntu, Windows and dependency policy, including the native AppContainer proof, real MCP smoke and installer-profile validation. The Windows-only unsafe boundary can create a fresh per-user AppContainer profile with zero capability SIDs and expose lifetime-bound `SECURITY_CAPABILITIES` for process creation. Native CI creates the proof child suspended with `STARTUPINFOEXW` / `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`, verifies `TokenIsAppContainer` before resume, and proves that an ungranted user-owned sentinel readable by a normal process is denied inside the no-capability AppContainer. This remains a kernel isolation foundation rather than a production process-path integration.

**3C3C2B1 runtime execution-class propagation (COMPLETED).** PR #67 merged as `0f369336` from exact green final head `e249e99f`; exact-head CI #301 and post-merge `main` CI #302 passed Ubuntu, Windows and dependency policy. The server-resolved `ProcessExecutionClass` now reaches `ProcessManager`, and the runtime independently rejects `Interpreter` / `RepositoryCode` with `IsolationUnavailable` before job allocation or spawn. MCP still has no class selector, and this gate does not re-admit high-risk execution.

**3C3C2B2 AppContainer explicit-stdio primitive (COMPLETED).** PR #68 merged as `1cac5076` from exact green final head `a48258c1`; exact-head CI #305 and post-merge `main` CI #306 passed Ubuntu, Windows and dependency policy, including native AppContainer tests, the real MCP smoke and installer-profile validation. The Windows-only primitive now duplicates stdin/stdout/stderr into dedicated inheritable handles, restricts inheritance to those handles with `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, applies zero-capability `SECURITY_CAPABILITIES` in the same suspended launch, exposes token verification before resume, and terminates unfinished children fail-closed. Native CI proves both the existing ungranted-file denial and observable captured stdout through the production primitive. The primitive is not yet selected by `ProcessManager` and does not grant workspace access or re-admit high-risk classes.

**3C3C2C1 internal AppContainer launcher proof (COMPLETED).** PR #70 merged as `3d660196` from exact final head `fdefc199`; exact PR CI #309 and post-merge `main` CI #310 passed Ubuntu, Windows and dependency policy, including four native tests that invoke the real helper binary, the real MCP smoke and installer-profile validation. A separate non-distributed `optic-bridge-isolation-launcher` accepts one bounded internal request, recanonicalizes executable/cwd paths, creates a fresh zero-capability AppContainer, gives the target `NUL` stdin, forwards only stdout/stderr and verifies the child token before resume. This proof itself added no MCP/policy authority or high-risk re-admission.

**3C3C2C2A runtime launcher wiring (COMPLETED).** PR #72 merged as `d31086e1` from exact final head `49ffa305`; exact PR CI #318 and post-merge `main` CI #319 passed Ubuntu, Windows and dependency policy, including a native test that routes an `Interpreter` start through the real pinned helper with logical `process_count = 1`, plus the real MCP smoke and installer-profile validation. Windows `ProcessManager` can now select a canonical pinned sibling helper for direct internal high-risk starts, runs it under the same Job Object with a +1 kernel-process allowance, preserves the logical workload process budget, sends the bounded internal request over stdin and keeps output under the existing bounded drains. At that C2A gate the app only discovered a sibling helper when present and the release bundle did not yet distribute it. Policy/MCP admission was unchanged and continued to deny `Interpreter` / `RepositoryCode`; no workspace or network authority was granted.

**3C3C2C2B1 exact-file workspace read grants (COMPLETED).** PR #74 merged as `11232b90` from exact final head `0c82f0c`; exact PR CI #330 and post-merge `main` CI #331 passed Ubuntu, Windows and dependency policy. Windows now has a revocable handle-bound read-only ACL grant for one exact non-reparse file, restricted to cryptographically ephemeral AppContainer profiles and using the Package SID already present in the token. Native CI proves denial before grant, exact-file-only read success, denial of an ungranted second file, no append/write authority, explicit revoke and Drop cleanup.

**3C3C2C2B2 exact-file grant lifecycle/wiring (COMPLETED).** PR #76 merged as `8d127a3b` from exact final head `5aaf3957`; exact PR CI #336 and post-merge `main` CI #337 passed Ubuntu, Windows and dependency policy. The isolated helper/runtime can now carry at most 32 exact-file read grants derived only from server-owned `FileRead + WorkspacePrefix` authority. `ProcessManager` resolves only exact regular files and rejects fixed-tool grants, duplicates, directories and out-of-workspace targets. The Windows grant path uses opened workspace/target handles, final-path containment and child-lifetime revoke guards. `WorkspaceAll`, write/directory grants and caller-supplied grant lists remain excluded. Current operator process leases still mint no `FileRead` workspace scopes, so public high-risk policy remains fail-closed.

**3C3C2C2C1 operator-owned exact-file process read provisioning (COMPLETED).** PR #78 merged as `70cde40f` from exact green final head `3b871bdf`; CI #347 passed Ubuntu, Windows and dependency policy. Windows startup can explicitly bind exact workspace files to an already-authorized `Interpreter` / `RepositoryCode` executable with `--allow-process-read-file`; only that matching application-owned process lease gains `FileRead + WorkspacePrefix(file)`. Fixed tools, non-Windows provisioning, unsafe/non-file/duplicate/out-of-workspace paths and caller-selected grants fail closed. MCP schemas are unchanged and policy still denies high-risk execution. Validation exposed a separate compatibility issue: ordinary `cmd.exe` / `findstr.exe` did not consume the granted temp-file ACL like the controlled Rust probe, so no broader ACL or policy claim is made.

Phase 3C3C2C3 representative toolchain characterization is completed through PR #82 (`8ea01de3`, exact green final head `915b38f4`, CI #357) and PR #83 (`b5a53b08`, exact green final head `25d2eed7`, CI #361). Node is the first complete positive profile: real helper/AppContainer startup succeeds, one exact granted file is readable and an ungranted sibling is denied. Python/Java currently stop at `STATUS_DLL_NOT_FOUND`; direct pinned Cargo starts successfully while direct rustc still has a loader dependency. These are compatibility observations only and mint no new authority. Phase 3C3C2C4 is completed through PR #85 (`bb649833`, exact head `6fcc25d1`): PR CI #365 and post-merge `main` CI #366 prove the zero-capability high-risk AppContainer path denies the tested Node TCP loopback connection and the host listener accepts no sandbox connection. This does not establish universal network containment or the direct `FixedTool` path.

**3C3C2C5A-C5C exact strong-isolation eligibility (COMPLETED).** PR #87 (`c5cafd0`), PR #88 (`d7aca659`, exact green head `02f977f5`, CI #375/#376) and PR #89 (`f5bf9e0d`, exact green head `a96fc58f`, CI #377/#378) add a separate exact `ProcessIsolationEligible { executable, class }` marker, internal fail-closed provisioning and narrow policy semantics. High-risk policy can allow only when the same active lease already has exact `ProcessExecutable` plus exact matching eligibility; wrong/missing path/class remains denied and network/resource/session/identity checks remain independent. Production startup still provisions an empty eligibility set, and neither CLI nor MCP can mint it, so public high-risk execution remains unavailable.

**3C3C2C5D packaged isolation helper (COMPLETED).** PR #90 merged as `5d3f64a7` from exact green head `815a73c8`; PR CI #380 and post-merge CI #381 passed. The release/installer now place the proven isolation launcher beside the bridge, the doctor verifies canonical sibling layout, installer-profile CI proves byte-exact install/reinstall, and owned-root uninstall removes it. Distribution remains runtime availability only and does not mint eligibility.

**3C3C2C5E selected Node operator eligibility (COMPLETED).** PR #91 merged as `1cab83f4` from exact green head `f89cd7f6`; PR CI #383 and post-merge CI #384 passed. Only exact operator-selected Windows `node.exe` already authorized as `Interpreter` can receive the production isolation marker; the sibling helper is mandatory, MCP cannot mint eligibility, eligible high-risk leases are capped to one logical process, and Python/Java/Cargo/rustc stay excluded.

**3C3C2C5F installed Node operator profile (COMPLETED).** PR #92 merged as `b4412b08` from exact green head `df6742f3`; PR CI #386 and post-merge CI #387 passed. The default installer remains process-free; explicit `-EnableIsolatedNode` emits only the existing C5E authority, exposes the four lifecycle tools with prompt-gated `process_start`, adds no process workspace-read grant, and is verified by a real installed MCP `node --version` doctor.

**3C3C2C5G real ChatGPT Desktop Node smoke (COMPLETED).** PR #93 prepared the exact CI-built Desktop validation bundle and real plugin-manager installation path (`c6bba66`); PR #94 tightened the installed process surface (`694ee3f`); PR #95 preserved hidden plugin metadata through artifact round-trip (`1f8449e`); and PR #97 fixed the real-machine Windows `TEMP`-present/`TMP`-absent compatibility edge without widening authority (`4e6f21a`, exact green head `4e27bce`, PR CI #397, post-merge CI #398). The exact #398 artifact was then installed on the target Windows machine. After a full Desktop restart, a new normal Chat surfaced explicit approval for only canonical `node.exe --version` with `network=false` and `process_count=1`; one-time approval completed through MCP `process_start/result/read` and returned Node `v22.15.1`, exit code 0. No process workspace-read grant, workspace file-tool call, mutation, network request or alternate executable was used. The proof closes the selected Node Desktop gate only; broader high-risk profiles remain future work.

**Phase 3D adversarial multi-session closure (COMPLETED).** PR #99 merged as `d9068334` from exact green head `fdccf3a4`; PR CI #401 and post-merge CI #402 passed. A single bounded A/B/C runtime scenario composes cross-session lease/JobId denial, per-session process-pressure isolation, owner-scoped revoke/reap and capacity reuse: A cannot address B's lease/job, A's local pressure does not disturb B, A revoke/reap removes only A, and C reuses reclaimed capacity while B remains active. This closes the documented internal Phase 3 adversarial gate without exposing public session minting/renewal.

**Phase 3E1 aggregate declared process-memory reservation (COMPLETED).** PR #101 merged as `097b6a37` from exact green head `b8447deb`; PR CI #406 and post-merge CI #407 passed. The existing 8 GiB per-job memory budget is now composed with 16 GiB bridge-wide and 8 GiB per-session declared-memory admission ceilings. `Running` / `TerminationUncertain` ownership retains memory reservation; proven terminal state releases it. Windows retains its independent per-job Job Object memory cap, and Job Object setup is completed before the JobRecord is published.

**Phase 3E2A Windows host-memory observation (COMPLETED).** PR #103 merged as `e2b37be0` from exact green head `3c765d9e`; PR CI #410 and post-merge CI #411 passed. The Windows platform crate now reads validated total/available physical memory through `GlobalMemoryStatusEx` while keeping the unsafe boundary platform-local. This is observation only: no process admission or authority changed. **Phase 3E2B** is the next resource gate and will convert that snapshot into a conservative, testable emergency-headroom admission decision; heavy-task scheduling/classes and optional I/O governance remain separate.

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
