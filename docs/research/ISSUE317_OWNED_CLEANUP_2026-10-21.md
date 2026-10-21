# Issue 317: owned cleanup feasibility study

Date: 2026-10-21. Status: experimental design and native primitive tests, NOT a production fix.

## Decision before production changes

Main verified at 80fe108b288d2b0b6676d6128d9e9beb922eda3f. PR318 remains the only
open PR at 49fe3efd34c9ffe9c10c1c5945b37da2c6a74028; CI39585600089 attempt1 is
red. No merge is authorized. Previous research head49b64e213e144be4ba5a608616183add8e9d47a1:
normal CI39613682202 succeeded; research39613681993 failed its bounded independent
CI aggregation wait, although both native reproduction jobs succeeded. That failure
is preserved. This branch starts directly from main and imports only the two
previous #317 fixture/support sources, not the #319 diagnostics.

**Gate: production modification is not justified yet.** Native containment can
stop a cooperative synthetic tree, but the current synchronous Copy runner has
no session owner, reservation registry, retained-uncertainty API or shutdown
recovery authority. A new error variant alone would drop the actual resources.
A Unix process group is escapable and does not let an ordinary embedded library
reap arbitrary orphan descendants. A private test subreaper does not license
turning the multithreaded Bridge into a global subreaper. These are compatibility
gates, not reasons to weaken INV-021/022/023/025.

## Existing contract audit

hardened_command.rs, main blob513f291f71b2ab80e2d1de01e287ec0c7575ae67:
- validates absolute executable/cwd and nonzero budgets, clears ambient environment;
- input is bounded before cloning, captured stdout+stderr share the byte ceiling;
- blocking readers keep draining after ceiling, writer uses write_all then flush;
- monotonic timeout starts after spawn/threads, polls only direct child every5ms;
- direct exit, timeout and overflow all reach unbounded joins;
- child.kill errors are ignored; child.wait / try_wait errors return immediately;
- MissingChildPipe after spawning and fallible joins use early return: existing
  JoinHandles may be dropped without joining, child handles without termination.
  This is a source-derived exceptional-path problem, not an injected Rust proof;
- stdout join error/panic can bypass stderr/writer joins; output-error precedence
  can obscure TimedOut / quota errors. Thread::spawn itself may panic on failure.

GitReadService, GitIntegrationService and SessionWorktreeManager store this runner.
Their exhaustive map_runner_error matches must be audited if errors change.
Session worktree uses stdin for object/tree protocols; cannot remove the writer.
ReadonlyMcpServer::run_admitted_read deliberately keeps admission alive inside
blocking work even after request cancellation. SessionRegistry::revoke sets the
revocation signal, denies new admission, then waits on admissions_drained.
SessionLifecycleManager cancels process jobs only after that drain.
Returning from an uncertain Git operation and dropping its admission can let
cleanup proceed while effects remain. An asynchronous owner transfer must carry
the admission/reservations and participate in session seals/reaping.

Existing ProcessManager/LimitedJobObject are useful native components but not a
drop-in solution: JobId/session/resource accounting differs; Windows wait includes
job emptiness, generic Unix group lifetime differs; monitor drain/error paths
need their own review. No guarantee is inherited merely by using the wrapper.

## Experiment and falsifiable assertions

scripts/research/issue317_owner_study.py is a TEST harness in Python, never part
of core and not a proposed mandatory dependency. It uses the same Rust parent/
holder fixtures as previous reproduction, but substitutes an explicitly retained
owner rather than the production runner. Thus its timings are NOT before/after
production performance claims.

A single reserved slot owns process, Job/group, three possible non-daemon I/O
threads and pipe objects. Windows starts CREATE_SUSPENDED, assigns unnamed
kill-on-close Job, checks no fixture entry, then resumes its sole primary thread.
A separate outer watchdog owns the worker before resume too. Linux launches a
fresh session/group and uses a disposable private subreaper; known fixtures never
escape. The independent outer subreaper uses pidfds only for adopted fixture
children during fallback, never scans/signals unrelated machine PIDs.

