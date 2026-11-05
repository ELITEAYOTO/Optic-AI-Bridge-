# Issue #323 — lifecycle, TaskLease and original host stop vs Windows Resume (5 Nov 2026)

**RESEARCH-ONLY / intentionally RED Windows acceptance. NOT a fix, not a native issuer, not an MCP permission. Windows V1 NO-GO.**

## Exact provenance

- Research parent `79ef671b811a3ae0225832c0df3c86d4ebb1c540` (tree `7712eaa706259a5537c035b1e72dd9d72ead6a2d`), after the prior 12/12 original-counted direct `SessionRegistry::revoke` experiments. Parent itself descends from `c5b2ab9`, `845e16c` and the still-unmerged PR #511.
- At research initiation: `main=89ea451d89cb125d6517949c83660465315ff475`; [PR #511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/511) still OPEN, first-attempt PR Windows CI [#41885824035](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41885824035) failed; #323 OPEN. A later research-branch green run does not approve #511.
- Authorized isolated Windows 10 LAB; no changes to installed daily Bridge, GitRead MCP routing, Windows packaging, security policy or privileged execution API.
- This change adds **one deliberately ignored acceptance test only**, plus dated documentation. All new OS process effects occur solely inside disposable Windows tests.

## New real OS negative test

`job::tests::research_lifecycle_and_host_stop_with_counted_owner_pre_resume_must_deny_code` (intentionally `#[ignore]`, opt-in only).

1. Create fixed original `System32/cmd.exe` with real `CREATE_SUSPENDED | CREATE_NO_WINDOW` and `echo REAL_CHILD_ENTERED`. The parent creates a zero-length stdout marker **before** child creation and passes the physical stdout handle to the child. A separately duplicated non-inheritable original PROCESS HANDLE remains owned by test emergency custody.
2. Construct real `Arc<SessionRegistry>`, `Arc<TaskLeaseRegistry>` and `Arc<ProcessManager>`, passing those exact registries into a real `SessionLifecycleManager::new`. Register the original session and its TaskLease, then acquire a genuine counted `OwnedSessionAdmissionPermit` and retain its original-generation stop watch. Independently construct a real `HostStopSource` and capture its original `HostStopWatch`; no mock `AtomicBool` substitutes for its production implementation.
3. The existing Windows `LimitedJobObject` assigns the suspended root to a real constrained kernel Job and verifies the original PID, unsignaled PROCESS handle, original Job active process count **1**, and physical stdout marker length **0** at the exact post-assignment/pre-`ResumeThread` hook.
4. Inside that hook, a concurrent thread calls real **`SessionLifecycleManager::revoke(&original_session)`**. Synchronize on thread entry; boundedly observe the original session stop published **before** the counted admission drain, plus refusal of all future original session admissions. Independently and boundedly observe the registered TaskLease become inactive through the **lifecycle** revoker (not direct TaskLease::revoke). Revoke remains blocked while the original counted permit is retained.
5. Publish actual `HostStopSource::request_stop()` and assert irreversibility and the original host watch. At this point the **session, lease, host** stop sources are all observed BEFORE native Resume.
6. The original native wrapper still executes `resume_process_threads` / **actual `ResumeThread`**. It is not wired to the stopped original session/lease/host owner. The genuine permit remains counted throughout the kernel effect. After that call returns, drop only the owner permit, observe the lifecycle revoker drain and return with `session_changed=true` and `revoked_leases=1`, then join its thread.
7. The same pinned root physically exits and outputs `REAL_CHILD_ENTERED`, proving code executed AFTER all stop sources were published; the deliberately ignored test panics. The independent original HANDLE provides test-only emergency cleanup. The Job is retained for observation; a kill acknowledgement alone is never called a physical terminal receipt.

Result: **12/12 deliberately RED (exit 101)**, verified each time using required explicit markers `ISSUE323_LIFECYCLE_HOST_STOPS_PUBLISHED` (`session=true, lease=true, host=true, job_active=1`), `ISSUE323_LIFECYCLE_DRAIN_AFTER_OWNER_DROP`, `ISSUE323_LIFECYCLE_HOST_POSTSTOP_EXECUTED`, and actual `REAL_CHILD_ENTERED`. Each run has separate private stdout/stderr and SHA-256. Externally supervised 15s per test; no observed timeout. Evidence manifest `LIFECYCLE_HOST_12_REDS_MANIFEST.json` SHA-256 **`FFC7C4461B55913C634A0739D4FB80B888688DC839E596BE70FA43E9D1D8A013`**.

## Existing ID-reuse positive control (not a new native OS proof)

The pre-existing `task_lease_registry::tests::native_original_lease_receipt_survives_id_reuse_and_foreign_registry` was explicitly rerun on the Windows 10 LAB and **PASS 1/1 (exit 0)**. It demonstrates original lease-receipt protection across revocation/removal, opaque ID reuse and foreign registries in the runtime test. **No claim** is made that a reused lease ID was exercised *during* this new physical Windows Resume interleaving. Positive-control stdout SHA-256 `14D89EDD9D38800602A46F26B9E0EB5011673C4D9361F9310EAB75B9433B633F`; stderr SHA-256 `00427B4A2F1AD82E3F2AB399ED9C64EF13728507DB8609421D1BA27CC6CF8FB0`.

## Security claims explicitly NOT established

| Boundary | Status |
| --- | --- |
| SessionLifecycle directly publishes real original session and TaskLease stop promptly, before counted-drain completion | **Observed** (fixture) |
| Original host stop observed irreversible before native resume | **Observed**, using in-process `HostStopSource::request_stop`; **not** complete process-host shutdown |
| Actual `ResumeThread` still executes original child code after three stops | **RED, 12/12**; intentional acceptance failure |
| Session revocation completes only after original counted owner dropped | **Observed** |
| TaskLease same-ID reuse/foreign registry never revives old generation | **Existing separate runtime unit PASS**, not same native interleaving |
| Service-owned single non-cloneable original-generation native issuer with common close/admission and final stop commit | **NOT IMPLEMENTED** |
| Direct TaskLease removal/reuse concurrent with this physical OS syscall, policy epoch, wrong target, host peer Drop and actual app shutdown | **NOT PROVED** |
| CreateProcessW and ResumeThread in-flight native tickets reconciled with all revokers | **NOT PROVED** |
| Original root signal + Job empty + separate physical stdout AND stderr EOF + stdin/handles and quota retention + crash recovery | **NOT PROVED in this test** |
| Real Git after revoke through MCP #355 (unignored Windows acceptance) | **NOT PROVED** |

**Interpretation:** an original session admission retaining revocation cleanup is NOT an execution fence. Independent flags and snapshots cannot prevent `ResumeThread` after external stop publication; likewise a green ordinary CI which deliberately skips this `#[ignore]` RED cannot attest live native GitRead.

## Reproduction & next work

Run only under bounded, isolated Windows kernel process supervision, with emergency original root handle cleanup:

`cargo +1.99.0 test --locked --offline -p optic-bridge-windows --lib job::tests::research_lifecycle_and_host_stop_with_counted_owner_pre_resume_must_deny_code -- --ignored --exact --nocapture --test-threads=1`

The expected `101` is an intentional **security counterexample**, not a passing integration gate. Test output and SHA-256 live in the private LAB's `_evidence`; do not upload raw logs.

The correct next design needs **one service-owned noncloneable original-generation native-call issuer** with a shared close/admission boundary across the *real* session direct/lifecycle and TaskLease direct/session/removal, policy epoch/target identity, host stop, Job/internal watchdog and original unrenewable deadlines. Never hold authority/JobStore/policy mutexes across Win32 CreateProcessW/ResumeThread, filesystem hash, pipe IO or waits. A call admitted before stop may complete later; retain its ticket and original Job/process/pipe custody, never resume a suspended root after stop; on timeout/poison/failed kill classify **Uncertain**, not terminal. A correct real Git MCP #355 and physical two-stream EOFs remain mandatory before any V1 GO or native route activation.

**Do not merge PR #511, tag/release, lower negative coverage or switch production GitRead to the native owner. #323 remains OPEN, Windows V1 NO-GO.**

## First-attempt local regression gates (Windows 10)

On the exact research worktree, the six gates ran and returned exit **0** in their **first** attempt: pinned Rust 1.99 formatting; locked/offline Windows crate tests (**84 passed, 8 intentionally ignored**); locked/offline runtime tests; full workspace tests; workspace all-features; strict Clippy all targets/features with `-D warnings`. Private stdout, stderr and per-file SHA-256 are preserved for each gate in `_evidence`. The unmodified first-attempt gate manifest `FIRST_ATTEMPT_GATES_MANIFEST.json` has SHA-256 **`3A4312A5CDCD90133ACBBAAF6BFA1E2C35DBF82E989652130B9533E421C2DAFC`**. A GREEN ordinary suite, with the deliberately RED test skipped, is **not a production security gate**.
