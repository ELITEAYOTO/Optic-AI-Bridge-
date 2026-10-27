# Native GitRead registered-lease witness — proposed contract (#317)

## 27 October 2026 — preserve the *original* TaskLease generation on prelaunch observation (#390)

Precreated native Git status custody may now ask for a **read-only original-generation authority snapshot**, never a spawn permit. It holds the exact admitted TaskLease witness and compares its revocation `Arc` record identity with the new registry-backed observation. Even identical opaque TaskLease IDs from another registry are denied; a new session registry cannot replace the original owner. The current registry observation revalidates executable authority and typed policy under the unchanged service-owned action envelope/targets. Original session/transport permits, process capacity and the empty Job/pipes/target handles are not released by a failed snapshot. The original monotonic admission stop check is evaluated before and after.

Do not infer synchronization with `CreateProcessW` from this snapshot, or allow it to launch/resume. Subsequent policy revocation can still race a later CreateProcess; the native owner must establish that final gate and retain ownership for terminal OS/EOF proof. The synthetic fixture tests never run Git. Preserve compiler-red [#40517792327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40517792327) separately from the still-unmet #352/#355 real runtime acceptance. #317/#323 OPEN, Windows V1 NO-GO.


## Integration proof for original-target pins (#388)

**Confirmed integration after PR #388 (27 October 2026):** `main=720a38833705e661049841cfd39852e770687643`. PR HEAD `0b29e833a53195f199b5d3b53f808091d020bdc5` passed [CI #40513121389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40513121389), first attempt, four jobs individually SUCCESS and all three native-specialized workflows successful. The reviewed HEAD and squash tree are identical (`1d79045f8ac60db9dd2dc2ffe6e23b72af0811f9`). Independent [push/main #40514100157](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40514100157) on that exact integrated SHA completed SUCCESS, attempt 1, all four Windows/Ubuntu/dependencies/release jobs successful.

**Scope remains pre-spawn only:** original Git executable and repository root opened and pinned by verified Windows handles, with the already counted admissions, TaskLease witness, unstarted Job and stdio. No child, final synchronized `CreateProcess` authority, native terminal/EOF proof, MCP GitRead switch, physical Windows 10/11 acceptance or release. Keep #317/#323 OPEN, #352/#355 original acceptance RED, and Windows V1 NO-GO. This documentary commit itself changes the HEAD; revalidate live `main`/`push/main` separately after any merge.


## 27 October 2026 — original-target pins are now owned before native spawn (#388 integrated)

After the exact admitted and registered `PreparedNativeGitStatus` has retained the original session and transport permits and the true unstarted Job/pipes, `precreate_native_custody` also opens two noncloneable Windows target pins: `PinnedExecutableFile` (read sharing, denies writes/deletes) and `pin_native_git_read_repository_root` (true directory READ/LIST desired access, no delete sharing). The expected executable and repository are taken **only** from the original service command; their actual handle-resolved final paths must match the command's canonical paths. A moved repository fails before native Job adoption and releases the known-no-child reservation. Pins remain held with the empty Job, not independently public or usable for spawn.

Why special directory pin: the ordinary `pin_directory_chain` uses zero desired access for unrelated callers. [Windows test-first RED #40510524394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40510524394) observed two missing protections, and [intermediate Windows RED #40511257834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40511257834) proved its zero-access root handle still permitted renaming the root. The new GitRead-specific read handle has a dedicated real Win32 test and avoids changing generic callers.

These pins are **not** an independent re-verification of the TaskLease's original cryptographic executable content version at the final launch boundary, nor a synchronized revocation permit. No process exists, no root exit/Job-empty-after-run/stdio EOF has been proven and MCP `git_status` still uses the legacy runner. Always preserve original #352/#355 red evidence and #317/#323 OPEN; Windows V1 NO-GO.


## 27 October 2026 — genuine pre-created Windows Job and stdio, still no spawn (#387)

With the existing `RegisteredNativeGitStatus` holding original counted
session/transport permits, TaskLease generation watch and process capacity,
`precreate_native_custody` adds a real Win32 Job Object, configured from its
existing limits, two open-parent-writer anonymous output pipes and a `NUL`
stdin handle. The underlying kernel Job starts empty (actual Job accounting),
and tests distinguish idle pipe from observed EOF. Revocation after
registration blocks preparation; revocation after precreation triggers stop
while original capacity remains held. No client controls argv or targets.

**No native spawn exists** in this interface: no opened pinned Git executable,
root, assignment/resume, final synchronized policy and epoch check, watchdog,
positive terminal Job-empty+root+EOF after execution, or restart recovery.
No process was ever created in the tests, so a zero Job count is not evidence
of real termination. A later guardian must retain every original permission
and OS ownership handle until physical terminal proof; do not use RAII
no-child cleanup once an actual process may have been created.

This change does not connect MCP GitRead. Original #352/#355 remain RED;
#317/#323 OPEN, Windows V1 NO-GO. The [PR #387](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/387)
tracks initial compiler RED and exact-head CI.



## 27 October 2026 — registered process capacity, no native launch (#385)

Windows-only `ProcessManager::reserve_prepared_native_status` consumes the
noncloneable, original `PreparedNativeGitStatus` without readmission. It
requires the process-manager canonical root to match the configured GitRead
service root, verifies the original stop horizon, and registers an
owner-scoped private action using the existing process-store reservation.
The original verified TaskLease's **WorkloadClass** is carried into the
accounted record. The associated `RegisteredNativeGitStatus` retains the
transport and session permits and original lease witness, action and stop
predicate. It has no launch method. A synthetic no-child dropped reservation
becomes Failed and releases the permits; this is **not** a native terminal
proof and must never be generalized to a possibly created process.

Important: the existing `LimitedJobObject` type is a *limits
configuration*, not an opened or registered kernel Job. At this stage there
are **no OS Job/root/pipes, no executable pin or identity recheck at
CreateProcessW, no synced revoke-before-spawn authority, no child**. This
helper is not wired into the live MCP `git_status` handler; that path still
uses the legacy `HardenedCommandRunner`. Before any future child is enabled
the registered owner must hold real OS handles and the identity/revocation/
policy/epoch/monotonic deadline checks atomically with launch, and keep all
capacities until positive root/Job-empty/EOF proof even on failure.
Test-first compiler red [#40501489580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501489580)
was not runtime behavior; green [#40501987327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501987327)
and [native preparation #40501987334](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501987334)
refer only to candidate `9a66557d29cfe66da7f06a68ca80e68be0576a02`.
Original #352/#355 remain red; #317/#323 open; Windows V1 NO-GO.


**Status: inert runtime prerequisite, not a production native process launcher. Windows V1 NO-GO.**

The `verify_registered_native_git_read_preflight` helper combines two controls which were previously only independent prerequisites on `main`:

1. `TaskLeaseRegistry::get_active_with_revocation_watch` returns the actual active, executable-identity-verified registry record and an original-generation revocation witness under the same lock.
2. `PolicyEngine::evaluate_native_git_read_preflight` checks the real typed ActionEnvelope/session/epoch, GitRead capability, exact canonical repository and FixedTool executable scope, nonzero resource ceiling and full monotonic watchdog horizon. Relative targets and legacy lease-less envelopes are rejected by this new helper.

The resulting `NativeGitReadLeaseWitness` is read-only and generation-bound, **never an authority to spawn**. Revocation may publish during/after preflight; `is_revoked() == false` does not grant permission to create a process. A future integration must first prove actual MCP session admission and protected Git metadata handling, opened/pinned exact Git executable, pre-created and registered native Job/owner with retained quotas, and perform a final policy/deadline/revocation/OS-identity check at the correct pre-spawn boundary. Failures and `TerminationUncertain` must retain ownership/capacity.

This PR deliberately does **not** change `authorize_git_read`, `GitReadService`, `HardenedCommandRunner`, stdio, session admission, the Windows owner or legacy GitRead permissions. It does not solve original #355 (real post-revocation Git spawn) or #352 (actual elapsed pipe completion). The contract has synthetic, non-executing fixture tests for legacy rejection, unknown registry lease, wrong root/session/epoch/budget, and revocation followed by same-ID reuse. It makes no claim about physical host-loss recovery, real EOF, AppContainer #185 or Windows 10/11 release acceptance #323.

Review before promotion: verify the exact PR HEAD Windows+Ubuntu CI, retain any initial failures; do not bulk-merge #353/#358 and do not close #317/#323. Independently verify push/main CI if merged.

## 25 October 2026 — subsequent admitted-session preflight slice

The *additional* `begin_admitted_native_git_read_preflight` binds the trusted
application-owned `SessionHandle` (never a client-supplied handle) to a **genuine
counted `OwnedSessionAdmissionPermit`**, then uses the previous registry+policy
preflight. It returns a non-cloneable `NativeGitReadAdmittedPreflight` retaining
both original-generation session and TaskLease stop conditions, including their
captured monotonic expirations. Failed preflight releases the session permit;
successful preflight must be retained by a future native Job guardian through
real terminal evidence. An unrelated or forged `ActionEnvelope.session` fails
before admission, and native-only lease-less requests still fail.

Tests deliberately create **no native process**. They exercise wrong/unknown
session, missing lease, exact grant, original expiration, revoke-before-removal,
session capacity held while a witness exists and same-ID TaskLease replacement.
This step does NOT connect the live MCP GitRead handler or guarantee race-free
pre-spawn authority. It does NOT satisfy either original behavioral acceptance
#352 or #355, nor Windows guardian loss, Job-empty, real EOF or V1 installation.
No legacy GitRead permission change, no unsafe additions, no release.

The admitted helper now takes a `NativeGitReadTrustedContext` grouping the server-owned session handle, registries and policy engine, rather than a long unstructured parameter list. The `ActionEnvelope` and the trusted session are still compared independently before any counted admission.

## 26 October 2026 — owned transport permit boundary (#317)

The admitted native GitRead preflight additionally **consumes the real
`RequestPermit`** supplied by a future MCP worker. The non-cloneable owner
retains its transport concurrency reservation alongside the counted session
admission and original-generation TaskLease witness until that owner is
dropped **after real native termination proof**. The preflight rejects an
already-expired transport permit, and `is_stop_required_at(now)` includes its
captured monotonic deadline even when the session and TaskLease last longer.
Failed admission automatically drops the request permit and counted admission;
tests assert that the transport slot is not released while the owner exists.

This is intentionally **inert**: live `ReadonlyMcpServer::git_status`, the
legacy `HardenedCommandRunner`, the process/Job lifecycle and tool permissions
have not changed. A future native worker must actually transfer its permit to
this owner and retain the owner through Job-empty/root and real pipe EOF; this
helper alone does not demonstrate or authorize real process creation.
Original #352/#355 stay RED. Windows V1 NO-GO.

## 26 October 2026 — transferred MCP worker admissions (research #317)

A future MCP native worker can now transfer **its already counted**, non-cloneable
`OwnedSessionAdmissionPermit` and `RequestPermit` via `NativeGitReadOwnedPermits`
to `finish_owned_native_git_read_preflight`, rather than causing a second
admission. The exact trusted `Arc<SessionRegistry>` identity is checked
(`Arc::ptr_eq`) before accepting the transferred session; a different
registry with an identically-valued session handle fails closed, and the
original transport slot and foreign counted permit are RAII-released.
Revoked/expired original admission and transport are also refused. The older
`begin_admitted_native_git_read_preflight` remains compatible by acquiring
the same owned permit then delegating to this new validation function.

This is **not wired to live `ReadonlyMcpServer::run_admitted_read`**, which
still uses a borrowed session admission and the legacy GitRead subprocess.
No child, Windows Job or pipes are created by this transfer primitive.
There is **no actual OS process termination proof**; the original #352 and
#355 acceptance stays RED and Windows V1 stays NO-GO.

## 27 October 2026 — live MCP owned read worker (#380)

The MCP worker now obtains one `OwnedSessionAdmissionPermit` and passes it
with the original `RequestPermit` by value to `run_owned_admitted_read`.
The existing borrowed read interface delegates to that same worker without
readmission. Git reads use the owned interface and retain both reservations
through execution and sensitive-output verification. Legacy GitRead policy and
the hardened runner remain unchanged; no native launcher is activated.

Non-executing MCP tests transfer the actual worker permits beyond worker
completion, prove that synchronous revoke still drains that admission and the
transport slot remains held, then reject a revoked session. A second test feeds
those original permits into native preflight and verifies that legacy GitRead
with no TaskLease cannot gain native authority. Existing Git sensitive-path and
cancel/drain tests remain applicable. A synthetic no-process fixture may drop
its permits; a future native guardian must retain them through real OS proof.

Test-first SHA `5e6c35ca33592373d033af38bc44788f1479e282`,
[CI #40466543183](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466543183):
Windows and Ubuntu fail compilation with three E0599 errors for the absent
owned worker interface. This is **compiler evidence**, not runtime reproduction.
Initial [#40466491881](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466491881)
failed formatting; retained separately. Implementation candidate `ba05382e353f401198dcb979f48701b8d442e05b` passed
[CI #40466899740](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466899740)
4/4 at attempt 1 (Windows/Ubuntu, strict Clippy, format, dependencies and release
guard). Final PR-head and independent post-merge CI are tracked in
[PR #380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/380); this receipt
alone is not evidence for a later SHA.

Next: registered service-owned Windows guardian consuming these same permits
and the canonical TaskLease, resource/Job/pipes pre-registration and synchronized
final launch authorization. No Job-empty, root-exit, EOF or crash-recovery proof
is supplied by this worker change. Original #352/#355 remain RED;
#317/#323 OPEN, Windows V1 NO-GO.

## 27 October 2026 — preserve the native GitRead action horizon (#381)

The admitted native GitRead preflight captures the original absolute monotonic
`ActionEnvelope.resources.timeout_ms` deadline with checked addition. Its stop
predicate now also stops at that boundary when transport/session/lease outlive
the action. Expiry requests stop; it does not drop either counted reservation
or prove native termination. Both fresh and transferred worker admissions use
the same corrected finish path. Legacy GitRead policy/runner is unchanged.

**Behavioral test-first:** `ff2ad7d472663a182e2b241d2afc8345adfc6d70`,
[CI #40470425246](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40470425246),
attempt 1, passed formatting and strict Clippy, then failed the exact 100ms
action-boundary assertion on Ubuntu (289 passed / 1 failed) and Windows
(361 passed / 1 failed / 11 ignored). This is runtime evidence for the inert
contract, not real Git/Job/EOF acceptance. The regression also checks that an
expired action retains transport/session capacity until explicit no-process
fixture cleanup. Final-head and independent-main receipts are maintained in
[PR #381](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/381).

Baseline `main=ab45498c32b2cb9cdfddfdeaa32f0230447d9de0` already integrates
[owned MCP worker #380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/380),
with independent [push/main #40467892414](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40467892414)
SUCCESS 4/4 at attempt 1. No native launch activation. Next remains registered
service-owned Windows Job/root/pipes custody, canonical TaskLease and final
synchronized launch authorization. Original #352/#355 remain RED;
#317/#323 OPEN, Windows V1 NO-GO.

## 27 October 2026 — service-owned status preparation (#382)

`GitReadService::prepare_native_status` consumes the original worker's
`NativeGitReadOwnedPermits`, derives the fixed status command from the service's
configured canonical repository/executable, and verifies the registered TaskLease
against those same targets. There are no caller-selected target strings,
arguments or independent output-limit inputs. The action's output budget must
fit the service ceiling; the actual spec uses that exact budget.

The noncloneable `PreparedNativeGitStatus` retains that command, an immutable
copy of the original ActionEnvelope and the admitted preflight. Its absolute
deadline is the minimum of captured action/transport/session/lease horizons;
preparation never restarts a relative timeout. Native command/admission/envelope
fields stay crate-private for the future guardian. Read-only metadata does not
grant spawn or sensitive-result publication authority.

Four non-executing tests cover fixed command/environment and immutable action
budget/epoch, missing lease and wrong service repository/executable/session/epoch,
zero/excess output, original deadline and retained session/transport capacity.
They use canonical synthetic directories and pinned **inert, non-executable
bytes**, directly constructing a unit-only service fixture; they do not validate
a real repository through the constructor or execute Git/native children.
Test-first `b895bea86490448fca3d5a328852885e58f1c542` /
[CI #40472797939](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40472797939)
failed compilation on Windows/Ubuntu (E0432 + four E0599: absent interface),
not runtime behavior. Initial formatting RED
[#40472675779](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40472675779)
is preserved separately; first implementation
[#40473274272](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40473274272)
also failed formatting only and remains recorded. Final-head/integrated receipts belong to
[PR #382](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/382).

This method is **not wired to live MCP GitRead**, creates no native Job/root/pipes,
reserves no native CPU/memory/process quota and grants no launch authority.
Next is registered Windows guardian custody of this same prepared value before
creation, final synchronized policy/identity/revoke/deadline validation and real
root/Job/EOF/crash proof. Original #352/#355 stay RED; #317/#323 OPEN, V1 NO-GO.

## 27 October 2026 — uncertain native reservations preserve stream observations (#383)

`NativeProcessReservation::finish` no longer marks stdout/stderr closed when
the effective state retains process ownership (`TerminationUncertain`, including
defensive normalization of `Running`). Existing per-stream observations are
preserved; active/memory/CPU/output reservations remain held and the record
cannot be reaped as terminal. Ordinary terminal completion behavior is unchanged.

This corrects an existing private-worker producer before it can be reused by
the native Git guardian. It does not establish root/Job/EOF evidence or make
generic captured-output flags a native termination receipt. The Windows-only
regression uses the real process-store reservation with **simulated** uncertain,
Running, post-creation-drop and one-stream-observed transitions; no child is
created and host-memory telemetry is synthetic.

Test-first `7da5c15299ca012ffcb216771da2121298c3e833` /
[CI #40476042507](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40476042507),
attempt 1: format and strict Clippy passed, Windows runtime failed the exact
unobserved-EOF assertion (366 pass / 1 fail / 11 ignored); Ubuntu passed because
this producer is Windows-only. Final-head/integrated receipts are tracked in
[PR #383](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/383).
Original #352/#355 remain RED; #317/#323 OPEN, Windows V1 NO-GO.
