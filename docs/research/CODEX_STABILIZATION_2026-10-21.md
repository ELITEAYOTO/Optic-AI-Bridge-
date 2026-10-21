# Hosted stabilization research — 2026-10-21

Research only. No user-PC execution, no release, no merge, no new PR.
Main at campaign start: 80fe108b288d2b0b6676d6128d9e9beb922eda3f.
The only open PR is #318 at 49fe3efd34c9ffe9c10c1c5945b37da2c6a74028.
Its first-attempt CI 39585600089 remains red (Windows job 119173829254).
This research branch's CI cannot satisfy that different source head.

## #319 — source critical path

Source anchors: app/src/main.rs main(), provision_process_leases();
runtime/src/task_lease_registry.rs register(), capture_process_authorities();
runtime/src/process_authority.rs capture_with_pin();
core/src/version.rs ContentVersion::from_reader_bounded();
windows/src/executable.rs open_pinned_executable();
mcp/src/server.rs constructor / empty ServerHandler implementation;
mcp/src/transport.rs receive() / send().

Order before first response:
1. Tokio runtime construction occurs before the app's entry marker. A missing
   entry marker cannot distinguish loader/runtime creation from missing capture.
2. Synchronous argument/config parsing, canonicalization, filesystem metadata,
   process-manager construction, exact executable/profile checking.
3. Optional Git construction/recovery and mutation recovery. These can perform
   synchronous filesystem or command work, but the failing TEMP-only doctor does
   not configure these optional services. Do not attribute its failure to #317.
4. Session/read authorities, OS entropy for IDs, process lease provisioning.
5. Registration preflight takes the registry mutex then releases it. Full
   executable identity capture happens OUTSIDE that mutex; a second locked
   capacity/duplicate validation authorizes insertion. Startup is sequential.
6. Identity: canonicalize + regular-file check; open native read-only pin; full
   same-handle BLAKE3 streaming read, <=512 MiB (+one overflow sentinel), fixed
   64 KiB stack buffer. Windows pin permits read sharing and denies write/delete
   sharing throughout authority lifetime. A byte ceiling is not a time ceiling:
   synchronous open/canonicalize/read can wait on OS I/O without a Rust deadline.
7. Server constructs bounded filesystem/routers/governor; transport construction
   wraps stdin/stdout but does not yet answer requests.
8. serve(transport).await performs RMCP initialization using ServerHandler's
   default handshake; no custom initialize file hash is registered. The marker
   mcp-service-started is emitted AFTER this await, so absence is ambiguous
   between handshake receive/send/notification and pre-service startup.
9. JSON-line receive waits for newline/bytes/EOF with bounded line size; send
   serializes bounded JSON, obtains an async writer mutex, writes and flushes.
   No general per-write wall deadline appears in this adapter. A peer retaining
   input or not draining output can stall protocol progress; that does not prove
   it happened in the original doctor, which sends a tiny initialize and reads.

The pre-transport identity path runs synchronously on the main future. Tokio
worker availability cannot make a response appear before the server exists.
Scheduling pauses and Windows filesystem/filter behavior remain plausible,
unidentified contributors. Existing success-only I/O measurements do not
establish the cause of the 30-second natural failure. No repeated cache/buffer
benchmark is scheduled by this campaign.

### New diagnostic experiment

OPTIC_STARTUP_TIMING=1 enables at most eleven fixed progress events for the FIRST
identity capture per process: path/pin/hash boundaries, entry into first Read,
its return, 16/64/256 MiB crossings, EOF and completed hash. No path, digest,
credential, ID, argv or file bytes are printed. All reads and complete identity
checks/pin/byte ceilings remain unchanged; no prefetch/cache or timeout change.

The reader wrapper adds no background thread. A synthetic held reader has an
independent two-second self-deadline and is explicitly released by its observer.
Tests prove read-entered can be observed before the pending Read returns, byte
consumption/digest invariants, and once-only checkpoint cardinality. This tests
diagnostic visibility, NOT reproduction of the natural 30-second failure.
Read errors can produce a returned marker without EOF/hash-complete; interpret
those events literally. Missing progress beyond a threshold is not proof of
the next native syscall being blocked.

