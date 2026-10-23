# #317 — close, stop and quiescence dependency proof

2026-10-23. Research only; production main da34eae7d0efda39d9fb3c6786ec843546926fe9.
Branch codex/317-authority-close-study, based on bcfdf8ff13d685a5809d0ccad179f1b93c3b7791.
No production runner/public revoke contract change, PR327 change or merge.

## New proof

Two process::close_contract_tests use the real SessionRegistry, lifecycle,
ProcessManager and JobStore. The only owner is an inert synthetic JobRecord;
NO OS child is created. A scoped operation worker holds the genuine admission.
The test caller does not add an unrelated permit. Every channel/observation
wait is bounded; scoped threads are joined even on unwind.

Negative control: actual lifecycle.revoke publishes Revoked then remains waiting
for that worker's permit. The actual record's stop_requested remains FALSE:
the lifecycle cannot reach cancel_session yet. The worker would wait for this
stop before releasing admission. This constructs the dependency, then recovers
finitely through actual out-of-band ProcessManager.cancel_session. It is not
an observed natural production/native deadlock.

Positive prototype: cfg(test)-ONLY research_close_admission acquires the ACTUAL
registry mutex, sets its real revoked/stop state and returns (changed, pending).
With the worker admitted, it returns (true,1), then (false,1). Calling the actual
ProcessManager cancellation BEFORE synchronous drainage lets the worker advance.
While observation is deliberately withheld, the record is TerminationUncertain,
still counted; record eviction and physical session removal are refused.
After a finite synthetic proof signal, the worker marks its inert record Stopped
and releases admission; real lifecycle revoke and seal/reap then complete.
Session B continues admission/revoke while A is closed and retained.

The helper is neither a public API nor native launch linearization. It is
absent from production builds. No ProcessRun/GitRead capability is invented:
the injected inert record is accounting-only and never authorizes execution.

## Compatibility result

Moving close and stop earlier changes reports even when later cleanup is correct:
the subsequent unchanged lifecycle.revoke returns session_changed=false and
cancellation_requests=0 because those actions already happened. The test asserts
that difference. A future implementation MUST aggregate the first-close and
first-stop receipts into the existing report, or explicitly approve a new API.
It must not silently reinterpret pending as quiescent or completed revocation.

## Proposed direction for review (not implemented)

Use an authority-owned per-session state cell, never an external launcher mutex.
All direct registry/lifecycle/rollback close paths must share its close primitive.
Normalize and verify authority, pins and budgets before a bounded launch commit.
Under the same cell mutex, check Open/fresh deadline and transactionally register
an unforgeable committed owner ticket. Close sets Closed/stop in that same cell.
No Win32 call or waiting for owner observation may occur under a global registry
lock. A later native call is permitted only for a ticket committed before close;
it still observes stop and remains owned. This is an ENGAGEMENT linearization,
not a promise that no CPU instruction or ResumeThread occurs after closure.
If product instead requires ResumeThread itself before close, a different native
call/lock liveness contract is needed. Do not disguise this distinction.

Pending tickets/native owners must have one root and the same bounded resource
accounting until Job/EOF/worker proof. Async native owner lifetime and legacy
synchronous admission drain cannot be conflated. Preserve legacy revoke behavior
until compatibility review; introduce explicit close/pending/quiescence reporting
only under an approved contract. Repeated/concurrent close needs shared receipts,
not double-counted cancellation. Epoch/record identity must prevent remove/reuse
ABA; monotonic expiry and task-lease closure need their own verified fence.

Other gates: genuine authority-owned commit API, atomic Job association in the
service, durable uncertain-root shutdown, protected output publication, executable
and cwd pins, task lease and effective deadline. None is completed by these tests.

## Local native acceptance slice

The existing three real-registry/Win32 tests are now run under operator-authorized
Windows10 supervision, with a synthetic explicit cwd and temporary directories
under the evidence destination. Negative stale-check resume, close-wins and
launch-wins remain distinct. This validates the old wrapper protocol locally;
it does not turn that wrapper into a production authority-owned gate.

An initial compiler E0277 (borrowed non-Sync mpsc Receiver in a scoped thread)
is preserved locally; the receiver is now moved with cloned worker authority.
No lint/deadline/security limit is weakened. Exact-source CI/local details are
in the separately supplied sanitized evidence archive. #317/#323 remain NO-GO;
#182, #185, #319 and PR327 stay independent. No whole research branch merge.
