# #317 — private read-service root, candidate contract

Date: 2026-10-23. Status: **experimental, unwired, ADR-0035 not accepted**.
Code: runtime process/read_service.rs. Tests: process/read_service_tests.rs.
Base: PR327 ef3208d. Do not merge this branch wholesale into main.

## Decision after the scoped proof

The local scoped prototype proved the common JobStore/lifecycle accounting path.
It is preserved as a local evidence artifact, not a second implementation in the
repository. The retained candidate is one private service kernel reusing
PreparedReadCommand and the actual ProcessManager store/admission algorithms.

It is safe to study this kernel without extending authority: AuthorizedRead has
private fields and NO production constructor; only its test module mints a
synthetic command. No production caller exists. It does not execute GitRead or
authorize ProcessRun/HostStartup. The real Git authorizer, executable/cwd pins,
task-lease effective deadline and lifecycle embedding remain prerequisites.

The ownership contract is explicit:
- Requester receives JobId only; native payload stays in a service worker.
- Root owns every worker JoinHandle; no worker handle or I/O thread is detached.
- Worker owns native Job/Child/pipes/outcome across fallible steps.
- Errors/panics inside the step boundary keep that driver outside the unwind,
  request stop and mark TerminationUncertain. Capacity remains charged.
- Pending shutdown retains the same root, workers and resources.
- Dropping the root requests stop and JOINS workers. If native observation can
  never recover, Drop can block indefinitely. This is an explicit fail-closed
  backstop, NOT a bounded application-shutdown guarantee.

An application must use try_shutdown and keep the root alive when it reports
pending/busy. We do not pretend memory-forget or dropping a handle is quarantine.
A product decision about persistent uncertainty, host shutdown and process
crash remains necessary before wiring this into the Bridge application.

## Shared admission, state and output

Before native handle preparation, insert a Running record in the SAME JobStore
used by ordinary/private process jobs, through prepare_store_for_start_with_workload.
ResourceBudget is nonzero and fits HardLimits. Workload class is carried by the
unminted authorized token, not silently forced to Standard.
CPU hard cap equals reservation; memory/output/process limits go to the native Job.
Count, CPU, memory and output are checked globally and per session. Shared store
mutex serializes ordinary and service admission: no two independent quota pools.
This unifies the candidate with managed/private jobs only. The historical Git
runner is still outside this integration; not all Bridge Git work is accounted
by this candidate before an actual migration.

Per-root retained worker handles are additionally bounded by max_active_process_jobs,
including finished workers not yet joined. Finished handles are joined on submit
or shutdown; unbounded completed thread history cannot accumulate.
The worker-index map reserves capacity before any native effect, avoiding an
allocation during transfer of a running worker's JoinHandle. Generated IDs are
checked against both registries before insertion; no existing owner is replaced.
These are source-level transfer guards, not claims of a reproduced OOM/collision.
This does not claim exact host RSS equals child resource budgets; controller
thread overhead is bounded by that worker count.

Record status remains Running or TerminationUncertain until root exit, empty Job
AND both output EOFs are proven. Once proven, move captured buffers into existing
record output instead of copying another full output reservation. History keeps
the normal output reservation/eviction semantics. Failed/incomplete observation
never becomes clean success; cancellation/deadline/output failures remain distinct.
An ordinary nonzero exit retains the ordinary Exited + nonzero code semantics.

A failed preparation or thread creation before any native start is known-no-spawn,
not an uncertain live child. A partial native start belongs to the same driver
until proof. No callback takes ownership of that driver across unwind.

## Lock and shutdown rules

Submit serializes bookkeeping and uses short SessionRegistry/JobStore admission
sections. It never polls native pipes while holding those registry locks.
Workers enter neither global store nor session registry while observing native
state; they update only their own record/output state.

try_shutdown sets the shared stop flag BEFORE a nonblocking try_lock of worker
bookkeeping. Busy is not completion: owner retained, signal already delivered.
Every native watchdog watches that same root flag independently of the observer.
SessionRegistry's real pre-drain revoke flag is also watched directly.

Drop holds exclusive root access, signals stop and joins workers. Workers do not
need the bookkeeping mutex, so that join does not introduce a mutex cycle.
This is not proof that arbitrary OS calls or corrupted state recover in bounded
time. A poisoned record or persistent native error can retain resources and block
the fallback; never deploy it as a hidden bounded shutdown.

## New native proof