Doctor fixed-label extraction recognizes the eleven new labels; the 24-line,
32,768-character / 400-ms collection and 4,096-character preview caps remain.
With many earlier approved markers, later markers can still fall outside these
caps. Progress covers only the first capture, not later verify() calls. Diagnostic
stderr writes themselves are synchronous and can block a non-draining peer;
the added eleven short lines are fixed per process, not an unrestricted trace.
No instrumentation is a security attestation or a proven root-cause correction.

## #317 — deterministic supervised characterization

Unchanged production HardenedCommandRunner blob:
513f291f71b2ab80e2d1de01e287ec0c7575ae67.
The runner's polling deadline starts AFTER process spawn and I/O thread creation.
It limits polling the direct child, then joins blocking readers/writer without a
deadline. It does not use ProcessManager's JobId/TerminationUncertain machinery.
Output quotas bound captured bytes, not pipe lifetime. TimedOut can itself be
delayed by the joins; stdin write errors are reported only after joins return.
No new production fix to this ownership boundary is proposed here.

Run the example exclusively through the independent Python supervisor:
- cargo build --locked -p optic-bridge-runtime --example issue317_pipe_fixture
- python scripts/research/issue317_supervisor.py --fixture-exe
  target/debug/examples/issue317_pipe_fixture[.exe] --output evidence/issue317.json

The example alone is not an operational tool. Synthetic direct parent creates
a known descendant, waits for its readiness and exits. Supervisor separately
confirms parent death and descendant liveness; it retains pipes for one second
AFTER parent death against a runner deadline of 200ms, then releases the holder.
Cases: no inherited pipe (control), stdout, stderr, both, and retained stdin
with a bounded 256 KiB writer. No production project, shell, network or secrets.

Windows: independently assigned Job Object with kill-on-close BEFORE a file
gate permits spawning descendants. Failure to assign fails closed. Job active
count must reach zero. Linux: fresh owned process group and a supervisor-only
subreaper adopt synthetic orphans; group kill, wait/reap and PID checks verify
cleanup. Known helper code cannot escape its ownership group; the holder has
its own eight-second maximum wait. Finally blocks release and terminate ONLY
fixture-owned resources; no arbitrary PID stop API or detached reader strategy.
CI step/job deadlines provide additional external bounds.

A successful research test means the expected baseline deadline violation was
observed and safely recovered. It does NOT mean the production runner is fixed
or its advertised deadline passes. The result JSON distinguishes control,
observed blocking, actual runner elapsed time, error and quiescence. Failure to
reproduce or cleanup remains a failed research job; no retries-to-green.

### Candidate recovery strategy, not implemented

A production correction needs owned descendants from before execution (Windows
job / appropriate Unix group), prompt cleanup on direct exit and on failure,
cancelable/closed pipes, bounded confirmation and joined I/O workers. Returning
after a channel timeout while leaving blocking reader threads detached is
insufficient. Assign-before-spawn races, inherited handle ownership, spawn/wait
failures, stdin BrokenPipe, output exhaustion and termination-uncertain resource
retention all need native regression before changing the runner contract.

## #185 — distinct unchanged unresolved issue

Historical original 10-second timeout had no launcher/Node output; that is a
failed proof, not successful confinement. A distinct successful 9-second probe
showed target_resumed at 14ms and target_exited at 8952ms. That particular slow
run happened AFTER resume; it does not identify a cause in the no-output run.
The request writer in process.rs discards write/shutdown errors; this is a known
observation gap, not a demonstrated delivery failure. Existing native CI retains
the original 10-second AppContainer network proof without relaxation. No extra
identical retries or changes to ACL/network/isolation are added here.

## Evidence and acceptance

