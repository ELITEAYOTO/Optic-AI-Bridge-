# #317 — Owned admission and genuine native reservation in one independent worker

**Status:** TEST-ONLY RESEARCH. No executable-child creation, policy authorization, result publication, durable shutdown, source-controlled release change or V1 acceptance.
**Source base:** `main=100c43b53c795be5678c8b4f2c0af4353ca5b196` (PR 336 fully accepted with independent main CI #40027279676).
**Research antecedent:** `research/317-native-owner-generation-commit-2026-10-23=b6653f7` (Windows+Ubuntu CI #40025477442 green). This branch extracts only the short original-record commit marker and binds it to the *production* `OwnedSessionAdmissionPermit`; it does **not** import the experimental native process launcher.

## Why the new experiment is useful

A `SessionAdmissionStopWatch` does not keep the synchronous `SessionRegistry::revoke` admission count alive. Conversely, an `OwnedSessionAdmissionPermit` alone does not identify an already-registered native process owner. The previous short commit proof used a **borrowed** session permit, so it did not demonstrate the lifetime of a worker independently owning both resources.

This test creates a native `LimitedJobObject` reservation through the real `ProcessManager::reserve_native_private`, which registers it in the genuine JobStore **before** any permission to use it is claimed. The same worker thread takes ownership of that reservation and a real counted `OwnedSessionAdmissionPermit`; no child is spawned.

## Invariants checked at the shared close/commit lock

The `#[cfg(all(test, windows))]` method `OwnedSessionAdmissionPermit::research_commit_registered_owner`:
- Checks the native reservation's actual JobRecord owner against the admitted session handle.
- Under the **existing** `SessionRegistry.sessions` mutex, checks the same original generation's `Arc<AtomicBool>` record identity, both the original admission deadline and current record deadline, and the recorded revoke state.
- Claims a test-only, one-use atomic marker held on the actual `NativeProcessReservation`. Checks `stop_requested`; performs **zero OS operations, JobStore locks, disk I/O or callbacks** under the session mutex.
- Returns a non-cloneable borrowed receipt that keeps *both* owner objects borrowed while the test holds it; it observes the generation-bound read-only stop signal. A later close requests stop even if an earlier cancellation scan missed registration.

A false stop flag is not a positive authorization. Task lease, policy epoch, pinned executable/repository, typed GitRead effect and result publication remain independent. The claim does not itself create, resume, terminate or release processes.

## Hosted negative and positive schedules

1. **Commit first, then direct revoke:** the actual independent Rust OS thread owns the `OwnedSessionAdmissionPermit` and `NativeProcessReservation`. Duplicate commit is rejected. Direct revoke publishes stop and waits. The worker reports observed stop but retains both resources pending an external test release signal; JobStore still contains the entry and `remove_quiescent_inactive` is rejected. After *explicit* release (safe because no child ever existed), the admission drain may finish.
2. **Close first, then commit:** direct registry close wins, so a registered but uncommitted owner cannot commit; the native reservation stays accounted for until explicit release. Close cannot finish before the worker's genuine permit drains.
3. **Wrong owner and immutable deadline:** cross-session reservation is refused; original admission expiration remains binding even after a later renewal. A newly admitted generation of work may claim the previously unused owner after fresh checks. These tests do not certify Git policy/lease validity.

Failure to observe native termination, stdout/stderr EOF, writer completion or tree-empty state is not a signal to drop either real production owner or quotas. The experimental drop of a never-launched empty Job is not evidence for cleanup after actual execution. A future service must handle uncertain shutdown and creator-host death independently.

## Hard boundaries

- All extra Rust fields/methods are compiled only under `#[cfg(test)]` on Windows. Ubuntu uses the unchanged production implementation plus existing core tests.
- This research does **not** prove that `NativeProcessReservation` can safely be retained after a process starts. It demonstrates only the typed pre-creation reservation/commit/lifetime arrangement.
- `SessionLifecycleManager::revoke` still waits for counted permits before `ProcessManager::cancel_session`; a permanently unobservable native job can deadlock graceful shutdown if the service waits for it while retaining admission. A future out-of-band stop phase, nonblocking shutdown ownership and crash/power-loss guardian remain necessary.
- Host-memory sampling within global JobStore mutex remains a separate concern; avoid reversing this and the session mutex during future wiring.
- Distinct forced-handle-reuse lifecycle cleanup ABA research `5edc4792` and security issues #176/#182 remain unresolved.
- PR #327 and Codex's supervised native 22-mode fixture remain research-only and untouched. Windows 10/11 physical-device acceptance has not occurred. Issues #317/#323 remain **OPEN, V1 NO-GO**.
- No operator PC, self-hosted runner, private repo data, real project or remote access is involved.

## Evidence required

Run exact-head rustfmt, strict Clippy, full runtime library tests, Windows explicit owned-native schedule with real JobStore and Ubuntu compatibility. Preserve the **first failing** job logs and all subsequent commit SHAs. Separate normal production promotion requires a comprehensive, executable service owner and independent Windows crash/uncertainty acceptance; a green synthetic fixture alone is NOT grounds for merging this test-only API into main.
