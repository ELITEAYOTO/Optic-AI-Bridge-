# Physical process signal to original stdio cleanup

## Schema 4 — unread stdout must not retain root exit (#429)

The existing stalled-output case now requires root exit 1 within the unchanged
five-second wait while parent stdin and stdout's reader remain open and output
is not consumed. Original cleanup precedes input join and confirmed output join;
queued output remains after exit. JSON records `characterization_only: false`,
`stdout_writer_joined`, `exit_with_unread_stdout`, both open-end observations and
`released_by_stdout_reader_close: false`. This gate is for ordinary synchronous
Windows pipes with metadata traffic; it does not cover active admitted work.
The normal Windows signal/headless cases and separate Unix characterizations
remain required. [Ownership and limits](../decisions/ADR-0038-WINDOWS-STDOUT-OWNERSHIP.md).

Preserve first #429 HEAD `27f6b95c92b4a278bf2f81697c8c342b1668f6f4`,
[CI #40919730293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40919730293):
Windows passed three signal cases then failed in the observer's `phases()` with
IndexError while stderr contained a partial timing prefix. The stdout case was
not reached, so this is a harness failure, not target behavioral RED. The
corrected observer waits for complete valid records and rereads final logs on
process exit; all phase and exit assertions remain. Three offline parser tests
run before the physical proof. Final exact-head/main receipts belong in #429/#323.

Historical schema-3 characterization below remains evidence of the prior defect,
not of the new ownership implementation. Windows V1 remains NO-GO.

After the observer correction, second test-first HEAD
`e9aa3dc7d21db4a617bb0cd5d8f166b8c86c192b`,
[CI #40921106974](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40921106974),
reached the actual target: all four normal Windows signals and headless EOF
passed, then unread stdout exceeded the unchanged five-second root exit wait.
Stderr confirms host stop, original cleanup and input join first. This is target
behavioral RED on the unchanged Tokio-output runtime, separate from the earlier
observer failure. The correction also requires the output-join marker in every
normal Windows/headless case. In the stalled-output case, neither parent endpoint
is closed or drained to make exit pass; headless EOF still closes input normally.

First corrected owner `029726bc22e72b42537a92b81bb737cf37e6eba3`,
[CI #40922838253](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40922838253),
passed all four jobs and all six Windows cases. Unread stdout reported 4280 bytes
before stop and 3852 after cleanup, both parent ends open, both joins and exit 1,
with normal supervisor reaping/empty Job. Subsequent review adds synchronous-mode
attestation to both original IO factories and an overlapped-handle rejection
regression. That final guarded source still requires its own exact-head/main CI.

## Current schema 3 — separate unread-stdout characterization (#428)

Schema 3 preserves all prior normal cases and adds one Windows-only
`stalled-stdout` case with `characterization_only: true`. A passing report means
that a known output-lifetime limitation was reproduced, **not** that bounded
stdout shutdown is implemented. Historical schema-2 evidence below remains valid
for its earlier, narrower input-reader scope.

After MCP initialization and a metadata round-trip, the parent sends a fixed
32-request `session_info` batch (at most 4096 input bytes), then stops reading
stdout. The parent uses unbuffered pipe handles and read-only
`PeekNamedPipe`/`GetNamedPipeInfo` observations; it neither duplicates the output
reader nor drains it during the retention interval. It sends physical Ctrl-Break
inside the existing private console, with the original Job/supervisor safeguards.
The fixture requires, in order, host-stop observation, original-session cleanup,
and `stdin-reader-joined`, then a root still alive after 500 ms with both parent
pipe ends open and queued output still present. It closes **only stdout's reader**,
requires ordinary exit 1 within the existing five-second exit bound, and checks
that parent stdin stayed open through exit. Forced termination remains failure
cleanup only; success requires a reaped worker and an empty containment Job.

First candidate `9e0750b4cdb742bcb8a834180f594471bd9c5951`,
[CI 40877904438](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40877904438),
attempt 1, all four jobs successful: Windows reported 4280 queued bytes before
stop and after cleanup, with a reported pipe capacity of 4096 bytes. These are
observed values, **not** a strict fullness threshold, a universal capacity bound,
or a sampled kernel write-thread state. Microsoft's APIs permit these observations
on [anonymous pipe handles](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-getnamedpipeinfo);
[anonymous pipe writes can wait for a reader](https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipe-operations).
The process-lifetime observation and output-reader release are the asserted result.

Expected reports are six Windows cases (four normal signals requiring cleanup,
input join and exit with parent stdin open; headless EOF/join; this characterization)
and the four existing Unix signal characterizations. The metadata batch proves
neither active admitted file/process work nor uncertain-stop behavior. It changes
no runtime code, task authority, deadlines, input-handle support, or GitRead gate.
Windows V1 remains **NO-GO** until independent outstanding gates have their proofs.

## Current #427 Windows synchronous-pipe gate (schema v2)

Windows now requires normal root exit within the existing external 5s wait while
the parent input pipe is still open. JSON records exit_with_stdin_open=true,
final_exit_code=1 and stdin_reader_joined=true; cleanup must precede the reader
join marker. The headless EOF case separately requires code0 and a join marker.
The sender closes input only after the Windows exit requirement has succeeded;
failure recovery cannot make it pass. Native ownership is specified by
[ADR-0037](../decisions/ADR-0037-WINDOWS-PIPE-STDIN-OWNERSHIP.md).

The Windows binary claims a standard synchronous pipe once before grants; console
and disk input are rejected. The dedicated reader is cancelled on its exact thread
and actually joined after original admission cleanup, never detached. Unknown IO
completion may still retain that owner; this is not a universal process-exit bound
or active-work/stdout/native Git/Job/EOF acceptance. No policy deadline is reset.

Test-first bb0efacb67b98fe4e8e530b6554e7b1503bbf758 CI #40869915928 failed the real
Ctrl+C exit wait after receipt/cleanup with input open. Corrected-code
eb886b72b9b7021af95748fa6263583993b0f20d CI #40871949465 passed 4/4 plus three
native workflows: four physical Windows open-input exits and reader joins, and
normal headless EOF. The headless join assertion is strengthened on final HEAD.
Final-head and independent-main receipts remain required in #427/#323.

Unix retains its former characterization: alive_500ms_after_cleanup and explicit
input closure before reaping, with exit_with_stdin_open=false. It is not included
in the Windows exit acceptance. The original #426 method and observations below
are historical for Windows and still describe the Unix characterization.

Scope: Optic AI Bridge only. Windows V1 **NO-GO**, no release or native GitRead
activation. This proof observes the real Bridge binary; it does not provide a
global host/policy native-launch fence or a process-exit deadline with open stdin.

## Observable boundaries

With the existing `OPTIC_STARTUP_TIMING=1` diagnostic opt-in, three fixed stderr
markers expose listener registration, a successfully received OS stop and return
from the original cleanup path: `host-shutdown-listening`, `host-stop-observed`,
`original-session-cleanup-complete`. The last marker follows successful cleanup
handling, including the existing already-unknown-session tolerance. It is not a
separate registry-state, native termination or stdout/stderr EOF receipt.

Normal diagnostics-off operation emits none of these markers. No caller supplies
a stop flag, changes permissions, renews the original deadline, releases admitted
capacity or bypasses `SessionLifecycleManager::revoke`. Existing real-registry
RMCP tests separately check original session/TaskLease stop and retained admission.

## Hosted external supervision

`scripts/tests/host_shutdown_supervisor.py` refuses execution unless the standard
GitHub-hosted runner environment is present. CI builds the binary on both OS,
runs the fixture and preserves JSON even on failure. Source head, checkout SHA,
run and attempt are distinct fields; incomplete evidence is never acceptance.

Windows runs Ctrl+C and Ctrl+Break before initialize and after a real MCP
initialize/session_info exchange. The worker owns a hidden private console. An
external Job is assigned before the worker may create Bridge; the exact console
client set must equal worker plus Bridge before any event is broadcast. Only the
worker installs its own protective handler; Ctrl+C ignore inheritance is cleared.
Successful fixture exit and empty external Job are checked before recovery.

Ctrl+C cannot be restricted to a nonzero console process group, hence the private
console requirement ([Microsoft API contract](https://learn.microsoft.com/en-us/windows/console/generateconsolectrlevent)).
A separate CREATE_NO_WINDOW case checks listener registration, real MCP and normal
EOF. It does not claim delivery of a console event without a console.

Unix sends SIGINT/SIGTERM directly to the Bridge PID, before/after initialize.
The worker runs in a dedicated process session; failure recovery targets that
session only. These physical cases use the default session mode; #425's injected
notification regressions additionally cover supervised renewal and held admission.

## Keep stdin ownership explicit

The worker holds stdin open until both stop and cleanup markers are observed.
It records whether Bridge remains alive 500ms later, then closes stdin and
requires exit code 1 preserving the Interrupted result. The headless EOF case
requires exit code 0. Output and observation phases are bounded externally.

An alive observation characterizes retention; absence of retention in one sample
does not prove bounded exit. Closing stdin is an explicit prerequisite of this
fixture's reaping check. No forced-exit or drainage timeout is added. Any forced
kill is failure recovery and cannot make a case succeed. The external fixture Job
is not the product's service-owned native Git Job or descendant/pipe acceptance.

Tokio's blocking stdin reader and legacy admitted IO can retain process lifetime.
Physical open-stdin shutdown ownership, stalled IO, abrupt kill, Windows console
close/logoff/shutdown and a live host/policy effect fence remain separate gates.

## Evidence

Exact-head and independent-main receipts are maintained in PR #426 and issue
#323. Preserve first CI #40862459702 at a0957a9f6cc1884779bcd9082b70ebc945edecd6:
the fixture passed an invalid `--workspace` option and failed before signal
delivery. It is an invocation defect, not a product shutdown reproduction. The
corrected fixture uses the real positional workspace argument. That first Windows
job was cancelled by the correction; it supplies no physical Windows result.

Corrected code candidate fd537e38a5b10f083405fe6af606b32252732f4a passed CI
#40862883957, four jobs SUCCESS, attempt 1. Windows job 123410310998 reported
four physical console cases plus one headless MCP/EOF case; Ubuntu job
123410311001 reported four physical signal cases. All eight signal samples
reached cleanup with stdin open and remained alive at the 500ms observation;
all exited with code 1 after input closure. Headless EOF exited with code 0.
These samples establish the stated signal-to-cleanup path, not a process-exit
bound while input remains open, Windows 10/11 certification, retained active-work
shutdown or the native Git/Job/descendant/pipe contract. The final documentation
HEAD and independent post-merge main still require their own complete CI and
JSON checks; do not substitute this earlier candidate receipt.
