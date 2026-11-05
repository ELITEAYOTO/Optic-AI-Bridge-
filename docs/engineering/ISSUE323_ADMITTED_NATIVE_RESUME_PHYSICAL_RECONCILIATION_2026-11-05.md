# Issue #323 — an admitted native Resume still executes after original stop (5 Nov 2026)

**Research-only kernel evidence. Windows V1 remains NO-GO. This is not a production native issuer or a terminal receipt.**

## Exact baseline and experiment scope

- Parent research commit `503d905649ccf1dc50b7cbc4694710621e149686`, tree `ec8e3fe10e049592b20b65f92bb014e4fa0bc2f0`. This parent introduced an explicitly **test-only**, non-Clone `NativeIssuerPrototype` that closes new CreateSuspended/Resume admissions by CAS vs an irreversible atomic CLOSED bit. Production `LimitedJobObject` and GitRead MCP remained otherwise unchanged.
- At research start: `main=89ea451d89cb125d6517949c83660465315ff475`, [PR #511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/511) **OPEN and unmerged**, its original Windows PR CI [#41885824035](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41885824035) failed. Research-branch CI green never approves PR #511. [Issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) stays OPEN.
- A separately authorized disposable **Windows 10 LAB** ran all the experiments; no installed daily Bridge, Optic Desktop, REL, plugin ChatGPT, live native GitRead route, production process policy, GitHub `main`, release or tag was modified.

## New deterministic real-Win32 interleaving

The shared fixture `exercise_pre_admitted_resume_with_original_job_custody(preterminate_job)` resides only in Windows crate **unit tests**, within `job.rs`. It runs under external per-process supervision.

1. Start a fixed `System32/cmd.exe` root with `CREATE_SUSPENDED | CREATE_NO_WINDOW` and a zero-length genuine file bound to **its stdout HANDLE**. Independently duplicate and retain the **original** PROCESS HANDLE before the wrapper. An emergency root HANDLE owner performs bounded test cleanup even on assertion failure.
2. Construct genuine `Arc<SessionRegistry>`, `Arc<TaskLeaseRegistry>`, `Arc<ProcessManager>` and a `SessionLifecycleManager` sharing exactly those registries. Retain the **original counted `OwnedSessionAdmissionPermit`** and its original stop watch; construct a genuine original `HostStopSource` and watch.
3. **Before the real SessionLifecycle revoker or host-stop publishes**, call the real test-only native prototype's `issuer.issue(NativePhase::Resume, || ...)`. This performs the actual atomic ticket admission and holds one **original RESUME in-flight ticket throughout the entire synchronous native wrapper callback**.
4. Inside that callback, the wrapper assigns the suspended original PID to its **real Windows constrained kernel Job**, observes `Job ActiveProcesses=1`, original root unsignaled and stdout marker zero. At the exact post-Job/pre-native-`ResumeThread` hook, a concurrent revoker calls the real `SessionLifecycleManager::revoke`, publishes the original session and associated TaskLease stop, and remains unable to finish its counted admission drain until the original owner is released. Publish the real original host-stop irreversibly too.
5. Now, **after all those stops are observable**, `issuer.close()` publishes irreversible closure, but the **already admitted** RESUME ticket remains counted `(creates=0,resumes=1)`. The hook returns. The wrapper subsequently invokes its **real original `resume_process_threads` Win32 function**. This is deliberately NOT a new post-close admission; it is the existing pending callback.
6. After native wrapper return, the original issuer's ledger reports `(0,0)` and sealed drain succeeds; separately release the session owner and await/join the real lifecycle revoker. **A completed issuer drain does not imply native root exit, Job empty, output EOF or safe quota return.** The physical root observation and stdout marker are checked independently.

### Control A — explicitly ignored RED: closure is not retroactive cancellation

`job::tests::research_pre_admitted_resume_can_execute_after_original_stop_must_be_blocked`

No kernel stop occurs in the boundary hook. The already-admitted callback executes real native `ResumeThread` **after** session/TaskLease/host stop and issuer closure. Its original process physically exits only **after executing `REAL_CHILD_ENTERED`**, observed from the genuine stdout handle. The test deliberately panics on that violation: **12/12 behavioral RED (exit 101)**. Markers: `ISSUE323_INFLIGHT_RESUME_ADMITTED_BEFORE_STOP`, `ISSUE323_INFLIGHT_STOP_AND_ISSUER_CLOSED`, `ISSUE323_INFLIGHT_LEDGER_DRAINED_NOT_TERMINAL`, `ISSUE323_INFLIGHT_POSTSTOP_EXECUTED`.

This decisively disproves any claim that `AtomicU64` issuer CAS plus close alone guarantees “no physical ResumeThread after stop publication” when a call was already admitted.

### Control B — nonignored GREEN: explicit original Job/root reconciliation

`job::tests::research_pre_admitted_resume_original_job_stop_prevents_child_code`

The **same** already-admitted callback and revoker/host interleaving occurs. The only controlled difference is an explicitly test-owned call to terminate the **exact original kernel Job** after the stops and before the already-admitted callback reaches native Resume. The test does **not** trust the termination request as completion: it waits on the **independently pinned original PROCESS HANDLE**, polls the **same original Job** until `ActiveProcesses=0`, and confirms zero child stdout bytes before returning from the hook. The callback still calls the ordinary Win32 resume function; for this fixture, no suspended child thread remains, so the native wrapper returns an error. The test independently rechecks original PROCESS signaled and marker length zero **after** the call and proves the issuer ticket drained only after return. **12/12 GREEN (exit 0)**. Markers: `ISSUE323_INFLIGHT_ORIGINAL_JOB_PRETERMINATED` and `ISSUE323_INFLIGHT_PHYSICAL_RECONCILIATION_PASS`.

The limited proof is **original Job empty + original root signaled + physical stdout file zero**. This is **not** a two-physical-EOF receipt, nor proof of all descendant stream references, stdin/quotas/handles, or a design where revocation itself triggers the termination. The explicit kernel stop is **test-only**, not a real service-owned revoker binding.

### Preserved previous controls

- New admission after close: `research_prototype_issuer_seal_after_real_lifecycle_and_host_stop_denies_kernel_resume` still physically refuses the native Resume callback and confirms independent original root termination: **12/12 GREEN**.
- Atomic prototype controls for irreversible closure, 512 refused new Create/Resume attempts and bounded in-flight drain: two tests per group, **12/12 group PASS**.

Overall: **48/48 independent externally supervised executions matched expected behavior**, with 12 intentionally RED and 36 GREEN/PASS. Every run has private stdout/stderr and individual SHA-256. External 15-second bound per run with process-tree forced-stop on timeout; **no timeouts**. The evidence manifest `_evidence/P0_ORIGINAL_INFLIGHT_48_RUNS_MANIFEST.json` has SHA-256 **`09F964A1E5199BBD666405940FFBC1AF98D42B3801D526CD4345B5492B0FE9AF`**. Do not publish raw private evidence.

## Exact source and test rules

The test-only shared helper is not part of production. The default Windows unit suite executes the positive Job-custody control but **skips** the intentionally RED counterpart.

```powershell
cargo +1.99.0 test --locked --offline -p optic-bridge-windows --lib job::tests::research_pre_admitted_resume_original_job_stop_prevents_child_code -- --exact --nocapture --test-threads=1
# Deliberate RED, Windows disposable LAB ONLY, external watchdog REQUIRED:
cargo +1.99.0 test --locked --offline -p optic-bridge-windows --lib job::tests::research_pre_admitted_resume_can_execute_after_original_stop_must_be_blocked -- --ignored --exact --nocapture --test-threads=1
```

The expected exit 101 is a counterexample, **not** test success in the release acceptance sense. Keep all negative fixtures intact; a green ordinary CI skips ignored behavioral RED.

## New constraints on the future service-owned issuer

- One **unforgeable, original-generation service authority** must own the sole native call admission/closing protocol and the **original kernel Job/process/pipe custodians**; merely copying the atomic prototype or keeping an inert `NativeCallStopTracker` alongside it introduces split authority.
- Real session **direct and lifecycle** revoke, TaskLease direct/session/remove/reuse, policy epoch and pinned target, host stop/shutdown, original absolute deadline and failure/poison must share the close decision; a polling loop observing independent flags then calling `issuer.close()` has another admission window.
- Admission and Win32 **kernel entry are distinct events**; once admission precedes revocation, the user-mode caller may reach `ResumeThread` after stop. A single CAS cannot linearize the kernel call itself against concurrent stop. Revoke must promptly publish stop and initiate **actual original Job/root containment termination** or equivalent fail-closed ownership protocol, while allowing already-admitted calls to be counted and safely reconciled. Never hold registry/JobStore/policy mutexes across Win32 Create/Resume, hash, pipe I/O or waits.
- No process terminal receipt until the original PROCESS HANDLE is signaled, original Job is empty, **both physical stdout and stderr pipes reach independent EOF**, stdin/readers/writers/handles and quotas are accounted, **both** Create/Resume native tickets have retired, and a post-stop generation cannot restart work. Any unproven state is **UNKNOWN**, not a terminal success. A successful termination request or an empty native ledger alone is insufficient.
- The next real integration acceptance is **nonignored Git-through-MCP** on Windows with post-revoke code-execution denial, in-flight syscall stress and descendant EOF, with exact SHA/tree first-attempt CI. Keep Windows V1 NO-GO and #323 open, and do not merge blocked #511 or enable live native route on these test-only proofs.

## Local first-attempt validation on Windows 10

On the exact research worktree after introducing the two-case shared fixture, all six bounded Windows first-attempt gates exited **0**: pinned Rust 1.99 formatting; locked/offline Windows crate tests (**88 passed, 9 deliberately ignored**); runtime crate tests; full workspace tests; workspace with all features; and strict Clippy over all targets and features with `-D warnings`. Ordinary gates skip the deliberately RED acceptance. Private per-gate raw stdout/stderr and SHA-256 remain under `_evidence`, with first-attempt manifest SHA-256 **`2DE19AC3A9FC14F1E04AD06A01CCE0E94D48C794E3DA01AF5AC321A45BFA1A28`**. These successes do not establish service-owned authority, complete physical EOF, or Windows V1 release acceptance.
