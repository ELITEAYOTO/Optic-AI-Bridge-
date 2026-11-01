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
