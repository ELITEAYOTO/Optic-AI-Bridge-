# #317 — Prepared native read-command owner (research)

Status: experimental Windows helper, no production Git call site; ADR0035 PROPOSED.
Base: research squash d016a6d (PR326). Production main remains 3fe3de0.

## New seam

PreparedReadCommand reuses OwnedJob, NativeJobDeadline, CapturedOutputPipe and
resume_process_threads. prepare creates and limits the actual Job, output pipes
and watchdog before any child exists. The service ledger adopts this payload
before start. with_owner permits a nonterminal service visit under per-owner
single-observer arbitration, outside the ledger mutex. Errors never release it.

start overrides stdio and creation flags, forces null stdin, creates suspended,
retains Child before assignment, assigns to the limited Job, checks stop again,
then resumes. Every fallible partial step leaves the SAME owner available for
stop/recovery. Command's stored writer duplicates and the parent's original
writers are closed after creation, including errors. No stdout/stderr reader
threads, stdin writer threads or temporary output files are created.

The existing finite watchdog uses a non-inheritable duplicate of this exact Job.
It checks at most four owner-bound flags without the payload mutex. A request is
not proof. poll_terminal requires root exit, Job ActiveProcesses=0 and both pipe
EOFs, with one combined output ceiling. Root exit triggers descendant cleanup.
Native and pipe errors retain ownership. A missing executable produces status
None after no-spawn proof, not successful execution. Starts/results are one-shot.
Terminal outcome metadata is latched before delayed result retrieval. is_success requires an actual zero root status and rejects cancellation, deadline and output-limit flags; a root code alone is not command success.

Drop requests containment cleanup and joins the watchdog; it is a BACKSTOP,
not a terminal certificate. Service shutdown must retain uncertain records.
This helper alone does not meet that service-lifetime requirement.

## Evidence design

- Real SessionRegistry admission, pre-spawn ledger/native adoption, real revoke
  before drain. While the observer holds the payload lock, the external Job
  supervisor first confirms two processes (host + real executed child), then
  confirms only the host remains within 2s of revoke. Only then does it release
  the observer, which independently checks the inner Job and both EOFs. Local
  first observation: 24ms waiting after detection of the revoke marker; a later sample was already empty at that observation. This does not measure the exact flag-to-exit latency and is not an upper-bound guarantee.
- Ten native command cases: normal, nonzero, stdout ceiling, combined stdout/
  stderr ceiling, stdout/stderr retained by descendant, timeout, null stdin,
  missing executable and pre-create stop. Normal deliberately delays result
  retrieval 1100ms AFTER proof to verify stable deadline metadata; its reported
  wall duration includes that artificial delay, not command execution latency.
- Three native error injections: invalid process handle for assignment;
  invalid handle for resume lookup; invalid Job handle for termination. These
  are controlled Win32 API errors, not spontaneous valid-Job failures. Actual
  suspended/assigned roots and pipe resources survive the error, then recover
  under their original owner. Failed termination leaves one live Job process
  and cannot yield an outcome. Every final tree/EOF proof precedes release.
- Exact GetProcessHandleCount before/after each error case, after warming one
  actual successful spawn, must match. Cold first spawn retained two additional
  host handles; dropping Command did not remove them. Warming actual spawn
  produced a stable baseline and subsequent counts matched (83 ->83 locally).
  This does NOT identify those initial handles or prove zero cold-start overhead.
  First red + diagnostic comparisons are retained in local evidence.

Hosted workflow: issue317-registry-native.yml, plus normal Windows/Ubuntu CI and
existing native process gates. Native tests are ignored in ordinary unit runs
because they require independent outer ownership; the workflow selects them
explicitly. It preserves bounded sanitized failure assertions and partial JSON.

## Remaining gates before production migration

- Shared actual ProcessManager/global AND per-session reservations, including
  parent output allocations/transient chunks (up to 16KiB per poll), watchdogs,
  outstanding/uncertain counts and history. Current ledger quotas are research.
- Real executable identity/pin, canonical cwd/env/scopes and GitRead authority
  carried through creation. A native Command is not authorization or sandbox.
- Trusted service worker lifetime, shutdown/crash recovery and bounded repeated
  cleanup. Generic registry Drop is not a quarantine/recovery authority.
- Lifecycle stop-before-drain, seal_quiescent/try_reap must count native owners.
  Current tests prove sequencing, not product wiring.
- Native wait/query/read failure retention and assignment/resume races need a
  fuller matrix. Injected bad handles do not reproduce failures on valid handles.
- HostStartup authority must remain separately authenticated and bounded.
- Only null stdin supported; do not route stdin-bearing Git/blob/CAS commands.
  GitRefOutcomeUncertain is separate and mutations cannot be blindly retried.

No production GitRead migration, general HardenedCommandRunner fix, accepted
ADR, security relaxation, new default authority or V1 readiness is claimed.
