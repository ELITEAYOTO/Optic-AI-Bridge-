# Issue 317 — Precreated kernel Job and create-time child containment

**Status: Windows-only, test-only research.** Not an accepted ADR or production GitRead fix, and not evidence of host-crash recovery or V1 approval.

**Exact baseline:** reviewed `main=100c43b53c795be5678c8b4f2c0af4353ca5b196` with independent full CI [#40027279676](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40027279676) successful. Research branch `research/317-kernel-job-create-time-2026-10-24`.

## Source-level defect boundary

In the current `optic-bridge-windows/src/job.rs`, `LimitedJobObject::new` only saves validated numeric budgets. The ordinary wrapper performs `CREATE_SUSPENDED`, then `OwnedJob::create`, then `AssignProcessToJobObject`, then resumes the child. An external host crash between suspended creation and assignment can leave the unowned child alive. The previous [owned-admission/JobStore research](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6375617633) did not create a kernel Job at all.

Codex's separately validated research (`codex/317-native-registration-authority=bcad195208bc5ecb0222fbfe5cd0595d07cb5938`) uses `PROC_THREAD_ATTRIBUTE_JOB_LIST` for create-time membership, but is not the production runner. This new test isolates that crucial Win32 boundary against **the current main crate and its existing actual `OwnedJob::create` implementation**.

## Test implementation

- Under `#[cfg(test)]` only, `job_create_time_research.rs` calls the existing private `OwnedJob::create(1, 512MiB, 25%)` **before** `CreateProcessW` and verifies the owned kernel Job is empty.
- Builds one fixed, isolated startup attribute list with `PROC_THREAD_ATTRIBUTE_JOB_LIST` holding the genuine Job handle.
- Launches only the fixed **current Rust test executable**, targeted at one ignored synthetic child test, via `EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT`. There is no post-create `AssignProcessToJobObject`; handle inheritance is disabled and the child gets a minimal explicit environment rooted in a disposable fixture directory.
- Immediately verifies Job accounting sees exactly one process **while the child's primary thread remains suspended**, and its marker does not exist.
- In one test, resume the primary thread; wait for known process exit and query the Job until empty, all with finite time limits.
- In another test, terminate the exact owned Job **without ever resuming**; prove child exit, empty Job and absence of the execution marker.
- Rust RAII owns the handles. `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` is configured by the existing `OwnedJob::create` and provides a last-handle cleanup fallback for an assertion panic. A returned terminate call is not itself emptiness proof; the test explicitly queries.

## Separate independently supervised *creator-host death* regression

The research now also contains a **fixed ignored creator-host** which waits on a supervisor gate before attempting Win32 creation. The Windows-only external script `scripts/research/issue317_kernel_job_host_death.py` creates its **own outer kernel Job with kill-on-last-close** and assigns the **exact known synthetic creator-host** before permitting it to create anything. The host precreates/configures its own inner Job, creates one fixed suspended child already associated at `CreateProcessW`, verifies one member, and signals readiness.

The supervisor verifies the outer Job contains exactly **two** processes (host plus suspended child), keeps the outer Job handle OPEN, and kills **only the exact creator-host handle**. It then requires the outer Job's active count to become zero **without closing or terminating the outer Job**; the inner Job's final-handle closure on creator death must terminate the suspended child. The child must never have written its execution marker. On failure, the supervisor explicitly terminates and drains its outer Job and preserves sanitized diagnostic evidence; it never finds or kills arbitrary PIDs. All rendezvous and waits are bounded. This provides *host-death* proof for that narrowly fixed Windows fixture, not power-loss or a full Git runner.

Neither the child nor the issuer is a project tool, and no real Git command is executed. The test is research-only, subject to first-run actual hosted evidence; this paragraph is a test design, **not yet a claim of a successful CI run**.

## Not proved; remaining required experiments

This is a same-host process creation and stop test. **It does not yet prove sudden creator-host death containment with an outer independent supervisor**, nor descendant EOF/pipes, controller failures, power loss, crash-observation errors, or an authenticated owner ticket. The separate Codex multi-mode supervised fixture proved a research shape but the production integration and exact main APIs still require independent review.

No service root owns the kernel Job here. No new capability, policy/lease, typed GitRead effect, pinned executable, result publishing, JobStore admission or `OwnedSessionAdmissionPermit` wiring is added. The existing normal runner and its vulnerable assignment window are intentionally unchanged. Do not migrate any Git command or merge the full research branch on the basis of these results.

## Required checks

Pinned Rustfmt, strict Windows crate Clippy, complete Windows crate library suite, **explicit two named test executions**, Ubuntu formatting/compile compatibility, first-error logs and exact source SHA. Only label success after inspecting actual hosted Windows results and verifying the two tests executed. In later work, add outer-supervised creator-death test with true retained original child/job observation and avoid false-positive cleanup from the supervisor itself.

Issues #317/#323 stay open; V1 stays NO-GO. PR #327 and other native research branches remain isolated. No user PC execution, Codex quota, self-hosted runner or release action was used here.
