# Issue 317 — Immutable admission stop watch (incremental production primitive)

**Date:** 2026-10-23. **Scope:** generation-bound, read-only stop observation from a *genuine* `SessionAdmissionPermit`, without native process execution. This is a small production API increment, not an end-to-end native owner or V1 acceptance.

**Baseline:** `main=dedca5f4ce7a09b1e3474bd72c84ac56e2337dc9` after research-contract PR 333. The post-merge independent full CI passed [run 40019752828](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40019752828). New branch must demonstrate its **own** exact-SHA Windows/Ubuntu checks before integration.

## Problem

PR 331 added `SessionRevocationWatch`, a read-only `Arc<AtomicBool>` tied to the admitted **original session generation**. It intentionally covers only explicit revocation, not monotonic expiry. An eventual long-lived Git owner must stop on the **earlier of** explicit revoke and the session deadline captured at admission, even when a later session renewal increases the live record deadline.

A worker retrieving only a fresh grant by opaque handle could incorrectly observe another session generation if the same handle is re-registered. A worker retrieving a renewed deadline by handle might inappropriately extend its originally authorized execution time.

## Small implementation

The `SessionAdmissionPermit::stop_watch()` accessor returns a cloned `SessionAdmissionStopWatch` containing:

- the existing `SessionRevocationWatch` to the original admitted `Arc<AtomicBool>`, captured while the true admission was registered under `SessionRegistry.sessions`;
- the **immutable** `MonotonicTime` expiry from that same admission's `grant`.

`SessionAdmissionStopWatch::is_stop_required_at(now)` performs **only**:
`revocation.is_revoked() || now >= captured_expires_at`.

The caller supplies the trusted monotonic time; this accessor does not invent a timer, wake a worker, extend a grant, register an owner, authorize effects, revoke a lease or claim that any process exited. The watch may outlive its admission permit; the **permit itself must remain owned appropriately** until durable process cleanup. Stop observation by itself is not native shutdown.

A false result means just “neither of these two stop conditions has been observed at this instant”; it is **not** positive authorization to commit, create, resume, read private content or publish output. Task lease, policy, resource budget, operation deadline, process ownership, provenance and the future non-bypassable authority commitment are separate requirements.

## Deterministic tests, no native child

1. **Renewal:** A admitted at expiry 100 remains stop-required at 100 even after the registry is renewed to 200; a second genuine admission at 101 captures the new 200 expiry independently. Both become stop-required after a genuine revoke.
2. **Generation reuse:** after true revoke, full admission drain and registry removal, an intentionally re-registered handle obtains a fresh false stop watch while old watchers remain permanently true. This does not claim that normal application session provisioning reuses IDs.
3. **Worker stop-before-drain:** a real worker holds a genuine session admission until its own immutable watch sees direct registry revocation; synchronous `SessionRegistry::revoke` publishes stop first, then waits for the worker permit. The worker releases and revoke returns under bounded test deadlines; an unrelated session B remains valid.

The tests establish **registry-level stop-before-drain**, not real `SessionLifecycleManager` native cancellation or a full service-root shutdown under OS observation failure.

## Compatibility and future work

- Existing `SessionRegistry::revoke`, `SessionLifecycleManager::revoke`, `SessionRevokeReport`, approvals, leases, process JobStore and all existing callers are unchanged.
- Existing `SessionRevocationWatch` retains its explicit-revoke-only semantics.
- Native GitRead `HardenedCommandRunner`, long-lived service root, process stdin/stdout/EOF/quarantine, generation-bound owner commit, lock policy, Windows create-time Job membership and durable shutdown are not implemented here.
- Cross-generation *cleanup* after re-registration remains a distinct research-only negative control (hosted [run 40020639068](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40020639068)); this API does not fix it.
- Hosted CI on exact branch head and independent post-merge `main` CI remain necessary. The AppContainer/Node intermittency (issue 185) must not be hidden by retries.

**Do not close issue 317 or V1 gate issue 323 based on this primitive.** The 23 October documentation-only PR 333 caused an accidental GitHub auto-close from negated closing keywords; avoid closing-verb-plus-issue-number phrasing in future PR bodies and commits.
