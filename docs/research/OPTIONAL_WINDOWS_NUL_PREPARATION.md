# Optional Windows NUL preparation — proposed contract

Status: design only, 2026-10-19. Operator approved developing this direction;
no local device permission change is authorized or implemented by this document.
The disposable native comparison is evidence of compatibility, not an installer.

## Purpose and authority

Stock Cargo opens null stdin for child processes. The current capability-free
profile cannot open NUL; the exact-profile disposable comparison can run the
fixed offline fixture after a temporary permission grant. This does not establish
general build-script, dependency, private-project or storage safety.

Keep ordinary core, AI tools and project descendants unelevated. Preparation is
optional and selected locally. Refusal, UAC cancellation or unavailable recovery
must leave that execution mode unavailable; never silently retry a failed task
under a different identity or with broader authority.

## Candidate mechanism to prototype

Use a narrowly scoped, separately elevated local broker for the lifetime of an
explicitly selected execution lease. Prototype it on disposable Windows first.
Do not ship a general privileged command runner or a device/path/PID ACL API.
The broker may open only the fixed NUL character device. It must verify the
object type and keep the object handle bound throughout grant and revoke.

The only proposed addition is a noninheriting read/write ACE for one freshly
created, owned AppContainer package SID. Reject arbitrary caller-supplied SIDs,
preexisting foreign profiles and reused/stale leases. No Everyone, Users,
AllPackages, owner, SACL, desktop, installed-runtime or registry permission edits.

The trusted local core owns profile creation and transfers a bounded lease
request. Authentication must bind the actual core instance, operator logon and
profile creation record; a process ID, path string or secret stored in the
project is insufficient. Untrusted project children must not be able to acquire
or extend a privileged lease through the same IPC endpoint. This authentication
and profile provenance are unresolved implementation gates, not existing claims.

## Preview, apply and stop

Before elevation, show the fixed resource, exact profile SID, concrete rights,
expected execution scope, cleanup behavior and any existing recovery record.
Require explicit local administrator approval for that concrete preparation.
The approved development direction does not replace this step.

On apply, verify expected device/identity state, preserve unrelated DACL entries,
and durably record the exact owned change before admitting project execution.
Idempotent replay must never duplicate or broaden the ACE. Do not replace a
whole device DACL with a stale captured snapshot during rollback.

Stop admitting work before shutdown. Terminate the owned job tree and verify
quiescence before removing its exact SID entry; verify restored access denial,
then retire the owned profile and recovery record. Record revoke failure as a
real unresolved condition, not successful cleanup. Do not claim a hard lease
expiry alone revokes an ACL when the responsible process has died.

## Crash and recovery gate

An administrator-owned recovery record must live outside the editable project,
with bounded size and strict identity/resource validation. Startup recovery and
uninstall must remove only acknowledged Optic-owned changes. Cover core death,
broker death, power loss between write/apply/acknowledgment, abandoned profile,
reboot, concurrent leases and unrelated concurrent ACL edits. A startup-only
cleanup cannot promise immediate revocation while all recovery code is stopped.

Do not enable this on a user's PC until interrupted-step recovery and independent
verification pass on disposable hosts. Decide whether a small supervised broker
is sufficient or an installed recovery component is necessary from these tests;
do not add a persistent service merely to hide an unresolved lifetime problem.

## Acceptance before a user test

- Unprepared mode reproduces NUL denial; declined preparation makes no change.
- Prepared offline fixture checks, fails a real assertion, fixes and retests.
- A second unrelated profile remains denied; no broad package grant is added.
- Requests from project descendants, foreign profiles and stale leases refuse.
- Stop/revoke and every interrupted apply/recovery step preserve unrelated ACEs.
- Held or changed device state, failed recovery and missing evidence fail closed.
- The ordinary core contains no elevation path callable by MCP or project code.
- Limits for writable output, dependency restore and secrets remain separate.

Until these gates pass, keep the existing cfg(test) mutator restricted to the
explicit disposable CI comparison. Never spoof its hosted-CI guards locally.
