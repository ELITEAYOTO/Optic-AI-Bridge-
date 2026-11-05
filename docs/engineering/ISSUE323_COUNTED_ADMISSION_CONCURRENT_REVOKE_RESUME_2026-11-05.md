# Issue #323 — counted original admission vs concurrent revoke at the real Windows Resume boundary (5 Nov 2026)

**Status: security RESEARCH / deliberately RED acceptance, NOT A FIX. Windows V1 NO-GO.**

## Provenance and scope

- Baseline: exact research commit `c5b2ab9672c2f96a4774128ab14a1343e62e1205`, tree `a0491ed180890adc3372198365203ec3cab8f606`; its parent is `845e16c38dffc24123c21f6c0360d70330f97235`.
- The production `main` was `89ea451d89cb125d6517949c83660465315ff475` at start. PR #511 remains **open/unmerged** with failed Windows CI first attempt [#41885824035](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41885824035). These observations do **not** approve that PR.
- This experiment runs in a separate authorized Windows 10 LAB; no live MCP route, user installation, release, policy or production process permission was altered. `optic-bridge-windows` has existing **dev-only** access to `optic-bridge-runtime` and `optic-bridge-core` for the research tests.
- Reference: [prior real registry direct-revoke research](ISSUE323_TRUE_REGISTRY_REVOKE_RESUME_RESEARCH_2026-11-05.md), which deliberately **dropped** its admission before revoking. This experiment now retains the genuine original counted `OwnedSessionAdmissionPermit` across the native `ResumeThread` attempt.

## Two distinct proofs

### Positive, nonignored PASS: prompt stop publication vs original drain

`job::tests::counted_original_session_revoker_publishes_stop_without_premature_drain`

Uses the actual `Arc<SessionRegistry>` and `begin_owned_admission` to retain the **original** counted permit plus its stop watch. A genuinely concurrent worker invokes the real direct `SessionRegistry::revoke`. The test synchronizes on explicit worker start, then boundedly observes the original stop flag, checks that active session admission is now denied and that revoke has **not** returned while the permit is owned. Only after `drop(admission)` may the real direct revoker finish and return `Ok(true)`; the worker is joined. No native process is created by this positive control.

Result: **12/12 PASS (exit 0)**; exact evidence marker `ISSUE323_COUNTED_DRAIN_PASS`.

### Deliberately ignored behavioral RED: physical Windows root resumes after stop

`job::tests::research_owned_counted_admission_concurrent_revoke_pre_resume_must_block_code`

1. Constructs a real, separately retained original `OwnedSessionAdmissionPermit` on `Arc<SessionRegistry>`, a real registered TaskLease, and original session watch.
2. Creates fixed Windows System32 `cmd.exe` with `CREATE_SUSPENDED | CREATE_NO_WINDOW`. The stdout marker file is opened zero-length by the test **before** creation; its handle is bound as the real child's stdout. An independently duplicated noninheritable original PROCESS HANDLE provides bounded physical observation and emergency cleanup.
3. The existing `LimitedJobObject` assigns the **same suspended root** to a real constrained kernel Job. Hook `wrap_child_with_before_resume` verifies original PID, Job active process count 1, root unsignaled, and marker length 0 **before** native Resume.
4. Inside that post-assignment/pre-Resume hook, a real concurrent thread starts the direct `SessionRegistry::revoke`. After observing the original stop publication under a bounded timeout, it verifies the revoker has **not** completed while the original counted permit remains held; direct `TaskLeaseRegistry::revoke` also succeeds. Both original active record queries deny.
5. The existing Windows wrapper still enters `resume_process_threads` and performs real `ResumeThread` despite original stop. The test **retains** the counted permit during this native call. Only after the wrapper call returns does it drop the permit, independently await and join the blocked revoker, then check the original root's real exit and marker `REAL_CHILD_ENTERED`.
6. A panic is **intentional**: the invariant “original stop published before Resume means child code is not executed” remains violated. Test-owned original PROCESS HANDLE and Job provide emergency cleanup on failure; a kernel stop request is not claimed as completion proof.

