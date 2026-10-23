# #317 — Owner commitment, admission close and native quiescence protocol

**Status:** RESEARCH / proposed acceptance contract; not an accepted ADR, a production correction or V1 approval.
**Source baseline:** `main=3d27daa7fcb9fd0fb1abc4f14513b9ae18a58e10` (2026-10-23).
**Scope:** local Windows 10/11 Git *read-only* candidate only. Existing Ubuntu CI stays. This document changes no runtime policy.

## 1. Concrete source observations (on the baseline)

- `SessionRegistry::begin_admission` increments a counted admission under the registry mutex and captures the **same original-record** read-only revocation signal as `SessionRevocationWatch` (#331). A historical handle can later be reused without retargeting this watch.
- `SessionRegistry::revoke` publishes `revoked=true` and that signal under the registry mutex, then synchronously waits on a Condvar for all counted permits; it does **not** initiate a native stop. `SessionLifecycleManager::revoke` requests `ProcessManager::cancel_session` only *after* the registry drain. Existing callers depend on these orderings.
- In the experimental `ReadServiceRoot::submit` at `codex/317-service-retention=1c7dac9`, the genuine permit covers submission, **not** the native worker. This lets revoke finish while an owned worker can remain pending. `JobStore` correctly still blocks seal/reap while its entry is active or uncertain. Moving the permit directly to that worker without an earlier independent stop risks waiting forever inside revoke.
- At `codex/317-native-registration-authority=bcad195`, a **test-only** fixed executable uses `PROC_THREAD_ATTRIBUTE_JOB_LIST` with `CreateProcessW`: the Job is associated at process creation, unlike the old separate suspended-spawn/assign window. Its 22 externally supervised hosted native modes succeeded; it is not a production GitRead implementation. Windows 10/11 deployment compatibility is not yet certified.
- Production `HardenedCommandRunner::run_inner` still joins pipe reader/writer threads after direct-child completion with no whole-tree/EOF deadline. Neither experiment fixes that existing code path.

## 2. Two operations must never be confused

**A. Authority close:** the irrevocable publication that no *new owner commitment* is permitted for this session generation. It issues persistent cancellation intent for already-admitted operations. It does **not** certify native termination.

**B. Proof of quiescence:** the service has positively observed root exit, zero processes in its Job, both output EOFs, writer completion or null-input proof, controller/watchdog completion, and durable result classification; only then may it release the corresponding admission, native handles and reserved quotas. A failed observation is `TerminationUncertain`, not success.

A `SessionRevocationWatch::is_revoked() == false` is **not** authorization; a true watch is a stop signal only. The captured *absolute monotonic deadline* must be checked independently of the explicit revoke bit and must not be extended by subsequent session renewal. Task lease, typed GitRead effect, policy epoch and resource ceilings remain distinct authorization prerequisites.

## 3. Proposed ordering / linearization

A future **internal, non-client-constructible** Git owner would follow:

1. Validate typed intent and trusted session/task/executable/repository scopes. Obtain a genuine session admission and the original-generation watch. Derive a finite operation budget from monotonic now, immutable admitted session deadline and task deadline.
2. Outside session and global JobStore locks, create/configure an **empty** bounded native Job and pipe/controller resources, retaining ownership on all failure paths; acquire an actual host memory sample without native API calls under the JobStore mutex. Reserve finite shared capacity and register an identified, **not-yet-running** JobStore owner **before** irreversible native creation. Do not free this record until terminal proof.
3. Under **the same authoritative session close/commit serialization point used by every direct and lifecycle revoke path**, recheck original session generation, monotonic expiry, policy/task validity, stop and shutdown. Commit only a short, typed internal ticket that refers to the already-owned/registered record; do not perform Win32, disk I/O, allocation-heavy callbacks or waits while holding the authority lock. A research-only gate is not sufficient if production paths can bypass it.
4. After leaving the authority lock, check stop/deadline, create the child **suspended and already Job-associated at creation**, then check stop/deadline before resume. All handles remain root-owned and cancellation must be deliverable *out of band* even if the caller goes away. A legitimate close can race with the final check/resume; the contract linearizes **permission to engage**, not every instruction executed afterwards. If close wins before commit: no child creation. If commit wins: any later close requests stop, including during the native call.
5. Publish an authorized result only after native terminal proof and a fresh *publication* policy/lease/revoke check. Otherwise retain bounded private output only as needed for diagnosis and return an explicit cancellation/uncertainty classification without leaking post-revocation bytes.

**No single early `ProcessManager::cancel_session` scan solves the late-registration race.** A prepared admission may outlive a scan and register later; the original-generation irrevocable watcher is needed in addition to the short commit gate. The JobStore reservation alone is not the authority to launch.

## 4. Shutdown / revoke compatibility choice

**Preferred candidate for review:** separate internal `close + stop-request` from `drain + proven-quiescence`, while preserving the existing public synchronous revoke report until its callers and liveness guarantees are audited. An internal pending receipt must refer to retained actual owners, not to a timeout or to a detached worker.

- **Close phase:** publish the irreversible generation-bound stop intent while disallowing new commits and admissions. It must not wait on worker-held admission permits. A worker observing its captured watch can request stop without first requiring `ProcessManager::cancel_session`.
- **Stop phase:** initiate stop for any *already* registered root and its Job. A stop request and `TerminateJobObject` return are not proof of empty Job or EOF. The watcher also covers registrations after an earlier cancellation scan.
- **Drain phase:** asynchronously or under an explicitly documented waiting API, await actual worker/permit completion and cleanup. Only the existing admitted-generation owner may release its own reservation. `seal_quiescent` and `try_reap` must continue to reject active/uncertain entries. Do not silently change `SessionRevokeReport` into a “terminal process” receipt.
- **Shutdown phase:** stop admitting work, retain all native owners and quotas while pending, expose a read-only pending count/reason and a bounded *request* API. A `Drop` joining a permanently unobservable worker can still block forever. A separate lifecycle/guardian contract or explicit host-exit fallback must be designed **before claiming bounded graceful shutdown**. An externally supervised fixture is not a durable production guardian.

**Rejected as premature:** transferring permits blindly and retaining current “cancel after drain”; moving `cancel_session` before the drain *as one scan*; treating a cancellation flag, `try_wait`, elapsed deadline or direct child exit as native terminal proof; detaching or forgetting owner/threads; reclaiming shared capacity from an uncertain Job; assigning Job after spawn while claiming crash safety; publishing Git patch bytes after revocation without a separate provenance decision.

## 5. State and responsibility table (proposed, not code)

| Observable state | Admission / commit | Stop requested | Root / Job / pipes / quota owner | May publish result? | May seal/reap? |
| --- | --- | --- | --- | --- | --- |
| Prepared, uncommitted | Admission held; commit must revalidate | Watch/expiry may become true | Identified service root; reservation registered | No | No |
| Committed, not created | No other authority inferred | If closed/expired, fail before native create | Same root | No | No |
| Native running or suspended | No new permission from old ticket | Persistent watcher, deadline, root shutdown | Same root | Only after completed proof and fresh policy | No |
| Termination uncertain | Closed for new engagements | Yes / retry observation without pretending success | **Same** root and capacity | No successful output | No |
| Proven quiescent | Admission released only now | May be set | Terminal history only; native reservations releasable | Fresh policy check required | After registry/lease checks |
| Service shutdown pending | No new submissions | All owners signalled | Service root retained; no silent detach | No | No |

The state model is *per owner and session generation*; it does not imply a new public state enum or license to overwrite existing `ProcessStatus` classifications.

## 6. Acceptance experiments required before a runtime PR

1. **Close-before-admission / late registration:** use real `SessionRegistry`, real admission and JobStore; an early empty cancel scan must not let a later owner create a child. Check attempt-count, not merely eventual child death.
2. **Commit-vs-direct-revoke and lifecycle revoke:** schedule both orders; prove the single shared authority linearization, no unrelated-session interference, and no Win32 call within a session or JobStore lock. Cover record removal/re-registration of an opaque handle (ABA).
3. **Worker-held permit:** true watch causes real worker stop/drain without waiting for a late cancel scan. With observation withheld, close remains published and owner/permit/quota remain retained; an API claiming bounded shutdown must not block indefinitely.
4. **Windows create-time Job membership:** assert no pre-assignment survivorship gap for tested supported OS builds, including failed `CreateProcessW`, failed `ResumeThread`, revoked suspended child, descendant-held pipes, controlled host termination, inherited Job handles and process breakaway rejection. All cases run under an independent outer Job and empty-tree proof; capture first failures.
5. **Deadlines / resource bounds:** monotonic expiry before commit and before create, controller-clock expiry, session renewal not extending a captured deadline, combined stdout/stderr ceiling, memory/CPU/process accounting, nullable and actual stdin lifetime.
6. **Native observation uncertainty:** repeated query/terminate errors hold capacity and handles; recovery completes without double release; host shutdown is classified separately from graceful quiescence.
7. **Real sealed read-only GitRead:** action/task/session/host authority, executable pin, repo and cwd identity, controlled env, bounded output, result suppression after revoke. Review #176/#182 patch provenance **separately**; Git mutations/CAS are out of scope.
8. **Integration evidence:** exact-head Windows + Ubuntu + dependency and specialized hosted CI, Windows 10/11 operator-authorized acceptance, documented failing first attempts, then independent exact-integrated-`main` CI. A green research fixture never closes #317 or #323.

## 7. Next engineering increment / no-go boundaries

The smallest safe next **runtime** candidate is an authority-owned, generation-bound, *short* owner-commit gate with a sealed internal issuer and actual previously registered owner. Its direct and lifecycle revoke paths must share the close serialization and notify the independent watcher before waiting on permits. Add deterministic interleaving tests **before** considering a public two-phase API. Independently audit current `SessionLifecycleManager` consumers and `SessionRevokeReport` before changing their semantics.

Production code, `ADR-0035` acceptance, #182 policy, PR #327, remote access, release settings and V1 #323 remain unchanged by this document.


## 8. Source-level lock and caller audit before implementation

The existing implementation adds two *independent* liveness constraints which a service integration must not conceal:

- `ProcessManager::cancel_session` (main `process.rs`, near line 720) first acquires the **global JobStore mutex**, then reads each matching record's `state` mutex before publishing `stop_requested`. It signals already-owned `Running` **and** `TerminationUncertain` records (PR #330), but is neither lock-free nor a substitute for an owner-specific watch. If a record is registered late or JobStore is contended, a one-time scan cannot be the sole stop delivery path.
- `ProcessManager::prepare_store_for_start_with_workload` (near line 1004) calls `ensure_host_memory_headroom` while its caller holds JobStore. On Windows, the default provider uses `query_host_memory_snapshot`; this is native host observation inside a global lock. The experimental sample-scope adapter is not in production. A new owner must **not** assume cancellation is independently prompt while this call runs. Moving the observation outside requires an atomic reservation policy and freshness contract, not a fabricated snapshot or a relaxed limit.
- `ProcessManager::start` (near lines 589–595) schedules the isolated launcher stdin write/shutdown in a Tokio task and currently ignores both I/O errors. This is separately tracked in #185 as a diagnostic blind spot; it is **not** evidence that those writes caused the intermittent Node/AppContainer timeout. This #317 proposal does not modify it.

Before implementing the pending receipt or shortening a lock, construct an explicit lock-order graph for `SessionRegistry.sessions`, a future owner commit gate, `ProcessManager.jobs`, per-record state/output locks, task/approval stores and any native handle lock. For every path, prove that closing authority and the per-owner stop latch remain observable when a native call or observation is deliberately stalled. No IPC, OS observation or callback of uncontrolled duration may be introduced inside the commit critical section.

**API consumer inventory to obtain:** all direct `SessionRegistry::revoke` callers; all `SessionLifecycleManager::revoke` callers including `seal_quiescent`, `try_reap` and approval cleanup expectations; all `SessionAdmissionPermit` holders; all paths that can insert, terminate or remove JobStore entries. A public-return-semantic change is **not** part of the next slice.
