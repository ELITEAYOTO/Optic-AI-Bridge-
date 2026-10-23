# #317 — Session close, native launch, and shutdown: contract review

**2026-10-23 — PROPOSED / RESEARCH ONLY.** No permission to change the production session API, GitRead owner, release protections, or V1 feature freeze. No operator-PC or self-hosted test is required.

## 1. Verified boundaries

- Production base: `main=da34eae7d0efda39d9fb3c6786ec843546926fe9`. Its `SessionRegistry::begin_admission` increments `active_admissions` under the registry mutex. `SessionRegistry::revoke` marks the record revoked and publishes `revocation_signal.store(true, Release)` **before** waiting on the admissions condvar. Other sessions may progress because the condvar releases the registry mutex. `remove_quiescent_inactive` refuses a record still holding admissions.
- Production `SessionLifecycleManager::revoke` invokes synchronous registry revoke **first**, then revokes both approval classes, task leases and process jobs. `seal_quiescent` refuses active jobs, and `try_reap` retains active owner records.
- Candidate `codex/317-service-retention=1c7dac973c8f41f27b303811041b54031f7272ae` is **experimental**. Its `ReadServiceRoot::submit` drops its admission permit at submit return; service workers retain native resources without an admission permit counted through their lifetime. `try_shutdown` reports retained workers; `Drop` joins and may wait forever during persistent observation failure. See [candidate admission/shutdown audit](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/1c7dac973c8f41f27b303811041b54031f7272ae/docs/research/ISSUE317_SERVICE_ADMISSION_SHUTDOWN_2026-10-23.md).
- Independent [Windows Job-list research](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/8b72b55ad80f2d73e2948203ad47f8632ae7efee/docs/research/ISSUE317_NATIVE_LIFECYCLE_DECISION_2026-10-23.md) proves atomic **Job assignment at process creation**, selected stdio inheritance, retained descendants and bounded error cleanup for synthetic tests. It does **not** establish revocation versus `ResumeThread` linearization. No tested Windows platform primitive is yet wired to production.

## 2. Contract distinction

**Admission closure is not native termination.** A session may have an already-accepted operation which still owns a suspended child, running child, descendant-held pipe, pending observation or uncertain termination. The following states must remain distinct in any future service API:

| State | Authority / owner | Safe operations |
| --- | --- | --- |
| `Active` | New effects may be admitted if grant/epoch/deadline/scopes pass | Admit within effective budget |
| `Closing` | No new effects; publish cancellation immediately | Reject new admissions/renewals; request stop |
| `PendingNativeProof` | Owned Job, handles, workers, quotas and outputs retained | Bounded observation/retry; **not** seal/reap or claim success |
| `Quiescent` | All admitted effects ended; no native Job members, both output streams finalized and no live worker owner | Seal; disclose terminal result only if its authority policy permits |
| `Reaped` | Subordinate process/lease/worktree/approval state released in the established cleanup order | Reclaim session capacity |

These are **proposed concepts**, not deployed enums or a new state machine on `main`. `TerminationUncertain` must remain an explicitly owned **pending** outcome, not an alias for `Quiescent`.

**Backward-compatibility hazard:** production `revoke()` currently returns only after admission permits drain. Moving a long-lived permit into a worker *without first changing lifecycle semantics* can move the known permanent-observation hang from service `Drop` into public `revoke()`. Do **not** implement permit transfer alone. A different split between `close_admission` and `await_quiescence` (or an independent durable owner) needs an API/call-site review and product approval. In particular, `SessionRevokeReport` currently has no terminality/pending field, and callers must not infer native quiescence from its existence.

## 3. Launch versus close: required linearization

A check of the revocation atomic immediately before `ResumeThread` is insufficient: the signal can flip after the check and before the native resume.

**Candidate per-session protocol for review (not approved implementation):**

1. Provision a distinct, session-scoped launch/close authority fence; avoid holding the **global registry mutex** across a Win32 call. Define a lock order with registry, JobStore and worker indexing before coding.
2. A launch may pre-create its bounded Job and a **suspended, already-Job-associated** child only while it has a valid admitted owner and effective absolute deadline. The child must remain owned during any transient failure.
3. The launch's **commit/resume gate** and revocation's **close gate** must serialize at a single linearization point. Under that gate, the launcher rechecks exact session epoch, closed state, cancellation and time budget before native `ResumeThread`; the closer closes admission and publishes stop intent.
4. If **close wins**, the child must never resume; terminate the bounded Job, retain handles until exit/EOF proof and return pending/aborted (never an authorized effect).
5. If **resume wins**, close still publishes stop immediately, requests native termination and remains pending until completion proof. The system may NOT promise that zero instructions execute after close in this ordering; it can promise only that no *new* launch commits after close and that quiescence is independently proven before session release.
6. Keep output publication/credentials policy tied to the final authority epoch; do not publish stale post-revocation success solely because the child exited zero.
7. If observation errors persist, maintain the responsible supervisor, Job ownership, quotas and worker handles; no false terminality, detached orphan thread or silent fallback to spawn-then-assign.

Whether a per-session gate may encompass `ResumeThread` requires a bounded-native-call and lock-inversion review; if not provable, a separate launch-authority broker/transaction protocol is necessary. The above must not be implemented as an unrelated private mutex that cannot serialize with real revocation.

## 4. Evidence and deterministic acceptance matrix

**Real production-registry tests, not a model:** this research branch includes two `#[cfg(test)]` tests in `session/registry_contract_tests.rs`. They exercise actual `SessionRegistry` synchronization:

- Two permits admitted for A; A revocation closes admission before waiting, publishes the native stop signal on Windows, rejects new permits, and refuses physical removal while either permit remains. A completes only after the *second* permit is released; repeated revoke is idempotent.
- While A is closing, B still admits and revokes independently. A revoked during drain refuses renewal and B can still renew. Tests never touch Git credentials, Win32 native process launches, or network.
- Execute on hosted **Linux and Windows**, with pinned Rustfmt, strict Clippy and tests at the **exact source SHA**. A passing test asserts current registry behavior; it does **not** claim a full worker-lifetime authority fence or resume atomicity.

**Required future research slices (unimplemented):** (a) independently supervised negative control that forces real `ResumeThread` after a last-check-to-close barrier and proves the race without an uncontrolled child, (b) a candidate shared commit/close gate with both winner orderings under real session authority, (c) cancellation before/during native create, after Job association and immediately around resume, (d) native uncertainty, pipe/descendant retention and counter proof across `try_shutdown` / `seal_quiescent` / `try_reap`, (e) proper authorization-minted read-only Git action with binary/cwd/environment pins, budget, lease, effective deadline, (f) Windows 10/11 operator-approved acceptance and independent post-merge CI.

**NO-GO:** no `ReadServiceRoot` production wiring, API contract change, feature expansion, V1 release or merge of the research stacks on this evidence alone. #317 and #323 stay open; PR #327 and #182 remain independent. Keep first-red runs with exact commit association.

## 5. Decision required before production patch

Prefer investigating **two-stage close + independent quiescence** while retaining current API behavior for synchronous effects until consumers are audited. The precise public API, pending response/report, durable owner and shutdown policy remain **unapproved**, not silently selected by this note. A completed revoke/seal must not imply an unproven native cleanup. Prepare a separate focused design/implementation PR only after consensus on that contract, native launch linearization and failure-path proof.
