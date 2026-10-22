# #317 — Real SessionRegistry signal versus native Git owner: acceptance boundary

Status: **RESEARCH ONLY, NOT A PRODUCTION FIX OR ACCEPTED ADR**.
Date: 2026-10-22. Scope: Windows V1 stabilization, without user-PC execution.
Base production main at review: `82ad1234f184591e0f2c80c86296a65a883ecc20`.
Study branch: `bootstrap/issue-317-windows-rust-owner`.
This note extends, but does not supersede, proposed ADR-0035 and the #317
native integration contract. Do not merge the research branch wholesale.

## Source-derived facts

1. Real `SessionRegistry::revoke` first sets `revoked=true` and stores
   `revocation_signal=true` with `Release` semantics, then waits on
   `admissions_drained` for `active_admissions=0`.
2. On Windows, `SessionRegistry::watch_active` returns a cloned
   `Arc<AtomicBool>` for one validated session and its actual grant. A watcher
   can observe `Acquire` without holding the registry mutex or occupying a
   normal transport request slot. It is a **signal**, not proof of Job termination.
3. A `SessionAdmissionPermit` currently has lexical Rust lifetime tied to the
   registry; dropping that permit ends admission. Dropping the MCP requester
   reply receiver alone need not drop a service-owned permit. However simply
   retaining a permit is insufficient to keep a native Git owner accounted.
4. `SessionLifecycleManager::revoke` calls this blocking registry revoke
   before `ProcessManager::cancel_session`. Git commands through
   `HardenedCommandRunner` are not in that process registry; direct-child
   observation and unbounded joins remain the issue #317 defect.
5. `seal_quiescent` and `try_reap` currently consult the session
   `ProcessManager` only. Neither is presently guarded by a Git-owner table.
6. `watch_active` alone cannot authorize a new native launch: the
   admission/owner-registration/creation race against revoke must fail closed.

## New scoped real-registry regression

The Windows-only `session::registry` unit test
`revoked_session_signal_reaches_admitted_worker_after_caller_drop` exercises
**real** `SessionRegistry` instances, grants, permits and revocation
flags with two different sessions. An admitted server worker survives loss
of the caller reply receiver. Revoke of session A closes new admissions and
signals A **before** its held permit drains; unrelated B retains admission
and its signal remains false. When service-held work releases its permit,
the real revoke completes.

The worker in this test is deliberately **not a native Git owner**. No
Win32 Job or pipe termination is observed by this regression itself.
The 13 prior supervised native Windows cases remain independent evidence,
not transitive proof of a real production integration.

## Proposed typed service seam (review before implementation)

- `GitOperationAuthority::Session`: validated application principal,
  project, server-minted SessionHandle, lease, canonical repository,
  executable pin and effect kind; bound to the real revocation flag.
- `GitOperationAuthority::HostStartup`: **separate** internal privilege
  for narrowly scoped startup recovery; never fabricate a session to fit it.
- `OwnedGitOperationId`: service-minted, opaque, not a user-supplied PID.
- `reserve_before_spawn(authority, budget)`: atomically reserve finite
  process/CPU/memory/output/I/O/uncertain-state capacity. Deny before effect
  when any required resource or proven containment is unavailable.
- `adopt_suspended_root(id, handles)`: retain native child, Job, pipes and
  optional stdin writer before any resume. Assign restricted Job before resume.
  If revoke raced against registration, observe flag and stop before resume.
- `request_stop(id, validated_authority)`: independent of ordinary MCP
  transport capacity; no cross-owner stop, no direct PID API.
- `observe_terminal(id)`: require native tree-empty + process exit + stdin
  worker joined/cancelled + stdout/stderr terminal EOF before releasing owner
  and resource reservations. Missing or failed observation retains an explicit
  uncertain owner, not a discarded join thread or just an error enum.
- `count_outstanding(session)`: include unresolved Git records in both
  session seal and reap. The registry must not be dropped as cleanup for
  uncertain jobs; crash/shutdown ownership needs a separate native proof.
- `GitRefOutcomeUncertain` is independent of `TerminationUncertain`;
  read-only Git migration comes first, then separately audited CAS recovery.

## Concurrency ordering to prove next

1. Acquire real session admission and bind verified authority.
2. Reserve service owner and register its immutable identity **before spawn**.
3. Recheck revocation and authoritative capability/lease state at side-effect
   admission. A revocation racing between 1 and 3 must forbid resume or
   require an owned stop.
4. On revoke, notify owners through the existing pre-drain flag and trusted
   control; do not wait for an ordinary transport slot to free.
5. Retain owner while native stop/pipe cleanup is uncertain. Revoke may need
   an explicitly bounded wait/recovery result, but must **never falsely seal**
   or reclaim the session and reservation while Git still owns live handles.
6. Session A's stop/reap must not affect session B nor HostStartup.
7. Interrupted host/crash and Git mutation outcome need independent evidence.

### Explicitly unproved

No production owner API, cross-process crash recovery, resource quotas across
multiple Git operations, genuine Win32 failure retention, authenticated
out-of-band stop routing, Windows 10/11 installation acceptance, or real Git
read/CAS migration is delivered by this test. It does not resolve #319/#185 or
#176/#182. Keep #317 OPEN and do not relax the sandbox, permissions or deadlines.

## Validation required for this research change

- Exact research SHA hosted Windows `cargo fmt`, Clippy and all unit tests
  including the new `cfg(windows)` regression.
- Existing Ubuntu cross-platform tests and dependency policy.
- Dedicated native owner workflow remains tied to the unchanged Win32 source;
  the last exact native fixture source still has 13 supervised green cases.
- Retain first failed CI attempts and publish exact SHA/run IDs in issue #317.
