# #317 — authority-owned gate and native owner continuation

Experimental, no production wiring. Base 92574db93aa41de4582d972665275ad7f12f7826;
branch codex/317-authority-native-owner. Main and PR327 are unchanged.

## First increment: stale close versus record replacement

The previous commit checked gate identity before launch, but not before close.
Two deterministic regressions now demonstrate a real defect in that TEST-ONLY
candidate: a direct revoke or prompt close snapshots the old gate, waits on it,
the already-closed/admission-free record is removed and re-registered, and a
launch holds the new record's distinct gate. The stale closer used to publish
revoked through the OLD gate despite the in-progress new guarded effect.

The exact handshake uses Arc ownership counts: old record + test-held gate +
blocked closer snapshot. All channels are finite and threads scoped/joined.
Initial native-Windows unit result: two expected regression failures, each
observed Ok(true) instead of rejecting the retired generation. An initial
compiler borrowing error is separately preserved; it is not the defect proof.

Both close paths now revalidate Arc::ptr_eq under the registry lock before
publication. A stale close returns UnknownSession for the captured, retired
generation and does NOT claim that a replacement session was closed. Normal
provisioning generates a fresh handle; deliberate reuse through public register
is an adversarial test. Production API semantics are unchanged: additional
synchronization and checks remain compiled only in research test builds.

This is not a complete ABA model or a production fix. A future public authority
must define immutable session generations and receipt scope explicitly rather
than silently treating an obsolete close as success on a replacement record.
Poison recovery remains allowed for closure only; new commitments still fail.

First hosted attempt #39909308575 at 6076bf8 failed before creating any jobs.
The added plain YAML run scalar ended its Rust module filter with `:: `,
which YAML interprets as a mapping delimiter. The corrective commit uses a
literal block; no test, lint or required check is removed and the red is kept.

## Second increment: actual instrumented authority and atomic native owner

The runtime unit-test host now links the actual `#[cfg(test)]` registry gate to
fixed native Windows fixtures. Unlike the app integration negative control,
this host DOES contain the gate instrumentation. A separate Windows dev-only
support crate holds unsafe FFI; runtime's forbid(unsafe_code) remains intact.
Cargo tree checks that this fixture is absent from the app's NORMAL dependency
closure. No optional production runner/API/permission is introduced.

The prepared keeper owns the genuine admission, a shared ProcessManager record
and its actual reservations, native Job/Child/pipe handles, captured output and
one independently signalled stop worker. Job association happens inside
CreateProcessW using JOB_LIST, with HANDLE_LIST for exactly stdin NUL and both
writers. Environment/cwd are explicitly synthetic; no ambient credentials.
Limits: two processes,256MiB,25%CPU,16KiB combined output and a4s native deadline;
each matches or fits the existing test host's actual HardLimits. No host-memory
headroom observation is faked.

Fourteen externally supervised modes discriminate close-before-create,
close-before-resume, launch-wins, direct close during a paused native critical
section, failed creation, failed resume, failed stop, failed query, deliberately
unavailable observation, caller Drop, retained descendant pipes, output overflow,
panic under gate, and post-warmup handle recovery. Before releasing the native
pause, an Arc-count witness proves direct revoke has actually captured the gate;
a mere "about to call" channel is not used as proof of lock contention.

Observation suppression and invalid-handle API probes are controlled faults,
NOT natural failures on a valid owned Windows Job. QueryInformationJobObject
with NULL would inspect the caller's current Job; the rejection probe instead
passes a PROCESS pseudo-handle (wrong object type) while retaining the real Job.
All primary errors stay failures after successful recovery. Reference:
[QueryInformationJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-queryinformationjobobject).

The caller's small Client Drop signals stop without a native call or join; an
identified keeper remains. Consuming begin_shutdown returns either Complete or
Pending containing the SAME preallocated keeper Box and an optional error kind.
Repeated pending results preserve its address, admission and reservations.
Complete requires root exit, empty Job, both EOFs and stop-worker jointure; no
active thread is joined inside a pending polling step. Native child allocation
cannot occur after a second create attempt or outside the gate in these modes.

The root is NOT silently dropped on uncertainty. Its fallback native/controller
destructors can still perform synchronous cleanup under the external harness
deadline. They are NOT an accepted bounded production application shutdown.
The test never pretends that a type can make an unrecoverable kernel call finish.
An application supervisor must retain Pending; host-exit/durable recovery remains
a separate contract. The controller retries stop independently until joined,
including a stop requested while Job creation is not yet stabilized.

Local first matrix passed. Cold host handle counts increased1–4 depending on
mode; their cause is NOT established. A targeted second full launch/close cycle
in the same host returned84→84 after a recorded80→84 first cycle. This finite
post-warmup stability is not proof of zero cold overhead or zero leaks forever.
Outer Jobs all ended empty; this is distinct from in-host handle observations.

