# Physical process signal to original stdio cleanup

## Supplemental component — counted worker with OS-pipe EOF (#438)

This test is not a new physical supervisor case and does not change schema8's
Windows10/Unix4 criteria. It exercises the actual private run_admitted_read
worker/admission/transport/governor with a fixture-owned anonymous OS pipe,
using [Rust's pipe/EOF contract](https://doc.rust-lang.org/std/io/fn.pipe.html).
It is not the public fs_read authorization/path implementation or a child process.
The fixture uses its existing fixed logical clock; no service-clock provenance,
original-horizon expiry or renewal behavior is proved by this test.

One selected test runs two modes: abort the async request waiter, or await the
helper's own1000ms timeout. After the operation's entry handshake, the worker
attempts one-byte synchronous pipe Read; the peer sends no bytes and retains the
sole writer. An entry handshake is not a sampled kernel-thread wait stack.
Lifecycle revoke must publish original session and lease Revoked states but remain
unfinished. Require transport active count1, no read completion and failure of a
20ms probe to acquire the single blocking slot, while another session/lease stays
active. Then the EXTERNAL TEST PEER closes the writer, require Read to return0,
revocation to complete, transport count0/blocking slot usable, original new work
refused and one original lease safely reaped without touching the other owner.
The local writer's RAII drop also releases a read during test unwinding.

EOF is supplied by the fixture, not revocation: no bounded production read-stop,
pooled-thread termination, OS host-signal/stdio route, public file-read policy,
native Git/root/Job/stream cleanup, uncertainty/resource/fence or installed
Windows acceptance is inferred. Physical host evidence remains separate and its
held_admission_proven field remains false. No runtime hooks/behavior changes.

First head3e9c9ab66ce5948385d7103f43eabe0a275e8078,
[CI41058447719](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41058447719)
FAILED3/4 attempt1: Ubuntu explicitly passed this test; Windows aborted earlier
at capability_free_appcontainer_denies_node_loopback_connection. Node target was
resumed166ms, then observed TimedOut10052ms with no target stdout/JS readiness or
network-attempt marker and accepted=None. MCP test/physical cases were not reached
on Windows. No isolation breach or cause attribution is inferred. Three existing
serialized diagnostic comparisons passed, without replacing the original red.
[Failure receipt6523399321](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6523399321)
remains retained. No blind same-head rerun or timeout/probe relaxation was added.

An early focused exact one-test step now precedes the unchanged full workspace
Test gate; it exposes this evidence even if a later integration aborts. Full Test
still runs with its prior parallelism and remains required. This changes test
execution order, not the product or the network test, and does not explain/fix
the startup timeout. Source d092cb846ca76bde4468e74cff5e07ec88cb8935,
[CI41059298644](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41059298644)
SUCCESS4/4 attempt1: exactly1 selected/1 passed focused test on both OS, plus the
same test explicitly passing in the full workspace; schema8 Windows10/Unix4
reports complete. Final-head and independent-main receipts remain in #438/#323.
Local fmt/diff checks only, no build/test/product execution. V1 NO-GO, no release.

## Schema8 — active isolated Node AND unread Bridge stdout (#437)

Adds Windows active-job-stalled-stdout to the nine previous Windows cases; Unix
keeps four characterizations. This changes only the fixture/observer, not runtime
code or operator authority. Initialization and real process_start/read/result RPCs
use one unbuffered parent pipe reader for this case. Original Node image,
actual AppContainer token, nonce/PID, retained query/synchronize-only handle and
membership in the original external Job are attested as in the existing active case.

After that attestation, send exactly32 bounded metadata-only session_info requests
(ids1000..1031, input at most4096 bytes), stop consuming stdout, wait for queued
bytes and settle250ms. Recheck that the same retained Node handle is still
unsignaled and both parent pipe ends remain open. No further response read occurs.
Send physical Ctrl+Break through the unchanged strict private-console inventory.
One unchanged five-second deadline covers both ordinary Bridge exit1 and the
retained Node handle becoming kernel-signaled. Require stop before original
cleanup before stdin join before stdout join. Both parent input and the sole
stdout reader must stay open through exit; close/drain cannot provide success.
The original30s child budget must not be reached, no process_stop RPC is used,
and failure-recovery kills cannot provide a passing result. Original45s worker,
25s ownership, startup/readiness bounds and normal external Job-empty/reaping remain.

First exact source `997bd7333210128d4595fd057ac3a40bdc9afb52`,
[CI41050193728](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41050193728)
SUCCESS4/4 attempt1, complete schema8 Windows10/Unix4,17 observer tests each.
The combined case observed4140 queued bytes, reported pipe capacity4096, Node
still live after the queue observation, and combined exit observation16ms after
stop (integer-rounded sample). These are observations, not a strict fullness or
capacity bound, a sampled kernel write stack, instantaneous exit or universal
latency guarantee. Final candidate and independent-main receipts remain in #437/#323.

The report explicitly retains held_admission_proven=false/native_git_proven=false:
metadata calls and an already-started managed job do not prove physical work
inside a still-counted admission. No uncertain-stop/pre-exit resource/profile/
product-Job/target-stream cleanup, shared host/policy launch fence, startup-stall
fix or installed Windows10/11 acceptance is inferred. Registry missing-state wait
semantics are unchanged; existing compatibility tests require UnknownSession.
Local17/17 observer tests and diff check are supplemental; the first sandboxed
attempt had12 temporary-directory PermissionErrors and is not a product RED.
No local Bridge/native fixture/build launch. V1 NO-GO; no release/tag/GitRead activation.

## Supplemental lifecycle regression — competing cleanup (#436)

This is **not a new physical shutdown case**. The earlier audit conjectured an
UnknownSession failure if another collector fully removes an inactive original
between get_active and revoke. Full source inspection found that publication,
revocation and final removal already tolerate missing records. No error fallback
was added; no failing runtime reproduction or RED is claimed.

The test `session_lifecycle::tests::reap_after_competing_cleanup_preserves_other_session_and_lease`
uses real session/lease registries and the normal lifecycle manager. For each
expired and explicitly revoked original, it observes inactivity, runs another
collector to complete removal, then resumes the first collector's actual cleanup
tail. The tail is privately factored without operation/order changes or a
production test hook. Require the winner to remove the session and one lease,
the resumed collector to return exactly the default report, original records to
be absent, and another live session/lease to remain usable; active cleanup is
still refused. No child process is created.

This is a deterministic single-thread scheduled interleaving, not a natural
scheduler race, parallel stress, a reaper waking during counted-admission drain,
a quota/worktree/native-handle completion proof or a fix for earlier startup
failures40940330506/41003133144. Existing schema7 Windows9/Unix4 physical gates and
all original bounds remain unchanged. V1 NO-GO, no native Git/release activation.
Exact-head and independent-main receipts belong to #436/#323; offline formatting
alone is not behavioral evidence.

First source candidate `7f4437bbbd1486e3486dc40e3264fff6622600e4` passed
[CI41023248971](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41023248971)
4/4, attempt1, with the new test explicitly `ok` in both platform logs and
complete schema7 Windows9/Unix4 reports. Applicable native preparation
[41023249099](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41023249099)
also passed on that exact head. These first-source results do not replace the
final candidate and independent-main checks recorded in #436/#323.

## Test-only original-authority observer after concurrent reaping

#434 headf4c8ba429cc49aefdbad48de0958f46f5adbb6bc passed [PR41004466674](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41004466674)
4/4 and applicable native preparation41004466593. Squash317c4f1ae8c5fcc8580dfdeaed52e3262a4ed314
has the same tree. Independent [main41005173901](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41005173901)
FAILED3/4: Windows job123898328162 passed complete schema7 Windows9, including
native expiry10094ms/live input/exact inactive launch refusal; Ubuntu job123898328117
failed stdio_eof_revokes_original_session_independent_of_renewal_mode with
supervised=false, before Unix physical cases. [Receipt6515367478](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6515367478)
remains preserved. The CI assertion printed only a boolean; actual registry
results/interleaving were not captured and must not be invented.

The test helper original_closed required both originals to return Revoked.
The real lifecycle permits a started spawn_blocking reaper to outlive supervisor
cancellation and physically remove already-revoked state after quiescence.
Removal orders process history, leases, then session. A deterministic local test
creates real registries, checks live=false, revokes both originals, checks closed,
physically reaps them, asserts UnknownSession/UnknownLease, then checks closed.
Before the fix this ONE selected test FAILED at the final observer assertion;
no live process or source behavior changed. The initial unqualified exact filter
matched zero tests and is excluded from reproduction evidence.

The test-only predicate now accepts only Revoked OR UnknownSession for the
known-provisioned original session AND Revoked OR UnknownLease for its original
lease. It does not accept live, expired, wrong-owner or unavailable states.
Two additional real-registry tests reject each mixed live/revoked pair and retain
other-session liveness. Existing counted-admission/drain assertions are unchanged.
EOF failures print both current result classes (without grant/identity details).
All12 stdio tests pass after the fix, app all-target/all-feature Clippy and fmt
pass. This repairs a proven observer bug; exact CI interleaving is still inferred,
not a claim that production EOF cleanup had failed or was changed.

No production runtime, renewal/drain, authority, deadline, physical9/4 case or
no-retry change. Require exact-head and independent-main evidence; preserve both
older startup failures. The prelaunch startup stall remains unproved at syscall
level despite new fixed diagnostics. No native Git, held-admission/global fence,
quota/profile/security/installed-platform acceptance or V1 GO; no release.

## Prelaunch startup failure and unfinished identity stages — 30 October

#433 finalbfcf8f902d5c8f3752bb4af6e1160ac1bef1ac4f passed [PR CI41002570074](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41002570074)
4/4, attempt1, schema7 Windows9/Unix4. New expiry case observed native exit10078ms,
original session reaping, live Bridge/input and exact inactive start refusal.
Squashfc7f2b71352e7cee4938ca75907d769988426dd7 has the same tree, but its
[independent main41003133144](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41003133144)
FAILED3/4, attempt1. Six Windows physical cases passed, then active-job worker
timed out10s waiting for host-shutdown-listening. Bounded worker/stderr evidence
ends at read-authorities-provisioned (1ms), with no process-leases-provisioned,
server-created or listener record. No Node had been launched. The original
expiry case was not reached. [Failure receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6515093418)
is preserved; exact blocked operation and cause remain unproved.

Source narrows this interval to process-lease eligibility/ID/registry/executable
identity capture. The old identity-duration record is emitted only after ALL
capture work returns, so it cannot locate an unfinished call. With existing
OPTIC_STARTUP_TIMING=1, the follow-up emits fixed OPTIC_PROCESS_IDENTITY_PHASE
boundaries: capture-started before canonicalization, path-checked after canonical
path/file checks, pin-opened after retained pin/metadata/final-path acquisition,
and hash-complete after full bounded content hashing. The application also marks
process-leases-started immediately before provisioning. Last observed boundary
can narrow a future failure; absence of capture-started does not prove file IO.
These are coarse phases, not syscall traces, stack samples or causal attribution.

All fields are fixed stage names/elapsed times, with no paths, hashes, content,
lease IDs or authority detail. Existing completion summary remains; timings may
include diagnostic-output overhead and are not strict operation latency bounds.
The opt-in flag is sampled once per capture. Five existing identity/pin/launch/
bounded-observation tests and app/runtime all-target/all-feature Clippy pass.
No canonicalization, sharing/pin lifetime, full hash/512MiB bound, registry rule,
authority, deadline, physical9/Unix4 success rule or no-retry behavior changes.
All native/applicable workflows and independent-main gates still apply. A later
green does not prove the startup issue fixed; preserve both failed main receipts.
V1 NO-GO, native Git inactive, no release or protection relaxation.

## Schema 7 — original session expiry with real active isolated Node

Adds Windows `active-job-expiry`, keeping the eight previous Windows and four
Unix cases. The existing operator CLI supplies `--session-lifetime-seconds 10`
and no supervision/renewal. The Node program,30s job budget,256MiB memory,
4096-byte output/process-count1/no-network grants and actual MCP path remain.
The nonce/PID, operator image, actual AppContainer token, retained query/wait
handle and original external Job membership checks are shared with earlier
active cases. The managed Node must still be running before5s from Bridge birth;
a cold/late start fails this precondition without extending any deadline.

With stdin held open, no process_stop or host signal is sent. The SAME retained
Node handle must signal between10..20s from original Bridge birth, followed by
the complete exact `Optic AI Bridge session expiry cleanup reaped 1 session(s)`
record within that same20s horizon. Birth is sampled before Popen, never reset
at readiness. The original session Clock/expiry begins after process creation;
the lower10s assertion rejects an early unrelated child death. The20s observer
bound allows the existing5s reaper cadence; it does not modify production timing
or any prior5s shutdown gate, and precedes the unchanged30s child budget.

Bridge and stdin must remain alive through reaping. A second identical fixed
Node process_start request must receive ONLY JSON-RPC error code-32600 and exact
message `optic.session_inactive`, without a result. A generic failure, different
policy/parameter error or accidental success cannot satisfy authority denial.
Only then does the worker close input and require normal Bridge exit0, original
cleanup and both IO joins within the original5s EOF bound. Final supervisor
reaping/empty original external Job is still required before recovery closure.
The45s worker and25s readiness-ownership bounds are unchanged. No fixture kill
or retry can supply success; #432 bounded first-failure evidence is retained.

The raw-response observer is split from the existing positive receive helper;
all earlier positive callers still reject error frames. Sixteen offline tests
include strict inactive error matching, complete exact single-session reaping,
unchanged64KiB log guard and unchanged positive/error RPC separation. They do
not launch Bridge or prove actual expiry. Final exact-head/main JSON receipts
belong in the PR/#323; previous failed main40940330506's cause remains UNKNOWN.

This observes an original session expiring with an actual MANAGED process,
followed by denial after reaping. It does not prove a held counted admission,
linearizable expiry-versus-launch race, pre-exit quota/profile/target-stream or
product Job-empty cleanup, uncertain-stop recovery, production native Git,
host/policy commit fencing or installed Windows10/11 acceptance. Flags for held
admission/native Git remain false. No product, authority, protection, deadline
or release change; Windows V1 NO-GO.

## Schema 6 failure-evidence supplement — first cause retained

#431 final head `288b9ed03246034fcb9d009e643095370a4d8f62` passed
[CI40939351677](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40939351677)
4/4 and all three native workflows, with complete Windows8/Unix4 reports.
Squash `9af7232937525a5ba7c597027edeef267c4a80dc` has the same tree, but
independent [main CI40940330506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40940330506)
FAILED3/4: six Windows physical cases passed, then `active-job worker exited
before ownership check`. No active-job EOF or signal acceptance was reached.
The early parent assertion lost the worker traceback/Bridge stderr when the
temporary directory was deleted. The original cause is UNKNOWN, with no evidence
to attribute it to #185, #319, a guard refusal or the launcher console mode.
[Failure receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6506475217).

Schema 6 now permits an optional `failure` object on unsuccessful runs. Case-body
errors capture event, exception class/message (last 1024 characters), worker log
and Bridge stderr tails BEFORE recovery. Each tail reads at most 4096 bytes from
the file end, records bytes kept/truncation, and decodes invalid UTF-8 safely.
Missing/unreadable diagnostic files become explicit evidence rather than masking
the case failure. Cleanup-only failures are recorded too; later cleanup failure
cannot replace an already retained first case cause. The final JSON is still
written/printed after failure; captured strings survive temporary-directory deletion.

Eleven offline tests include oversized/malformed log tails, missing/unreadable
files, survival after temporary deletion and preservation of the first cause.
They never launch Bridge or deliver a signal. The existing 64KiB ordinary log
guard remains strict; oversized diagnostics do not turn a failure into success.
All eight Windows/four Unix cases, original Job containment/reaping, 45s worker,
25s ownership and five-second shutdown limits, strict console inventory and
no-retry rule remain unchanged. Product code is unchanged. A later green run
cannot explain or claim to fix the lost startup failure; retain the original
failed receipt and require new exact-head/independent-main evidence. V1 NO-GO.

## Schema 6 — private-console Ctrl+Break with actual active Node (#431)

Preserves all seven Windows and four Unix schema-5 cases, adding Windows
`active-job-signal`. It uses the same real MCP process_start/read/result path,
operator-selected Node, nonce, native image/token/liveness check, retained
query/synchronize-only process handle and original external Job membership.
The Node is running before physical Ctrl+Break. Stdin stays open through exit;
no stop RPC, fixture kill or natural 30s budget expiry supplies success.
One unchanged five-second deadline covers Bridge exit 1 AND native Node handle
signal. Original host-stop/cleanup and both IO joins must be ordered; the outer
supervisor still requires normal worker reaping and empty original Job before
recovery closure. This observes a managed job, not a held counted admission.

Broadcast requires the console inventory to contain EXACTLY worker and Bridge.
The private helper now opts into LimitedJobObject.without_console(); default
managed launches remain unchanged. The wrapper combines CREATE_NO_WINDOW with
CREATE_SUSPENDED itself, so its pre_spawn does not overwrite a caller's desired
helper mode. Job limits, assignment-before-resume, ownership and authorization
are unchanged. [Windows flag semantics](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags)
exclude console inheritance for this console helper. No new generic flags,
IPC authority, driver, dependency or protection change is introduced.

Test-first head `050e3a6f3165de647188129601ada6a7232f1d55`,
[CI #40937939931](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40937939931),
passed the earlier seven Windows cases, then failed the unchanged exact console
set assertion with three attached PIDs. Ctrl+Break was NOT delivered. This is
observed extra-console-client refusal; the launcher explanation follows source
inspection (the extra PID's image was not queried). Preserve the failure and do
not reinterpret it as active-job cleanup failure or expand the console allowlist.
Local all-target/all-feature app/runtime/Windows Clippy passes after removal of
an obsolete unused import; offline parser tests remain supplemental. Final
exact-head and independent-main results belong in #431/#323.

Schema 6 renames the active-case telemetry to `before_shutdown_ms` and
`shutdown_to_kernel_exit_ms` for both EOF and signal. Rounded elapsed samples
are observations, not universal latency guarantees. #430's final schema-5 head
`d063615bd30b39022fbc0da15fa7d229c2409a1a` and main
`440c17b6a0eedfaac5a507647eebb232491908c0` passed respective CIs
#40935035115 and #40936393957 (4/4, attempt1); prior schema-5 fields below are
historical. No claim covers pre-exit quota release, uncertain-stop handling,
physical target stream EOF, product Job-empty/profile cleanup, native GitRead,
host/policy launch fencing or Windows 10/11 installed acceptance. V1 NO-GO.

## Schema 5 — actual managed isolated Node during transport EOF

Adds one Windows `active-job` case, preserving the six earlier Windows and four
Unix cases. The hosted operator selects the canonical installed Node executable,
grants its existing interpreter/AppContainer eligibility and invokes real MCP
`process_start`. The fixed in-memory program emits a random nonce and its PID,
then stays alive with an interval. It reads no project file and requests no
network, extra process or source-read grant. Existing 256 MiB/4096-byte/30s
budgets, host-memory admission and sandbox rules apply without weakening.

The worker reads that nonce/PID through the actual `process_read` result, retains
a non-inherited query/synchronize-only native process handle, checks its image
against operator Node, queries the actual token's AppContainer bit and verifies
the process is unsignaled. Token access is query-only. The parent independently
checks membership in its original retained external Job before acknowledging an
atomic, bounded ownership record. [Job process lists include nested Jobs](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_process_id_list).
The fixture enables no privilege and requests query/synchronize rights only;
it issues no arbitrary PID termination, console broadcast or new runtime test
hook. The headless worker avoids affecting unrelated consoles.

MCP `process_result` must still say running and the native handle must remain
unsignaled immediately before the fixture closes Bridge stdin. No `process_stop`
RPC is sent. A single five-second deadline covers BOTH normal Bridge exit 0 and
the retained Node handle becoming kernel-signaled. EOF must start before 20s of
the 30s child budget and completion must precede that budget, excluding an
observed natural timeout as the success path. Original cleanup and both IO-join
markers are required. The parent requires worker reaping and its external Job
empty BEFORE failure-recovery closure can terminate any remaining process.

The original 45s worker bound includes the new ownership handshake; it is not
extended. Fixed launcher phases are enabled for first-failure diagnostics.
Missing Node, memory refusal, startup failure, wrong identity/membership or
uncertain wait fails the case; no retry/fallback can make it pass. Local offline
tests exercise record parsing and atomic publication only; physical proof remains
hosted-only and final exact-head/main JSON receipts belong in #323/the PR.

This observes a genuinely running MANAGED job after `process_start` admission
has ended. It does NOT prove a held counted admission, pre-exit quota-release
ordering, product-owned Job-empty or physical target stdout/stderr EOF, helper
profile/ACL artifact cleanup, uncertain-stop recovery, native GitRead, or the
host/policy native launch fence. `held_admission_proven` and `native_git_proven`
are explicitly false. Windows V1 remains NO-GO, no release.

First exact candidate `f8afe2b4271dde6d0f3f2edcf67e5a1fbe4a27cb`,
[CI #40933810147](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40933810147),
completed SUCCESS4/4, attempt1, with seven Windows and four Unix cases. The active
case confirmed the original Job, image, actual AppContainer token, running state,
retained-handle signal and normal empty-Job cleanup before the natural budget.
The final observer strengthens this with a direct total wall-clock assertion and
`eof_to_kernel_exit_ms` measurement; require that exact final-head/main evidence
rather than substituting the earlier report.

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
