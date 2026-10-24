# Issue 317 — Retaining native ownership during uncertain Job observation

**24 October 2026 — RESEARCH ONLY.** This does NOT make the production Windows runner safe, nor establish a durable service owner, bounded shutdown or V1 readiness.

## Source and accepted baseline

- Production base: `main=100c43b53c795be5678c8b4f2c0af4353ca5b196`.
- Parent combined-owner research: `research/317-owned-native-kernel-tree-2026-10-24=75024b0efea65c5595a3737838041aae197fecfa`, [CI #40042298153](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40042298153) Windows/Ubuntu green (Windows 347/Ubuntu 270).
- This branch: `research/317-native-observation-uncertain-2026-10-24`. Its ancestry **contains prior test-only research** and must NEVER be wholesale merged into `main`.

## Threat model and bounded regression

The real service can receive a stop signal and issue `TerminateJobObject`, and the root process may exit, yet **a Job Object accounting query may fail or be temporarily unavailable**. The service must never infer that the whole process tree is empty from termination request completion, root exit, pipe EOF, an unavailable metric or time expiry. A terminal status or admission release under that uncertainty can enable premature reuse of process/session budget.

A new fixed, Windows-only and feature-gated native wrapper method `ResearchKernelJob::observe_active_with_injected_failure(unavailable)` models the failure by returning `io::Error` without querying Win32 when injection is enabled; with injection disabled it performs the real `QueryInformationJobObject` through the existing safe `active()` method. This is **deterministic synthetic fault injection, not an observed genuine Win32 error**. The fixture is not a general-purpose launcher. The Windows child is the fixed current libtest executable, created suspended, associated atomically with a precreated real kernel Job using `PROC_THREAD_ATTRIBUTE_JOB_LIST`.

In the added regression, the worker owns a genuine counted `OwnedSessionAdmissionPermit`, one-use original-session authority commitment, real `ProcessManager` JobStore reservation, kernel Job and exact suspended child. Direct `SessionRegistry::revoke` publishes stop without calling `ProcessManager::cancel_session`. The worker requests kernel termination, confirms root exit, receives a synthetically failed accounting query, and enters a bounded **held-uncertain** test phase. While held:

- The direct revoke still waits on the genuine counted admission.
- JobStore still records the original private action/JobId and `ProcessManager::result` is **nonterminal `Running`**, rather than claiming `Stopped` or `TerminationUncertain` as completed.
- `remove_quiescent_inactive` rejects premature cleanup; an unrelated session remains usable.
- The worker retains the admission, reservation, kernel Job, and child handles.

Only after an external explicit test release does the worker retry an actual native accounting query, observe **zero members**, publish JobStore `Stopped`, and release the owner/admission. The waiting revoke then finishes. The admission is never dropped because of the injected error alone. `Result::Running` under this synthetic uncertainty means **not confirmed terminal**, not evidence that a process is actively executing.

## Limits and next acceptance slices

The test has finite gate waits so GitHub-hosted CI fails instead of hanging. It does **not** prove a production solution for persistent errors, bounded service shutdown, actual Windows API failures, leaked native handles, descendant-held stdout/stderr, true root/descendant EOF, OS power loss, or service-host crash with a real combined owner. A production service must supply a retained guardian, out-of-band stop, separate closing and quiescence APIs, and an uncertainty/quota policy; simply retaining a permit forever could still block synchronous lifecycle revoke. Existing API `finish(ProcessStatus::TerminationUncertain)` can set a record's finished flag and close streams, so this test intentionally never calls finish while uncertain. That case needs separate production scrutiny.

`#![forbid(unsafe_code)]` in the runtime is unchanged. All Win32 operations remain in the Windows crate's explicit **non-default** `issue317-owned-kernel-fixture` feature. This branch does not touch GitRead, release or user-PC flows. PR #327 remains independent. Issues #317/#323 must remain open; Windows V1 **NO-GO**.

Acceptance requires exact-head hosted Rustfmt, strict Clippy, full Windows+Ubuntu runtime regressions and an explicit named Windows uncertain-observation test. Preserve first failures. Do not claim green before the actual logs prove the named test ran.