Result: **12/12 behaviorally RED (exit 101)**, with `ISSUE323_COUNTED_REVOKE_PUBLISHED`, `ISSUE323_COUNTED_DRAIN_COMPLETED_AFTER_OWNER_DROP`, `ISSUE323_POSTSTOP_RESUME_EXECUTED`, real child output `REAL_CHILD_ENTERED`. Each run has a separate stdout/stderr file and SHA-256. Runs use external per-run 15-second supervision with fail-closed forced process-tree stop on timeout. This is a **real OS interleaving**, not a fake PID, mocked termination or synthetic revoke flag.

**Both test types passed their intended expectations for all 12 repetitions.** Manifest for the 24 individual executions: `COUNTED_P0_REPETITIONS_MANIFEST.json`, SHA-256 `D3AE6AD340980195E0D72253D66C3DAD4E9C89F665102E4C8540573661F58F6B`, private LAB `_evidence` directory. Do not upload the private raw logs.

## What this does and does not prove

| Observation | Classification |
| --- | --- |
| Original-generation stop is published promptly while a real admission is counted | **PASS**; actual SessionRegistry concurrent direct revoke |
| `SessionRegistry::revoke` cannot finish admission drain before original permit drop | **PASS**, also checked across actual native Resume |
| Original TaskLease record is independently revoked before Resume | **PASS** for real direct `TaskLeaseRegistry::revoke` |
| Windows root is assigned to actual Job before Resume and physically executes after revocation | **RED**, 12/12 |
| Session + TaskLease + host + policy common issuer with synchronized final native admission | **NOT IMPLEMENTED** |
| Concurrent *lifecycle* revoker, TaskLease removal/re-ID, host epoch, policy change | **NOT TESTED HERE** |
| In-flight Windows CreateProcessW and ResumeThread fully tracked and reconciled | **NOT PROVED** |
| Complete root/Job/physical stdout+stderr EOF and stdin/handles/quotas before terminal | **NOT PROVED** |
| Real post-revoke GitRead **through MCP** (#355), sensitive metadata, crash recovery | **NOT PROVED** |

An affirmative session or joint TaskLease observation is not a native execution grant. Original stop publication is not “all native system calls completed”, and an empty ledger or successful `TerminateJobObject` is not physical terminal proof. A native issuer must serialize **call registration and final stop observation** with original-generation authority, while permitting prompt stop publication even if an OS call stalls. Do not hold session, lease, JobStore, policy or stdout mutexes across `CreateProcessW` / `ResumeThread` / waits / hash IO. Already-admitted syscall effects must remain owned and be reconciled, never retroactively denied or counted as completed.

## Exact reproduction and release safety

Standard suite includes the positive control and **skips the deliberate RED**:

`cargo +1.99.0 test -p optic-bridge-windows --lib --locked --offline`

Independently run the acceptance on an isolated Windows machine, with external timeout and retained emergency cleanup:

`cargo +1.99.0 test -p optic-bridge-windows --lib --locked --offline job::tests::research_owned_counted_admission_concurrent_revoke_pre_resume_must_block_code -- --ignored --exact --nocapture --test-threads=1`

The expected `101` is **proof of an unresolved defect**, never acceptable production success. A green CI which skips ignored tests does **not** authorize production native GitRead.

**Next:** design one service-owned noncloneable original-generation issuer with common session/lease/policy/host/Job close-and-admission fencing; prove all direct/lifecycle revokers, expiry and native Create/Resume interleavings, and physical stop/stdio proof. Only after valid real Git-over-MCP nonignored Windows RED→GREEN and exact SHA/tree CI may any production migration be considered. Keep #323 OPEN, Windows V1 **NO-GO**, no merge/release/tag.

## Local first-attempt regression verification

On the exact research worktree before commit, all six bounded Windows 10 gates exited 0 on their **first** attempt: `cargo +1.99.0 fmt --all -- --check`; `test -p optic-bridge-windows --lib` (**84 pass / 7 ignored**); `test -p optic-bridge-runtime`; `test --workspace`; `test --workspace --all-features`; and strict `clippy --workspace --all-targets --all-features -- -D warnings`, all with `--locked --offline` where applicable. Each gate has raw stdout/stderr and SHA-256 in private LAB `_evidence`, manifest SHA-256 `12E4AC2140B0963DF361838E38786602737339432C9A3FB761C87D640A940F50`. Passing the ordinary gates (which skip ignored behavioral RED fixtures) is **not** an authority to merge or enable native GitRead.
