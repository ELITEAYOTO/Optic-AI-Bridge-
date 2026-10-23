# #317 — native registration with persistent authority stop

PROPOSED research contract, no production migration. Base67514dc includes main
442d9e0 and the five synthetic shared-gate controls. Reuse only the inspected
fixed native dev fixture from c1a9c32, not that branch's runtime architecture.

## Decision before implementation

Use a root-owned prepared slot and genuine admission before creating any child.
Bind its controller to the actual session record's revocation AtomicBool, captured
under the registry mutex while that admission prevents legitimate record removal.
Keep that Arc through late registration: it is a persistent intent, not a one-off
JobStore scan. A closed session's signal must remain obtainable by its already
admitted keeper for cleanup, while a new engagement remains forbidden.

Allocate/configure the empty Job, pipes and controller outside the authority gate.
Reserve actual ProcessManager memory/CPU/output capacity and register the record
before commitment, also outside that gate. The root retains the prepared keeper
on failure. The short gate callback only validates freshness and commits a typed
local ticket for that already-registered keeper. It does not query host memory,
call Win32, wait, read pipes, hash executables or accept an arbitrary client callback.
Fresh deadline check inside commit prevents stale pre-gate time from authorizing
an expired prepared admission. Renewal never extends its captured absolute budget.

After gate release, create atomically in the preconfigured Job, suspended, then
check stop/deadline again before ResumeThread. A previously committed operation
can race with close after that check; engagement is linearized, not every CPU
instruction or ResumeThread. Independent stop remains responsible for this case.

The worker owns the admission until root exit, empty Job, both EOFs, exit-code
observation and controller jointure are proved and terminal output published.
No successful revoke, exhausted deadline or stop request substitutes for proof.
Ordinary ProcessManager cancellation of current jobs is NOT redefined as closing
authority for future jobs: persistent intent here means actual authority revoke.

## Scope of non-bypassability

The new entry point is private, test-only, with fixed executables and root methods
controlling preparation/registration/commit/start. It does not make all existing
production process entry points use this boundary. A production sealed authorizer,
task leases, executable/cwd pins and output publication rights remain required.
Direct registry revoke and lifecycle revoke are tested because both must publish
the same authority stop, without a separate launcher-private mutex or manual scan.

## Shutdown hypothesis

The root has finite capacity and retains uncertain handles, quotas and admission.
Request shutdown only signals. Polling can still block in Win32. A retaining Drop
may wait until proof rather than falsely release uncertain resources; it is NOT a
bounded global shutdown guarantee. Any deliberate destructor-block/host-exit test
must be externally Job-owned first, preserve the uncertain state and report host
containment separately from internal quiescence. A durable external guardian and
its ownership receipts/recovery protocol are a separate design, not implemented
by claiming the laboratory supervisor is a production service.

## First native checkpoint

Four local fixed cases pass: actual lifecycle revoke automatically stops the
committed native owner and drains its genuine permit without a manual cancellation
scan; the no-latch control keeps a live child until a test-only rescue scan; failed
creation and failed resume remain Failed after proved recovery. The external Job
is queried to verify host ownership BEFORE opening the effect gate, and all four
trees end empty. These local results were obtained on an uncommitted worktree;
hosted evidence must identify the subsequent exact SHA independently.

First compiler failure (module path), and strict Clippy's deprecated AtomicU64
fetch_update rejection on the pinned toolchain are retained. Fixed by explicit
child path and try_update, with no lint suppression. A real host-memory snapshot
is captured outside both locks; the same pure headroom formula checks current
JobStore reservations under its lock. No fake provider, lower reserve or native
system-info call in the authority/JobStore critical section is used.


## Second native checkpoint: actual boundary schedules

Fourteen local cases pass: the first four controls plus closure before admission,
late registration after an EMPTY early scan, closure after registration but before
engagement, closure between atomic suspended creation and resume, native failure
before commitment, uncommitted-start denial, client Drop with authority still
open, direct registry revoke, repeated observation error and termination rejection.
A second session completes an actual fixed native process during A preparation
or suspended-phase pauses. No authority or JobStore lock spans these native calls.

The late-registration case obtains a genuine admission, pauses after empty native
preparation, observes real lifecycle close and a zero-result cancellation scan,
then registers already-stopping and fails commitment. Root polls no-spawn proof
before releasing admission. A short committed launch still does not promise that
all instructions precede close; the paused deterministic case only proves denial
at its final check and automatic stop of its already-owned suspended child.

Repeated query rejection retains an uncertain JobRecord, admission and ticket
while the outer supervisor already sees host-only. Capacity rejects preparation
before invoking its closure; shutdown requests do not release this keeper. Four
observation errors recover to Failed only after actual proof. A real invalid-handle
termination rejection keeps the VALID Job and child alive until fault recovery;
no manual cancellation rescue is used outside the negative no-latch control.

Root start errors are now latched internally, rather than relying on the test
caller to remember failure. New scenarios are isolated in a private fixture
module; no production execution path is migrated. Worktree-labelled local JSON
is preserved as such; exact-head CI remains a separate checkpoint.


## Controller-expiry regression

A frozen monotonic producer-clock fixture combined with the REAL independent
controller deadline reproduced a missing stop guard: after the controller had
spent its budget, Root still reached Native::create. The regression checks the
fixture creation-attempt bit, not merely whether Windows happened to reject or
kill the child. The first local red and external empty-tree proof are retained;
its sources are preserved in7d5c1e2. This is a deliberately separated-clock
scenario, not a natural Windows clock failure or a production incident.

Commit and start now also inspect the controller's irreversible deadline_expired
flag. The first corrected case denies before any Native::create attempt and
finishes TimedOut only after no-spawn/EOF/controller proof. The clock or timeout
was not extended to obtain green. Current normal app builds still omit Root.


## Correction of the first lock-audit claim

The first checkpoints sampled memory outside JobStore, but a transitive call in
prepare_store_for_start_with_workload ALSO invokes ensure_host_memory_headroom
under that lock. Thus the earlier statement that the preliminary sample alone
removed all native memory queries was INCOMPLETE. The green first/second native
runs prove their observed outcomes, not that stronger source-level invariant.

The research manager now has a scoped actual-observation adapter: capture uses
the real WindowsHostMemoryProvider outside locks, then the unchanged quota policy
reads that exact observation from a per-thread scope. No fabricated snapshot,
reserve decrease, callback query under lock or production provider change. The
scope rejects capture while JobStore is busy and expires on exit; provider calls
without a scope fail closed. Root uses try_lock after capture, avoiding waiting
on a queued old sample. The unit test verifies real scoped data, rejection under
lock, and rejection after scope exit. Production sampling/freshness policy still
requires review before any integration; no worst-case OS latency is claimed.
