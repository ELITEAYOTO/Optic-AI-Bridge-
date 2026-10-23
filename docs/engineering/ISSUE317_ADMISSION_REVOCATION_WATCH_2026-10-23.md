# #317 — Read-only generation-bound revocation watch (integration slice)

**Baseline:** `main=442d9e0cdf7474bb9c159a83ac1a0fc1b2c2c6d6`.  
**Purpose:** a small, independently verifiable production primitive that does **not** change the synchronous `SessionRegistry::revoke` contract or migrate the experimental native process owner.

## Problem and contract

The normal registry stores a private `revocation_signal: Arc<AtomicBool>` in each session record. Its `SessionAdmissionPermit` previously contained only a `SessionHandle` and a grant. A separate worker trying to retrieve stop intent from the opaque handle would need another lookup and could lose historical record identity after removal/re-registration.

This change copies the **same** signal Arc into the *real* admission permit **under the same registry mutex** that counts the admission, and exposes a **read-only** `SessionRevocationWatch` via `permit.revocation_watch()`. The watch contains a private `Arc<AtomicBool>` and has only `is_revoked()` plus `Clone`; callers cannot flip the authority latch through this API.

The watch is bound to the original SessionRecord allocation/generation, not to the possibly reused opaque handle. Its `Acquire` load observes an explicit `Release` publication in `SessionRegistry::revoke`.

This is **observation**, not authorization:
- `false` is **not** permission to start, resume, publish or extend work.
- Expiry is checked separately using a trusted monotonic clock; expiry does **not** automatically publish explicit `revocation_signal`.
- Task lease, policy epoch, resource budget and effective operation deadline must be enforced independently.
- `true` signals cancellation request, **not** successful termination or permission to discard handles and quotas.
- The watch may outlive the admission permit; the registry remains responsible for draining admissions. The watch keeps only its small signal Arc alive, not the underlying process or admission slot.

## Existing behavior preserved

No change to `SessionRegistry::revoke` locking, wakeups, synchronous wait, return bool, idempotence, public errors, approvals, process cancellation ordering, `JobStore`, `HardenedCommandRunner`, task/lease validation or native Windows APIs.

Normal `begin_admission` returns the same permit semantics with one additional private Arc clone and an optional new **read-only** access method. Existing consumers can ignore it and retain their current behavior.

## Deterministic acceptance

`registry_revoke_tests.rs` uses the actual registry and admission permits to prove:

1. A direct revoke publishes a durable stop to an already-admitted watcher while revoke still waits on a real permit, without flipping B's unrelated watcher.
2. After the permit drains, removal and re-registration of **the same opaque handle** creates a new distinct revocation watch; the old watch remains permanently revoked and never observes the replacement session as open.
3. Expiry alone leaves explicit-revocation watch false, preventing downstream code from mistaking this read-only watch for the mandatory monotonic TTL check.
4. A second true explicit revoke flags the replacement generation independently.

No synthetic or real process is created by these tests. A successful Windows/Ubuntu CI on the **exact branch SHA**, plus the unchanged full-workspace checks, is required before integration. Successful original Codex native research does not substitute for this CI.

## Remaining #317 and V1 blockers

This change **does not prevent the lost-stop registration race on its own**: a worker must reliably subscribe to this watch, reserve/register its durable owner before irreversible native effects, and coordinate close/commit under one non-bypassable authority boundary. A watch by itself neither wakes a blocked thread nor stops a process.

`SessionLifecycleManager::revoke` still drains admissions before native `cancel_session`, so the worker-permit/stop-request cycle remains unresolved. A bounded durable Pending owner/guardian, Windows native error recovery and two-phase close/stop/quiescence receipts are still required. Post-revocation output/provenance and GitRead authorizer gates (#176/#182), Windows timing (#185/#319) and V1 acceptance (#323) remain open.

**No #317 close, no V1 release, no PR #327 change and no experimental code transplant are authorized by this slice.**