Initial compiler reds (including the preserved forbid-unsafe rejection), scoped
channel ownership errors and strict Clippy findings remain local evidence.
Native helpers moved into the dev fixture rather than weakening the runtime
boundary. Explicit Windows Job ownership documents the deliberately surviving
descendant; no lint suppression, detached I/O thread or timeout increase.

Still NOT proved: production authority/authorizer, effective lease/clock across
queueing, crash during CreateProcessW itself, Win11 acceptance, valid-handle
kernel failure, unconditional bounded close while a native call holds the gate,
or durable application-root shutdown. The gate held across real native work
orders resume and close, but a stuck call could indefinitely delay A's close.
Other-session authority and actual JobStore admission remain independent.

## Third increment: short commit ticket, native calls outside the gate

The long-gate native test proves ordering but also the liveness problem: actual
direct revoke has captured A's gate yet cannot publish Closed until the native
critical section returns. B remains usable, but that does not make A closure
bounded. A finite pause demonstrates this dependency, NOT a hung kernel call.

The alternative research_commit_ticket is cfg(test) only. Under the SAME
authority-owned gate it validates the actual record/generation, registry origin
and admission's principal/project/policy epoch, reads the injected trusted Clock
fresh and commits immutable owner ID, stop signal and absolute deadline. No
native call, executable hashing, pipe read or waiting for terminality occurs
under that gate. The keeper and real shared reservation already exist.

The non-Clone ticket contains the GENUINE SessionAdmissionPermit by value.
Committed and Denied variants both retain it continuously; denial returns the
original permit to the prepared keeper. There is no temporary zero admission
count, allocation at shutdown, transferable bare success enum or unowned task.
The keeper stores the ticket before calling Win32 and drops it only after full
native/pipe/controller proof. Owner ID is checked before effects; the fixture
also permits only a single native create attempt.

Deadline is min(prepared absolute owner budget, current session expiry, captured
admission expiry). Mint reads Clock AFTER gate wait and never restarts the
budget. Native start reads the same Clock before create and again before resume;
the independent stop controller receives the remaining absolute deadline after
preparation. Four pure-registry tests cover close with retained ticket, denied
commit returning admission, foreign-registry denial and fresh clock/budget after
gate wait. The manual clock is used only for deterministic expiry injection.

Six new native cases compare ticket close-wins, cancellation after commitment
but before create, committed launch-wins, DIRECT revoke during a paused native
phase, deadline reached after suspended creation and client Drop before create. In the decisive case,
actual direct revoke publishes Closed while the launch is still paused AFTER
atomic creation. The independent watcher kills the child and the outer Job
confirms host-only while the keeper/admission/reservations are retained. Native
resume is then denied; proof/drain follow after the phase releases. All nineteen
modes passed locally before adding the client-Drop regression. That regression
first failed: session state alone allowed creation after the requester signalled
stop. The keeper now checks requester/record/deadline stop flags before native
effects as well. The original red and externally confirmed cleanup are retained.

Two additional host-exit cases deliberately retain the keeper, real admission,
reservation and a blocked legacy revoke while observation remains uncertain.
One also injects real invalid-handle termination failures, leaving the child live.
The outer pre-armed Job supervisor terminates ONLY the synthetic host handle.
The containing tree goes from two processes to zero in the stop-error case and
from one to zero in the observation case. These are crash-containment proofs,
NOT graceful quiescence, EOF/controller jointure or durable recovery. They do not
exercise a genuine permanently hung kernel call or Windows system crash.
The final matrix comprises twenty normal owner/proof cases and these two
explicit host-exit containment cases; its source/result are recorded separately.

**Different contract:** the short ticket linearizes ENGAGEMENT before native
creation, not ResumeThread itself. Previously committed work can be in progress
after close. Checks minimize cancelled work, but the final check/resume interval
is NOT atomic; a committed operation could execute instructions after close.
The long-gate comparison supplies stricter resume/close ordering at the cost of
depending on a native return. Neither contract may be silently substituted for
the other in a production API. No new engagement ticket can be minted after close.

This is a SESSION ordering ticket, not a complete permission authorizer. A
production version must own a trusted common Clock and verify task lease,
typed action/policy, resource receipt, executable/cwd pins and protected output.
The fixture injects Clock for tests and binds an existing keeper ID; it does not
authorize a caller-supplied Git binary or establish sensitive Git provenance.

The Ubuntu checkpoint39914043236 on2769fd6 remains RED: a test-only reference-count
witness was unused on Linux. Its cfg is now Windows+test, without suppressing
Clippy. Native Windows39914043475 on the same source passed all fourteen cases;
do not call the combined checkpoint wholly green. The final source enables the
normal full CI plus the independent app-native production-path negative control.
The latter must still reproduce the old race because normal runtime builds do
NOT include any research gate/ticket. Green research does not fix PR327 or #317.
