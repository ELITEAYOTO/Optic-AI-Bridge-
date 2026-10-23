# #317 — Owner-scoped cancellation includes uncertain Job records

**Status:** narrow candidate, subject to exact-head CI/review; **not** a fix for the full #317 lifecycle deadlock or Windows native ownership.  
**Base:** production `main=da88d471a079fe2e63cf636d8924f6d12dc82377`.  
**Target:** hosted Windows/Ubuntu tests, no operator-PC processes.

## Current issue and change

`ProcessManager::cancel_session(session)` previously requested `stop_requested` only when a retained owner's `JobState.status` was `Running`. Yet the same module explicitly defines `status_holds_process_ownership` as **`Running | TerminationUncertain`**; uncertain Jobs continue occupying quota and cannot be reaped because OS termination is not verified. An uncertain record can therefore have a false stop signal and remain under native ownership.

This candidate reuses the existing `status_holds_process_ownership` predicate to request stop for **both** states, preserving:
- Session-owner matching and bounded traversal of the existing JobStore; no new PID APIs, scans or global state.
- The existing `swap(true, Ordering::AcqRel)` atomic so each not-yet-signalled Job increments the cancellation count exactly once; subsequent calls are idempotent.
- Retained Job records, output buffers, process reservations, terminal proof and removal conditions. Setting stop does **not** mean the OS process is gone.
- No modification of `ProcessManager::stop(session, job)` public command behavior or native launch order.
- The current error and Mutex-locking behavior; a poisoned job/store mutex still returns `StateUnavailable`.

A deterministic regression injects synthetic `Running`, `TerminationUncertain` (both signalled/unsignalled), proven terminal `Exited`/`Failed` and another session's `Running` record. It verifies owner-scoped first cancellation count, idempotence, no stop sent to terminal/unrelated records, preserved uncertain quota and JobStore membership, and separate cancellation of B.

## Observable scope and limits

This is an intentional but **very narrow behavior change**: `cancel_session` can now request cancellation and report an additional request for an un-signalled `TerminationUncertain` Job. It does not relax any authorization: only session-owned retained processes receive this request. It is a stop **request**, never a successful-termination receipt.

The current `SessionLifecycleManager::revoke` **still calls synchronous `SessionRegistry::revoke` and drains admission permits BEFORE invoking `ProcessManager::cancel_session`**. Consequently this improvement does **not** resolve the demonstrated worker-held permit → process-stop dependency. Even after a stop is requested, a process can remain uncertain indefinitely; the application-root guardian/owner-shutdown contract is still unimplemented.

This test exercises actual ProcessManager accounting but **not actual Win32 Job termination**. For V1 we still need authority-owned close/commit ordering, native owner and pending lifetime, results after revocation, real GitRead authorizer/provenance (#182), Windows timing (#185/#319), and Windows 10/11 acceptance. #317 and #323 remain NO-GO.

No release, merge authorization or V1 claim follows solely from green CI.
