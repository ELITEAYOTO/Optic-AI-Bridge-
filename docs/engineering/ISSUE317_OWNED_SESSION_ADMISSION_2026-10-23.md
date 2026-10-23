# Issue 317 — Non-cloneable owned session admission (incremental primitive)

**Date:** 2026-10-23. **Production candidate:** small API/lifetime building block only; *not* native process ownership, GitRead migration or V1 approval.
**Base:** reviewed `main=c52b514b2309aa82ee78d9cdbf78ce98dced64eb`, after docs-only PR 335. The test-only registered native commitment research is separate at `b6653f700af6028882e74232214001024822a7f8` (hosted Windows/Ubuntu CI 40025477442).

## Specific problem

The existing genuine `SessionAdmissionPermit<'_>` borrows `&SessionRegistry`. Future long-lived **service-owned** work needs to retain a counted admission even when the original request/caller returns, and pass it between threads, while direct `SessionRegistry::revoke` and `SessionLifecycleManager::revoke` continue to publish the original-generation stop before their synchronous admission drain.

A merely cloned `SessionAdmissionStopWatch` observes stop but **does not keep an admission counted**. A service dropping its counted permit immediately after submit can let revoke claim quiescence while a native worker still owns an active process. The proposed owned permit addresses *only this ownership lifetime gap*.

## Proposed implementation

- `Arc<SessionRegistry>::begin_owned_admission(handle, now)` returns a **non-cloneable** `OwnedSessionAdmissionPermit`, holding the same registry via `Arc`, original `SessionGrant`, original-generation revocation signal and opaque handle.
- Both borrowed and owned permit constructors share one private `admit_record` critical section: check handle existence, explicit revocation, monotonic expiry, checked admission-counter increment, then capture **that exact original record**. Both permits decrement the **same** `active_admissions` counter through the existing `finish_admission` on Drop. No fabricated synthetic permit, manual counter release or unowned observer can decrement that count.
- `OwnedSessionAdmissionPermit::stop_watch()` combines explicit original-generation revocation with **captured** monotonic admission expiry from the already-integrated PR 334. A later renewal does not extend a previously admitted operation. The registry is retained for safe Drop after the caller releases its initial `Arc`.
- The new API grants **no** task lease, owner-commit ticket, policy epoch, executable authority, OS handle, process execution permission or publishing right. Unlike research code, it is compiled as an actual runtime API, but unused by production workers until a separate audited integration is approved.

## Native ownership obligation (not delivered here)

The future native service root must retain this permit **until positive tree/job/pipe/controller termination proof**; merely observing stop and dropping it is insufficient. A persistently uncertain process must continue occupying its real owner and quotas. The current synchronous revoke contract may block while a future owner remains uncertain, so an explicit close/stop versus terminal-drain API and durable guardian still need design. The owned permit must not be `Clone`, `forget`-ed, detached or released on a guessed timeout. If the host crashes, `Arc` and permit destructors cannot provide cross-process cleanup or power-loss guarantees.

The borrowed permit API and `SessionRevokeReport` are preserved; normal policy/lease/authorization checks are unchanged. The forced handle-reuse ABA **cleanup** race remains separate from the stop-watch identity guarantee.

## Regression contract

1. Owned admission retains the registry after the caller drops its initial Arc, and correctly drops the last Arc when the permit is destroyed. Captured stop deadline remains unchanged after renewal.
2. A genuine owned permit is moved onto an **independent** OS thread. Another real borrowed permit is held for the same session. Direct revoke publishes stop while both admissions remain counted; an attempted reaping/removal is denied, and revoke waits until **both** are released. Unrelated session B remains active. Once drained, a deliberately reused opaque handle captures a new watch.
3. Unknown, expired and explicitly revoked sessions reject new owned admission and cannot increase active counts.

Run pinned Rustfmt, strict Clippy and all runtime tests on exact source head under hosted Windows and Ubuntu, then full normal CI (including AppContainer, dependency/release controls) and independent post-merge main CI before integration. Preserve failures in the original attempt instead of retry-to-green.

## Remaining blockers

- Owner creation/registration before any native effect and **short, shared close/commit linearization** must still be made production-private and non-bypassable. Research-only receipt from `b6653f7` is not integrated.
- No service-root storage, persisted stop latch, Job-aware Win32 create-time association, stdin/stdout/stderr tree EOF, error quarantine, uncertainty guardian or bounded service shutdown is delivered here.
- Windows process containment is not proven by these simulated admission tests; production GitRead still uses `HardenedCommandRunner`.
- Security/provenance decisions 176/182, AppContainer intermittent behavior 185, first-response 319, supported Windows 10/11 acceptance and V1 gate 323 remain separate. ADR-0035 remains PROPOSED. No operator-PC tests, self-hosted runners, release, Electron or scope expansion.
