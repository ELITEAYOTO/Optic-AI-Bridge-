# Experimental fixed read-link admission

Local contract module only. No HTTP server, real Rust process, user workspace,
real credential, persistence or product authority is exposed by this step.
The next integration composes this behavior with the generated-workspace Rust
link, then validates actual upstream cleanup. Python remains an experiment
harness, not a new core dependency.

One application-selected dummy principal owns one fixed binding. Only session
information, a four-entry listing of `public` and a1024byte read of
`public/hello.txt` are selected internally. A caller cannot supply upstream
tool names, file arguments, project paths or session selectors. Another owner
is refused. Local credential rotation preserves principal identity while old
requests/late results fail epoch checks; rotation does not mint authority.

Eight retained upstream tasks maximum, with immediate refusal and no waiter
queue. Caller timeout (maximum15seconds) or cancellation leaves the admitted
task and capacity reservation intact until completion. Completion callbacks
retrieve detached failures without logging exception strings. Local stop closes
admission first, suppresses late results and waits at most5seconds; incomplete
drain returns false and never resets/reopens the gate.

Seven deterministic/local async tests cover fixed calls, owner denial, rotation
before worker dispatch and while awaiting results, redacted exceptions, invalid
limits, caller cancellation, timeout accounting, twelve-call concurrent pressure
and local-stop suppression. These are fake-upstream contract tests, not evidence
of actual Rust cancellation or remote FranceStudent behavior.

This gate is loop-owned. Dummy registry checks are snapshots; they do not
serialize an actual Rust effect against local session revoke. Integration must
retain the Rust lifecycle/admission barrier and distinguish credential denial
from real application-session cleanup. An upstream task that never completes
keeps its slot; the service lifetime and child teardown remain separate gates.
No automatic retry or replacement task may hide uncertain cleanup. Output bounds
and structured parsing remain obligations of the actual fixed read adapter.