An external Job owns the gated test BEFORE any service action.
The actual kernel, not the earlier scoped mock, demonstrates:
- abandoned synthetic caller reply while worker holds native resources;
- per-session quota rejection, shared ordinary admission rejection and retained
  count/memory/CPU/output;
- cross-session JobId result/stop denial;
- caught synthetic worker observation panic (driver kept outside catch_unwind);
- repeated synthetic observation error retains owner/capacity;
- real lifecycle revoke A, denied new submission, seal/reap blocked while retained;
- independent session B remains Running;
- recovery releases A only after native/pipe proof;
- pending root shutdown retains B and refuses new submissions;
- external accounting sees only the test host during retained uncertainty;
- with a REAL bookkeeping mutex held, try_shutdown reports Busy and external Job
  accounting changes host+child (2) to host (1) BEFORE that mutex is released;
- a separate direct root Drop, without prior error/shutdown, stops and joins a
  real active child;
- real lifecycle revoke stops another child while a caller admission remains
  held; revoke has not returned until that permit is released (pre-drain proof);
- missing executable recovers with Failed/no active reservation.

These are synthetic service tests, NOT actual MCP transport cancellation.
Observation failures are injected before native polling, not real Win32 I/O
failures. The caught panic is at a controlled step boundary, not exhaustive
panic/poison/OOM/crash recovery. Prior PR327 native bad-handle tests remain valid
but do not magically extend this proof.

Eight service-level result cases additionally verify normal/code7 exits,
stdout and combined-output ceilings, retained stdout/stderr descendants,
timeout and null stdin. These test THIS kernel's mapping into ProcessResult,
existing output buffers/EOF and common reservations, rather than merely
repeating the earlier native-owner primitive matrix.

First kernel commit c51c331ecab9060d9172e1989ab0fb9a3f4ab4ed passed normal
CI39881749896 and native39881749958 on attempt1. That source does not include
the later service result matrix/pre-drain tests. Exact final-source validations
must be recorded separately; they do not replace PR327's original red CI.

A separate unit test invokes THIS kernel's submit with an ordinary synthetic
accounting record and refuses CPU/memory/output overcommit before native prepare.
Only that negative test fixes its host-memory observation; no real native
positive test bypasses host-memory headroom. Its executable is intentionally
absent, so even an admission regression cannot launch a child without a
supervisor. No production memory protection is changed.
The Windows native phase test separately covers stop after actual assignment
and before resume. It does not eliminate the last-check/ResumeThread race.

## Initial failures preserved

- Compiler E0596 in initial test wait helper: Fn was changed to FnMut.
- Clippy drop_non_drop on a JobId-only request model: replaced with a real bounded
  reply channel model, without lint suppression.
- First expanded native test expected Revoked but worker-capacity check won first.
  Session validation now precedes capacity lookup. No native launch occurred;
  the failing run's outer Job was recovered. This is a new candidate contract
  ordering issue, not a claim of a pre-existing main vulnerability.
- Pre-start regression reproduced Failed for both requested stop and expired
  deadline, before any child executed. Keep the first red JSON. The fix retains
  the launch error kind and snapshots actual owner cancellation signals BEFORE
  cleanup requests. Interrupted-by-owner starts preserve Stopped/TimedOut;
  real missing-executable errors remain Failed. Native limits are unchanged.
- The test harness created a relative Windows cache under its inherited checkout
  cwd during empty-environment fixture launch. Opaque cache files are quarantined
  locally and excluded from publications. Children now have explicit synthetic
  cwd; TemporaryDirectory lives under the evidence destination. A post-fix run
  found no relative cache recreated in the checkout. No OS configuration change.

## Remaining gates before a real GitRead migration

1. Review the root lifetime and persistent-uncertainty shutdown policy explicitly.
2. Mint AuthorizedRead only from real GitRead/task authority with executable/cwd
   pins and effective monotonic deadline; reject stale authorization at effect.
3. Bound submission wait/preparation at service level. The current private helper
   uses a mutex and Win32 allocation; native timeout alone does not prove an
   end-to-end admission/preparation deadline.
4. Add actual MCP abandon/cancel integration, admission-drain concurrency, lease
   expiry and real service/application teardown.
5. Complete native read/query/wait/termination errors and poison/recovery tests.
   Do not use invalid handles as proof about kernel failure on valid handles.
6. Retain #185's original red and satisfy applicable exact-source Windows/Linux
   gates before any integration. No main merge or release here.
7. Migrate ONE read-only path; general stdin and Git CAS remain separate gates.

No #182 policy change, permission extension, timeout increase or new feature.
