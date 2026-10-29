# Physical process signal to original stdio cleanup

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
