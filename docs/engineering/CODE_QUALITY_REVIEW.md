# Code quality review and bounded refactoring tracker

## Narrow read-only process launch / error-path review — 2026-10-21

**Reviewed source SHA:** `30295539fc6911f221cee6ac3c59785398c7a050` (`main` immediately after #312). Files and boundaries: `crates/optic-bridge-runtime/src/process.rs` Windows isolation launcher stdin delivery and process monitor, `crates/optic-bridge-app/tests/appcontainer_network_isolation_windows.rs` native network-denial proof, plus `.github/workflows/ci.yml` diagnostic triggers. Scoped to launcher handoff, *not* a complete process lifecycle, filesystem, CI or codebase audit.

- **Input/effect/authority:** app-owned, bounded serialized isolation request is piped to a fixed pinned outer launcher; current native CI uses an explicitly opt-in phase marker. No model-supplied path/secret is published by the fixed-label launcher diagnostics.
- **Error/uncertainty:** detached async `stdin.write_all` and `shutdown` results are ignored. Parent return marks outer process spawn, not confirmation that payload was delivered, parsed or inner Node resumed. This is an **observability gap**; actual delivery failures have not been observed in failed runs, so the 10s root cause is unresolved.
- **Ceilings/cleanup/test:** existing 10s test deadline, owned Job Object containment, termination-uncertain handling, no-accepted-connection requirement and normal Node attempt/exit assertions must not be weakened. Previous natural CI failures and repeat timings are recorded in [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185#issuecomment-6312184351); accepted #312 and independent main CI are green, **not** failure reproduction.
- **Finding status:** reviewed-with-observability-gap, no demonstrated runtime correctness fix; no patch/CI added from this review. Future optional fixed-label delivery timing/error measurements require synthetic failure injection and native Windows validation. No claim of line-by-line coverage of other components.

Older review records follow with their own exact SHAs and may not describe current integration.

## Audit coverage clarification — 2026-10-21 after verified #305 integration

At verified `main=43d3a4aeedee04dd03e895e27e81d3bc5af6652c`, post-merge [CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed first attempt. Further **scoped** source corrections since the prior inventory include canonical `fs_list` authority [#303](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/303), protected Git-control mutations [#304](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/304), and cursor-aware Git-log response fitting [#305](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/305), each with focused tests on hosted Windows/Linux.

These do **not** establish a line-by-line review of every crate, script or workflow. The original 212-file/95-Rust-file counts below were recorded at an **older SHA** and must not be treated as a live inventory. High-risk native process, directory-race and recovery/persistence boundaries remain only partially covered. Future audits should record (path, exact source SHA, trust boundary, limits, errors/uncertain state, tests, concrete finding/no-finding); only demonstrated defects justify changes. No mass formatting/refactoring in the name of cleanliness. See [reliability register](RELIABILITY_DEBT.md).


## Reconciled source-review checkpoint — 2026-10-21

Last verified main before this documentation change: `83329d48789c20b1fc235d3096c3199dc9dc5176`, [CI #39514832814](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514832814) green first attempt on exact SHA, zero open PRs. #293–#297 are merged. A focused registry review established and fixed a specific resource-usage ordering fault in `task_lease_registry.rs`: duplicate and global/session capacity conditions are now preflighted before potentially costly pinned executable identity capture, and **rechecked under the final insertion lock**. Hosted native Windows and Linux negative/concurrency regressions passed (#297). #294–#296 supply narrow startup timing observations; they do not constitute a general performance or security audit.

The file-by-file quality audit is **not complete**. Do not transform this targeted finding into an assertion that the repository is 100% clean or that the 10s Node and 30s MCP timing intermittencies are resolved. The reviewed core remains pre-alpha; the operator's freeze on new features and speculative refactors is in force. Historical sections below reflect their dated scopes; live GitHub and the top-level status own the integrated state.

Status: IN PROGRESS, source inventory recorded 2026-10-20. This is a
review plan and partial source inspection, **not** a completed file-by-file audit
or a claim of production security.

## Latest verified scoped corrections (2026-10-20)

[PR #290](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/290) (Windows Node timing markers, not a root-cause repair) merged with passing hosted exact-head and post-merge CI. [PR #291](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/291) (Windows all-staging preflight before cleanup, with a passing two-journal native regression) merged as `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`; independent [main CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) passed Windows, Linux and dependencies on that SHA on its first attempt. The earlier #286–#288 'candidate' notes below were written before their respective merges; verify the canonical [STATUS](../../STATUS.md) / [handoff](GITHUB_HANDOFF.md) for integrated state. There has **not** been a complete file-by-file audit, and multiple-file deletion atomicity, directory identity, power-loss durability, and Windows startup intermittencies remain open. Feature work is paused by the operator.

## Starting inventory

At reviewed main `bca2eae82eacf5df89b2fd70339751723ec86167`,
GitHub's untruncated recursive tree contained 212 tracked files:
95 Rust, 10 PowerShell, 6 workflow YAML, 86 Markdown and 15 other
config/data files. Sizes alone do not establish defects.

Measured large Rust files on that tree included `runtime/process.rs` (2,679
lines), `app/main.rs` (2,320), and `runtime/project_inputs.rs` (1,524).
The latter contained approximately 1,250 lines of inline test content. This
extraction was merged by PR263 into `src/project_inputs/tests.rs`
without changing production capture logic. Its exact head `2d8a410` passed
Windows/Linux Rust, dependency policy and native preparation before squash
into main `bcb5e8a`. This is not an exhaustive file audit or a behavioral fix.

## Review order and completion rules

Proceed in small, separately validated PRs rather than simultaneously
moving files and changing security behavior:

1. **Safe structural extractions:** separate existing test modules when
   this reduces noise without changing test identity/assertions; verify
   Linux and Windows builds/test discovery on the exact resulting head.
2. **High-risk native boundaries:** process/owned-job lifecycle, journal
   recovery, Windows file/path/AppContainer primitives, resource limits,
   monotonic leases, cancellation and shutdown.
3. **Authority and mutations:** policy, session/task ownership,
   preconditions, worktrees/Git integration, sensitive metadata and
   default-deny process/network/secret scopes.
4. **Adapters and operator surface:** MCP request/response limits,
   transport, CLI configuration parsing, installers, optional experimental
   alpha behind its own reference branch.
5. **All remaining files:** scripts, workflow triggers/runner identities,
   configuration, documentation links, archived evidence and repo hygiene.

For **each file**, record the reviewed commit and path, purpose/ownership,
caller/dependency boundaries, input trust, allowed effects, time/byte/entry
ceilings, error/uncertain outcomes, OS-specific assumptions, test coverage,
and exact findings or 'no finding in reviewed scope'. For every bug, file
a specific test-backed correction; avoid large unsourced ratings. Distinguish
not-reviewed, reviewed-without-finding, confirmed issue and resolved-by-PR.
No "all files audited" statement is permitted until that inventory is complete.

## Organization principles

The six existing crates and normative documentation owners stay intact.
Do not shuffle public contracts/ADRs, old research, archive refs or scripts
whose paths are embedded in workflows/installers solely to tidy appearance.
Prefer child modules organized by responsibility to generic "utils" files.
Keep runtime Rust independent of optional Node, Electron, Python and Docker.

Review the first extraction and its native checks before choosing whether to
split `process.rs`, `main.rs` or `server.rs` further. Production splits
require separate tests for any boundary or behavior that changes.

Source and CI rules: [AI handoff](../../AI_HANDOFF.md),
[security invariants](../security/SECURITY_INVARIANTS.md),
[repository workflow](REPOSITORY_WORKFLOW.md), [current handoff](GITHUB_HANDOFF.md).
No operator-PC or self-hosted execution. Research Cargo and Electron failures
retain their independent, documented status.

## Focused process-runtime source review — 2026-10-20

Source base: `main` `3dac14efe1a5040a9bae4dc936c5c43f9917b1bb`.
Reviewed: `crates/optic-bridge-runtime/src/process.rs` source layout,
owned-job record fields, admission/retention checks, monitor/termination
uncertainty and existing unit/integration test module structure.
This is **targeted coverage**, not a line-by-line proof of the full runtime.

- Production ownership: `ProcessManager` stores jobs keyed by opaque `JobId`;
  `owned_job` returns the same unknown-job error for absent and foreign
  jobs. Terminal-only same-session history eviction is explicit.
- Runtime resource admission checks cover active/uncertain process count,
  output, process memory, workload class, CPU and Windows host-memory
  headroom. The native owned job and monitor continue to require bounded
  termination confirmation; unresolved termination retains resource
  ownership. These observations are **not** evidence of universal safety.
- Scoped finding: `process.rs` had 2,679 lines, including 1,115 lines of
  inline tests. A mechanical refactor retains the production prefix exactly
  and moves the complete existing test body into `process/tests.rs`
  (27 test attributes and 70 assertion macros observed). The parent uses
  `#[cfg(test)] mod tests;` and test module identity remains
  `process::tests`. The new parent has 1,564 lines.
- No security/behavior defect has been proven by this inspection. The
  extraction changes no effect, deadline, capability, isolation or
  cleanup rule. Acceptance requires GitHub-hosted Windows **and** Linux
  formatting, Clippy and tests on the exact PR head. No operator-PC tests.

Remaining review: fully examine the spawn/launcher and environment branches,
concurrency/lock ordering, output module, cross-session interactions and
Windows-only cleanup with native tests before claiming broader assurance.
Any confirmed defect belongs in a distinct regression-backed change,
not hidden within this mechanical relocation.

## Confirmed process output diagnostic gap — 2026-10-20

Source review of `crates/optic-bridge-runtime/src/process/output.rs` on
`main` `a9d9a31b4e1635c35ac53c8225a4c5b4348aeb05` found that a
reader error followed the same branch as clean EOF: both exited the drain
without setting any observable failure. Consequently a child exiting zero
could be returned as `Exited` with `output_truncated=false` even if stdout
or stderr had become unreadable. This is a concrete source-level reliability
finding, not an observed production incident.

PR266 delivered the correction on main as `5a8157e`, from source head
`4e4c712` with all four applicable hosted Windows/Linux Rust, dependency
policy and native-preparation checks passing. It tracks a distinct bounded
read-failure flag in the private job record and refuses an otherwise
successful result after incomplete drain. It retains the original budget,
ownership, CPU/memory reservation, deadline and termination-uncertainty
contracts. Two synthetic async-reader regressions cover failed reads and
clean EOF; these do not establish full end-to-end failure injection or
power-loss durability.

## Git-domain source layout consolidation — 2026-10-20

Base: `main` `24a40e106ced2e66c373ec7787ad077eb16fc9b8`.
The ten prior `src/git_*.rs` files were moved, byte-identically, into
`crates/optic-bridge-runtime/src/git/` using concise domain-local filenames:
`blob_batch.rs`, `change_set.rs`, `integrate.rs`,
`integration_authority.rs`, `merge_commit.rs`, `merge_plan.rs`,
`read.rs`, `tree_manifest.rs`, `tree_write.rs`, and `worktree.rs`.

The **crate-root module names remain unchanged** through explicit
`#[path = "git/…"]` declarations in `src/lib.rs`. Consequently existing
`crate::git_*` references, public re-exports, tests and private ownership
boundaries retain their module identities without a new `git` API facade.
No Git behavior or security rules were intentionally altered.

The source blobs were verified identical on both Git trees. [PR #268](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/268) merged as `d79d60b` after the exact PR head `b2b5207` passed hosted Windows/Linux Rust, native preparation and dependency-policy checks. This proves the chosen layout passed the exercised tests, not a comprehensive Git security audit. Do not repeat such relocation in one unbounded repository-wide PR.

## Intermittent Windows MCP smoke observation — 2026-10-20

On `main` `24a40e1`, CI
[run 39356540735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39356540735)
failed the `Smoke isolated Node doctor with TEMP-only environment` step
after a 30-second MCP response timeout; the bridge remained alive until the
test ended. Rust unit/integration tests, Linux and dependency checks passed.
A **failed-job-only rerun** of Windows job `118412524278` succeeded with
the same source revision, without modifying tests or security settings.
This characterizes an intermittent failure, **not** a proven root cause or
permanent fix. Preserve both attempts and investigate bounded startup/stdio
diagnostics if it recurs. A green rerun must never erase the first failure.

## Session-domain physical layout — 2026-10-20

Source base: `main` `e98c2010d5ed9a5f452259ab85721f2a65a79ddf`. A bounded layout change groups four
session-owned modules under `crates/optic-bridge-runtime/src/session/`:
`lifecycle.rs`, `registry.rs`, `reusable_approval.rs` and `worktree.rs`.
The previous `session_*.rs` files are represented by their **unchanged Git blob
identities** in the new directory. The crate root uses explicit `#[path]`
declarations to preserve the existing `session_*` Rust module names,
private intra-crate references, nested test identities and public exports.
No policies, expiry clocks, authorization or worktree effects are modified.

This is a physical organization improvement only, **not** a completed security
review of sessions, leases or worktrees. [PR #270](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/270)
was merged as `636a8b5` after the exact head `4d5ff0a` passed all four
applicable hosted Windows/Linux Rust, dependency and native preparation
checks; main push CI 39360825623 also passed. The separate TaskLeaseRegistry,
ToolProfileRegistry, approval brokers and Git modules remain unmoved pending
their own scope-specific review.

## Process drain join diagnostic gap — 2026-10-20

Review base `main` `636a8b5e917fd731dbd8566e118958f141f67542`: after child termination the monitor
discarded the `JoinHandle` results of both stdout/stderr drain tasks.
An unexpectedly panicking or cancelled drain could therefore leave incomplete
output without setting `output_read_failed`, while an otherwise successful
exit was still reported as `Exited`. This is a source-level reliability
finding, **not** evidence that a production panic occurred.

The targeted `fix/process-drain-join-diagnostics` change awaits and checks
both joins, sets an existing private incomplete-output flag, and marks an
unexpectedly abandoned stream closed. The pre-existing deliberately aborted
drains following `TerminationUncertain` retain their existing semantics.
Native stop, bounded output budget, resource reservation, session ownership
and policy code remain untouched. Two synthetic Tokio regression tests
exercise unexpected task panic and intentional abort. [PR #271](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/271)
merged as `8610c8d` after exact head `fb9ffdb` passed all four applicable
hosted Windows/Linux Rust, dependency policy and native preparation checks.
This does not prove that the monitor itself cannot panic or that inherited
pipe handles always drain; those are distinct future review targets.

## Focused registry/lifecycle source inspection — 2026-10-20

On `main` `636a8b5`, inspected `src/session/registry.rs`,
`src/session/lifecycle.rs`, and `src/task_lease_registry.rs` around
session admission, expiry, renew, revoke, revocation waiting, lease ownership
and physical reap. The reviewed paths check monotonic expiry, close admission
during revoke, drain admitted operations, and preserve lease policy epoch
validation. They contain 10, 17 and 13 test attributes respectively.
**No additional reproducible defect is established by this scoped read.**
This is not full concurrency, platform, whole-file or source-wide assurance.
Future work: adversarial lock-order stress, exact authority provenance,
process monitor shutdown and MCP TEMP-only intermittent first-response failure
without a timeout waiver or unsupported compatibility claims.

## Mutation/recovery domain organization — 2026-10-20

The existing runtime mutation and recovery sources are grouped in
`crates/optic-bridge-runtime/src/mutation/` as the exact unchanged Git
blobs `atomic.rs` (`mutation`), `journaled.rs` (`journaled_mutation`),
`recovery.rs` (`recovery`), `authorized.rs`
(`authorized_file_mutation`), `authority.rs` (`mutation_authority`)
and `transactional.rs` (`transactional_file`). This is a six-file
**physical relocation only**, not a journal format, permission, scope,
type, recovery, file IO, timeout, precondition or public API change.

`src/lib.rs` preserves the six original crate-root Rust module identities
with explicit path declarations; existing `pub use`, `crate::...` call
sites and inline test namespaces are intentionally unchanged. The reviewed
sources include 63 unit-test attributes, which remain attached to their
original modules. Other file/domain services (`filesystem`,
`directory_create`, `artifact_publication`, `project_inputs`) were
deliberately left untouched pending separate ownership review.

Git blob identity establishes unchanged source bytes, **not** runtime
correctness. [PR #273](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/273)
merged as `58ebc5c` after exact source head `9758492` passed four hosted
Windows/Linux Rust, dependency policy and native-preparation checks.
Post-merge main CI 39363288153 passed. This is not a completed
line-by-line audit of these modules or an endorsement of the journal schema
for arbitrary crash/power-loss conditions. Do not re-run this as a broad
repository-wide relocation.

## Fail-closed journal record framing — 2026-10-20

Source review of the mutation recovery parser found that it skipped
**all** empty newline-delimited segments. The writer emits exactly one
newline after each JSON record, never a complete blank record, while a
crash-torn *nonterminated* final record must remain tolerable. A completed
blank record anywhere else was silently accepted and could conceal
anomalous on-disk evidence. This is a concrete source-level integrity
gap, not an observed user incident.

The `fix/recovery-reject-empty-lines` change distinguishes the one empty
split item after the normal final terminator from unexpected complete
blank records. It refuses the latter as `CorruptJournal`, preserving the
journal instead of retiring it. Two regression cases exercise an empty
record between transitions and after the last record, plus a separate
test that a torn nonterminated tail still resolves from the previous
complete record. No schema format, state transition, effect, lease,
file-output budget or permission boundary is widened. [PR #274](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/274)
merged as `58a72a2`, from exact source head `283c9e0`. Linux, dependency
and native preparation checks passed. Windows initially failed an unrelated
AppContainer Node-loopback timing probe; three targeted serial diagnostic
retries passed, and the failed Windows job was rerun unchanged and passed.
The original failure remains preserved in run 39363540900, not waived.
See the reliability debt register for the still-unknown root cause.

## Recovery-journal read/retire lifecycle — scoped source review, 2026-10-20

Reviewed `runtime/src/mutation/recovery.rs` open/read/retire methods on
`main` `b7ab450` and `c2282ff`, not every caller or concurrent filesystem
interaction. A constructed valid hard-linked journal was accepted by the
read path; #277 fixed that narrow defect with descriptor/native-handle checks
and passed four exact-source hosted checks before squash into `c2282ff`.
The subsequent retire path still unlinked a hard-linked entry by pathname.
PR #278 merged as `b15fe76` after exact head `9778b49` passed all
four hosted checks. The change adds an explicit rejection regression,
native handle deletion on Windows and descriptor link/identity checking
before Unix unlink.
Source inspection does **not** prove that Unix concurrent pathname replacement,
journal appends, directory identity or staging cleanup are fully safe. Those
are independent review candidates; avoid combining them into a large refactor.

## Mutation journal append hard-link boundary (PR #282, 2026-10-20)

Source review of `runtime/src/mutation/recovery.rs` identified the only
remaining state journal operation of the three read/append/retire paths
using pathname-only metadata followed by a separate append open. PR #282
merged into `main` as `bf79b320` after exact-head native Windows/Linux,
dependency and private preparation checks. Windows now uses an exclusive
validated state-file handle; Unix validates device/inode/single-link identity
on the opened descriptor. Existing byte and sync barriers remain. The negative
regression preserves both original external and journal evidence bytes.
This review does not establish safe concurrent Unix pathname/size mutation,
full parent-directory pinning, staging cleanup or power-loss durability.


## Staging cleanup evidence and deletion identity (PR #284, 2026-10-20)

Scoped source review of Windows `journaled.rs::cleanup_recovered_staging`:
the reader was closed before a separate pathname delete. The bytes/hash
check allowed a foreign hard-link alias with matching content. PR #284
reuses existing native no-reparse single-link exclusive delete handle and
tests rejection with **both** file links and the durable journal preserved.
This is a targeted recovery-evidence boundary correction, not a complete
file-by-file code review, parent-directory race solution, or proof of
sudden-power-loss durability. Its exact validation and merger are recorded
on the PR, not presumed by this review note.

## Recovery state-root setup preflight — narrow source review (2026-10-20)

`mutation/recovery.rs::open` performed `create_dir_all` before workspace
confinement checks. For an absent nested state-root inside the workspace, a
denied operation nevertheless created a directory tree. Proposed preflight
rejects a canonicalized existing in-workspace ancestor before creation and
retains post-creation canonical validation. A negative workspace-unchanged
test and an outside-root control cover the precise behavior. This is not a
general directory TOCTOU or persistence proof. Check the corresponding
scoped PR's exact-head native CI and merge state separately.

## Prepared recovery journal publication — no-clobber scoped review (2026-10-20)

Reviewed `mutation/recovery.rs::begin_for_action` after #285 on
`main` `49c55d7`. The preflight `ensure_action_path_absent` was separated
from an overwriting `fs::rename` syscall. The candidate uses OS-native
atomic no-replace publication, plus preexisting-evidence and concurrent
publisher tests. It preserves the durable file write and existing state
machine. No conclusion is drawn about folder identity under replacement,
power-loss directory durability, other unsupported OSes or unrelated process
isolation. Exact branch CI/merge must be checked live.


## Prepared-journal Linux directory persistence (2026-10-20)

A focused follow-up after #286 found `renameat2(RENAME_NOREPLACE)` did not
sync the containing directory after publication, even though the prepared
file had been synced. The candidate adds `sync_all` on the already-open
Linux recovery directory, and injects sync failure to prove stable journal
evidence remains recoverable while admission returns an error. This is a
limited Linux filesystem persistence barrier, not a sudden-power-loss
certification, Windows flush guarantee or ancestor pinning fix. Verify the
source PR's exact CI/merge state separately.

## Recovery scan cleanup ordering — 2026-10-20

Review of `mutation/recovery.rs::inspect_pending` identified eager deletion
of a valid unpublished prepared file before later stable records were
validated. A later corrupt stable journal causes failed inspection despite
already erased earlier temp evidence. The scoped candidate defers temp
cleanup until after successful stable classification. Regression covers
failure with both files preserved and successful retry cleanup. This does
not make temp deletion atomic across multiple entries, solve concurrent
namespace replacement, or establish hardware power-loss durability.
