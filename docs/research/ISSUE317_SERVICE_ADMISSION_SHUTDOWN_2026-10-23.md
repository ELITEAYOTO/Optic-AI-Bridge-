# #317 service audit: admission, shutdown and future authority

Date: 2026-10-23. Experimental branch codex/317-service-retention.
Audited baseline c047a0562503d637e46038d032d67e0be4b5ed3f.
Decision status: recommendation for review; no production integration.

## Findings and admission decision

The service submit holds SessionAdmissionPermit only until it returns JobId.
Its worker may not yet have called PreparedReadCommand::start. Revocation closes
new admission and sets the watched atomic BEFORE waiting for admissions to drain.
Once submit returned, that wait does not cover this worker. SessionLifecycleManager
then revokes leases and requests process cancellation. Running and
TerminationUncertain records still prevent seal/reap and keep reservations.
This is reservation/submission admission, not full operation admission.

The synchronous AuthorizedGitIntegration instead holds its permit around the
entire integration call. Thus substituting this service silently would change
the meaning of the existing admission fence. No current production caller of
AuthorizedRead exists: this is a demonstrated candidate integration gap, not a
demonstrated bypass through deployed GitRead.

Recommended V1 contract: retain effect admission through the owned operation,
including native terminality, pipe EOF and protected result finalization. Supply
pre-drain stop intent independently, as the real session atomic already does.
Caller cancellation must not drop that admission. Do not use an artificial
caller-held permit as proof that a worker has it. An owned permit would need
an Arc-backed registry guard transferred transactionally before native effects;
acquiring it later in a worker is not equivalent unless the intervening revoke
and admission failure paths are specified and tested.

Consequence: persistent uncertain ownership can also prevent synchronous revoke
from returning. The application needs an explicit pending-revocation operation
or supervisory contract; merely putting the full permit in the worker would
move the indefinite wait from Drop into revoke. Do not implement that alone.

Alternative: explicitly define revoke as close admission + request stop and
make every consumer require a separate quiescent seal before cleanup. This can
fit asynchronous jobs, but requires auditing all authority/worktree/result
consumers and documenting the changed Git contract. It is not approved here.

The replacement native test holds the actual worker before start, calls real
lifecycle revoke without any extra permit, observes retained accounting and
failed seal/reap, then releases the worker: it never creates the fixture marker,
reports Stopped, and permits reap only after proof. Existing observation-failure
coverage also demonstrates revoke returning while an owner remains uncertain.

## Last check versus native resume

PreparedReadCommand checks stop before creation, creates suspended, retains the
Child, assigns the real Job, checks stop again, then resumes. The shared atomic
can change between the final check and ResumeThread. The watchdog can terminate
that assigned Job; it cannot promise zero instructions executed after revoke.
The existing assignment-phase test covers revoke BEFORE that check, not this
remaining interval. No atomicity claim is made by the new delayed-start proof.

A strict no-resume-after-close contract needs one linearization mechanism shared
by registry close and native launch (with a defined winner and stop-before-drain
ordering). Holding the registry mutex across Win32 resume would introduce a
potentially blocking native call into authority closure and needs lock/deadline
analysis. A private unrelated mutex or another atomic check does not prove it.
No such production mechanism is added speculatively.

## Shutdown decision and ownership

try_shutdown signals stop before try_lock, reaps only finished threads, and
returns a responsibility count. Busy/error is not completion. The SAME root
must remain under an application supervisor, and its record capacity stays
charged. It is not sufficient to preserve an enum after dropping its payload.

Drop signals stop and joins all workers. An injected observation failure makes
the worker retry forever, even if the watchdog has already emptied the Job.
The new bounded supervised test demonstrates that the root destructor does not
complete during an observation-failure window, and joins after that window is
released. It does not inject a real permanently failing Windows kernel call.
The source loop establishes the conditional indefinite-wait risk.

Available policies:

| Policy | Benefit | Cost / gate |
| --- | --- | --- |
| Retain root in a live application supervisor; stop accepting work, retry bounded polling | Keeps every handle, worker and reservation with an identified owner | Application exit may remain pending indefinitely; must be visible and accepted |
| A separate, independently supervised owner process | Host can have a bounded request lifetime while owner continues recovery | New IPC/crash/authority design and budgets; not a small patch, not implemented |
| Explicit terminal host exit after failed graceful drain | OS closes host handles, kill-on-close can terminate assigned jobs | No in-process EOF/result/recovery proof, persistence/reconciliation needed; not a clean success |

Recommend the first as the explicit experimental policy; keep production
integration blocked pending an application shutdown decision. No detached
worker, forgotten allocation, premature budget release, timeout around join or
implicit forced host termination is introduced. This audit does not make Drop
bounded. A safe unconditional bounded Drop cannot also promise continued
in-process ownership when native observation never recovers.

OwnedJob uses KILL_ON_JOB_CLOSE without breakaway flags. Its watchdog duplicate
is created non-inheritable. On host crash, last-handle close can kill assigned
members, but does not establish result durability or observed EOF. Important
additional gap: a suspended root created BEFORE Job assignment is not protected
by that Job on abrupt host death. Child handle close alone is not process kill.
The current normal error path retains/kills that exact Child; crash bypasses it.
Do not claim crash-safe ownership from suspend-then-assign alone. An atomic Job
association at creation, or an independently owned launch supervisor, requires
separate compatibility tests before asserting that stronger guarantee.

Native documentation: [Windows Job objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
Jobs manage lifetime/resources; they are not a complete sandbox.

## Future AuthorizedRead authorizer checklist

Keep fields private and production construction absent until the above gates.
One verified authority must normalize a typed GitRead action, authenticate its
session and task lease, evaluate policy on a fresh common monotonic Clock and
derive an absolute effective deadline from all applicable expiries and budget.
Recheck at actual effect, including queue/preparation time; the current captured
now and later relative native timeout alone do not enforce this.

Retain executable identity/hash and opened native pin; canonical repository/cwd
identity must outlive the effect and cover replacement races. Build an absolute
Git invocation from permitted structured operations, fixed arguments and a
small controlled environment (including Git config/hooks/external command and
credential behavior). Never accept a caller's arbitrary Command, raw arguments,
creation flags, inherited handles or arbitrary cwd/environment as authority.
Cross-session result/stop, post-revoke result release and sensitive output must
remain independently checked; #182 provenance policy is unchanged.

Native start overwrites creation_flags with CREATE_SUSPENDED and replaces all
stdio. Rust specifies that creation_flags sets the flags (Unicode environment
is added). Thus an earlier caller-set BREAKAWAY flag is not deliberately merged
by this implementation. This source/API conclusion is not a new native hostile-
flags regression. Other Command configuration is not all reset: accepting a
general Command in a future authorizer would still be unjustified. Reconstruct
one internally rather than trying to inspect opaque settings. The primitive
calls ordinary spawn, not caller-supplied spawn_with_attributes.

Reference: [Rust CommandExt](https://doc.rust-lang.org/std/os/windows/process/trait.CommandExt.html).
Consulted online reference was 1.99.1; repository/native toolchain remains 1.99.0.

Required future regressions: genuine owned admission survives request loss;
revoke before admission / during preparation / assignment / final-check-resume;
expiry while queued; pending revoke and shutdown under persistent uncertainty;
no session reap or capacity release until proof; hostile flags/handle settings;
binary/cwd replacement; controlled environment; host death before/after assign.
Host death experiments require independent ownership before any fixture effect.

## Verdict

Candidate retention and cleanup tests remain useful. Full effect admission,
strict launch/revoke ordering and application/crash shutdown are NOT yet proved.
Service is not ready for the first production GitRead migration. No policy,
permission, deadline or process limit is weakened; no main/PR327 merge.