On direct exit/error, output overflow or unchanged200ms polling deadline: request
whole-tree termination, confirm empty, join every worker, close streams, then
release slot. Confirmation budget2s is a cleanup budget, NOT an increased command
deadline or an assertion of a200ms total-return bound. Each outer worker has12s
deadline and known Rust holder has8s autonomous lifetime.

Matrix: normal, nonzero, nul-stdio/selective inheritance, held stdout/stderr/both/
stdin, timeout+heldoutput,4096-byte overflow. Extra injected spawn/thread-start/
read/wait/terminate/query failures exercise ownership decisions. The missing
executable is an actual native spawn failure; other named errors are deterministic
injections, NOT naturally failing OS calls. Retained stdin yields actual write
failure after termination. A skipped termination keeps the real holder alive;
an injected query failure withholds confirmation even after kill. Both must keep
owner, slot, pipes/join handles and deny new admission until explicit retry
confirms/reclaims. No detached I/O thread is accepted as cleanup.

Limits: test single-owner memory state is not production session identity,
admission integration, concurrent registry, crash recovery, CPU reservation,
kill-failure oracle or adversarial descendant confinement. The jobs in this
primitive experiment do not establish production resource budgets. No AppContainer
or network change/test occurs. No link to #319/#185 is asserted.

## Candidate design for later review

Retain public run/run_with_input facade only if host services share a bounded
owner registry supplied at construction. Reserve before spawn; bind internal
operation token to session/admission where applicable (host startup Git needs a
separate explicit host owner). Keep handles and worker join ownership in a
non-Copy entry. Immutable command/error outcome and cleanup state are separate.

Transitions: Reserved -> SuspendedOwned -> Running -> Stopping ->
QuiescentAndDrained -> Released. Any spawn/setup/wait/kill/query/drain failure
after native creation enters RetainedUncertain, not Released. It stores original
reason, bounded diagnostics, exact native handles, pipes, joins, reservation and
admission responsibility. Retry cleanup is owner-scoped; eviction/session seal
must reject it. Registry capacity is fixed and stops new launches when exhausted.
Host shutdown must join/recover its owner service; no detached global cleaner or
unbounded Drop wait disguised as a deadline. If whole-tree confirmation is
impossible, report incomplete shutdown and retain authority rather than reuse.

Windows: reuse reviewed suspended creation and explicit handle inheritance;
configure approved CPU/memory/process limits BEFORE resume, retain Job handle,
terminate then query accounting AND root wait, cancel pending pipe I/O and join.
Job emptiness alone does not prove every pipe is closed if handles escaped to
another authority. CancelSynchronousIo races, pending-read completion and thread
creation failures require native proof; closing a handle behind a blocked thread
is not accepted without a documented native cancellation contract.

Linux: a process group is only a cooperative lifetime boundary. setsid/setpgid
or descriptor transfer can outlive it; waitpid only reaps owned children. Do not
send signals using stale group IDs after ownership is lost. A dedicated worker/
subreaper or stronger kernel membership boundary needs a distinct reviewed design
and native escape/cleanup tests. No namespace/cgroup/security changes are proposed.

Alternatives rejected: timeout around join/detach; kill direct child only;
return Uncertain with no owner; global leak registry without finite capacity;
moving every Git operation to AppContainer or ProcessManager without preserving
its authority and compatibility contract; calling Job/group a sandbox.

## Validation ledger

No results are preclaimed here. Hosted workflow issue317-owner-study.yml produces
per-OS JSON and assembles an archive ONLY after its dependent jobs complete.
Independent normal CI is sampled once and recorded as completed/success,
completed/failure, pending or unavailable. Pending is never success and does not
reclassify successful primitive experiments as a production correction.
Original failed attempts remain visible; no retry merely for a green badge.

