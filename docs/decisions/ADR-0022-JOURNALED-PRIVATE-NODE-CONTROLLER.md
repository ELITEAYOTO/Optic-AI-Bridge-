# ADR0022 — Host-only journaled private Node controller

Status: CANDIDATE / NO MODEL ENTRY POINT
Date: 2026-10-16

The private preparation worker may consume its prepared lease for one installed,
explicitly pinned Node executable and one entry in the sealed input manifest.
ProcessRun still requires the live owning session/process lease, matching epoch,
denied network and interpreter/repository execution class. Every captured input
shares the authorized read lease. Immutable inputs are reverified under native
directory pins; no source directory is recursively granted to the child.

Arguments and the complete environment are fixed by the controller. Node gets
read permission for private input copies, no write/child/addon grant, no inherited
HOME/PATH/NODE_OPTIONS or credentials, and private temporary/application paths.
Node permission flags supplement a zero-capability AppContainer; a configured
owned Job is assigned before the only resume. Core never requires installed Node.

The existing bounded ProcessManager reserves job/memory/CPU/output capacity and
owns owner-scoped results, stop signals and stdout/stderr. A retained ActionId can
be reconciled after a lost start reply; the same retained action cannot start
twice. Eviction or host restart does not provide durable exactly-once semantics.
Request/blocking-I/O permits survive the synchronous start effect even when the
caller disconnects. Terminal records follow owned-tree and profile cleanup.

An independent watchdog is armed before resume using the smaller of the action
budget and initially observed session/read/process TTLs. Atomic revocation/stop
observation avoids filesystem hashing in the running monitor. The launch fence
ends after resume, before post-resume journal I/O; revocation can therefore return
and request owned termination while the worker is blocked on bookkeeping.
Uncertain termination retains the stage; neither a timer nor a stop request proves
the tree empty. Cleanup uses the shared I/O governor and exact owned identities.

ADR0020 specifies the optional exclusive journal, strict transition/capacity
bounds and restart evidence. No journal observation restores authorization, opens
a PID, recreates a native profile or replays an effect. Unknown/partial records
remain a local operator reconciliation gate. stdout/stderr remain bounded in RAM
and are not persisted as durable crash logs in this increment.

Validation includes nine installed-Node cases (roundtrip/write denial, stop,
overflow, timeout, session/read/process revocation, authorization expiry, and
revocation while the owner is paused before journal acknowledgement), journal
capacity/corruption/restart and preparation cleanup. A separate opt-in deliberate
owner exit runs only on disposable CI and verifies quarantine without replay.
The pause hook exists only under cfg(test), has a bounded wait and cannot be
enabled through a production environment variable. Windows/Linux exact-candidate
CI and native artifact reproduction precede main integration.

This is a host API foundation, not a complete remote alpha. Explicit project
enrollment, restart reconciliation, durable bounded output diagnostics, private
writable build outputs/artifact publication, dependency restore and GUI execution
remain separate authorization and validation gates. No existing MCP tool routes
to this controller and no general build command or policy edit is introduced.
