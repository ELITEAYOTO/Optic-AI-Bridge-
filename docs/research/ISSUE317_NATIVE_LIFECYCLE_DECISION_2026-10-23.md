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
[`issue317_job_list_atomic.py`](../../scripts/research/issue317_job_list_atomic.py).
The script runs on hosted Windows only, with a deliberately independent outer
Job owning the test host *before* the host can create a suspended child.
Only that synthetic host is killed; every test, including the negative
control, closes and empties the external Job.

**Initial baseline successful:** exact source `14f08517dc5cd3c9b8ab342c4f2050eaefa39b43`,
[workflow #39893560796](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39893560796),
first attempt completed/success:
- Legacy suspended child created outside inner Job: external count **2 -> 1**
  when host dies; the unassigned child survived and was then recovered by
  the independent supervisor.
- Attribute-based Job assignment with `CreateProcessW`,
  `STARTUPINFOEX`, `PROC_THREAD_ATTRIBUTE_JOB_LIST` and
  `EXTENDED_STARTUPINFO_PRESENT`: count **2 -> 0** on host death,
  with the child previously observed inside its inner Job.
- There are no stdin/stdout/stderr writer/readers in this fixed
  fixture. The command is a synthetic suspended Python sleeper,
  `bInheritHandles=FALSE`, not an authorized Git command.

**Negative launch proof:** exact source `7c6044bd5a1d4a453c6eb9d3e001a5600650b073`,
[workflow #39893750004](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39893750004),
completed/success, first attempt. In addition to repeating the legacy
2 -> 1 and atomic 2 -> 0 cases, an intentionally invalid Job handle
caused the native launch path to reject creation with **1 -> 0**
external members after host death (host only before it); no child was created
and there was no fallback to unassigned spawn. All outer Jobs cleaned.

**First red preserved:** [#39893688142](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39893688142)
on `07628ba9e` failed `JSONDecodeError` because the test host wrote the
readiness file directly and the supervisor observed it before JSON writing
completed. Replaced the readiness file through `os.replace` after complete
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
is closed, no ADR was accepted, and no source was merged to `main`.

**Next slice:** implement and test a **test-only Windows platform-crate**
primitive using extended process startup attributes and pinned executable
identity; pre-create and configure limits, explicit handle inheritance,
owned PROCESS_INFORMATION handles and failure cleanup. Test invalid
attribute/Job handle, cancellation before spawn and around resume,
nested Job interoperability and host crash under independent supervision.
Only then review a production migration design; no implicit fallback to
spawn-then-assign and no GitRead wiring in this experiment.


## 7. Rust platform-crate native continuation (23 October 2026)

**Source under test:** `2ca8996208bd1c701efd67d035fcfc3abce4c44b`, [Windows Rust CI #39895108162](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39895108162), completed **success on first attempt for this exact source**, with `cargo +1.99.0 fmt --all -- --check`, native `optic-bridge-windows` test binary compilation, `cargo +1.99.0 clippy --locked -p optic-bridge-windows --all-targets -- -D warnings`, and independent supervised native execution. No Rust source was merged to main.

New **test-only Windows platform crate**:
- [`atomic_job_list_probe.rs`](../../crates/optic-bridge-windows/src/atomic_job_list_probe.rs), included only behind `#[cfg(test)]` by `job.rs` (itself Windows-specific); no public runtime API or changed production launch path.
- Uses real existing `OwnedJob::create` with KILL_ON_JOB_CLOSE, finite active-process/memory and CPU hard caps **before** child creation. It sets up `STARTUPINFOEXW`, an initialized/dropped aligned attribute list, `PROC_THREAD_ATTRIBUTE_JOB_LIST`, non-inherited handles, `CREATE_SUSPENDED` and `EXTENDED_STARTUPINFO_PRESENT` for `CreateProcessW`. It owns the resulting native process/thread handles until host exit and has an exact-child fallback termination on normal Rust scope exit. Its executable is its own synthetic test binary, never untrusted Git or MCP input.
- [`issue317_rust_job_native.py`](../../scripts/research/issue317_rust_job_native.py) allocates an independent outer Job **before releasing** the Rust-host gate; it kills only the known test-host process and confirms zero active outer Job members, even when an assertion throws.
- [`issue317-rust-atomic-native.yml`](../../.github/workflows/issue317-rust-atomic-native.yml) executes on hosted Windows with evidence and strict lint.

**Native observed cases (synthetic only):**
- Atomic create under already-configured inner Job: outer Job before host death **2**, after exact-host death **0**; native test observed inner active-process count **1**.
- Intentionally invalid Job handle: `CreateProcessW` rejects creation, inner count **0**; outer Job before host death **1**, after host death **0**, no fallback spawn.
- Both cases supervised and cleaned. These results are from the actual Rust implementation, not merely the previous Python/ctypes API experiment.

**First failed runs retained as evidence:**
- [#39894678310](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39894678310): Rust 1.99 minimal toolchain lacked rustfmt. Corrected installation rather than skipping formatting.
- [#39894761787](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39894761787): exact rustfmt diffs from the new module; formatting updated.
- [#39894989421](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39894989421): Clippy with `-D warnings` exposed unnecessary pointer-cast parentheses; corrected without lint suppression.
- Final exact-source [#39895108162](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39895108162) passes all of the above on source `2ca89962`. Prior successful Rust native compile/run on [#39894844345](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39894844345) at `149fb22d` remains a separate older source checkpoint.

**Unproven and NOT authorized for production:** a native Windows10/11 physical-device acceptance matrix; process creation racing with unexpected host termination *during the kernel call*; exact identity and pinning of an external Git executable; explicit stdio and handle-inheritance matrix, native error/error-state retention, post-revoke output refusal, stop-vs-resume linearization, async revocation/drain ownership contract and bounded/persistent shutdown. Existing production `PreparedReadCommand` still uses spawn/suspend/assign. These tests cannot close #317, #323 or accept ADR-0035.

**Next recommended narrow effort:** design an RAII test-only API to combine a configured Job, inherited stdio handle list and direct Win32 creation under one owner with exact failure rollback; *also* decide session close/admission semantics and owner shutdown policy before any integration into GitRead. Reproduce bad-handle and inherited-output cases with an independent external supervisor. Avoid a production fallback that silently uses the known unsafe spawn-then-assign gap. Keep #182 provenance separately pending operator approval.

## 8. Narrow Rust handle-inheritance and no-spawn cleanup matrix (23 October 2026)

**New exact-source proof:** [Windows Rust workflow #39895889270](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39895889270)
completed **success on attempt 1 for SHA `b6158e0a61f2c5b76a2d058ac48b9578ca92feb3`**, verifying
Rustfmt, `cargo test --locked -p optic-bridge-windows --lib --no-run`,
`cargo clippy --locked -p optic-bridge-windows --all-targets -- -D warnings`
and independently supervised native execution. No code on `main` was changed.
The 4-case supervised JSON evidence is preserved as the artifact of that run.
The Python/ctypes research workflow remains distinct.

Native supervised cases (independent outer Job pre-arms the exact test-host
before releasing any process-creation gate; all processes are synthetic):
- **`atomic`**: preconfigured Windows Job is assigned during CreateProcessW;
  outer Job accounting **2 → 0** after terminating only the exact host.
- **`no-inherit`**: inside the test host, create a private
  `CapturedOutputPipe`, deliberately mark its spare writer handle
  `HANDLE_FLAG_INHERIT`, then create an inner-Job-owned child with
  `bInheritHandles=false`. Close the only parent writer and poll for
  **EOF while the child remains suspended/alive** (inner count 1);
  outer Job **2 → 0** after synthetic host death. This proves absence
  of inherited ownership of *that spare pipe writer* for this configuration.
  It does **not** prove redirecting stdout/stderr or selected stdio handle
  inheritance with `STARTF_USESTDHANDLES` and a handle allowlist.
- **`invalid`**: an invalid Job handle leads to rejected launch,
  no child, outer Job **1 → 0**.
- **`missing`**: one warm-up and **16** further failed CreateProcessW
  attempts targeting a synthetic, provably nonexistent executable.
  `GetProcessHandleCount(GetCurrentProcess())` is **exactly unchanged**
  across the 16 calls, the configured inner Job remains empty, and the
  outer Job shows **1 → 0**. This proves no persistent handle-count
  increase in this bounded failure test, not exhaustive native resource
  lifetime or crash-during-system-call behavior.

The distinct source candidates [#39895764236](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39895764236)
and [#39895785869](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39895785869)
failed **Rustfmt during source validation before compilation**, and remain
visible rather than being rerun on a different source. The exact formatted
candidate above passed all checks first attempt.

**Security interpretation:** `PROC_THREAD_ATTRIBUTE_JOB_LIST` now has
reproducible test-only Rust proof of host-exit containment, rejection of
invalid Job and missing executable launch, finite failed-launch handle
count, and a *negative* ambient pipe-inheritance assertion. These are
not an authorized standard-stream launch API, not validation of
`PROC_THREAD_ATTRIBUTE_HANDLE_LIST` selective stdout/stderr handles,
not a strict revoke-versus-resume linearization, and not a substitute
for durable session/owner shutdown. `PreparedReadCommand` in production
still has the suspend/assign crash gap. Windows10/11 real-device,
stdin-backpressure, inherited grandchild output, native fail-injection,
end-to-end GitRead/lease/pin provenance and actual V1 acceptance remain
NO-GO; ADR-0035 remains PROPOSED.

**Next isolated proof:** deliberately allowlist only selected
stdout/stderr inheritable write handles using `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`
alongside the Job-list attribute, with a fixed resumed synthetic child,
combined output ceiling, both EOFs, explicit handle cleanup and a
supervisor proving no inherited unrelated handle. Only after that
should the team consider a production `PreparedReadCommand` migration
proposal, independently of the unresolved session admission, Drop and
stop/resume contracts.

## 9. Positive Rust selective stdout/stderr inheritance (23 October 2026)

**Exact-source proof:** [Windows Rust workflow #39897059425](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39897059425) completed **success, attempt 1** for `ad39741ed666c6cb3977eb399766cf91257362f0`. Hosted Windows compilation, Rustfmt and strict Clippy `-D warnings`, followed by independent outer-Job supervision, all succeeded. Source stays under `#[cfg(test)]` in `optic-bridge-windows`; production `PreparedReadCommand` is not changed.

Test-only `run_selective_stdio` now uses two simultaneous `STARTUPINFOEXW` attributes: `PROC_THREAD_ATTRIBUTE_JOB_LIST` for association at process creation, and `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` listing only the three required inheritable **stdin NUL, stdout pipe writer, stderr pipe writer**. `CreateProcessW` receives `bInheritHandles=true`, `STARTF_USESTDHANDLES`, `CREATE_SUSPENDED` and `EXTENDED_STARTUPINFO_PRESENT`. A fourth writer, deliberately marked inheritable, is **not** allowlisted. The synthetic fixed Rust child is resumed only after native Job membership is observed and the unrelated pipe has reached EOF **while that child is still suspended**, demonstrating denial of the extra handle.

After native resume, the fixed child emits distinct `OPTIC317_STDOUT_FIXED` and `OPTIC317_STDERR_FIXED` markers. The owner polls both `CapturedOutputPipe` readers in 1-KiB chunks, imposes a **combined 16-KiB test assertion**, requires **both EOFs and an empty inner Job** before accepting, and retains the exact child PROCESS_INFORMATION handles until the end. This is a bounded test fixture, not a generic production capture service.

The 5 supervised cases are now `atomic`, `no-inherit`, `invalid`, `missing` and `stdio`. The new `stdio` case reported outer Job **1 → 0** after killing only the synthetic test host (inner Job already empty after normal child completion); all five independently recovered an empty outer Job. Earlier scenarios preserve their proofs. The [native logs](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39897059425) record `selected_stdout_stderr_and_spare_denial=true`, `complete=true`.

First candidate [#39896947005](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39896947005) and second candidate [#39896967200](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39896967200) remain **red, Rustfmt before compilation**; format adjustments were applied on a new SHA rather than weakening the gate.

**Limits:** no general-purpose arbitrary-process command API, binary/cwd/environment pins, secret or credential policy, delegated permissions, stdin backpressure, process-output descendant retaining writers, output-limit enforcement in a running production service, Windows 10/11 client-device certification, atomic stop-versus-resume handshake, host death *during* CreateProcessW, bounded shutdown or shared session admission/revocation proof. A test assertion on 16 Kio is not an OS-level output quota. #317 and V1 #323 remain NO-GO; ADR-0035 stays PROPOSED; no production migration or merge is authorized.

References: [Microsoft UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute) (HANDLE_LIST needs inheritable handles and `bInheritHandles=true`); [CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw) (STARTUPINFOEX extended attributes and valid std handles); [Windows handle inheritance](https://learn.microsoft.com/en-us/windows/win32/procthread/inheritance).

**Next narrow risk work:** synthetic retained stdout/stderr grandchild ownership/EOF, early native failure while carrying both attribute lists, and interleavings between session admission/stop and thread resume. Decisions about asynchronous revocation and shutdown responsibility remain a separate architecture gate. Do not wire this test-only function to production GitRead.


## 10. Root-exited, descendant-retained stdio (23 October 2026)

**Exact-source proof:** [hosted Windows Rust CI #39898344023](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39898344023) completed **success, attempt 1** at source `8a3dc8f21c41b984a4b0074557f1460607b4f760`. Rustfmt, locked Windows test compilation, strict Clippy with warnings denied, and independent outer-Job supervision all passed. There are now six test-only cases including `retained-stdio`.

The fixed synthetic root Rust test binary launches inside a configured Job via `STARTUPINFOEXW` Job-list and selected standard-handle attributes. After resume, it emits a root marker, spawns an exact self-test descendant with inherited stdout/stderr, waits for the descendant's atomically published readiness, then exits normally. The descendant writes fixed markers to BOTH inherited streams and remains alive, thereby retaining the writers. No shell, untrusted executable, real project, or user PC is involved.

**Before stop**, the owner requires: (1) `GetExitCodeProcess` on the exact root handle returns **0**, (2) inner Job active count is **1** (the descendant), (3) root and descendant markers are present in bounded captures, and (4) **neither** pipe has EOF. This demonstrates that *root process exit is not a valid EOF/Job quiescence fence*.

The owner then requests out-of-band `TerminateJobObject` on its own Job, never by guessing a descendant PID. It polls until **both stdout/stderr EOF and an empty inner Job** are proven under finite deadlines and a combined **16 KiB test assertion**. The independent external supervisor pre-owns the Rust test host before any child is allowed to spawn, terminates only that exact host after the case has finished, and verifies outer Job empty. Logs show `retained_descendant_stopped_and_both_pipes_eof=true`, `external_jobs_cleaned=true`, and `complete=true`; all five prior synthetic modes also passed on this exact SHA.

**Preserved first red:** [#39898204819](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39898204819) and [#39898270581](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39898270581) failed at Rustfmt on older SHAs before compiling. The subsequent distinct SHA above passed without weakening any test.

**Limits:** this is still a `#[cfg(test)]` platform crate fixture. It does not correct production `HardenedCommandRunner` or `PreparedReadCommand`, guarantee bounded shutdown under permanent native observation failures, resolve admission/revocation versus resume, prove stdin backpressure, handle host crash during `CreateProcessW`, certify Windows 10/11 physical clients, or establish authorized GitRead executable/cwd/environment pins or sensitive-patch provenance. A test assertion is **not** an OS-enforced output quota. #317/#323 remain NO-GO, ADR-0035 PROPOSED, PR #327 separate.

**Next isolated step:** test failures inside `CreateProcessW` **after configuring both Job-list and stdio HANDLE_LIST attributes**, proving no child, no inherited pipe retention and no persistent handles after repeated failure. Then define a proper revoke/start/resume ownership contract before any production migration.


## 11. Dual-attribute CreateProcessW failure rollback (23 October 2026)

**Exact-source evidence:** hosted Windows [Rust CI #39898791128](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39898791128) completed **success, attempt 1** at `7286dc405d88998af0224efc811c432a73c12cc3`. This exact-source run passed full workspace Rustfmt, locked Windows test compilation, strict Clippy with denied warnings, and seven independent outer-Job-supervised Rust fixture cases. The new case is `stdio-bad-exe`; all six prior test cases remained passing.

The test constructs the same two native process startup attributes used by the positive stdio path: an already configured `PROC_THREAD_ATTRIBUTE_JOB_LIST` with KILL_ON_JOB_CLOSE/process/memory/CPU limits, plus a `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` containing explicitly inheritable stdin NUL, stdout writer and stderr writer (not an unrelated inheritable spare writer). It then points `CreateProcessW` at a **fixed, explicitly absent synthetic executable**, requiring **ERROR_FILE_NOT_FOUND** after attribute preparation, before any child process exists.

On this deliberately rejected launch, the test closes the parent writers and requires **EOF from stdout, stderr AND the unrelated pipe**, as well as **zero active Job members**. After one warm-up failure, it repeats **16** complete dual-attribute failed creations, always requiring a missing-binary error and no Job child. `GetProcessHandleCount(GetCurrentProcess())` before/after the repeated failures is **unchanged**. The pre-armed independent outer supervisor cleans its exact host and proves an empty outer Job (accounting **1 → 0**).

A defensive test branch also retains and terminates the exact native process/thread handles if the invalid executable were somehow unexpectedly accepted; it never resumes that child. The observed success is limited to a finite repeated-failure matrix, and does **not** prove exhaustive native error-path or crash-during-syscall cleanup.

**Preserved earlier red runs:** [#39898710881](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39898710881) and [#39898727564](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39898727564) failed Rustfmt before compilation on earlier source commits. Strict checks were kept; the new SHA above passed on its first attempt. None of this work changes `main`, production `PreparedReadCommand`, permissions, or the separate open PR #327.

**Combined checkpoint:** seven test-only scenarios now cover initial atomic Job membership, ambient no-inherit, invalid Job rejection, missing binary repeated fail, explicitly allowed stdio with non-allowlisted spare handle denial, descendant-retained stdout/stderr after normal root exit until owned Job termination, and failed native creation with both attributes initialized. They do **not** establish session revoke versus resume linearization, uncertainty-proof service Drop/try_shutdown, production GitRead authorization/pins, Windows 10/11 real-device acceptance, or V1 release readiness. #317/#323 remain **NO-GO** and ADR-0035 remains **PROPOSED**.

**Next security gate:** first choose and verify the session-admission/revoke/stop handshake and native uncertainty ownership model; then consider a narrow production read-only Git process-owner migration with explicit pinned executable, cwd, environment, deadlines and native resources. No wholesale research merge or new feature work.