The research workflow records OS/Python, exact GitHub source SHA/run/attempt,
per-case timings and cleanup in sanitized JSON. Its aggregate artifact contains
this report, synthetic sources and results only, no binaries or raw runner logs.
The normal CI remains independently required at the exact research head.
First failures and later source corrections are retained in Actions history.
Generated VALIDATION.md in the artifact states actual job outcomes; this source
document is a protocol, not a claim of execution before those jobs complete.

Next actions, in order:
1. Observe a NATURAL #319 failure with bounded in-flight milestones and the full
   approved-label failure evidence; distinguish OS read, hash work and handshake.
2. If #317 is reproduced on both hosted OSes, implement an isolated owned-tree/
   cancelable-pipe correction with complete cleanup regression BEFORE proposing
   integration. Never replace joins with detached threads.
3. Revisit #185 only with new delivery/loader/first-output evidence; preserve
   distinct original red attempts. #318 stays open, unmerged, with its own red CI.


## Retained first campaign attempt — 5f00209e181629ce9536a075f274648a38af9831

CI 39608173136 attempt 1 failed Linux formatting and Windows Clippy's
zombie_processes check for the intentional orphan fixture. Research workflow
39608173235 attempt 1: Linux job 119249744218 reproduced stdout/stderr/both
and stdin blocking at 1010–1015ms vs 200ms; all were externally released and
reaped. This research job first applied the recorded rustfmt-only diff before
compiling; it is initial characterization, not final exact-byte acceptance.
Windows job 119249743554 failed its control assumption: a descendant launched
with all selected stdio redirected to nul still kept the runner from returning
within one second after direct-parent death. That surprising observation is
retained, not classified as an ordinary Node/MCP failure.

Refinement: add a genuine no-descendant control; retain the original nul-stdio
descendant as a separate measured characterization with no guessed outcome.
The explicit-retained stdout/stderr/stdin tests still require the original
deadline violation and complete cleanup. The deliberate orphan is annotated
with a statement-scoped expected Clippy lint and its verified external ownership
reason; production lint/test/security gates remain unchanged. Commit source
formatting and require exact-source formatting in the final research workflow.
No change to the production HardenedCommandRunner implementation.

Windows nul-stdio retention needs separate interpretation; it may involve
additional inherited handles, but the experiment alone does not identify the
specific retained handle or establish a general Rust/Windows defect.


## Exact second attempt — 2f2f608d6768b073bbc3992d1177598e2f803beb

Supervised research workflow 39609013851, attempt 1, completed Windows and Linux
characterizations with all owned fixture processes quiescent. Windows Server
2025 / Python 3.12.10: no-descendant control 18ms; nul-stdio descendant 1025ms;
stdout 1024ms; stderr 1023ms; both 1026ms; retained stdin 1026ms, then BrokenPipe.
Ubuntu 24.04 / Python 3.12.3: control 5ms; nul-stdio 10ms; stdout 1013ms; stderr
1012ms; both 1012ms; stdin 1012ms, then BrokenPipe. Each runner deadline was
200ms; external release occurred one second after verified direct-parent death.
These are controlled deadline violations, not an infinite-duration benchmark.

Normal CI 39609013696, attempt 1, then exposed a test-only drop_non_drop Clippy
error in the progress checkpoint regression. Replace the explicit drop with a
lexical scope; no lint suppression for runtime/test code. Retain that red run.
Keep native I/O measurement INSIDE the progress wrapper, so existing read_ms
continues timing native reads rather than including progress stderr emission.

A further falsifiable Windows inheritance comparison adds selective-null:
the known hosted Python creates the exact same Rust holder with close_fds=True
and nul stdio, while the Rust parent still exits. Original nul-stdio and
explicit-held-pipe cases are retained unchanged. If selective-null returns
while ordinary nul-stdio blocks, non-standard inherited handles are implicated
in THIS fixture; the exact handle and Rust/OS mechanism remain unproven.
Failure to obtain that contrast stays visible. This is not a production workaround,
a Node startup fix, or authority to use ambient Python in the Bridge core.
