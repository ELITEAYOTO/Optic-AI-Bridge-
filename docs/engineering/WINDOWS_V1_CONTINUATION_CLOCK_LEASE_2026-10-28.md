# Windows V1 continuation — clock provenance and original lease stop

## Current 30 October continuation — terminal observer race

#434 headf4c8ba429cc49aefdbad48de0958f46f5adbb6bc/CI41004466674 passed4/4,
applicable native preparation41004466593 passed, source/test-merge/squash trees
equal82507febef50a46c3757af1cd4d8fe0e365fa0ac. Main317c4f1ae8c5fcc8580dfdeaed52e3262a4ed314
CI41005173901 FAILED3/4: Windows9 passed including real Node expiry/refusal;
Ubuntu EOF original_closed unit assertion failed, Unix physical cases not reached.
Receipt6515367478 verified. Observer recognized Revoked only; existing reaper
may physically remove both originals. Actual CI states unavailable, but a local
registry-removal regression deterministically fails before the test-only fix.
The correction recognizes only Revoked/UnknownSession AND Revoked/UnknownLease,
rejects live/mixed states, adds state diagnostics; all12 stdio tests/app Clippy
pass. Product lifecycle/drain/renewal, deadlines and physical9/4 criteria unchanged.
[Owning proof](PHYSICAL_HOST_SHUTDOWN_PROOF.md). Exact-head/review/squash/main/#323 still required.
Startup failure41003133144 remains unproved at syscall level; held admissions,
global launch fencing, native Git/resource/security/platform gates remain open.
V1 NO-GO; no release or protection relaxation.

## Current 30 October continuation — prelaunch startup failure captured

#433 headbfcf8f902d5c8f3752bb4af6e1160ac1bef1ac4f/CI41002570074 passed4/4,
Windows9/Unix4 including actual original-session expiry and denied new start.
Squashfc7f2b71352e7cee4938ca75907d769988426dd7 has identical tree, but its
independent main41003133144 FAILED3/4. New bounded failure evidence shows listener
registration timeout10s after read-authorities-provisioned (1ms), before process
lease completion/server/listener and before Node launch. Expiry case not reached.
Receipt6515093418 verified. Do not reuse the PR green as main acceptance.
Next diagnostic increment records opt-in fixed path/pin/hash capture stages and
process-lease entry; it does not skip any authority check, hash byte, pin or
registry validation, increase a timeout, retry, or claim to fix the stall.
Five targeted identity tests and Clippy pass. [Evidence](PHYSICAL_HOST_SHUTDOWN_PROOF.md).
Require exact-head/applicable checks/review/squash/independent main and #323 receipt.
Held admissions, uncertain ownership, global launch fencing, native Git, security
and installed Windows10/11 remain open; Windows V1 NO-GO, no release.

## 30 October continuation — original expiry with actual managed Node

Latest verified baseline main01148008ce5a3b59c61961f6de0e6b98b691912c,
CI40943437676 SUCCESS4/4; #432 diagnostic integration receipt6506843047.
The next test-only increment uses existing operator session lifetime10s without
renewal. Native identity/actual AppContainer token/original external Job checks
precede expiry; a retained handle must signal between10..20s from Bridge birth,
with readiness before5s. Require original session reaping and exact denied new
process_start while Bridge and input stay alive. Client EOF is sent only after
these observations; the old5s root exit/both-join gate remains. [Schema7](PHYSICAL_HOST_SHUTDOWN_PROOF.md).
Sixteen local observer tests are supplemental. Require exact-head CI, review,
authorized squash, independent main CI and #323 receipt. No physical local run.
The earlier40940330506 startup cause remains UNKNOWN; no retry-to-green or fix
is claimed. This is managed-work expiry, not held admissions, atomic launch
fencing, quota/profile/target-stream cleanup, native Git or V1 GO. No release.

## Current continuation — first-failure diagnostics after #431

