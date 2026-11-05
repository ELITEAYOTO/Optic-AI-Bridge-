# #323 — real registry revocation at the Windows post-Job Resume boundary (5 Nov 2026)

**Status: RESEARCH ONLY, NOT A FIX. Windows V1 remains NO-GO.**

## Immutable provenance

- Based on research HEAD `845e16c38dffc24123c21f6c0360d70330f97235`, tree `e814bd304ce01fd517f7c239462ebbd1de75151b`, itself a direct child of **unmerged** PR #511 HEAD `292ae8d9fa919f83bb520f5c59800080e89bc555`.
- `main=89ea451d89cb125d6517949c83660465315ff475` when this work started. PR #511 remains OPEN, and the Windows job on first CI attempt #41885824035 failed. Do not use another green branch to merge #511.
- Authorized local Windows 10 private LAB; no application, installer, or production repository was touched.
- Research test: `job::tests::research_real_session_and_tasklease_revoke_before_resume_must_block_code` in `crates/optic-bridge-windows/src/job.rs`. It is explicitly `#[ignore]`, deliberately run with `--ignored --exact`, and is **RED**. Dependencies on `optic-bridge-runtime` and `optic-bridge-core` exist in Windows' **test-only** dependency section.
- Original model-only RED on `845e16c` is left unchanged; this additional fixture exercises real registry methods and real kernel behavior.

## Actual controlled behavior

1. Launch original System32 `cmd.exe` via `CREATE_SUSPENDED | CREATE_NO_WINDOW`, with exact inherited stdout file created empty **before** process creation. Retain a separately duplicated noninheritable HANDLE to the original root exclusively for test emergency cleanup.
2. Verify PID, original PROCESS still unsignaled, and stdout 0 bytes. The existing `LimitedJobObject` wrapper assigns the suspended root to a real constrained kernel Job and verifies `ActiveProcesses=1`.
3. In the existing deterministic `before_resume` hook, call real `SessionRegistry::revoke(&original_session)` and `TaskLeaseRegistry::revoke(&original_lease)`. Confirm `true` from both and that a retained original `SessionAdmissionStopWatch` sees the revoked session, while subsequent registry active lookups reject the original session and lease. The original admission permit used to capture the watch is **dropped before** the direct revoke; this is not yet a test with a counted in-flight owner.
4. The existing wrapper does **not** read these registries, has no common issuer fence, and still calls real `resume_process_threads` / `ResumeThread`. The actual child writes `REAL_CHILD_ENTERED` after the real registry revocations; original HANDLE signals and the kernel Job owner is retained during observation.
5. The test intentionally panics because the child executed after the cancellation boundary. The test-only independently pinned original HANDLE is used for emergency cleanup. Neither the marker alone nor the kill request is a terminal certificate.

**Physical result:** first explicit run: `running 1 test`, `FAILED; 0 passed; 1 failed`, exit **101**, `ISSUE323_REAL_REGISTRY_REVOKE_PUBLISHED` followed by `ISSUE323_POSTSTOP_RESUME_EXECUTED` and `REAL_CHILD_ENTERED`. **12/12 independently invoked RED tests** satisfy all these predicates. Raw stdout/stderr and their SHA-256 values are retained only in the local `_evidence` directory. SHA-256 of the repeat manifest `TRUE_REGISTRY_REVOKE_12_RED_MANIFEST.json`: `EF2659C194A06B896B61860622BC5F5258C78863B0905415B25240AE5D3B12AC`. Do not upload private logs.

## Authority and lock-order audit — observed source, not a proposed fix

| Origin / current entry point | Publication and serialization observed | Missing for native Create/Resume |
| --- | --- | --- |
| Direct session `SessionRegistry::revoke` | Sets irreversible record bit + original `Arc<AtomicBool>` under `sessions` mutex, then drains counted admissions using a `Condvar` (wait releases lock) | Existing Windows Job wrapper cannot consume or commit against original session record |
| Lifecycle `SessionLifecycle::revoke` | First `publish_revocation_before_drain`, then `TaskLeaseRegistry::revoke_session`, then `ProcessManager::cancel_session`; drains session admission; repeats lease/process sweep for late registrations | Separate calls are not a single Create/Resume admission boundary; second sweep is cancellation, not an OS syscall fence |
| Direct TaskLease `revoke`, `revoke_session`, `remove_session` | Each publishes original lease stop under `leases` mutex before return/removal | No common issuer closure with direct session, policy, Job, host |
| Joint original observation `SessionRegistry::observe_joint_original_native_owner_under_revoke_locks` | Locks session mutex then **try-locks** lease mutex; refuses contention; no native syscall under either | Returns only a snapshot. Revocation after unlock can still precede `ResumeThread` |
| Host stop `HostStopSource::request_stop` / Drop | Irreversible atomic publication from trusted source; no caller-supplied reset | Independent of the session/lease lock and OS-call commitment |
| Policy epoch, deadline, path/Git pins | Checked at trusted preparation/snapshot, original horizon retained; policy evaluation outside registry locks | No atomic final policy and OS syscall fence; do not recalculate/renew original horizon |
| `NativeCallStopTracker` | Original stop flags and ledger counts; seals and drains accounted tickets, refuses poison/overflow; inert, **not a launch permit** | Not attached to live MCP or Windows Resume. Does not synchronize external revokers with `ResumeThread` |
| `LimitedJobObject::wrap_child_with_before_resume` | Real `CreateProcessW` suspended, then `AssignProcessToJobObject`, then `resume_process_threads` | No read/commit of original session/TaskLease/host/policy authority, or owner-held admission across the Resume effect |
| `ProcessManager::stop` / `cancel_session` | Per-job stop atomics, job-store/state lookup, repeat cleanup in lifecycle | Stop request is not root exit, Job empty, two physical EOF, or closed future native effects |

