# #317 — Early cancellation can miss a late Job reservation

**Date:** 23 October 2026  
**Status:** RESEARCH, **not** a production patch, ADR or approval to change revoke semantics  
**Baseline:** production `main=442d9e0cdf7474bb9c159a83ac1a0fc1b2c2c6d6` (PR #329 and #330 merged)  
**Branch:** `research/317-stop-registration-order-2026-10-23`

## Question

Can we resolve the synchronous `SessionLifecycleManager::revoke` admission-drain-before-process-cancel dependency merely by calling `ProcessManager::cancel_session` as soon as `SessionRegistry` publishes `revoked`, instead of after the permit drain?

**Answer:** No. The current `cancel_session` scans the *currently registered* bounded `JobStore`, sets a stop flag once for live/uncertain owned records and returns an integer count. It **does not remember a session-wide stop intent for reservations registered afterward**. `SessionRegistry` and `ProcessManager` presently have separate mutexes, and `ProcessManager` cannot reject a late reservation based on the revoked registry state on its own.

A genuine session admission may have been granted **before** closure but not yet published its Job record. Moving a single cancellation scan earlier creates a lost-stop race:

1. Session A grants an admission to a still-in-flight producer.
2. The authority publishes A as Revoked and immediately scans the current JobStore: **0 matching jobs, 0 stop requests**.
3. The already-admitted producer publishes a new A-owned Job record with `stop_requested=false`.
4. The one-off early scan has already returned. The new owned record retains quota but did not receive the requested stop. If a long-lived worker waits for that stop before releasing its admission permit, the later synchronous drain cannot complete.

This is a **state/ordering proof**, not a demonstration that the current production lifecycle has already moved cancellation or that an actual native process escaped supervision. The production lifecycle still cancels only *after* the synchronous registry drain; the narrow #330 change only expands what a scan sees once it runs.

## Bounded deterministic research controls

`crates/optic-bridge-runtime/src/process/tests.rs` (new functions beginning with `stop_registration_contract_`) uses the **actual** registry, actual admissions, actual stop/revoke API, actual `ProcessManager` and its JobStore, with a **synthetic JobRecord** instead of a child process. It is part of the test build only.

- **`early_stop_scan_misses_late_job` — intentional positive reproduction of a dangerous negative case.** An actual `SessionRegistry::revoke` publishes Revoked while a genuine permit remains active. Calling actual `ProcessManager::cancel_session` before inserting the already-admitted synthetic Job returns zero; inserting afterward leaves the stop bit **false**. A separate second cancellation scan can set it, but requiring a retry after an unbounded drain would not solve the potential circular wait. B stays usable, pending A ownership remains accounted, and the permit is explicitly dropped to finish this bounded test.
- **`registered_job_is_seen_during_close` — control.** The Job record is published before the actual registry close; a cancellation scan after close sees it and sets the stop bit. The registry still waits for the outstanding permit. A stop request alone does **not** prove terminal Job state.

Both run without any external executable, arbitrary PID action, user-PC access or native Job termination. A passing negative-control test intentionally **confirms the flaw in the proposed naive ordering**. All cleanup is of test-only temporary directories and in-memory records.

## Production contract required before stop-before-drain

A real solution must establish *one unavoidable ordering boundary* for authority close versus native effect commitment and Job ownership registration. Options to analyze:

1. **Authority-owned short commit ticket:** after close, newly committed launches fail. A worker admitted before close can keep a Job ownership reservation, but that reservation must be made visible to cancellation **before** a native effect or must inherit a persistent owner-scoped stop intent. Never hold a registry mutex through potentially unbounded Windows system calls.
2. **Durable stop intent plus atomic owner registration:** closing A marks A's stop epoch/flag in authority and informs an application-owned owner root; every new A reservation atomically checks this stop epoch in the same registration protocol and enters already-stopping, never with a fresh false stop flag after closure. The fence must reject stale session record generations and survive concurrent close/reap/renewal. Bound the stop-intent history to live owners/session records; do not introduce an unbounded tombstone map.
3. **Separate close, request-stop and terminal proof:** return a clearly named pending receipt rather than claiming completion when some Job/stdio/worker remains uncertain. Preserve the old public `SessionRevokeReport` until consumers and compatibility are reviewed. `seal_quiescent` and `try_reap` must continue rejecting retained native uncertainty.
4. **Failure consistency:** if registering an owner fails after any OS effect, keep responsibility in a durable native owner or independent guardian. A stop request must never be equated with `ResumeThread` ordering or a final stdout/stderr/result publication license.

A second cancellation scan after draining *can* catch reservations whose producers have all finished admitting, but **cannot** be relied upon to break the worker-permit → stop-request cycle itself. A synchronous one-shot pre-drain cancellation without a shared registration fence is not a safe partial patch.

## Acceptance and unresolved work

- Confirm real-registry synthetic negative/control tests on hosted Windows and Ubuntu at the exact branch SHA with whole-workspace Rustfmt, locked all-targets/features Clippy and no disabled checks.
- Require further real native Win32 work for atomic ownership, cancel-before-native-commit, quota retention, stop during registration, callback/panic errors, persistent observation failures and host crash. Prior native research #317 is **not** production wiring.
- Keep #317 OPEN, #323 V1 NO-GO and PR #327 independent. No migration of the production Git runner, no user-PC tests, no release, no change to main in this research slice.