Main `9af7232937525a5ba7c597027edeef267c4a80dc`, tree
`af05c48f06c2bf8ef024b5563a1f5c36b6f75543`: #431 PR CI40939351677 was
SUCCESS4/4 with Windows8/Unix4 and all three same-head native workflows green.
Independent main CI40940330506 FAILED3/4 at active-job readiness before original
Job verification. Its supervisor path discarded worker traceback/stderr. The
cause is UNKNOWN; receipt #323/comment6506475217 preserves this accurately.
The next small follow-up records bounded first-failure diagnostics before
recovery/temporary deletion, retaining the first cause across cleanup errors.
No process, authority, deadline, success criterion or no-retry rule changes.
Require its exact-head and independent-main gates. Do not claim that a later pass
fixes the earlier startup failure. [Contract](PHYSICAL_HOST_SHUTDOWN_PROOF.md).
Held admissions and native Git/host-policy/resource/security/platform acceptance
remain open; Windows V1 NO-GO, no release.

## Current active-job physical-stop increment — #431

#430 is squash-integrated at `440c17b6a0eedfaac5a507647eebb232491908c0`,
independent main CI #40936393957 SUCCESS4/4; final receipt #323/comment6505919815.
The next narrow increment keeps the private launcher outside Bridge's console
and requires physical Ctrl+Break with the actual AppContainer Node alive and
stdin still open. First test-only head `050e3a6f3165de647188129601ada6a7232f1d55`,
CI #40937939931, passed seven Windows cases then refused three console clients
BEFORE sending the signal. This is a console-boundary precondition failure,
not an observed shutdown failure after delivery. Do not weaken its inventory.
Typed CREATE_NO_WINDOW opt-in retains CREATE_SUSPENDED and existing Job limits,
assignment-before-resume and cleanup. [Schema 6](PHYSICAL_HOST_SHUTDOWN_PROOF.md)
owns the bounded physical contract. Require final exact-head/native workflow and
independent-main evidence plus #323 receipt, one PR at a time. Held admissions,
resource-release ordering, profile cleanup, native Git and host/policy fencing
remain separate. Windows V1 NO-GO, no release or security relaxation.

## Next real active managed-job EOF proof

#429 main `1387a3e9f4b2981c89572bf32ed9f815c6d6dacd` has independent CI
#40925991711 SUCCESS4/4. The next test-only increment adds a hosted actual isolated
Node job alive before client EOF, with operator-image verification, a retained
native process handle and membership in the original external Job. One unchanged
five-second window must cover Bridge exit and Node becoming kernel-signaled,
without a stop RPC, natural budget expiry or fixture recovery producing success.
Read [schema 5 and exact limits](PHYSICAL_HOST_SHUTDOWN_PROOF.md) and final #323/PR
receipts; offline parser tests are not physical evidence. Active managed-job
completion is separate from held counted admissions, uncertain release ordering,
profile/ACL cleanup, native Git streams/Job and global host/policy launch fencing.
Windows V1 remains NO-GO, no production authorization or release change.

## Current stdout owner candidate — #429

Baseline #428 main `c3265b79824142c3ff8dae32950fbb5afeb3c8f1` has independent
CI #40887676383 SUCCESS 4/4. #429 adds an original process-owned bounded Windows
stdout worker, exact-thread cancellation and actual join after original cleanup.
Pipe and redirected disk stdout share this path; character handles fail before
grants. Unix remains unchanged. [ADR-0038](../decisions/ADR-0038-WINDOWS-STDOUT-OWNERSHIP.md)
and [schema 4 proof](PHYSICAL_HOST_SHUTDOWN_PROOF.md) own the behavior and limits.

First test-first CI #40919730293 failed in the physical observer on a partial
stderr prefix after three signal cases, before reaching unread stdout. Preserve
it as a harness failure. A complete-record parser and offline tests repair the
observer without dropping any phase, exit, queue or containment assertion.
Require final exact-head and independent-main receipts in #429/#323; local six
native IO tests are supplemental. This is not active-work/uncertain-driver or
native Git/host-policy launch acceptance. V1 remains NO-GO, no release.

Actual target RED: observer-corrected `e9aa3dc7d21db4a617bb0cd5d8f166b8c86c192b`,
CI #40921106974, passed the five earlier Windows cases then timed out after 5s
with unread stdout despite original cleanup/input join. The source correction
has six native IO, 53 application unit and four actual stdio tests passing locally;
three offline observer regressions and app/Windows all-target Clippy also pass.
These subsets do not replace the exact final-head hosted gates.