## Reusable handoff

Continue GitHub-only on bootstrap/issue-317-owned-cleanup-study. Recheck main,
this branch exact SHA, PR318, both workflows and artifacts. No production code
was changed; do not cherry-pick this harness as a fix. Read this design and the
previous immutable research report at49b64e213e144be4ba5a608616183add8e9d47a1.

Next:
1. Decide/implement test-only Rust owner registry carrying real Git session
   admission and reservations; inject partial startup and uncertain termination,
   prove no premature seal/reap/capacity release.
2. Prove Windows native cancelable-pipe joins and Linux escape/identity/reaping
   contract under independent supervision. Include real exit/nonzero, every held
   pipe, quota/deadline, BrokenPipe, spawn/wait/kill/query errors and foreign-session
   preservation. Finite cleanup latency and resource recovery are distinct checks.
3. Only when those gates pass, review a minimal production delta on exact hosted
   Windows/Linux SHA; PR318 still occupies the PR slot. No merge authorized.

## Added falsifiable Unix boundary comparison

The Linux-only escape-both fixture starts its known holder in a new session and
retains stdout/stderr. The first cleanup must observe original group empty while
holder and reader threads remain live, enter Uncertain and retain capacity.
Recovery then uses a pidfd for the known adopted child of the PRIVATE test
subreaper, followed by all joins. This deliberately demonstrates why group-empty
is insufficient; it is not a proposed production PID-scanning escape repair.
The independent outer subreaper/watchdog still owns all synthetic descendants.
The final matrix is17 Linux/16 Windows. The original16-case first head remains
separate evidence. No original timeout or output ceiling changes.

## Preserved first native attempt

Source7959bc93adc03c360fc074d539d781ade56de988, research39681209526 attempt1:
Linux passed all16 primitive cases; Windows passed control then failed the
supervisor's instantaneous outer Job active-count assertion after worker wait.
This is a harness confirmation failure, not a new Rust runner failure. The initial
result is preserved and is NOT Windows acceptance. Hypothesis: root wait need not
coincide with outer accounting reaching zero. Replace the one-shot assertion with
the already specified bounded2s cleanup confirmation, record initial count and
confirmation elapsed, and also confirm fallback termination before closing the
guard. Command deadline200ms and acceptance of zero active processes are unchanged.
A still-nonempty job remains a failure. No workflow retry on unchanged SHA.

## Second native attempt: a useful negative ownership result

Source55c93721cc2acc67b0c404be11df8cf97aced4d6, research39681675221 attempt1:
Windows passed all16 cases. The initial outer accounting count was sometimes1
after worker wait; it reached0 within0–16ms of bounded confirmation. This supports
the harness timing diagnosis without any command-deadline change.

Linux passed the ten ordinary cases, then the escaped-holder comparison failed
the INDEPENDENT guard: an adopted fixture remained after the prototype reported
reclamation. The first-stage assertions had required original group empty,
escaped holder alive, pipe workers alive and ownership retained. Recovery killed
the escaped child through its pidfd and joined readers, but did not explicitly
waitpid that child before releasing the owner. EOF/group-empty/joined readers
therefore failed to prove complete reaping in THIS prototype. The outer guard's
finally path reclaimed adopted fixtures; the red result is retained.

Experimental correction only: after signalling the already-owned escaped holder,
poll waitpid(WNOHANG) with the2s confirmation bound before closing its pidfd and
retrying final owner cleanup. Also test refusal by actually invoking a second
start on the retained owner, rather than merely reading a capacity boolean.
No Rust production correction is inferred. This result strengthens the decision
to withhold a generic Job/group-only production patch.

Validation history: normal CI39681209313 succeeded on7959bc93 attempt1. Normal
CI39681675082 is independent of its red research workflow; inspect its final
result. Final generated provenance records the final exact head rather than
relabeling preceding source successes.
