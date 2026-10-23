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


## 6. Nonblocking close / pending-admission proof and bypass audit (23 October 2026)

**Checkpoint:** [hosted Windows + Ubuntu CI #39903229287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39903229287) completed **success on the exact source `389d5de4545df9ff54de20c1c06f0c37f9507bbc`** (attempt 1 for this SHA), with whole-workspace Rustfmt, locked runtime Clippy `-D warnings` and **4 actual-registry tests per OS**. Earlier [red #39903173756](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39903173756) failed Rustfmt on the prior SHA; kept and corrected without suppressions.

Two additional **`#[cfg(test)]`** cases extend the same source-level real-registry test module, **not the production API**:

1. `prototype_two_stage_close_is_immediate_but_never_reports_early_quiescence`: a *test-only helper* acquires the real `SessionRegistry.sessions` mutex, marks the real session revoked and publishes its existing stop atomic, then returns **without waiting** on the condvar. With two outstanding genuine `SessionAdmissionPermit` values, it reports **(new close, 2 pending)**; new admission is rejected, real removal returns `AdmissionsInFlight`, repeated close returns **(already closed, 2 pending)**, and an unrelated B session can continue. Dropping one permit yields **1 pending**, dropping both yields **0 pending**; only then can production `revoke` return idempotently and `remove_quiescent_inactive` reclaim the record. On Windows the existing real native stop signal is also asserted.
2. `external_wrapper_gate_cannot_intercept_direct_registry_revoke`: while an unrelated private launch-side mutex remains locked and a real admission permit is held, an independent thread invokes the **public actual** `SessionRegistry::revoke`. The real record becomes revoked and denies new admission **despite the wrapper mutex**. Revoke remains pending until the genuine permit is released. Thus an external wrapper lock cannot supply a non-bypassable security commit/close point.

The helper is private to the Rust **test module**; it directly mutates the actual registry record only during tests, and is neither an accepted runtime method nor an alternative to a security review. Its reported count is **admission drainage only**: it is **not** a signal that a Windows Job is gone, that stdout/stderr reached EOF, that native observation is complete, or that a worktree/lease/approval can safely be removed. The previous [real registry + suspended Win32 child test #39902524900](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39902524900) independently demonstrated the actual revoke-before-`ResumeThread` gap with a synthetic child under external Job supervision; this Linux/Windows source-level matrix does not replicate that native test.

### Concrete authority call-site and lock-order audit

Paths verified on `main=da34eae7d0efda39d9fb3c6786ec843546926fe9` (focused inventory, **not** an exhaustive transitive call graph):

| Caller or boundary | Current sequence | Future-contract risk |
| --- | --- | --- |
| `SessionRegistry::revoke` in `session/registry.rs` | Marks `revoked`, publishes stop, then **waits** under condvar until admission count is zero | A worker-lifetime permit linked to permanent native observation uncertainty can block the public revoke forever |
| `SessionLifecycleManager::revoke` in `session/lifecycle.rs` | Invokes synchronous registry revoke **first**; only afterward revokes both approval brokers, task leases and requests process cancellation | Simply holding a worker permit until owned Job completion would postpone the very cancellation expected to finish it: possible dependency cycle |
| `SessionLifecycleManager::seal_quiescent` and `try_reap` | Both invoke `revoke` before checking active jobs; reap cleans process history, leases, worktrees, then session | A deadlocked revoke blocks seal/reap; conversely checking only registry pending count cannot certify native quiescence |
| `SameRepositoryCoordinator::revoke` and rollback | Delegates to lifecycle; provisioning rollback follows with `try_reap` | API report semantics must distinguish prompt authorization close from eventual resource completion |
| `SameRepositoryMergeOrchestrator` in `same_repo_merge.rs` | Obtains two `seal_quiescent` results before publication | A future pending result must never be represented as a valid seal; merge/publish must keep requiring verified quiescence |
| Selected `optic-bridge-mcp` tests in `server.rs` / `git_tools.rs` | Invoke `sessions.revoke` directly while checking cancellation | A launcher-only gate cannot intercept every direct registry close path |

**Proposed lock order, subject to further proof:** acquire a session-scoped **authority-owned** commit/close gate, then briefly inspect/update session authority and epoch under the registry mutex. **Never** wait for admissions to drain or perform unbounded native observation while holding either. A close should publish stop and reject subsequent commits at a single point; only after releasing the short lock should an independent owner request Job termination and perform bounded progress checks. A native resume commitment that wins the gate can precede close; close must not promise that zero instructions execute afterward. The gate must be enforced for *all* real close pathways, not just convenience wrappers. Explicit lock inversion and Windows native-call-bound evidence are still missing.

**Recommended option to evaluate, not approved API:**
- Stage A: `close_admission` publishes permanent cancellation and returns a *pending* close token/status promptly; no new launch can commit afterward.
- Stage B: independent owner requests out-of-band termination, retains native handles and quotas under `PendingNativeProof`, and reports exact Job/stream/worker evidence.
- Stage C: `try_quiescence`/bounded await can create a seal only if admission, all subprocess descendants, both streams, approvals/leases and relevant worktree ownership are actually terminal. Reap remains impossible on uncertainty.
- Compatibility: assess every `SessionRevokeReport` consumer before changing its meaning, define expiry semantics, direct APIs and error results, and obtain an architecture/security decision before patching production.

**Do not do:** transfer all admission permits to unbounded workers under the current synchronous `revoke`; mark a native observation error as successful termination; return a “quiescent” seal when only admission count is zero; add a private mutex only at one launcher; merge the experimental Job stacks wholesale.

The findings narrow the next design decision but **do not close** #317, authorize ADR-0035, repair production `PreparedReadCommand` / `HardenedCommandRunner`, address Git provenance #182, or lift V1 #323 NO-GO. No PC test, actual Git operation, production ABI modification, feature addition, or merge occurred.