Owner candidate `029726bc` passed CI #40922838253 4/4 with six successful Windows
physical cases, including unread-output exit. Final review adds shared native
synchronous-mode validation before input/output spawning; FILE_TYPE alone is
insufficient for Rust File IO. A real overlapped-handle rejection first failed
locally, then the guard passed all 11 input/output tests. Require the final guarded
HEAD's own hosted and independent-main evidence; the earlier green is historical.

## Next output-lifetime evidence — #428

The schema-3 physical supervisor adds a Windows **characterization-only** case
for queued unread stdout. First candidate `9e0750b4cdb742bcb8a834180f594471bd9c5951`
passed [CI 40877904438](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40877904438)
attempt 1, 4/4 jobs: six Windows and four Unix cases.
Original cleanup and the owned input-reader join complete before the root is
observed alive after 500 ms. Closing only stdout releases exit 1 with input still
open. This isolates an output-lifetime limitation after the #427 input join.

The fixture uses a fixed batch of metadata `session_info` calls; it does not
exercise an admitted file/process worker, uncertain-stop handling, or native Git.
No sampled kernel-thread stack or strict pipe-full threshold is claimed.
Implementing stdout ownership/cancellation/join is a later increment; do not
reinterpret this passing characterization as an accepted stdout gate or GO.
See [the current physical proof contract](PHYSICAL_HOST_SHUTDOWN_PROOF.md).
No production code, deadlines, authority, Windows protection, or release changes.

## 29 October 2026 — Windows pipe input owner and real join

#427 makes one process-owned Windows synchronous-pipe stdin claim before session
provisioning. A dedicated owned reader and single-slot bounded channel replace
Tokio's uncancellable blocking-pool read. The owner survives original session/
TaskLease/admission drainage, then requests targeted cancellation and awaits real
join, including the stop-check/read-entry race. Unknown cancellation retains
ownership without detachment or timeout. Console/disk stdin is rejected; Unix
remains the old Tokio path. [ADR](../decisions/ADR-0037-WINDOWS-PIPE-STDIN-OWNERSHIP.md), [proof schema v2](PHYSICAL_HOST_SHUTDOWN_PROOF.md).

Behavioral RED #40869915928 reached cleanup then timed out waiting 5s with stdin
open. Corrected-code #40871949465 passed regular CI and the three applicable
native workflows; physical Windows cases exited with stdin still open and reader
join observed. Final exact-head and independent-main receipts must still be read
in #427/#323. Active work, stalled stdout/legacy IO, uncertain drivers, abrupt
interruption and live host/policy native launch fencing remain open. No authority
deadline/quota relaxation, forced exit, native GitRead activation or release; V1 NO-GO.

## 29 October 2026 — real OS signals reach cleanup; stdin still retains process

PR #426 adds a hosted external supervisor for real Bridge signals before/after
MCP initialize. Windows uses a hidden private console with exact worker/Bridge
inventory before broadcast; Unix targets the Bridge PID. A separate Windows
headless case covers registration and normal MCP/EOF. Corrected-code CI
#40862883957 passed 4/4; all eight signal cases reached original cleanup while
input remained open, and all retained their process at 500ms until the sender
closed input. The headless case exited normally. [Owning method and preserved first fixture error](PHYSICAL_HOST_SHUTDOWN_PROOF.md).

This proves the stated hosted signal-to-cleanup route, not bounded process exit,
active-work drainage, Windows 10/11 installation acceptance or physical native
Git/root/Job/EOF. Next address stdin/process lifetime ownership while retaining
admissions and original deadlines, then the live host/policy native-launch fence.
Exact final-head and independent-main JSON receipts remain mandatory in #426/#323.
No forced exit/drain timeout or GitRead activation; Windows V1 NO-GO, no release.

## 29 October 2026 — process shutdown requests close original stdio authority

PR #425 connects private process-owned OS listeners to the existing original-session cleanup across MCP initialization and serving: Ctrl+C/Ctrl+Break on Windows, SIGINT/SIGTERM on Unix. A ready stop takes precedence over a simultaneously ready service result; listener registration/stream errors fail closed. Successful cleanup preserves Interrupted or the source error, publishes original session/TaskLease stop and retains counted admissions until they drain. No MCP request supplies or resets this source.

