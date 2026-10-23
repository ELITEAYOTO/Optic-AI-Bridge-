# #317 — Admission, shutdown and atomic Windows Job launch: decision review

**Date:** 2026-10-23. **Status:** RESEARCH / PROPOSED, not approved, not a production fix.  
**Production baseline:** `main=da34eae7d0efda39d9fb3c6786ec843546926fe9` (docs-only PR #328; independent push CI [#39890588740](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39890588740) succeeded).  
**Research source inspected:** `codex/317-service-retention=1c7dac973c8f41f27b303811041b54031f7272ae` ([CI #39888522519](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522519), [native #39888522200](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522200), both successful on this source).  
**Existing independent PR:** [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327), still open, originally red in [#39845778580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39845778580) for separate #185. This note does not supersede that run or approve a merge.

## 1. Verified code paths and guarantees

- `SessionRegistry::begin_admission` increments an admission counter while the registry mutex is held; `SessionRegistry::revoke` sets `revoked=true` and publishes its stop atomic *before* waiting on the admission count. Dropping the scoped permit decrements that counter. This is a synchronous admission-drain fence in existing callers.
- `AuthorizedGitIntegrationService::{integrate_fast_forward,observe_target_head}` retains the permit across the synchronous Git effect. By contrast, experimental `ReadServiceRoot::submit` drops the permit when submission returns; its native worker is service-owned but no longer counted as an admitted effect.
- `SessionLifecycleManager::revoke` currently waits inside registry revoke *before* revoking approvals/leases and asking ProcessManager to cancel jobs. The registry's stop atomic can reach research-native watchdogs before that wait, but the ordinary cleanup ordering must not be silently changed.
- Experimental `ReadServiceRoot` owns and joins worker handles; its `ProcessManager` JobStore retains `Running/TerminationUncertain` owners and counts. `seal_quiescent` and `try_reap` refuse cleanup while they are active. `try_shutdown` sets stop before try-lock and reports pending responsibility. `Drop` joins workers and can therefore block indefinitely under permanently failing native observation.
- Experimental `PreparedReadCommand` creates `CREATE_SUSPENDED` first, then calls `AssignProcessToJobObject`, then rechecks stop and resumes. The independent host-death test shows an *unassigned suspended child survives abrupt host death before assignment*; the after-assignment control is contained. This is a real crash window in the candidate, not in-production evidence of a real user incident.
- There is no production authorizer for `AuthorizedRead`, no source-pinned GitRead migration, and production `HardenedCommandRunner` still joins pipe threads beyond its configured timeout. None of these research tests resolves #317 in production.

Evidence: [existing service audit](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/1c7dac973c8f41f27b303811041b54031f7272ae/docs/research/ISSUE317_SERVICE_ADMISSION_SHUTDOWN_2026-10-23.md), [ADR-0035 proposed](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/1c7dac973c8f41f27b303811041b54031f7272ae/docs/decisions/ADR-0035-WINDOWS-GIT-OWNER-STAGING.md).

## 2. Revocation semantics: a product/security decision is required

Two incompatible user-visible contracts must not be confused:

| Contract | Linearization and guarantee | Risk / condition |
| --- | --- | --- |
| **A. Synchronous completion fence** | Revocation closes admission and does not report *completed* until all previously admitted effects have terminal native and output proofs and results are finalized. The worker would hold an owned admission until then. | A persistent `TerminationUncertain` can make `revoke()` block forever. Moving the scoped permit into a worker alone is **not** acceptable. Out-of-band stop must precede any wait; all call sites must be audited. |
| **B. Two-stage close / quiescence** (recommended to **study**, not approve here) | Phase 1 atomically closes admissions and requests stop, returning an explicit **pending** state. Phase 2 alone proves quiescence, finalizes result visibility and permits seal/reap/resource release. | Changes current synchronous `revoke` semantics, call sites, worktree/lease cleanup ordering, and potentially MCP contract. Only safe if all consumers distinguish pending from completed. |

**Recommendation for design investigation:** introduce *names/contract tests first* for `RevokeRequested`, `StopRequested`, `PendingNativeProof`, `Quiescent` and `Uncertain`; identify whether old `revoke` remains synchronous and a separate asynchronous API is necessary. Do not report success while native ownership remains uncertain. A session must not be reaped or resource quotas released because the client disappeared.

The current `stop` atomic plus pre-resume recheck is **not** a common linearization point: a change can occur after the last check and before `ResumeThread`. A strict promise of *no instructions executed after a completed revoke* requires either exclusion of resume at the authority fence followed by proof of stop, or a narrower, explicitly accepted contract where a launch committed before closure can be stopped asynchronously. Holding the global session registry mutex across potentially blocking Win32 work is not accepted as a shortcut.

## 3. Host-crash containment: concrete Windows API candidate

Microsoft documents `PROC_THREAD_ATTRIBUTE_JOB_LIST` for **Windows 10+ / Windows Server 2016+**, passed through `UpdateProcThreadAttribute` / `STARTUPINFOEX` and `CreateProcessW` with `EXTENDED_STARTUPINFO_PRESENT`. The job handle list names Jobs to associate *during process creation*. This is the most promising research direction to remove the **post-spawn/pre-Assign** unowned interval.

Official references: [UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute), [InitializeProcThreadAttributeList](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-initializeprocthreadattributelist), [Job Objects / KILL_ON_JOB_CLOSE](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

**Important:** availability in Microsoft documentation is not a verified Optic implementation or native proof. Rust `std::process::Command::spawn` does not expose this particular creation-attribute setup in the existing candidate. A Windows-crate-owned launch primitive may need to own `CreateProcessW`, extended startup attributes, a constrained inheritable stdio handle list, process/thread handles, environment/cwd preparation, Job ownership, deadline and error cleanup. Preserve explicit memory/CPU/process caps and `KILL_ON_JOB_CLOSE` before launch. Do not add an unsafe fallback to spawn-then-assign on supported profiles: inability to create under the Job must fail closed or be explicitly excluded. Confirm interaction with an outer pre-existing Job/nested Jobs and with Windows 10/11 target builds.

**Alternative:** an independently durable launch/owner broker can close host-crash gaps when properly designed, but introduces authority, IPC, budgets, recovery and service-installation risks. It is a larger, separately reviewed architecture, not a quick fallback.

## 4. Native proof matrix before any GitRead migration

Use only synthetic child fixtures and an **independent pre-armed outer supervisor Job**; supervise the supervisor itself, enforce a finite external deadline and verify Job-empty cleanup, including first failures.

1. **Atomic-association candidate:** gate immediately after a successful `CreateProcessW` call with `PROC_THREAD_ATTRIBUTE_JOB_LIST`, crash only the synthetic owner host, prove **zero** remaining inner child process / no surviving suspended child; compare to the known old 2→1 reproduction and post-Assign 2→0 control.
2. **Failure matrix:** invalid Job handle/attribute setup, creation failure, stdout/stderr setup error, output cap, no-spawn cleanup, duplicated-handle inheritance, thread resume failure, stop before start, stop during creation, and cancellation immediately around resume. No unowned child or quota release before full proof.
3. **Windows compatibility:** hosted Windows (for automated proof), then only with renewed operator authorization a real Windows 10 and Windows 11 acceptance matrix. Check nested outer Job, no breakaway, no hidden inherited Job handles, no unexpected process execution pre-assignment.
4. **Revoke/admission races:** inject deterministic barriers before/after admission, publication of closed state, launch commit, native creation and resume. No synthetic caller-held permit. Assert exact worker/job-record state, observable pending-vs-complete semantics, output release refusal after revoked authority and session seal/reap protection.
5. **Shutdown uncertainty:** recoverable injected observation errors retain owners, handles and reservations; `try_shutdown` is pending, never success. Persistent native failure remains an unresolved design/no-go unless a proven durable supervisory ownership mechanism or an explicitly accepted blocking-exit policy exists.
6. **Integration preconditions (not this test slice):** authenticate `Effect::GitRead`, session/task lease and scopes at actual start; monotonic effective absolute deadline, pinned Git binary, canonical repo/cwd identity, controlled Git configuration/environment/argv, no arbitrary `Command` or raw creation flags. Only then migrate **one read-only** Git action; never Git CAS here.

## 5. No-go gates and next narrow increment

**NO-GO for #317 production wiring:** admission semantics unapproved, no full exit/uncertain-owner policy, crash-before-assignment candidate unresolved, no proven stop/resume linearization, missing true `AuthorizedRead` and effective deadline. #176/#182 sensitive Git provenance, #185 AppContainer, #319 first MCP response, #323 V1 freeze remain independent.

**Suggested next research increment:** Windows-only, test-only `PROC_THREAD_ATTRIBUTE_JOB_LIST` launch fixture in the platform crate, independently supervised and non-exported to production; compare old/new failure traces at exact SHA. Do **not** combine it with a `SessionRegistry` API change, GitRead migration, PR #327, #182 policy or Electron work. Submit a single reviewed code PR only after the candidate proof and clear chosen contract. Keep previous red CI attempts. This document changes no code or policy.


## 6. Windows-hosted CreateProcessW proof (23 October 2026)

**Scope:** synthetic Python/ctypes Win32 harness, not Rust production integration. Source:
[\`issue317_job_list_atomic.py\`](../../scripts/research/issue317_job_list_atomic.py).
The script runs on hosted Windows only, with a deliberately independent outer
Job owning the test host *before* the host can create a suspended child.
Only that synthetic host is killed; every test, including the negative
control, closes and empties the external Job.

**Initial baseline successful:** exact source \`14f08517dc5cd3c9b8ab342c4f2050eaefa39b43\`,
[workflow #39893560796](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39893560796),
first attempt completed/success:
- Legacy suspended child created outside inner Job: external count **2 -> 1**
  when host dies; the unassigned child survived and was then recovered by
  the independent supervisor.
- Attribute-based Job assignment with \`CreateProcessW\`,
  \`STARTUPINFOEX\`, \`PROC_THREAD_ATTRIBUTE_JOB_LIST\` and
  \`EXTENDED_STARTUPINFO_PRESENT\`: count **2 -> 0** on host death,
  with the child previously observed inside its inner Job.
- There are no stdin/stdout/stderr writer/readers in this fixed
  fixture. The command is a synthetic suspended Python sleeper,
  \`bInheritHandles=FALSE\`, not an authorized Git command.

**Negative launch proof:** exact source \`7c6044bd5a1d4a453c6eb9d3e001a5600650b073\`,
[workflow #39893750004](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39893750004),
completed/success, first attempt. In addition to repeating the legacy
2 -> 1 and atomic 2 -> 0 cases, an intentionally invalid Job handle
caused the native launch path to reject creation with **1 -> 0**
external members after host death (host only before it); no child was created
and there was no fallback to unassigned spawn. All outer Jobs cleaned.

**First red preserved:** [#39893688142](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39893688142)
on \`07628ba9e\` failed \`JSONDecodeError\` because the test host wrote the
readiness file directly and the supervisor observed it before JSON writing
completed. Replaced the readiness file through \`os.replace\` after complete
serialization (atomic publication), then validated the *new source* above.
This failure was a test-harness synchronization defect; it is not evidence
of native Win32 failure, and no security constraints were relaxed.

**Conclusion:** the documented Windows 10+ attribute is a **promising
confirmed native mechanism** for this isolated host-death gap, but the
real Rust runner still uses spawn/suspend/assign and remains vulnerable.
This test does NOT establish Windows 10/11 physical acceptance, Rust
process/stdio handle ownership, duplicate handle inheritance, native
error/pipe cleanup under faults, full Job hard limits, locked admission,
stop-vs-resume linearization, or shutdown responsibility. No #317 gate
is closed, no ADR was accepted, and no source was merged to \`main\`.

**Next slice:** implement and test a **test-only Windows platform-crate**
primitive using extended process startup attributes and pinned executable
identity; pre-create and configure limits, explicit handle inheritance,
owned PROCESS_INFORMATION handles and failure cleanup. Test invalid
attribute/Job handle, cancellation before spawn and around resume,
nested Job interoperability and host crash under independent supervision.
Only then review a production migration design; no implicit fallback to
spawn-then-assign and no GitRead wiring in this experiment.
