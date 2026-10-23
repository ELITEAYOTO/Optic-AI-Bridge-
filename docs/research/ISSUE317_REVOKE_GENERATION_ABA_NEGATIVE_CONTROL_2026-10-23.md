# #317 — Lifecycle revoke and reused opaque session handles (negative control)

**Date:** 2026-10-23. **State:** research branch only, NOT a production fix or V1 acceptance.
**Source base:** integrated `main=dedca5f4ce7a09b1e3474bd72c84ac56e2337dc9` (research contract PR #333).
**Branch:** `research/317-revoke-generation-aba-2026-10-23`.

## Source-level concern

`SessionRegistry::revoke` publishes an irreversible close and drains live admissions under a mutex/Condvar. It returns only `bool` (whether the session changed). `SessionLifecycleManager::revoke` then cleans approvals, reusable approvals, leases and running/uncertain process records **by opaque `SessionHandle`** without a returned record-generation receipt or a cross-registry cleanup transaction.

`SessionRegistry::remove_quiescent_inactive` can remove an inactive closed record with zero admissions. `SessionRegistry::register` accepts the same handle again after removal, creating a fresh session record and a **fresh** revocation watcher (PR #331). If a separate lifecycle reaper finishes the previous generation while another revoker has completed registry drain but not owner-scoped cleanup, the same opaque handle can be reused before that first cleanup step. The first revoker then addresses a new generation through the old handle.

This is distinct from the already preserved handle-level wait predicate in #329; it adds a **post-registry owner-cleanup** boundary. It does not demonstrate a naturally occurring user incident, guessed/forged session ID, or an untrusted remote capability. It is relevant to any future durable owner registration and generation-bound stop contract.

## Deterministic experiment (current undesirable behavior expected)

The branch performs a **behavior-preserving** private decomposition of `SessionLifecycleManager::revoke`:

1. Call real `self.sessions.revoke(session)?` as before.
2. Delegate its original approval/lease/process cleanup in the original order and with the original error handling to private `finish_revocation_for_handle(session, changed)`.

The new test `research_reused_session_handle_between_close_and_cleanup_reaches_replacement_approval` uses these exact two phases with a controlled phase boundary; it does not depend on thread scheduling or actually spawn children:

1. Provision synthetic original A and unrelated B using the real registries.
2. Obtain and drop an original admission and keep a read-only old-generation watch.
3. Perform real `SessionRegistry::revoke(A)`; confirm old watch is permanently true.
4. Remove A through the actual quiescent-inactive registry operation; re-register the **same** A handle as a fresh record and obtain its fresh watch; confirm fresh watch is false.
5. Issue a new real approval to replacement A and one to B.
6. Continue with the actual original post-close cleanup code, still keyed by A's opaque handle.
7. Expected **negative result**: cleanup removes replacement A's approval while the replacement session remains active; B is unaffected. The test **passes if this weakness is observed**, not because it is fixed.

This is a valid source-level counterexample to treating `revoke`'s returned boolean or `SessionHandle` as a generation-specific cleanup receipt. The test phase interleaving is **forced at a private refactor boundary**, not a native race recorded on a machine. Test approval behavior depends on the intentional re-registration precondition.

## Options — not yet accepted

- **Short-term containment:** prevent session re-registration with an opaque handle until all lifecycle cleanup for the prior generation is completed, *not merely* until admissions reach zero. This requires identifying and coordinating **every** direct registry, lifecycle and recovery caller; an extra boolean alone does not solve process owner release.
- **More explicit protocol:** mint a generation-specific cleanup/commit receipt and make approval, lease, JobStore and seal/reap authority generation-bound, with a finite owner and concurrency contract. This is architectural; no blanket change to typed permissions or task grants without separate review.
- **Do not** fix by guessing whether an opaque handle is random enough, waiting longer, adding arbitrary retries, prematurely freeing owners, or holding the global registry mutex across OS calls or external stores.

## Acceptance remaining

- Record exact-source Windows+Ubuntu hosted reproduction with 0 native process creation and no runner on the operator PC. Preserve first compile/format failures if present. A green negative-control fixture is NOT a production fix.
- Establish whether the public app's supported session provisioning/recovery paths permit same-handle registration; distinguish a reachable supported path from a generic public API capability. Trace all writers/reapers.
- Produce actual parallel public-API interleavings, with an external test supervisor if needed, **before** changing production behavior.
- Audit and specify generation identity across direct revoke, lifecycle revoke, approval/lease removal, process-Job cancellation, `seal_quiescent`, `try_reap` and any durable host startup owner. No cross-generation cleanup.
- Preserve current synchronous `SessionRevokeReport` and existing approval cleanup semantics unless a separately tested migration justifies otherwise.

No #317 / #323 closure; #176/#182/#185/#319 remain distinct. PR #327 stays untouched.