Test-first ce7e6c37efb26a6a66125f0a823bdf82373d5ee1, CI #40857686017, reproduced four behavioral failures on Windows and Ubuntu after format/Clippy: idle initialize ignored stop, initialized service did not publish stop, listener failure was ignored, and a ready service result won over ready stop. RMCP duplex tests inject the internal notification; they cover renewal off/on, retained admission and unrelated-session survival, not physical OS signal delivery. Exact final-head and independent push/main receipts belong to #425/#323.

Handling is cooperative. Tokio's blocking stdin reader can keep the runtime/process alive with an open idle input pipe even after authority revocation and RMCP cancellation; this PR adds no forced exit or drain timeout and proves no process-exit bound. Tokio listeners are registered when the process-owned future is first polled before initialization; handlers affect process signal behavior for its lifetime. Blocking startup before that point, abrupt kill, Windows console close/logoff/shutdown, unbounded admission drainage/legacy joins and physical native child/Job/EOF acceptance remain outside this proof. This is a returned service-close path, not a global host/policy revocation fence or native launch permission. The signal feature uses existing locked Tokio dependencies; no security or memory guard changes. Native GitRead stays inactive, Windows V1 NO-GO, no release.

## 29 October 2026 — original session horizon during stdio initialization

PR #424 bounds MCP initialization by the remaining original session lifetime, including time already spent in startup. Pre-initialize traffic does not renew that horizon; local session renewal still begins only after initialization. Already-expired or late-ready initialization fails closed and follows the existing original-session cleanup, which publishes session/lease stop before waiting for counted admissions. This does not bound admission drainage or fix the legacy process/pipe joins.

Real RMCP duplex regressions reproduced behavioral RED on Windows and Ubuntu at 14a9d1c3722fcc7312f54acf08181cbf8c10baa1, CI #40848166754: idle initialization outlived the horizon and expiry did not publish stop. They cover default/supervised idle initialization and retained admission during cleanup. The correction keeps the original deadline, error path, other-session independence and resource ownership. Exact-head and independent-main validation receipts are recorded in #424 and #323; this description alone is not CI acceptance.

No native GitRead launch or global host/policy revocation fence is added. Early setup errors, abrupt task/process interruption and physical child/Job/EOF proof remain separate requirements. Windows V1 stays NO-GO; no release. Local work remains lightweight; full validation uses hosted Windows/Ubuntu.

28 October 2026. Repository: **ELITEAYOTO/Optic-AI-Bridge- only**. Windows V1 **NO-GO**.

