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
