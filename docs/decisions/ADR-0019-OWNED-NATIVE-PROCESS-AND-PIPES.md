# ADR0019 — Owned AppContainer tree and bounded native pipes

Status: ACCEPTED / PLATFORM PRIMITIVES ONLY
Date: 2026-10-16

Containment consumes an owned not-yet-resumed AppContainer child, configures a
limited Job Object and assigns it before resume. No PID is accepted. Double
resume is rejected. A root exit is insufficient: completion requires the owned
job to report zero processes. Stop attempts termination and confirms that same
condition within a bounded 1..5000ms interval; errors preserve uncertainty.
Kill-on-close is the containment backstop, not proof of completed cleanup.
Policy, resource admission, authority identity and workspace scope remain the
runtime caller's responsibility. A Job Object alone is not a security sandbox.

CapturedOutputPipe owns one anonymous-pipe reader and an optional parent writer.
Only the writer handle can be borrowed for explicit child stdio. After creation
the parent closes its writer; the reader is neither exposed nor inherited. A
native availability probe followed by a bounded read is safe because no competing
reader can consume those bytes. Each chunk is 1..65536 bytes; an idle pipe is
separate from confirmed EOF. Callers must bound retained aggregate output and
must not equate truncation with successful execution. There is no disk spool.

Tests cover assignment while suspended, rejection after resume, double resume,
normal completion, suspended termination, invalid stop deadline, idle/data/EOF
and output chunk bounds. A focused Windows harness supports local reproduction.
These primitives expose no new MCP tool, dependency installation, network or GUI
authority. Optional Node/Electron/Rust toolchains remain outside Optic core.

## Independent deadline and stop observation — 2026-10-16

A contained suspended job may arm exactly one positive, nonrenewable native
watchdog before resume. It owns a noninheritable duplicate of that exact Job
handle, a one-message cancel channel and at most four atomic stop flags. No PID,
callback, registry lock, filesystem operation or policy lookup enters its worker.
The admitting runtime must bound concurrent jobs; this primitive grants no launch
authority. The deadline starts when armed, not at a later monitor or journal ACK.

The worker requests TerminateJobObject on expiry or a flag, observing flags at
bounded20ms waits. A termination request is not confirmed-empty acknowledgement:
the owner must still confirm the tree is empty before profile cleanup or capacity
release. Deadline status distinguishes timeout from a stop flag. Dropping the
owner cancels/joins the watchdog and releases its duplicate before closing the
final owning Job handle, preserving kill-on-close semantics. It cannot renew or
detach the job, recover a process after host death, or restore authorization.

The native regression keeps findstr waiting on an inherited input pipe while
deliberately doing no owner polling: both expiry and an atomic stop terminate the
owned tree. Zero, excess flags, rearming and arming after resume are refused.
