# Code quality review and bounded refactoring tracker

Status: IN PROGRESS, source inventory recorded 2026-10-20. This is a
review plan and partial source inspection, **not** a completed file-by-file audit
or a claim of production security.

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

A separate targeted change `fix/process-output-read-errors` tracks a distinct
bounded read-failure flag in the private job record and refuses an otherwise
successful process result after an incomplete drain. It preserves the existing
output-budget signal, ownership, CPU/memory reservations, deadlines and native
stop uncertainty. Two synthetic async reader regressions cover failed reads
and clean EOF, while hosted Windows and Linux CI must confirm compilation,
format, and native behavior before support can be claimed.
