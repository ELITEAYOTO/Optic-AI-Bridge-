# #317 — terminal publication and identified Pending root

Research continuation of cdb93ae114e7c293be339cf4b6e8d0ad9a603b58, branch
codex/317-terminal-pending-owner. No production runner, GitRead migration,
permission change or change to main/PR327.

## Reproduced terminal-publication defect

The previous research Owner::poll moved captured output into JobRecord, marked
both EOFs and set terminal status before querying the fallible root exit code.
A fixed invalid-handle GetExitCodeProcess probe, with the actual child handle
still retained, now reproduces this defect after native root exit, empty Job,
both native EOFs and stop-worker jointure. The initial regression fails with
`failed exit-code observation must not publish EOF`; the independent outer Job
confirms its whole synthetic tree recovered. This is an injected native API
failure, not a spontaneous kernel failure on the valid retained process handle.

The regression is preserved in e4efcc5. The fix obtains exit_code BEFORE any
shared output/EOF/status/overflow mutation. Both shared locks are acquired
before either record is modified. Nothing fallible remains between publication
and admission release. Three repeated exit-code failures must leave public
ProcessManager result Running, output bytes empty, both public EOFs false,
exit_code absent and admission/reservations retained. The private captured
buffers are not discarded. Once observation recovers, the primary failure
remains Failed; it is not converted into a successful command result.

This does not claim a transactional snapshot across arbitrary separate public
API calls: readers use the existing output/state locks and may read at different
times. It proves no incomplete terminal publication on this fallible path.
Mutex poisoning/panics during publication remain fail-closed/fallback concerns,
not a new production transaction protocol.

## Bounded root, research only

A cfg(test) PendingRoot owns a preallocated vector of identified keepers, bounded
to an explicitly configured 1..64 entries. Capacity and Closing are checked
BEFORE trusted preparation. A prepared keeper has no native child; launch occurs
only through the root-retained keeper. This is a private research staging
contract, not a public authorizer or a Rust type proof preventing an arbitrary
caller from misusing a generic constructor. Future production types must make
the Prepared-to-Launched transition unforgeable.

The application test host, not Client, owns the root for its whole scope.
request_shutdown borrows the root and only signals stop; it does not poll Win32,
join, detach, discard or return ownership to a requester. observe_pass keeps the
SAME keeper and capacity on false/error, with bounded per-owner error metadata
and saturating aggregate counters retained after completion. Only a proved
completion removes the entry. The actual Owner adapter sets TerminationUncertain
on an observation error while retaining its genuine session permit and shared
ProcessManager reservations.

Three platform-independent tests cover full capacity without invoking the
preparation closure, repeated pending shutdown, rejection after Closing,
repeated observation errors, no premature Drop, preparation failure and recovery.
The native pending-root-retention case uses the real registry, ticket, JobStore,
atomic Job creation, suspended child and ResumeThread. It distinguishes authority
Closed, root stop request, four failed observations, retained Pending, then proof
and Complete. A second session can still acquire genuine admissions. The outer
Job witnesses host-only while observation fails, independently of internal proof.

The full native matrix now has24 cases: the previous22 plus terminal-code-error
and pending-root-retention.22 ordinary/non-launch proofs and2 host-exit cases
must be reported separately. All native cases require the external Job BEFORE
the host opens its synthetic-effect gate. No test executes user projects.

## Shutdown limit and recovery architecture

A bounded entry count does NOT bound a Win32 call. observe_pass can still wait
inside a native call, and destroying a nonempty root would run existing keeper
destructors, including synchronous fallback cleanup/jointure. No bounded global
Drop is claimed. Tests keep the root live until recovery, never forget memory,
detach a thread or release uncertain reservations to make shutdown pass.

For strict host-exit requirements, propose an independently identified guardian
that owns or receives duplicated noninheritable Job/child handles, records an
ownership receipt before launch, and retains capacity until verified cleanup.
Its queue, receipts, retry policy, authentication, crash handling and failure
semantics require a separate design/review. It is NOT implemented here. A
successful stop request, drained admission or closed client is not quiescence.

## Minimal future production sequence, no PR created here

1. Private revoke close/signal/drain refactor, public behavior and reports intact.
   This alone does not fix #317 or make revoke bounded.
2. Reviewed two-phase closure contract with generation-bound receipts and all
   direct authority paths; monotonic deadlines and cancellation-before-drain.
   Choose commitment semantics explicitly: a short ticket is not atomic Resume.
3. Native ownership service/root with typed preparation, continuous permits,
   limits, failure receipts and explicit global shutdown/guardian policy. Cover
   stdin, normal/nonzero results, native wait/registration failure and saturation.
4. Only then migrate ONE read-only Git action through a verified authorizer,
   binary/cwd pins, task lease, policy and output authority. Keep #182 provenance
   separate; do not bulk merge research or replace all HardenedCommandRunner uses.

Each PR waits for the previous review/integration, exact-SHA Windows/Ubuntu
validation, and availability of the repository's single active PR slot. PR327
and its historical #185 red remain untouched. V1 #323 remains NO-GO.

First hosted checkpoint4bb5d63: contract workflow39921606093 failed before
creating jobs. The new plain YAML scalar contained a Rust module filter ending
in colon+space. Its correction uses a literal block on a new commit; the initial
red is retained. Other workflows on4bb5d63 must be allowed to finish rather than
cancelled or presented as validation of the corrected workflow SHA.
