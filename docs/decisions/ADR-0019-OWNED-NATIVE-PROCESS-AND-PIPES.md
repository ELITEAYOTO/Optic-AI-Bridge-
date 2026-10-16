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
