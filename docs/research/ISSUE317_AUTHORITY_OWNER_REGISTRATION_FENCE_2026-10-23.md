# Issue #317 — Authority-owned owner-registration fence (research)

**Date:** 2026-10-23  
**Status:** test-only hypothesis, *not* production wiring, no release approval  
**Baseline:** `main=442d9e0cdf7474bb9c159a83ac1a0fc1b2c2c6d6`  
**Branch:** `research/317-owned-registration-fence-2026-10-23`

## Problem it addresses

[Previous research](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/315bdc8882fd74930490932f31809df1a255351f/docs/research/ISSUE317_EARLY_CANCEL_RESERVATION_RACE_2026-10-23.md) showed that a **single early `ProcessManager::cancel_session` scan can miss an already-admitted worker whose owner registers afterward**. Merely moving the scan before the synchronous registry drain would create a lost-stop race.

The actual lifecycle still performs synchronous admission drainage **before** requesting process cancellation. Thus production retains the worker-permit → stop-request dependency. These tests study an ordering prerequisite, **not** the complete solution.

## Experiment: shared authority gate and bounded owner registration

The production implementation has **no new gate**. Only `#[cfg(test)]` builds of `session/registry.rs` add a per-session `research_owner_gate: Arc<Mutex<()>>` to the *real* session record. Direct `SessionRegistry::revoke` takes this same gate before publishing `revoked` and the existing atomic stop signal, and releases it **before** entering the synchronous Condvar admission drain.

`research_commit_owner(handle, now, publish)` also takes that gate, checks the real session record, expiry and record identity (`Arc::ptr_eq`, preventing a stale-gate/handle ABA bypass), and executes a **synthetic bounded JobStore registration** while retaining that gate. It releases the global registry mutex *before* the synthetic callback; independent session B can continue. A panic poisoning this per-session lock rejects new commits, but a direct real revoke recovers the lock to publish cancellation.

The experiment deliberately does **not** call `CreateProcessW`, `ResumeThread`, or any real Git command. Registry unit tests are instrumented, but normal production builds—including app integration tests—are not. A real application must make this commit mechanism **non-bypassable**, rather than letting callers write to JobStore directly or execute native effects before registration.

## Deterministic scenario expectations

| Case | Actual components and expected outcome |
| --- | --- |
| Close wins | Real A admission already exists; direct `revoke` publishes Revoked but cannot drain until its permit drops; a later synthetic owned registration under the shared gate returns `Revoked` and does not insert a Job |
| Owner registers first | Real A admission; synthetic Job owner committed before direct close, as `TerminationUncertain`; while direct revoke waits on permit, out-of-band `cancel_session` sends exactly one stop request, retains record/quota, then dropping permit lets revoke finish |
| Contended close versus commit | A's bounded synthetic owner-registration closure temporarily holds gate, a second thread calls **direct** `SessionRegistry::revoke`; unrelated B can commit. A registration finishes before A's revoke publishes closure, so the cancellation scan necessarily sees the registered owner |
| Expiry and panic | An expired A session cannot commit; a synthetic owner-commit panic poisons its gate so further commits fail closed, but direct registry revoke still publishes Revoked without waiting for an effect |

The tests are implemented in `crates/optic-bridge-runtime/src/process/tests.rs` and run only with a test build, using actual registry state and actual `ProcessManager` JobStore but **synthetic in-memory JobRecords**. Temporary workspaces contain no user data. They do not run on the operator PC.

## Unresolved safety and compatibility gates

1. **Stop orchestration is not automatic:** the test invokes `cancel_session` from a supervising test thread once closure is observed. Real `SessionLifecycleManager::revoke` still waits for permits before issuing stop. A future contract must separate *prompt authorization close* and *stop request* from later admission drainage without making a report falsely claim terminal proof.
2. **Native effects are outside this gate:** no proof that Windows process creation, atomic Job membership, ticket commit, `ResumeThread`, output publication, stdin, or actual GitRead follows this synchronization.
3. **Potential unbounded native call under a lock:** only the *bounded synthetic owner-record registration* occurs under the gate. A real native syscall or user callback must not hold a session mutex indefinitely; design a short trusted internal operation and separately owned Job/worker.
4. **ABA and panic only partly modeled:** a record identity check is present for new synthetic commits. Further tests must cover remove/re-register interleavings, direct lifecycle rollback, expiry/renewal, poisoned global sessions state, and cancellation receipts.
5. **Separate owner lifetime:** retaining a `TerminationUncertain` record and quota is correct but does not prove the OS Job is dead; pending-root durable shutdown, guardian ownership and bounded observation remain unfinished research.
6. **No production or V1 claim:** `#317`, provenance `#182`, timing `#185/#319` and V1 `#323` remain independent. PR #327 is a separate experimental native bootstrap PR and must not be merged wholesale.

## Required next design decision

Choose a **single authority-owned close/registration linearization API** where native ownership/reservation is established before any irreversible OS effect, then close becomes visible before issuing a stop request **without blocking on admissions**. After stop is requested, the caller may wait for admission drainage or return an explicit pending receipt, but must retain a responsible native root until Job/streams/worker completion is proved. Public `SessionRevokeReport` and seal/reap semantics must be reviewed independently; avoid a simplistic timeout, detached thread or dropping pending native resources.

The hosted Windows/Linux workflow is a *targeted proof only*, not the V1 acceptance suite. Exact-SHA run IDs should be recorded only after the final code+docs head successfully completes.
