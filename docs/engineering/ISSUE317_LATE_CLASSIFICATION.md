# #317 — reject successful results completed after the runner deadline

## Recovery/state checkpoint
Official repository only: ELITEAYOTO/Optic-AI-Bridge-.
Base: ea7374133522e0cad48a4fa54bdb6ac83ab5b19b.
Dedicated branch: fix/317-reject-late-command-success.
Operator PC: no commands, builds, test processes or source edits performed.
Local recovery location inaccessible — not verified (Desktop Commander usage limit;
do not retry/reconnect). Existing local branch/snapshots must remain untouched.
This separately authorized GitHub-only task does not resume the local experiments.

Phase: regression staged against unchanged production code; hosted results pending.
Next: inspect actual Windows/Ubuntu regression result and cleanup receipts before
making the production change. Preserve the first failure. No merge authorized.

## Narrow contract
HardenedCommandRunner may currently return Ok after reader/writer joins outlive
its existing measured deadline (prior #339 stdout/stderr and #341 stdin).
The intended production change rejects that late result as TimedOut ONLY after
existing I/O/output errors have been handled. No timeout, permissions, ceilings,
native ownership, error precedence or thread lifetime mechanism is relaxed.

This is a result-classification correction, NOT a bounded-return fix: joins can
still wait indefinitely for hostile inherited pipes. Deadline errors do not prove
that a command had no effects. Callers must still reconcile mutations.
The measurement interval still starts where existing production starts it,
after spawn/I/O thread setup; end-to-end startup budgeting is not added.

## Deterministic regression plan
Reuse the finite 1.6s fixed descendant approach of #339/#341, inverted to require
TimedOut for stdout, stderr, both, and stdin late completion with a 200ms budget.
Preserve fast success, immediate BrokenPipe, late OutputLimitExceeded and late
BrokenPipe priority. The expected baseline failure must identify an actual late
Ok result; a compiler/fixture failure is not product evidence.

Only fixed self-test executables and private synthetic fixture directories.
An independent Python owner gates execution until Windows Job or Linux session/
group containment is established. It retains its group leader until host-only
accounting is observed, then requires zero members before emergency cleanup.
Linux process-group ownership here assumes trusted fixtures do not escape the
group; it is not a sandbox or persistent guardian. No PID lookup is used to select
a target process; normal cleanup uses owned Popen/Job handles and the anchored
test-created Linux group. No unsafe is added to the runtime.

All fixture libtests are ignored by the default suite and must be explicitly
executed by the hosted supervisor, which records real test selection and first
failures. No binary/private data is uploaded. No external network fixture.

## Integration limits
#346 independently addresses a native error conversion and is not duplicated.
#327 remains unmerged. This branch alone cannot resolve #317/#323, native
uncertainty, lost stops, stdin cancellation or durable shutdown. V1 NO-GO.