29 October service-close increment: baseline main is now
`efb3fc28a47fba136ea912381fc9d1cc5fe1d08c`, tree
`69e7136857e4e320392453ab93d9111cfa23087c`; #421 independent
[push/main CI #40754598941](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40754598941)
is SUCCESS 4/4. The owner explicitly authorized local isolated work under
`E:/Optic-IA-Bridge` for this task on 29 October; this does not change the
standing restrictions for unrelated repositories, security settings or releases.
The new PR must still pass exact-head hosted CI and independent post-merge CI.

## Read current evidence before changing anything

Last independently closed checkpoint when this source was written:
`main=eea5438873293243cd747b0f646ed903fb9f1282`, tree
`81ef82229f52bebf62eafaae05c1a2b0756d91f5`, [push/main CI #40752232971](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40752232971)
SUCCESS 4/4. This continuation's own technical PR advances main after validation:
**read live main and the latest [#323 receipts](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) for its actual final SHA, tree and independent CI.**
Do not treat a pending workflow or this source's historical checkpoint as completion.

Read AGENTS, STATUS, ROADMAP, CHANGELOG, GITHUB_HANDOFF,
ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28 and NATIVE_GIT_READ_WITNESS_CONTRACT.
Old prompts are historical. One PR at a time; no new PR before the previous
squash and independent push/main CI are fully verified.

## Established slices, with limits

- #416: original transport clock identity, no retrofitting after admission.
- #417: original clock-issued session expiry and counted clocked admission;
  raw renewal strips future provenance, old permits retain original deadline.
- #418: original TaskLease generation/clock witness; foreign or absent origin
  rejected by the inert service-clock preparation. Legacy MCP remains unchanged.
- #419: actual executable identity IO moved outside TaskLease revoke mutex for
  lookup and renewal. Original pins retained; final generation/set/expiry
  rechecks; renewal publication all-or-nothing.
- #420: actual debug renewal panic reproduced (200 vs 100) and fixed without
  removing current validation or refreshing the original permit deadline.
- Current pre-drain lease-stop slice: behavioral RED was obtained on Windows
  and Ubuntu at cad47990342c8aa84a9ce6bcdb62943f7e90bdfa (#40753355684 and
  #40753355716): original lease stop waited on admission drainage. The corrected
  source publishes only flags early, retains records/pins/capacity and repeats
  the lease sweep after drainage. Require final corrected-head and independent
  main receipts in #323 before treating this slice as integrated.

All native status preparation/custody is still **non-executing**. No native
GitRead root has been launched by this owner. Existing real empty Job/watchdog
tests are not evidence of Git/descendant/root/stdio physical termination.
Production GitRead still uses HardenedCommandRunner and its reader joins remain
a known unbounded-liveness risk.

## Next concrete P0 boundary

Analyze actual service-owned host shutdown and policy authority before any
CreateProcessW/ResumeThread integration. Avoid a new caller-supplied boolean,
clock, identity or fresh lookup masquerading as original authority.

Observed on the checked production source:
- app main constructs `Arc<dyn Clock>` from StdClock and issues legacy numeric
  session/lease expiries; MCP workers still use the legacy admission interfaces.
  The internal clock-coherent native constructors are not wired into that path.
- This increment removes the `args.supervise_session` condition from original
  session cleanup. Both successful service completion and returned MCP initialize
  errors go through the original lifecycle/session, then propagate the service
  result if cleanup succeeds. The supervisor still starts only after initialize.
  Real RMCP duplex tests cover default/supervised EOF, initialize EOF and a held
  original admission: stop is published, the owner waits, session capacity stays
  occupied until drain and explicit reaping; another session stays active.
  No native child exists in these registry tests. Actual binary EOF/renewal
  regression tests are supplemental, not physical Git/descendant/pipe proof.
  Future abortion, abrupt OS exit and setup errors before MCP initialization
  remain outside this ordinary returned-result boundary. The admission drain
  remains potentially unbounded if legacy admitted IO does not finish; never
  detach it or discard pins/quotas to manufacture a successful shutdown.
- policy_epoch is carried from startup configuration/session/lease/envelope;
  equality of captured integers is not a live policy-revocation issuer.
- session and lease mutexes remain distinct. Positive sequential observations
  are not an atomic launch permission. Define where revoke or launch wins,
  retain ownership across stalled Win32 calls and never hold sensitive locks
  during IO, process creation/resume/waits/termination.

Prefer the smallest real service integration with non-substitutable original
authority, then prove its races. Do not add redundant empty-Job fixtures or
activate native GitRead merely because these inert prerequisites pass.

## Preserved failures and delivery gates

#418 initial HEAD 4e57f565 had natural Node/AppContainer RED in CI #40746209363:
10s timeout, empty Node output, no observed network attempt. The three built-in
later comparisons passed but did not explain or repair the failure. Original
cold Windows tests now emit fixed launcher phases; targeted native tests run
independently before specialized Node proofs. #185 remains OPEN. No retry-to-
green or timeout/ACL/network/sandbox weakening.

#319 natural MCP startup intermittence remains distinct. #176/#182 sensitive
read/provenance, Windows 10/11 actual platform and installed MCP acceptance,
physical termination/EOF, crash/uncertainty custody and explicit delivery
acceptance still block GO. No release/tag/installer publication.

## Operator scope and finish procedure

No project code/build/test/server/installer on the operator PC, no self-hosted
runner, Desktop or REL changes. Hosted Windows + Ubuntu only. Squash merges
were explicitly authorized by the owner; repository security-policy changes
and delivery still require specific authorization. Self-review is not an
independent approval.

At the end, verify final PR HEAD, every applicable job/log, source/test-merge/
squash tree identity, live main and independent push/main 4/4. Update #323 with
actual evidence, limitations and failures. Preserve any unfinished draft;
do not claim a simulated or non-executing observation is physical proof.

Owner requested usage monitoring and a clean finish around 8–10% remaining.
Read live account limits; a past percentage is not current. Keep enough margin
to finish CI, receipts and a self-contained handoff. No automation was created.
