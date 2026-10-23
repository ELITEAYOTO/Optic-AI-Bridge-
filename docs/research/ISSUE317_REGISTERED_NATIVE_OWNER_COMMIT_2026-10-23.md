# Issue 317 — Registered native-owner session commitment (test-only research)

**Date:** 2026-10-23. **Not production code; not an accepted ADR or V1 release proof.**
**Research base:** `main=c52b514b2309aa82ee78d9cdbf78ce98dced64eb` (verified docs PR 335 squash).
**Scope:** a Windows native JobStore *reservation* + original session admission + short serialized owner commitment; **zero child process creation**, zero new launch authority.

## Research question

Can a genuine already-registered (but not launched) Windows owner be committed once against the **same** SessionRegistry serialization point used by direct/lifecycle revoke, without holding its mutex across Win32 calls, JobStore locks, filesystem work or waiting for admission drainage?

This does NOT decide whether the full process can be created, resumed, stopped and cleaned up safely. In particular, the generic `HardenedCommandRunner` and the unwired `private_node_task` remain as before.

## Small test-only implementation

- `NativeProcessReservation` already originates from `ProcessManager::reserve_native_private`: it has a private record in the real JobStore and a bounded, not-yet-running native Job. For research, a `#[cfg(test)]` atomic `owner_commit_claimed` and a pre-created owner/session identity reader enable a single short nonblocking claim. Production reservation layout and methods remain **unchanged** in non-test builds.
- `SessionAdmissionPermit::research_commit_reserved_native_owner` (Windows unit tests only) obtains the genuine registry mutex and verifies: owner session matches, original record identity still equals the permit's captured `Arc<AtomicBool>`, original admitted expiry and live record expiry both hold, session is not revoked, native reservation is still unspent and stop has not already been requested. Then it atomically claims this native reservation exactly once under the *same* mutex used by direct registry revoke.
- It returns a **borrowed**, noncopyable `ResearchOwnerCommitReceipt` referencing both the genuine counted admission and real native reservation, plus read-only `SessionAdmissionStopWatch`. Closing the session after commitment still publishes a signal while its counted admission blocks synchronous drainage.
- The returned proof is **not** a policy/task lease/executable/repository check, a process start grant, a durable service guardian, a native Job emptiness receipt or a result publication license. No kernel/JobStore query occurs inside the session mutex.

## Acceptance scenarios

1. **Commit-before-close:** reserve a real native JobStore owner, then genuinely admit A; commit once under the session lock; second commitment refused. Direct registry revoke publishes explicit stop and waits for the owner's admission; stop is seen even before the waiting revoke returns. Dropping receipt and permit allows old synchronous drain to complete.
2. **Close-before-commit:** with a genuine admission and already registered owner, direct revoke publishes stop first; new commit is refused, the original permit drains and revoke returns.
3. **Wrong owner:** a real registered reservation belonging to B cannot be committed against a live A admission; B remains active.
4. **Captured expiry:** renewal of A's live record to a later time does **not** extend the earlier permit's deadline, and commitment at original expiry is refused. This is independent of a later operation's own admitted deadline.
5. All reservation fixtures use empty Jobs and no executable child; a native-host supervisor for real `CreateProcessW`, crash-before-association/rollback, suspended child exit, descendant EOF, leaked handles and uncertainty remains essential.

## Limits and explicit no-go

- This is **test-only research**: not a production commit issuer, and no runtime code calls the test-only APIs. The source and CI prove a serialization property of a fixed synthetic schedule, *not* a production Windows native owner implementation.
- The Job is created by `reserve_native_private`, which still performs bounded-capacity checks and Windows host-memory observation inside JobStore; its lock duration requires separate audit.
- The receipt's borrow lifetime proves that test code cannot drop the particular owner and permit while holding the receipt; it does not automatically make a service root own the receipt across a detached asynchronous worker or a host crash.
- No OS operations may ever be done inside the registry mutex. A revocation after commit but before `CreateProcessW` cannot be equated with “no process ever created”; future owner must deliver stop and verify actual tree termination.
- Direct and lifecycle session revocation must share this registry lock, but the task-lease, policy, profile and capability issuers are **separate authorities** that also require explicit fencing. A green test does not solve that.
- The forced same-handle re-registration cleanup race remains a distinct research condition (Windows/Ubuntu #40020639068) and does not demonstrate natural production exploitability.
- Issues 317 and 323 remain open with Windows V1 **NO-GO**. Other gates 176, 182, 185, 319 and research PR 327 are unchanged. Do not transplant this branch wholesale into `main`.

## Required evidence to advance

Capture first failures and exact-head hosted Rustfmt, Clippy, Windows targeted/full library and Ubuntu regressions. Later prototype must wire this **real reservation** into a sealed GitRead runner with independent outer native supervision, verify create-time Job association and lifelong owner retention even under pending shutdown, and then pass independent integrated-main CI. No user-PC or self-hosted execution in this research branch.
