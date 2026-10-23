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