**Existing lock order:** the joint observation is `SessionRegistry.sessions -> try_lock(TaskLeaseRegistry.leases)`; lease publication does not call back into the session registry while holding its mutex. Do not introduce `lease -> session` acquisition or hold any registry, JobStore or policy mutex across `CreateProcessW`, `ResumeThread`, path hashing, pipe waits, callbacks, or kernel stop waits. A blocking native call may finish *after* stop publication; that is an in-flight effect to reconcile, not a retroactively forbidden admission.

## Acceptance matrix for the future single service-owned issuer

Every source below must close **the same original-generation native admission gate**, not merely toggle independent flags. Each cell describes a REQUIRED behavior, **not** a passed test.

| Stop source | Before Create admission | Create already in kernel | Suspended root assigned, before Resume | Resume already in kernel | Running/root or descendant present |
| --- | --- | --- | --- | --- | --- |
| Direct SessionRegistry revoke | deny Create, zero child | stop promptly; count ticket until return, own and kill suspended result | no new Resume; original root retained | track in-flight Resume and reconcile physical effects | stop original Job; retain all output and quota until terminal |
| Session lifecycle revoke / reap | same, plus late registration sweep | same; no early authority cleanup | same | same | same; direct/lifecycle contracts must agree |
| Direct TaskLease revoke / session revoke / removal + ID reuse | reject original Arc generation, never rebind ID | same counted custody | no Resume despite new lease using same ID | reconcile admitted effect, never reissue new admission | original lease stays revoked, no quota release without proof |
| Policy epoch mismatch or target identity changed | deny before effect; no cached Allow as permit | previous admission counted; exact pinned identity retained | no Resume if mismatched | reconcile original effect | fail closed; never disclose sensitive Git metadata |
| HostStop direct/shutdown/Drop | deny, nonrenewable host origin | publish stop promptly | deny Resume | reconcile effect | guardian owns Job through real exit and EOF |
| Original absolute deadline | deny, never extend | in-flight may return late; never Resume after expiry | deny Resume | reconcile and stop | Unknown if original deadline passed without physical facts |
| Job/process stop, uncertain kernel kill | deny new native calls for original Job | preserve original PROCESS, Job and ticket | do not run child | reconcile | `TerminateJobObject=Ok` alone is insufficient |
| Ledger poison/overflow/crash/lost custodian | fail closed, permanently seal | Unknown, not zero | no Resume | Unknown, never invent drain | no terminal success or quota recycling |

**Required terminal conjunction:** no further admitted Create/Resume + every prior syscall returned/reconciled + same original root signaled + same Job active count 0 + independent physical stdout EOF + independent physical stderr EOF + stdin/writer/handles accounted for + original budget and permits released only after evidence. A Job kill request or one EOF is not enough.

## Next research test and release gating

The next meaningful escalation must hold the **original counted `OwnedSessionAdmissionPermit`** while invoking a real **concurrent** session lifecycle/direct revoker and synchronizing a real kernel Job-assigned, suspended root before Resume; prove stop publication while the old admission remains counted. It must then test TaskLease removal and ID reuse, host stop, policy epoch mismatch, in-flight Create/Resume, poisoned ledger, descendants retaining pipe writers, and guardian failure. A successful unit fixture with direct revocation but no retained owner does **not** satisfy those cases.

The eventual correction needs one noncloneable service-owned issuer and actual common close/commit order for all native effects, never a lone `if stop.load()`. Keep real GitRead MCP using the legacy runner until a separately supervised, nonignored **real Git after revoke** acceptance (#355) is GREEN with physical terminal proof. PR #511 remains separately blocked by first-attempt Windows CI, issue #185 remains open. No PR merge, production route change, tag, release or V1 GO from this research.
