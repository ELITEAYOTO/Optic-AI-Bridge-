# Windows V1 — #317 physical Job/pipe evidence, 2026-11-01

> This page describes **synthetic Windows kernel behavior**, not a working GitRead guardian.
> Source baseline before this test-only increment: `main=e857fa10da81420deaa7a47cac9a8f189d5ef5ae`, tree `e89560b73914b40650ac9ed502a26ecb7e5cbe5b`.
> The independent post-merge #475 [push CI](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41344202374) completed **SUCCESS, first attempt**.
> Check the actual GitHub main SHA and CI again after this documentation changes.

## Current chain and production limits

- [#473](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/473) fixed GitRead status on deep Windows paths; [#475](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/475) fixed the deep worktree verification by reading bounded registered Git metadata from the pinned parent repository. [#474](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/474) is closed.
- The **real production** `crates/optic-bridge-runtime/src/git/read.rs` still launches `GitReadService::status_with_max_bytes` through `HardenedCommandRunner`. Nothing here turns on the experimental native owner.
- [Draft #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355)'s ignored GitRead acceptance and independent Windows LAB on main still demonstrate the **RED** revoke-before-spawn contract: Git can launch after session revoke has been published. No linearizable authorization gate spans the original session/TaskLease, policy, host-stop, and real `CreateProcessW/ResumeThread`.
- [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains Windows V1 **NO-GO**. [#471](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/471) stays draft, pending policy approval about Git diff content. No release, installer or activation.

## Additional real Windows OS proof (test-only)

The existing `crates/optic-bridge-windows/src/job_list_launch_tests.rs` already has real native Windows Job-list process creation, watchdog, descendant and physical EOF acceptance scenarios. The new increment is **not the first root/Job/EOF test**. It explicitly connects the *pre-created* `PrecreatedNativeReadCustody` to a suspended synthetic root and proves that independently retained **stdout and stderr writer handles** each prevent terminal certification despite root exit and an empty Job.

The new `job::precreated_native_read_tests::native_317_real_suspended_root_job_and_both_physical_pipe_eofs` test, only compiled on Windows, runs an **actual copy of the Rust test binary** as a disposable synthetic child, not Git. It uses a private `PrecreatedNativeReadCustody` Job and original anonymous pipes, duplicates the original writer handles to the child, creates the child **suspended**, assigns it to the exact kernel Job **before** resuming, pins the real still-alive root HANDLE and confirms actual stdout/stderr marker bytes. The fixture writes markers and sleeps; it must only be stopped by the parent-owned Job watchdog. The parent does not make any network requests or use a real operator project.

The test checks:
1. Before watchdog termination, `confirm_post_watchdog_physical_stop` **denies** an invented success even with a pinned root.
2. On an original stop flag, the independent watchdog reports the exact successful `TerminateJobObject` result. The retained original root HANDLE becomes signaled and the Job becomes empty.
3. **Two deliberately retained extra writer handles** keep the original stdout and stderr physical EOF unobserved despite the completed kill and empty Job. The first bounded receipt must timeout. Releasing only the stdout duplicate must still timeout: stderr is an **independent terminal gate**.
4. Only after both extra duplicates are closed may the receipt succeed, proving original root exit, Job empty, stdout EOF and stderr EOF. `Child::wait` reaps the root; its exit is non-successful, as expected from Job termination.

The **first** local test attempt returned RED after 5 seconds because the test's own `std::process::Command` retained inheritable writer duplicates in the **parent**. The actual Job termination was insufficient to cause pipe EOF with a live extra writer. Fixing this **fixture-only ownership leak** by dropping the command object after spawn made the positive four-fact proof pass. The later negative writer-retention test makes that finding explicit, without changing watchdog deadlines or production code.

## Independent push/main RED after #476: asynchronous root exit

[PR #476](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/476) merged at `969cd5487bb7c3e5681b2cdad1ac12d0d7914bc0` only after four exact-head first-attempt workflows were all SUCCESS, including Windows, native lifecycle and pipe tests. However **the first independent push/main CI [#41346391693](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41346391693) failed on Windows at the exact new test**, while Ubuntu, dependency and release-evidence jobs passed. The log records **68 passed / 1 failed / 4 ignored** in the Windows crate, failing the unconditional `root.try_wait().is_some()` assertion immediately after the watchdog reported successful `TerminateJobObject`.

**Root cause:** successful kernel termination request is **not synchronous root exit**. The test made a scheduling assumption contradicted by the independent hosted runner. This is a genuine test contract bug, not a basis for changing the production watchdog or claiming #317 fixed. The source-visible corrective slice replaces the instantaneous assertion with a bounded polling observation of **both original root exit and original Job empty** under the **existing same five-second deadline**; only then does the test intentionally hold stdout/stderr open to verify that each individual EOF remains required.

**Evidence retention:** never re-run the failing `main` run to manufacture green. A correction requires a **new commit**, exact-head hosted Windows/Ubuntu CI, and separately verified push/main. Windows V1 remains **NO-GO** regardless of green test results.

## Follow-up, 2 November 2026: deadline after physical observations

Independent verification of the exact [#477 push/main CI](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41347680694): **SUCCESS 4/4, first attempt**, on integrated `main=dfff32f4affb522ab2728151b72c054559af36a9` and tree `75c4527340ad1507803118aa151139df88b42f36`.

Further Windows-only research discovered a separate **physical cleanup deadline** issue in the inert `PrecreatedNativeReadCustody::confirm_post_watchdog_physical_stop`: after querying the pinned original root, actual kernel Job accounting and both physical EOFs, the loop returned an `Ok(NativePhysicalStopReceipt)` **before** checking whether the nonrenewable absolute `cleanup_until` had already elapsed during those observations. This can publish a success with an out-of-budget completion; it does **not** add a new process creation or change the GitRead production path.

**Deterministic behavioral RED:** extend the existing real suspended-root/Job/both-pipes test. After attaining an authentic successful four-fact receipt, call a private clock-parameterized variant using the exact **same real handles and EOFs**. The first test clock observation is strictly before the cleanup horizon; the next one, after the OS reads, is strictly after. The original method returned **Ok** instead of `TimedOut`: test **FAIL exit 101**, and the genuine root Job/pipe proof remained intact. No output fakes, sleeps added to hit a race, credential data or external tests.

**Minimal production-native fix:** keep the existing `Instant::now` clock for the normal public method and place the deadline check **before** even the positive four-fact receipt, after the real OS/pipe observations. The clock parameterization is private to the same Rust module and exists only to make the test deterministic. No budget increase, no discarded checks, no new spawning API, no policy or legacy GitRead change. `Err(TimedOut)` retains native ownership for fail-closed teardown, as before. New targeted proof: **1/1 real Windows test PASS** using the identical test filter and kernel scenario.

**Still NO-GO:** even an in-time root-exit/Job-empty/stdout-EOF/stderr-EOF receipt **does not prove no future `CreateProcessW/ResumeThread`**. The separate synchronized authority issuer is absent; the draft revoke-before-spawn regression remains RED. See #323 and draft #355. Do not declare V1 ready from this increment.

## 2 November 2026 — issuer/revocation audit after #478

Baseline verified **independently**: `main=c1a40c93d4a2072faa231aec932bee6f6b08ccc1`, tree `9bf3b9211d94c8702556bf3d5815ace18270c653`, [post-squash CI #41407214849](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41407214849) **SUCCESS 4/4, first attempt**. [PR #478](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/478) only hardened the physical-stop receipt deadline; this did not make GitRead native.

### Direct source findings: four different authority boundaries

1. `session/registry.rs::publish_revocation_before_drain` and `SessionRegistry::revoke` serialize the original session stop-bit publication under **the session registry mutex**. The latter waits for existing counted admission permits to retire. A new owner cannot retain a permit indefinitely without a separate out-of-band stop/liveness plan.
2. `task_lease_registry.rs::{revoke,revoke_session}` publish the TaskLease generation stop-bit under a **different mutex**. `commit_original_native_read_lease` supplies only a lease-local observation, not a combined session-and-lease launch grant. An opaque ID can be removed/reissued, but an old owner must not borrow its replacement's authority.
3. `git/native_status.rs::LeaseObservedNativeGitStatusCustody` retains the original session and TaskLease receipts and configures an original Job watchdog; the `NativeCallStopTracker` counts admitted `CreateSuspended` and `Resume` operations. Its `request_stop` closes **future ticket admission** but explicitly does **not wait** for an already-counted native call to return.
4. `git/read.rs::GitReadService::status_with_max_bytes` still executes `HardenedCommandRunner`. Neither session/lease snapshots nor a real empty Job make the current production Git operation revoke-linearizable. [Research #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) remains intentionally RED, not a passing acceptance test.

### Missing protocol before another code PR

- Establish a **service-owned original-generation issuer** whose *all* direct and lifecycle revokers use the **same** close/commit serialization. Prove policy epoch, transport and host-stop are ordered as well; do not hold a registry/JobStore lock across potentially unbounded Win32 calls.
- Distinguish **stop publication** from **bounded in-flight native-call drain** and **physical termination**. A future sealed native-call ledger should close admission before acquiring its counter lock, then wait for *both* already admitted `CreateSuspended` and `Resume` calls to return before publishing a nonforgeable, original-owner-specific drain receipt. A poisoned counter must be **unknown**, not zero; timeout preserves the original resource owner.
- Even such a ledger receipt is **not** a real session/lease/policy/host revoke fence and **not** proof of root exit, Job empty, stdin completion, dual EOF or no future native call from a *different* issuer. Its integration with all original revocation publishers must be designed and tested before it is called a guarantee.
- Acceptance requires deterministic **revoke-before-create**, **create blocked inside syscall**, **revoke-before-resume**, **resume blocked inside syscall**, **simultaneous direct/lifecycle/lease/host revoke**, **expiry**, **ID reuse**, **poison**, and **externally supervised real Git**. Failures retain original Job/permits/quota. The eventual correct implementation must make the existing #355 real-Git acceptance GREEN without suppressing its first red.
- No experimental half-implementation has been published from this audit; the isolated 2 November LAB was restored to the exact baseline after a development-tool edit interruption. Avoid interpreting a compile-red missing-method probe as a demonstrated production defect. It established only that there is currently no synchronous ledger drain API.

**Release state remains NO-GO** under [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). The historical [#317 issue](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is closed in GitHub, but the larger Windows native GitRead owner acceptance is **not** completed; track that release blocker explicitly in #323 and research #355. No daily Bridge, Desktop, REL, installer or policy changes were made.

## Limits and next P0 implementation

This test does **not** prove a native Git launch, policy identity, cancellation, no-future-create/resume fence, handling an in-flight Win32 syscall across revoke, watchdog on host crash, real Git stdout/stderr output limits, or independent terminal proof for a Git subprocess tree. It is evidence that the existing OS custody *can* complete correctly and *refuses* success if either pipe stays open.

**Next mandatory implementation:** a trusted native GitRead issuer that retains exact pinned executable/repository, original session/TaskLease/policy/transport/host stop and absolute horizon, synchronizes revocation against OS creation and resume (including calls in flight), assigns to Job before resume, and holds all original permits and handles until a no-future-spawn fence plus physical receipts. Prove the draft #355 regression GREEN only with actual in-scope Git behavior. The V1/Release gate #323 stays NO-GO until all independent acceptance, security and release requirements are met.

## Reproduction / isolation

Use pinned Windows Rust 1.99, a fresh disposable clone and its own `CARGO_TARGET_DIR`; use a short synthetic `TEMP/TMP` to avoid unrelated Git MAX_PATH fixture failures.

```powershell
cargo +1.99.0 test --locked --offline -p optic-bridge-windows --lib native_317_real_suspended_root_job_and_both_physical_pipe_eofs -- --nocapture --test-threads=1
cargo +1.99.0 fmt --all -- --check
cargo +1.99.0 clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo +1.99.0 test --locked --offline --workspace -- --test-threads=1
```

Require **exactly one executed test**, not a successful zero-test filter. No self-hosted runners, workstation deployment or edits to the daily Bridge, Desktop or REL. Merge only after first-attempt exact-head hosted Windows/Ubuntu CI and independent main verification.
