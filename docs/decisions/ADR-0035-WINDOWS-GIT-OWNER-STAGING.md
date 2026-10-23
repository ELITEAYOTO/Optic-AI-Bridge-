# ADR-0035 — Windows Git command ownership: staged integration

Status: **PROPOSED; not accepted, not a production implementation**.

Follow-up service audit (2026-10-23):
[admission/shutdown decision and native proof scope](../research/ISSUE317_SERVICE_ADMISSION_SHUTDOWN_2026-10-23.md).
The private service's submit-only permit is not full Git effect admission;
pending ownership and joined Drop do not establish bounded application exit.
The suspend-before-assignment host-crash window also remains unproved.
These gates must be resolved before stage 5; this ADR remains PROPOSED.
Date: 2026-10-22. Owner: runtime/process/session/Git.
Baseline: main c94461446a49ee61dc527a74f4e198afc8e55b4d.

## Decision to be reviewed

Windows V1 is the functional target. Retain existing Ubuntu regression CI; do
not require new Linux confinement research before reviewing the Windows design.
Do not migrate HardenedCommandRunner until Windows native ownership, retained
resource accounting, session revocation and uncertain Git effects are proved.

Use the existing Windows crate as the only unsafe/native boundary. Reuse its
OwnedJob CPU/memory/process limits and CapturedOutputPipe single-reader primitive.
Create/configure the Job first; create a suspended root; retain ownership before
assignment; assign before resume. No breakaway flags or arbitrary PID API.
A Job is containment of process lifetime/resources, not a complete sandbox.

One operation owner must retain child/job/pipe handles, reservations and result
until tree terminality AND pipe-I/O completion are observed. Root exit alone
does not imply descendants or pipes ended. Caller cancellation cannot destroy
the owner. Missing observation retains authority/capacity, not just an error enum.

## Why the existing runner cannot be safely replaced in one local patch

The public Copy runner has only a timeout; its spec has no session/operation ID,
shared process/CPU/memory reservation or revocation signal. Git constructors and
HostStartup recovery also call it before session-bound authority exists.
SessionLifecycleManager::revoke waits for admitted operations before requesting
ProcessManager cancellation. Git owners are absent from that registry. A command
blocked while holding admission therefore cannot rely on that later cancellation.
seal_quiescent/try_reap must also observe retained Git owners.

A Git update-ref CAS can commit before timeout/error is observed. Native tree
termination uncertainty and Git ref outcome uncertainty are independent.
No automatic retry, clean failure or session seal may infer one from the other.

## Stages and acceptance

1. **This experimental slice:** test-only Rust owner in the Windows native crate,
   under independent outer Job supervision. Reuse actual native Job configuration,
   suspended assign/resume and output pipes. Show retained root/descendant ownership,
   output/deadline/cancel handling, zero reader threads, EOF and empty accounting.
   Startup has a separate test readiness guard; no production deadline changes.
2. Complete native stdin ownership/cancellation; explicit wait/read/write/create/
   assignment/resume failure matrix, prohibited breakaway, limits, owner crash
   and handle-leak measurements. Never detach a blocking writer.
3. Build one bounded service-owned operation registry with pre-spawn reservation.
   Define owner shutdown and retained-uncertainty recovery. Distinguish Session
   and HostStartup authority. Prove stop outside ordinary transport slots.
4. Wire admission-close -> owner-stop -> admission-drain -> lease revocation;
   gate seal/reap on owner proof. Test cancellation, expiry, caller Drop/panic,
   host shutdown and concurrent launch/revoke.
5. Migrate one Git READ path only after those gates, with exact source Windows
   and existing Ubuntu CI. CAS mutation migration needs separate outcome journal/
   ref reconciliation and interruption regression. No wholesale research merge.

## Alternatives rejected for production

- Timeout around join / discard JoinHandle: leaves unaccounted threads/handles.
- Kill direct child: descendants can keep pipes/effects alive.
- Return TerminationUncertain with dropped owner: does not preserve responsibility.
- Job alone without accounting/pipe EOF: request is not confirmed termination.
- Reuse session ProcessManager indiscriminately for HostStartup: invented authority.
- Wire all Git commands at once: conflates read-only effects and committed CAS.

## Scope of the native probe

It is compiled only with cfg(test), has no public production API and is never
called by Git or MCP. It uses stdin null; no claim about writer cancellation.
Injected missing proof is deterministic observation suppression, not a natural
TerminateJobObject failure. Test Drop requests cleanup as a backstop only; the
outer independent Job always terminates and confirms its own tree. Production
retained-registry, resource reservations, session gates and shutdown are NOT
implemented by this fixture. Do not merge it as a claimed correction of #317.

## Prepared null-stdin native slice — 22 October 2026

The research helper and its acceptance boundary are documented in
[the prepared-owner contract](../research/ISSUE317_PREPARED_READ_OWNER_2026-10-22.md).
It prepares a real limited Job/pipes/watchdog before child creation, is adopted
before start, and proves native stop during blocked ledger observation. Null
stdin is explicit. Shared production reservations, lifecycle seal/reap and
service shutdown are still missing. This evidence does not accept this ADR or
permit GitRead migration. Do not merge the whole research branch to main.

## Private service-root candidate — 23 October 2026

[The unwired kernel](../research/ISSUE317_READ_SERVICE_KERNEL_2026-10-23.md)
keeps native owners in service workers, uses actual shared ProcessManager
reservations, and lets existing lifecycle seal/reap see uncertain jobs. Stop
signals precede nonblocking bookkeeping observation; pending shutdown retains
the root. A joined Drop may wait indefinitely on persistent uncertainty. The
authorization token has no production constructor. This is not an accepted
shutdown contract or a GitRead migration; the ADR remains PROPOSED.
