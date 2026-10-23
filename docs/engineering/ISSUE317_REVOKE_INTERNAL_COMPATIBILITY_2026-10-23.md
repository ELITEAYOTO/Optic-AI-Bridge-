# #317 — internal revoke phase extraction

Narrow integration candidate on fix/317-revoke-internal-phases-2026-10-23, based on production main
da34eae7d0efda39d9fb3c6786ec843546926fe9. This is not a production fix for #317,
not a two-phase public API and not an integration of any native owner prototype.
Main, PR327, security policy and the Git runner remain unchanged.

## Internal change

SessionRegistry::revoke still acquires the same sessions mutex once, looks up the
record, publishes closure and the Release revocation signal under that lock,
then waits synchronously on admissions_drained. Two private helpers merely name
the operations: publish_revocation(record) and wait_for_admission_drain(handle,
guard). The second returns the same mutex guard, retained by revoke until return.
No unlock/relock gap is added between publication and the first predicate check.

No new lock, generation token, close receipt, clock, timeout, callback, public
method, state field or dependency is introduced. The existing condition-variable
loop remains a handle lookup and rechecks the predicate after every wait.

## Compatibility matrix

| Case | Before and after |
|---|---|
| Missing at initial lookup | Ok(false), no wait or publication |
| First revoke | changed=true, close+signal before waiting, wait for zero admissions |
| Already revoked | changed=false, still wait for any admissions |
| Concurrent revokers | Exactly one first-close true; others false; no successful return before their observed drain predicate |
| Partial drainage / unrelated notification | Notification is not proof; predicate remains the decision |
| Another session | Condvar releases the global mutex; other session remains usable |
| Renewal | Existing renewal before close works; renewal after close is rejected; no resurrection |
| Removed after drainage | Existing removal checks unchanged; later public revoke returns false |
| Mutex poison at entry | StateUnavailable before closure |
| Mutex poison on wait reacquisition | StateUnavailable; already-published closure/signal are not undone |
| Record missing on predicate recheck | UnknownSession, unchanged |
| Record replaced under the same handle before recheck | Current record is looked up by handle, unchanged; drain does not newly close that replacement |

The final row is a preserved limitation, NOT a new safety guarantee. A legitimate
concurrent remove/re-register after the last permit drains can change the record
seen by a waiting revoker. Introducing immutable generations or a different error
would change behavior and is outside this refactoring. The captured changed flag
still describes the record whose close was published. Future contracts must
decide generation and receipt semantics explicitly.

## Tests and evidence scope

Four new public-path tests first pass against the original main implementation,
before extraction. They use genuine permits, two concurrent direct revokers,
another session's final-permit notification, explicit notification, renewal,
absence, post-drain removal and deterministic poisoning at entry/reacquisition.
Success is driven by predicates/channels, not sleep durations; deadlines only
bound missing progress. All spawned test threads are joined. No native child.

Two additional private-helper tests compare guarded states with the frozen
original predicate loop: absence gives UnknownSession; a replacement record with
a genuine new admission is waited for without being revoked. This models the
state at reacquisition; it does NOT force every interleaving of a complete public
remove/re-register race, and does not claim exhaustive concurrency verification.
All16 registry tests, including existing renewal/admission regressions, are run.

The initial test compilation failed because SessionGrant lacks PartialEq; the
test assertions were corrected to inspect error variants. This is a harness
compile failure, not a demonstrated product failure. Local evidence retains it.
Original Codex checkpoint `0be9e7e2ef3761f4f7f5ec91eb2ac87728a89606` passed [hosted Windows/Ubuntu CI #39927465202](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39927465202) on that exact source. The new focused PR must pass its **own** exact-head Windows/Ubuntu CI; evidence from the Codex branch is not a substitute for the integration candidate's checks. The candidate is unmerged and does not constitute a #317 production fix.

## Non-goals / review boundaries

SessionLifecycleManager still waits for registry drainage BEFORE cancelling
processes. This dependency is deliberately unchanged and may remain unbounded
under the previously researched owner-held-permit scenario. No bounded shutdown,
new authorizer, native Job ownership or GitRead migration is delivered. #317 and
V1 #323 remain open/NO-GO; #182/#185/#319 remain separate. Do not merge research
branches in bulk or use this extraction to justify changing revoke semantics.
