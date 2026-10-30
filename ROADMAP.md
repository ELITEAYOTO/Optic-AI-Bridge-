# Delivery roadmap

## 30 October — preserve first pin-opened startup failure and bounded read progress

After host-stop binding, independent main41123153308 FAILED Windows active-job
before listener/ownership and before Node launch: six earlier cases pass, last
original identity phase pin-opened, no hash-complete within existing10s. New
host tests pass; no read-call/CPU/scheduler cause is established. Opt-in identity
read milestones now report the first two calls, then16MiB samples and EOF/error
metadata, at most70 records under the unchanged512MiB+1 read ceiling. Full core
hashing/pins/deadlines unchanged; this is evidence, not a startup fix. V1 NO-GO.
[Owning startup proof](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md).

## 30 October — bind original process-owned host stop

Trusted startup now owns an irreversible host-stop source, published on actual
OS stop or service completion/error before the peer future is cancelled. The
inert service-clock native owner requires and retains the original read-only
watch; missing/closed/foreign hosts fail closed and the independent watchdog
observes that original host as its fourth stop source. Legacy logical preflight
remains inert. No native launcher or shared create/resume fence is enabled.
Windows V1 **NO-GO**; [decision](docs/decisions/ADR-0039-ORIGINAL-HOST-STOP-PROVENANCE.md)
and [owning audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md).

## 30 October — denied watchdog with a live suspended root

Two hosted Windows fixtures extend the original bounded Job-list proof: after
actual watchdog ACCESS_DENIED on explicit stop or expiry, its worker is joined
but the original root handle must remain unsignaled and kernel Job count1.
Only subsequent trusted fixture termination using the unchanged full original
Job may signal the root and separately reach kernel-empty. The fixed system
root is never resumed; no handles are inherited. No product launcher/recovery
or resource policy changes. Exact-head and independent-main evidence remain
required; Windows V1 **NO-GO**.
[Owning authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md).

## 30 October — original runtime owner observes native termination outcome

The inert native Git status owner now exposes the original watchdog's retained
termination result without replacing pending/failure with success. A real
query-only duplicate of an empty fixture Job proves ACCESS_DENIED is retained
through both explicit stop and expiry, and remains readable after worker join.
No ACL, original Job rights, grants, horizon or cleanup order changes. Original
runtime session/lease-generation/Job-stop and deadline tests require the native
success receipt while counted resources remain owned. Exact-head hosted checks
and independent-main receipt remain required; Windows V1 **NO-GO**.
[Owning authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md).

## 30 October — retain the native watchdog termination result

The inert native Job custody now retains the completed TerminateJobObject
result: absent before an attempt returns, success only after native success,
or the original Windows HRESULT on failure. Cancellation without a stop or
expiry publishes no termination receipt. Deadline observation and worker
completion remain separate; even successful termination is neither root exit,
Job-empty, pipe EOF nor future launch closure (#439). No native Git activation.
Hosted exact-head and independent-main validation remain required. Windows V1
**NO-GO**. [Owning authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md).

## 30 October — native fence prerequisite: kill is not future admission closure (#439)

Keep successful Job termination and kernel-empty observation separate from closing
future creation/resume authority. The hosted sequential fixture allows a later
suspended system root in the same previously terminated empty Job, then performs
positive root/Job cleanup. Future guardian design must account for in-flight and
later native calls; repeating a stop receipt or snapshot alone is insufficient.
This does not implement the shared host/policy/session/lease boundary, bound a
blocked Win32 call or authorize native Git. No release; Windows V1 remains NO-GO.

## 30 October — physical pipe retention inside counted worker (#438)

Record component evidence that waiter cancellation/timeout cannot release the
original admission, transport or blocking capacity while an OS-pipe read remains
unfinished. The external fixture writer supplies EOF; this is not production
I/O cancellation or an integrated host/stdio/file-read shutdown proof. Keep those
acceptance gates, original-generation/fence, uncertainty/resource ordering,
security and installed-platform work separate. The unchanged full CI gate stays
required after an early focused one-test step; preserve first Windows probe
failure41058447719 without attributing or hiding it. V1 NO-GO, no release/native Git.

## 30 October — combined managed-job/output stop proof (#437)

Record the schema8 hosted active-Node plus unread-stdout case without replacing
any prior criterion: Windows10/Unix4, same five-second combined Bridge/Node gate,
original console/Job/native identity and both IO joins, both parent pipes open.
Next acceptance remains physical held counted work, safe uncertainty/resource
ordering, shared host/policy native-launch fencing and installed Windows10/11.
The registry wait-predicate compatibility currently rejects missing state; any
original-generation/drain change needs its own evidence and contract review.
Neither this conjunction nor #436 explains the historical startup stalls.
Windows V1 remains NO-GO; no release or native Git activation.

## 30 October — close the narrow competing-cleanup hypothesis (#436)

Record idempotence after another collector fully removes an already-observed
inactive original; do not add a speculative UnknownSession fallback. Both expired
and revoked cases must preserve the other live session/lease and deny active
cleanup. This closes only that audit question. Reaping during admission drainage,
held physical IO, uncertainty/resource ordering and shared host/policy/native Git
launch fencing remain separate work, as do the unproved startup stalls and
installed Windows10/11 acceptance. V1 NO-GO; no release.

## 30 October — keep terminal cleanup observation race-correct

Preserve main41005173901's Ubuntu EOF observer failure (Windows9 succeeded).
Reproduce actual revocation followed by registry reaping, then fix the test to
recognize both revoked tombstones and physical removal, independently for the
original session AND lease. Reject live/mixed states and report actual states
on future failure. Product cleanup, admission drainage and all deadlines stay
unchanged. Require exact-head and independent-main checks; then continue held
admitted IO/global launch fencing and startup/security/platform acceptance.
[Owning evidence](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md). V1 NO-GO.

## 30 October — identify the prelaunch startup stall

Preserve #433 main CI41003133144 and receipt6515093418: no Node launched, listener
registration exceeded10s, last phase read-authorities-provisioned. Capture fixed
begin/completion boundaries around canonical path, pinned executable open and
bounded full content hashing, plus process-lease entry. Existing completion-only
timing cannot identify an unfinished call. Require exact-head/all applicable
workflows, review, squash and independent main. A later green does not identify
or fix the underlying startup stall. Then address owned startup IO, host/policy
launch fencing and outstanding managed-resource/security/platform gates.
[Owning evidence](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md). V1 NO-GO.

## 30 October — real active managed job at original session expiry

Require operator lifetime10s without renewal, an attested active AppContainer
Node before5s from Bridge birth, retained native-handle exit between10..20s,
original session-reap marker and a live transport refusing another process_start
with exact inactive-session error. Only then close input and require exit/joins
within the existing5s EOF bound. Keep all eight prior Windows/four Unix cases
and first-failure diagnostics. [Schema7](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md)
owns limits; exact-head and independent-main validation remain mandatory.
This is not held-admission, atomic launch-fence or resource-release acceptance.
Then address those boundaries plus security/startup/installed-platform gates.
V1 NO-GO, no release.

## 29 October — preserve the actual first host-stop failure

Before another acceptance claim, preserve worker/Bridge diagnostics for early
active-readiness, original Job ownership and cleanup failures. #431 main CI
#40940330506 failed before ownership; its original worker cause was lost.
Keep its failure receipt and all case/deadline/containment guards. Require the
new exact head and independent main CI; a later green does not identify or fix
the earlier unknown startup cause. Then address held admissions, uncertain-stop
resource ordering, live host/policy/native Git and security/platform gates.
[Failure evidence contract](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md).
V1 remains NO-GO, no release.

## 29 October — active Node and private-console Ctrl+Break

After #430's independently verified active-job EOF proof, require a physical
Ctrl+Break with the real isolated Node alive, stdin held open and a console
containing exactly worker and Bridge. Keep all seven earlier Windows cases and
four Unix characterizations. Require the same shared five-second Bridge/Node
bound, both IO joins and normal empty external Job before recovery closure.
The private launcher must stay outside that console without changing Job limits
or assignment-before-resume. [Schema 6](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md)
owns evidence and nonclaims; #431/#323 own final exact-head/main receipts.
Then address held admissions, uncertain-stop/quota ordering and live native
host/policy/Git gates. V1 remains NO-GO, no release.

## 29 October — active managed job during actual transport loss

Require a real AppContainer Node job alive before EOF, original external Job
membership, native process-handle identity/liveness and signaled exit within the
same five-second bound as Bridge shutdown. Retain all prior stdio cases and
first-failure evidence; do not replace missing behavior with a fixture kill.
Then address held admissions, uncertain-stop/quota ordering, native Git and live
host/policy fencing plus the outstanding security/platform gates.
[Schema 5](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md) is deliberately narrower;
Windows V1 remains NO-GO pending its own final exact-head/main evidence and other gates.

## 29 October 2026 — finish the ordinary stdout lifetime gate

#429 must require physical exit with an unread stdout reader still open, retain
original admission cleanup and confirm both dedicated IO joins. Preserve initial
observer failure separately from target behavior; validate exact final-head and
independent-main CI. Then prove active-work/uncertain-IO ownership, the original
host/policy launch fence and actual native Git/root/Job/stream quiescence.
[ADR-0038](docs/decisions/ADR-0038-WINDOWS-STDOUT-OWNERSHIP.md) defines the narrow
output boundary; V1 remains NO-GO with #185/#319 and platform/security gates open.

## Windows V1 gate — stdout lifetime now has a separate characterization (#428)

The hosted schema-3 supervisor reproduces a root retained by unread queued stdout
after original cleanup and stdin-reader join; closing the output reader alone
releases exit 1 with input still open. This completes a **characterization step**,
not stdout-worker ownership, cancellation/join, or general bounded shutdown.
Next implementation must address that lifetime explicitly without weakening
cleanup or the existing Windows open-stdin exit gates. Active admitted work,
uncertain-stop policy, representative Windows versions and native GitRead
remain separate NO-GO gates. Keep release/tag creation disabled.

## 29 October 2026 — Windows input owner before broader shutdown claims

#427 requires Windows synchronous-pipe reader ownership, cancellation and real
join after original admission drainage; physical signal acceptance keeps parent
stdin open through root exit. Console/disk Windows input is outside this contract,
Unix retention remains separate. Next prove active-work/stalled-output/uncertainty
schedules and the live host/policy native-launch fence; do not infer a universal
shutdown guarantee from idle-input cases. [Owning ADR](docs/decisions/ADR-0037-WINDOWS-PIPE-STDIN-OWNERSHIP.md).
No native GitRead activation or release; Windows V1 NO-GO.

## 29 October 2026 — stdin ownership after physical host-stop proof

#426 observes real OS signals reaching original stdio cleanup on hosted Windows
and Ubuntu. Eight signal samples retained their process with input open through
500ms; reaping required explicit parent input closure. Next design and prove
owned stdin/process shutdown without premature admission/quota release or a fake
drain timeout. Then establish the live host/policy native-launch fence. Physical
signal proof is not active-child/Job/EOF or Windows 10/11 acceptance. [Owning proof](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md).
V1 NO-GO; no release/native GitRead activation.

## 29 October 2026 — cooperative process stop before native launch

PR #425 routes process-owned Ctrl+C/Ctrl+Break (Windows), SIGINT/SIGTERM (Unix)
through original stdio session/TaskLease stop-before-drain, including idle MCP
initialization. Internal notification regressions are separate from physical OS
signal delivery and process-exit acceptance. Next prove those paths with open
stdin, retained work and uncertain termination; Tokio blocking stdin and legacy
joins remain unbounded. Then establish a live host/policy native launch fence.
Exact-head and independent-main receipts: #425/#323. Windows V1 NO-GO; no release
or native GitRead activation.

## 29 October 2026 — service ownership before native launch

Validate unconditional original-session cleanup on default/supervised stdio EOF
and returned MCP initialize errors, retaining admitted capacity through drainage.
Then establish the independent original host/policy launch authority, interruption
ownership and real Git/root/Job/stdio termination contract described in the
[current continuation](docs/engineering/WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md).
This service-close slice does not authorize native MCP activation or a V1 release.

## 28 October 2026 — original lease stop before admission drainage

Baseline `main=eea5438873293243cd747b0f646ed903fb9f1282` integrates [#420](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/420); independent [push/main #40752232971](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40752232971) verified SUCCESS 4/4, identical tested/integrated trees.

The next targeted regression holds a genuine counted session admission during lifecycle revoke. Original session stop is already published before drainage, but the original TaskLease stop is currently published only afterward. A lease-only observer can therefore wait for a stop that depends on its own admission being released. The test-first candidate preserves that order to obtain behavioral RED.

Required correction: publish irreversible lease revoke flags before waiting for session admission drainage, retain the lease records/pins and counted resources, then repeat the lease sweep after drainage to catch registrations by already-admitted producers. Sum newly revoked counts from both sweeps without double counting; attempt all cleanup and propagate errors. This is stop publication, **not** destruction or physical process termination.

The real-registry test checks both original stop signals while the permit is held, failed early removal due to in-flight admission, revoke still waiting, a late registration caught after drain, exact revoke counts and eventual no-child reaping. No process is created; no policy/network/timeout relaxation. Final corrected-head and independent-main evidence must be checked before claiming integration. **Windows V1 NO-GO.**

See the new [current continuation](docs/engineering/WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md) (from engineering pages: `WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md`) for the next host/policy authority boundary and exact remote-check procedure.



Behavioral RED on first HEAD `cad47990342c8aa84a9ce6bcdb62943f7e90bdfa`: [Windows specialized #40753355684](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40753355684), job 123045630581, failed `lease stop waited for admission drainage`; ordinary [CI #40753355716](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40753355716) Ubuntu job 123045629922 failed the same runtime assertion (309 passed / 1 failed), after fmt/Clippy. No native child.

The correction publishes only irreversible TaskLease stop flags before drainage, retains original records/pins/capacity, and keeps the post-drain sweep. Newly revoked counts from both sweeps are combined; all existing cleanup attempts occur before either lease result is propagated. Final corrected-head and independent-main receipts are required before claiming integration.

## 28 October 2026 — session renewal admission/validation interleaving

Baseline `main=d66014405c7d4694ed3d7e31c0bf354a140f308d` integrates [#419](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/419); independent [push/main #40750356069](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40750356069) verified SUCCESS 4/4, identical tested/integrated tree.

The next targeted regression concerns a lifecycle debug assertion comparing an admission's captured expiry to a later registry observation. Admission and validation use separately released guards; a second valid renewal can extend that **same** session between them. Their numeric expiries need not remain equal. A private phase observer permits a deterministic real-registry interleaving without native execution. The first test-first candidate deliberately preserves the old assertion to obtain behavioral RED before correction.

Required final contract: ordinary renewal validation (active/nonrevoked, strict extension, maximum horizon and subordinate authority validation) remains mandatory. A successful report's previous expiry refers to this attempt's original admission, not a promise that no concurrent renewal happened. Existing operation permits keep their original deadline; session close between admission and validation must fail before extending any lease. Removing an invalid debug-only equality must not remove the real validation or change release-build authorization.

Two exact Windows-specialized tests cover the interleaved extension and interleaved close, alongside ordinary Windows/Ubuntu tests. Final corrected-HEAD and independent-main evidence must be checked before claiming this increment fixed/integrated. No Git process, launch authority, new permission/dependency/unsafe, timeout change or #185/#319 resolution. **Windows V1 NO-GO.**



Behavioral RED obtained on test-first HEAD `7553f7498178fdd8c704c6d8f931c9c41fe8d7e8`: [Windows specialized #40751137896](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40751137896), job 123037798106, failed the original equality with `MonotonicTime(200)` vs `MonotonicTime(100)`, and the regression then reported the outer renewal panic. This is an actual lifecycle/registry interleaving, no process simulation. Ubuntu in ordinary [#40751137862](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40751137862) failed formatting separately; no claim of Ubuntu behavioral RED.

The corrected implementation removes that debug-only equality and its unused observation variable, while preserving the **actual** registry validation and all later checks. It keeps the admitted original expiry in the report/old permit; it does not substitute the current renewed expiry as an operation deadline. Final-head and independent-main validation remain required.

## 28 October 2026 — TaskLease identity IO outside revocation mutex

Baseline `main=5e8d26e4f2b4bef2844cd4d4efb9c5d3876b3cfb` integrates [#418](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/418); independent [push/main #40748015791](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40748015791) verified SUCCESS 4/4, tested/integrated trees identical. #418's first natural Node/AppContainer red remains preserved in #185, not explained or erased by later green source.

Lookup and session-wide lease renewal previously canonicalized/opened/hashed executables while holding the lease revocation mutex. A stalled identity read could therefore prevent original revoke/removal from publishing. The next slice captures immutable lease/generation/clock metadata plus shared **original executable pins** under that mutex, performs real identity verification outside it, then re-enters the mutex to reject revoke/removal/replacement before returning. A concurrent renewal cannot extend the lookup's captured horizon or replace its provenance.

Renewal verifies outside the mutex, then validates the entire original active set, generation and captured expiry before any update. Concurrent additions/removals/reuse/revoke or overlapping renewal fail closed with the existing public lifecycle denial; no partial expiry publication. Retained Arc authorities keep Windows pins alive after physical registry removal until verification finishes. No reader/IO thread is detached or abandoned.

Three deterministic registry tests inject a bounded pause at the actual verification boundary while retaining real file pins and hashing afterward. They prove revocation/removal/replacement can complete before verification is released, stale lookup refusal, original horizon after renewal, unchanged-set all-or-nothing renewal, and real Windows write denial after registry removal. This is a concurrency/ownership proof with an injected phase stall, **not** an observed blocked kernel filesystem call or native Git termination. Specialized hosted Windows CI executes these exact tests.

No native launch, new unsafe/dependency, timeout increase, permission relaxation or public MCP error change. Cross-registry session/lease/policy/host effect linearization, actual service owner, physical root/Job/stdio and #185/#319 remain unproved. **Windows V1 NO-GO.** Verify final exact-head and independent post-merge CI before using this slice as integrated evidence.


## 28 October 2026 — original TaskLease clock provenance

Baseline `main=6bcdd77d3dcf259dfe03145fe583f9ff832e2974` integrates [#417](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/417); independent [push/main #40744570584](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40744570584) verified SUCCESS 4/4, tree identical to tested source.

The next inert slice issues a **new** TaskLease expiry on the retained StdClock from a positive checked lifetime. Lease identity/scope/capability metadata is retained; an existing registration or transferred witness cannot be retroactively tagged. Original registration provenance is captured together with the executable-verified lease and generation stop signal under the existing mutex. The service-clock native constructor checks **that retained witness**, not a second public-ID lookup, before binding the watchdog clock. Missing or foreign lease provenance fails closed and releases the no-child worker permits.

Successful untagged expiry extension removes provenance for future preflights; already captured witnesses retain their original horizon, origin and generation stop signal. Legacy registration/lookup/renewal and production MCP remain compatible. Clock-coherent fixtures now issue transport, session and TaskLease on the same service clock; the action horizon is sampled by preparation on that retained clock. Existing independent watchdog/revoke/rearm tests remain non-executing.

Tests cover raw/foreign/cloned origins, duplicate-registration refusal, invalid/overflow lifetime, real expiry/revoke, renewal and same-ID replacement, and native preparation rejection with permit cleanup. No unsafe/dependency, new native launch or security-policy change. Ordinary exact-head Windows/Ubuntu CI, specialized preparation and independent push/main must be checked before declaring integration complete.

**Remaining limits:** this proves an internal inert construction, not live MCP worker wiring, cross-registry atomic launch, HostStartup/policy cancellation, crash recovery, physical Git/descendant exit or pipe EOF. Raw legacy permits cannot arm this native watchdog. Native GitRead stays disabled; Windows V1 stays **NO-GO**.


## 28 October 2026 — original session clock issuance and admission

Previous increment [#416](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/416) is squash-integrated at `36fc443cfa70bd36873aa25ba2ca17161ec4276f`, tree identical to its tested HEAD; independent [push/main #40742225141](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40742225141) verified SUCCESS 4/4.

The next inert slice binds original session registration and owned admission to that retained service clock identity. The internal registration issuer computes a **new** expiry from its own clock sample and a positive checked lifetime; its SessionGrant parameter supplies identity/capability metadata, not an expiry to attest. It cannot tag an already registered grant or transferred permit. The clocked owned-admission constructor captures the exact registration provenance under the existing admission mutex, checks it against its clock and rechecks original expiry/stop after admission. Logical-only admission remains untagged even for a clock-issued session. Native service-clock preparation rejects missing/foreign session provenance before creating any native resource.

Legacy register/admit/renew remain compatible. A successful untagged renewal clears provenance for **future** clocked admissions; existing permits retain their original immutable expiry and identity. No renewal can extend their native horizon. Tests cover foreign/missing origin, raw admission, Clone, checked deadline errors, real expiry, revoke, session handle reuse, renewal and RAII rejection cleanup; existing empty-Job/watchdog tests are retained with clock-issued session fixtures.

**Not end-to-end authority:** TaskLease expiry still lacks provenance; production MCP still uses legacy GitRead. No child, new launch API, global session/lease/host/policy fence or physical terminal proof is added. No unsafe/dependency or GitHub-policy change. Windows V1 remains **NO-GO**. Exact final PR CI and independent post-merge CI must be verified before claiming this slice integrated.


## 28 October 2026 — transport clock provenance (first P0 slice)

Baseline `main=d7025f7a62edcbdd25545ebb794791072eca7d98`; independent [push/main CI #40711907388](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40711907388) verified SUCCESS 4/4; #415 merged.

The inert native preparation now rejects transport permits admitted without clock provenance or on another StdClock. A crate-private clocked admission captures the time itself and retains an allocation identity minted by StdClock; clones retain identity, new clocks cannot substitute it. Identity is attached only during original transport admission, never retrofitted to a transferred permit. Rejection consumes and RAII-releases the original worker permits before native registration; the legacy logical admission and production MCP path remain compatible.

This deliberately small slice establishes **transport provenance only**. SessionGrant and TaskLease expirations and session admission still carry untagged MonotonicTime. Their clock provenance is NOT demonstrated; successful inert preparation is NOT end-to-end temporal authority or launch permission. Action time is sampled by the retained preparation clock. Before native activation, bind original session/lease issuance and admission to that same authority, preserve captured deadlines through renewal, then prove unified policy/host/create/resume ordering and physical root/Job/EOF ownership. **Windows V1 NO-GO; #323 remains open; no native GitRead activation or release.**

Tests add equal logical deadlines with distinct origins, clock clone/foreign/missing transport identity, original deadline retention, native rejection cleanup, and valid clone preparation. Existing no-child watchdog expiry/rearm/revocation tests remain applicable. CI receipts must be checked on the final PR HEAD and independently after squash; no local project code was run. Historical local recovery notes were read but not restored or treated as current GitHub evidence.


## 28 October 2026 — service-clock watchdog binding

Continuation from verified `main=aa50c09570d5894118d33590657465df644299f3`, [independent CI #40693716141](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40693716141) SUCCESS 4/4. The non-executing native owner now requires preparation through the internal `prepare_native_status_on_clock` constructor. It samples the retained service `StdClock` itself and projects the earliest captured absolute deadline as `clock.origin + deadline`, using checked Instant arithmetic. Arm accepts neither a replacement clock nor a caller-supplied logical `now`. Logical-only legacy preparation has no watchdog-clock binding and fails closed with `ClockRequired`.

Tests cover projection retaining the original origin across clock clones and an already elapsed horizon, missing-clock rejection, actual independent watchdog expiry, and expiry-before-arm after renewal. Existing original session/TaskLease-ID-reuse/Job stop tests now admit their worker permits on the same retained clock. No child is created; quota/transport cleanup remains checked.

Validation receipts and final integrated SHA belong to the targeted PR and [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). This entry describes the change, not an unverified CI success. **Windows V1 NO-GO**, no release, no native MCP GitRead switch. Future real service integration must supply its original worker clock; permits still use untagged logical times, so foreign-clock provenance across arbitrary permits is not claimed solved. Host/policy cancellation, unified create/resume authority and physical root/Job/EOF receipts remain required. Older checkpoints below are historical.


## 28 October 2026 — PR #414: original-authority watchdog wiring

Technical source `8438534eabd4d4678d97b430180c45811af2a9db`: [CI #40691925094](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691925094) **SUCCESS 4/4**, including hosted Windows/Ubuntu, strict Clippy, fmt, dependency policy and release evidence guard. Windows job `122828738433` explicitly passed:
- `git_read::native_status_tests::original_watchdog_observes_session_lease_generation_and_job_stops_without_child`
- `git_read::native_status_tests::original_watchdog_rejects_expiry_before_arm_without_resetting_original_horizon`

Same-source specialized Windows runs succeeded: [native lifecycle #40691925103](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691925103), [private preparation #40691925095](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691925095), [Rust compilation #40691925262](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691925262). Initial candidate `0476ac28` failed fmt in [#40691730888](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691730888); review also corrected an erroneous textual timer edit before final validation. No retry-to-green.

The non-executing owner arms #412 from its retained original session, TaskLease-generation and Job cancellation signals. No MCP-controlled flags or public-ID lookup selects them. A conservative absolute Instant horizon is captured **before** preparation from the earliest original action/transport/session/lease horizon, and passed unchanged through handle duplication and worker startup. Session renewal, delayed arm and repeated arm cannot restart it. Tests separately observe the worker reacting to session close, original lease revoke despite ID reuse, and Job stop; expiry before arm after session renewal fails closed. Original transport/process capacity remains counted until no-child disposal.

**Limits / next priority:** source-to-service clock provenance and unified host/policy/session/lease create/resume ordering still require an integrated native guardian; the trusted logical `now` must come from the same service clock as admission. Independent registry receipts are not an atomic launch grant. Worker completion is not a successful TerminateJobObject receipt, root/descendant termination or stdout/stderr EOF. No process is created, and live MCP GitRead still uses HardenedCommandRunner. **#323 OPEN, Windows V1 NO-GO; no release.** Validate the final PR HEAD (including this documentation) and independent push/main CI before calling integration complete; the exact squash SHA and postmerge receipts are recorded in [PR #414](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/414) and [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). Earlier dated entries below are historical.

GitHub-hosted checks only. Local recovery disk is outside this task scope and unverified; no project build/test or installation ran on the operator PC.


## 28 octobre 2026 — jalon #412 : watchdog natif précréé, V1 toujours NO-GO

**État vérifié de `main` avant la présente PR documentaire :** `f773ab0860f12bdaa7c48baa1f358d64ad219d1c`. La PR technique [#412](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/412), HEAD exact `4a1a3f1aa7bf50ab0388c1f3e17935e894930400`, est fusionnée. Sa [CI de PR #40682603293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40682603293) et la [CI indépendante de `main` #40683212714](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40683212714) ont chacune fini **SUCCESS, 4/4 jobs** au SHA correspondant. Le runner Windows de PR (job `122796825413`) affiche `test job::precreated_native_read_tests::unstarted_native_read_watchdog_is_nonrenewable_and_deadline_bounded ... ok`. Les trois workflows spécialisés sur ce HEAD sont verts : [Owned Native Process Pipes #40682603185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40682603185), [Private Rust Compilation #40682603354](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40682603354), [Private Project Preparation #40682603219](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40682603219).

**Portée exacte #412 :** un seul fichier Rust, `crates/optic-bridge-windows/src/job.rs`. `PrecreatedNativeReadCustody` peut armer **une fois** son `NativeJobDeadline` indépendant sur le **Job original** : délai positif, au plus quatre `Arc<AtomicBool>` d'arrêt, refus du réarmement et de paramètres invalides, duplication non héritable du handle de Job et destruction du watchdog avant le Job. Test sur un **Job réellement ouvert mais sans aucun enfant** : échéance du watchdog observée, Job vide. Ceci **ne prouve pas** la terminaison d'un GitRead réel, des descendants ou l'EOF physique des pipes; ni l'autorisation atomique de lancer. Les véritables signaux d'arrêt d'origine et le temps restant depuis la deadline monotone ne sont **pas encore câblés** à cette primitive dans le runtime.

**Preuves précédentes conservées :** [#410](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/410) prouve en test Windows synthétique qu'après sortie du processus racine un descendant peut garder stdout/stderr ouverts, et que la terminaison du Job permet d'observer **séparément** Job vide + EOF des deux pipes. [#411](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/411) a consigné cette preuve en documentation. Aucun de ces tests ne migre le `git_status` MCP vers un nouveau launcher.

**P0 suivant (une PR à la fois) :** définir une liaison sûre entre la custody précréée et les **témoins d'arrêt originaux et non substituables** (session, TaskLease, stop du Job, host), conserver la **deadline absolue initiale** et empêcher qu'un renouvellement/redémarrage du compteur ne donne plus de temps. Ne pas exposer des signaux contrôlables par un appelant non fiable. Exiger des tests de révocation/expiration entre étapes sans créer de processus. Ce câblage n'est **pas** une barrière globale : session et TaskLease utilisent deux mutex distincts. Une véritable borne de linéarisation avec politique, identité de cible, HostStartup et CreateProcessW/ResumeThread reste à concevoir, sans verrou détenu pendant les appels Win32/I/O ni abandon incertain de ressources.

**Garde V1 :** [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN / Windows 10-11 V1 NO-GO**; [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) et [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) **OPEN**. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est fermée **administrativement**, sans preuve de correction du `HardenedCommandRunner` live dont les `.join()` des lecteurs de pipes peuvent rester bloquants. Ne pas activer GitRead natif ni publier d'installateur/release avant validation réelle. Interdiction d'intervenir sur le PC utilisateur, les dépôts Desktop/REL ou des runners auto-hébergés. CI GitHub Windows et Ubuntu uniquement. Méthode : **une seule PR ciblée à la fois, exact-head CI, revue, squash, puis CI indépendante de `main`**.

## 28 October 2026 — #408 real stdout/stderr EOF on fixed Windows root

**Integrated technical test:** [PR #408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/408) tested exact HEAD `e346a0b6f219ca55a360b292879b4f16468e8c01` with [CI #40675433145](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433145) **completed/SUCCESS, 4/4 jobs, attempt 1**. Hosted Windows job `122772868039` logged `job::job_list_launch_tests::resumed_job_root_produces_two_physical_pipe_eofs ... ok`. The three specialized hosted Windows workflows [#40675433176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433176), [#40675433141](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433141), [#40675433144](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433144) also succeeded on the exact HEAD. Squash-integrated `main=fb10d7e98664a6b4deda2c8b509bdf1048672618`, tree unchanged `b65ff73a7351bac620256fb923457062cc4bfed0`. Independent post-merge [push/main CI #40676169755](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40676169755) was triggered for this precise SHA; **check all four final jobs live**, do not equate the PR checks to that independent CI.

**Exactly one Rust Windows test source**, no production GitRead or MCP changes. A fixed system `cmd.exe /D /Q /C` echo fixture is created **suspended** with kernel `PROC_THREAD_ATTRIBUTE_JOB_LIST` AND explicit inherited-only `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, `NUL` stdin and separate bounded pipes. The test verifies Job membership before `ResumeThread`, real root exit code `0`, separate kernel Job-empty, **physical EOF of BOTH stdout and stderr**, and distinct child output markers. The original candidate's first CI failed formatting only; corrected on final HEAD. No user-controlled project, real Git, external network, or production owner is involved.

**Critical nonclaims:** no actual service-owned GitRead process, descendant-retained writers or bounded timeout with descendants, native owner/Job uncertainty recovery, shared session/TaskLease/policy/host commit/stop ordering or safe Win32-call stall handling. `HardenedCommandRunner` remains live for MCP GitRead with known potentially unbounded reader joins. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is administratively closed, not behavioral acceptance. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN: Windows V1 NO-GO**; #185 and #319 remain separate blockers. [Native authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md) is still the required next technical design/proof. No release or PC changes.

## LATEST P0 FREEZE GATE — 28 OCTOBER 2026, Windows Job resumed-root proof #406

**Verified integration:** [PR #406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/406) is **squash-merged**. Tested final HEAD `f4c7b8163d36434f5a26486625d4963c42a0dee8`, integrated `main=be59544537d5dc027b54e4bb5cd7f553f979d59d`, identical trees `f4d702915e429821bd658d6317014e0cd316241a`. [PR CI #40672893075](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893075): **4/4 jobs SUCCESS, first attempt**, exact final HEAD. Same-HEAD hosted Windows workflows [Owned Native Process Pipes #40672892946](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672892946), [Private Rust Compilation #40672893056](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893056), and [Private Project Preparation #40672893094](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893094): all SUCCESS. Independently triggered [push/main #40673602054](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40673602054) on the exact integrated SHA; **verify its completed/final four job conclusions live** before using it as post-merge proof.

**One test-only Windows file**, `crates/optic-bridge-windows/src/job_list_launch_tests.rs`: `job::job_list_launch_tests::resumed_root_exits_and_kernel_job_drains_after_job_list_creation ... ok` in hosted Windows Rust log. A fixed synthetic Windows `cmd.exe /D /Q /C exit /b 0` is created suspended, already Job-associated via `PROC_THREAD_ATTRIBUTE_JOB_LIST` at creation, then actually resumed; root exit code 0 and **separate physical Job-empty** observation are bounded. Original PR CI failed `rustfmt` on an earlier HEAD, corrected and preserved as a formatting failure, **not** a launch/liveness failure.

**NOT demonstrated:** no service-owned native GitRead launch/MCP switch; no descendants, stdout/stderr EOF, independent host/session/TaskLease/policy cross-authority final commit, Win32 blocking stop, uncertainty guardian, or legacy runner elapsed-time correction. Sessions and leases still issue independent receipts under different mutexes. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN**; [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) closed administratively, original pipe liveness not shown resolved; #185/#319 open. **Windows V1 remains NO-GO; no release.** Follow the concrete mutator/lock/host audit in [#323 native authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md); do not confuse static policy epoch equality with a live cross-authority issuer. Previous dated sections are historical snapshots.

## LATEST P0 FREEZE GATE — 28 OCTOBER 2026, after #404

[PR #404](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/404) : HEAD testé `cc7a77b3b5415c6b8d601c477c2373e39f84f7b6`, squash intégré `main=0d14502da39d90ccecd8f113981cd3bdb6520159`, arbre source/intégré identique `55bd4edc12e645ddc3e27a1cb799db932208cec0`. CI [PR #40666361354](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361354) **4/4 SUCCESS sur HEAD corrigé**, [native #40666361300](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361300) SUCCESS, et **CI indépendante push/main [#40667115232](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40667115232) `completed/success` 4/4 jobs, tentative 1, sur SHA intégré exact**.

**#404 ne modifie que les tests Windows** (`crates/optic-bridge-runtime/src/git/native_status_tests.rs`, +189 lignes) : `session_close_between_original_session_and_lease_receipts_denies_native_custody` et `dual_receipt_session_close_after_lease_commit_drains_and_stops` ressortent **`... ok`** dans les logs Windows. Le premier scénario prouve qu'une fermeture lifecycle de session *entre* les reçus session et TaskLease interdit de poursuivre malgré une TaskLease encore active. Le second prouve qu'une révocation directe *après* les deux reçus publie le stop de génération d'origine, attend la libération du permis compté, et conserve le Job Windows **vide** et les quotas jusqu'au drop. Aucun lancement Git.

**P0 ensuite :** auditer les vrais chemins mutateurs session/TaskLease/policy epoch/host shutdown et JobStore, prouver un protocole d'engagement/révocation au plus près de Windows Job-at-create avec arrêt hors verrou, puis processus réel/descendants/stdout/stderr EOF. Pas de nouveau snapshot présenté comme lancement autorisé. [Prompt complet daté 28/10](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-28.md). La PR documentaire crée un SHA ultérieur : **revérifier le nouveau main et sa CI indépendante après fusion**.

**Limites :** les reçus session (#400) et TaskLease (#402) sont émis sous **deux mutex indépendants**, et ne constituent PAS un fence atomique pour policy epoch, host shutdown, `CreateProcessW`, `ResumeThread` ou publication. Aucune preuve après exécution Job vide/descendants, stdout/stderr EOF, watchdog borné ou récupération de terminaison incertaine. MCP GitRead reste dans la voie legacy `HardenedCommandRunner` (pipe joins non prouvés bornés). [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est **CLOSED administrativement**, sans preuve physique suffisante du pipe-join ; [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323), [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) **OPEN**. #352 fermée non fusionnée ; #355 recherche draft. **Windows V1 NO-GO, aucune release.**


## CURRENT P0 — after #402 original TaskLease receipt (28 October 2026)

[PR #402](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/402): exact tested HEAD `04a0c025202dbf2dfbd8cf43d3314c64260e6bee`, squash `main=fc53da251ff1c6b396fcdddc8092b9d18712cfd9`; identical tree `d287df495e1543e8d04e083651a34fd9ed641aa6`. Exact-head [CI #40663011191](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011191) **SUCCESS 4/4** and [specialized native #40663011174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011174) **SUCCESS**; independent [push/main #40663744586](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663744586) **SUCCESS 4/4 on exact merged SHA**.

The Windows-only, non-executing native GitRead candidate now retains **two separately issued** original-generation stop receipts: session (#400) and TaskLease (#402). The TaskLease receipt is minted under the same `TaskLeaseRegistry.leases` mutex as direct/session revoke, physical removal and registration, requires exact original registration-signal identity and preserved admission-time expiry. Four hosted Windows tests prove stop after revoke, `revoke_session` and `remove_session`, same-ID reuse and foreign registry rejection, post-renewal deadline, preserved real **empty** Job and counted permits. Foreign/reused registration fails closed with existing `Revoked` (no new public MCP error variant).

**Next P0 gate:** design a sealed effect/policy issuer whose revocation/epoch updates are actually serialized with an enforceable launch/publish boundary; independent session and lease snapshots alone are not sufficient. Audit host shutdown and JobStore lock order, prove prompt out-of-band stop during stalled Win32 creation, then build a real suspended at-create-time Job-associated native GitRead owner. Prove process-tree zero, both output EOFs, stdin closure, bounded watchdog, uncertain-termination retention and nonpublication after revocation before any MCP migration or Windows V1 acceptance.

**No-go:** sequential session and TaskLease receipts are **NOT** an atomic session/lease/policy/host effect fence or CreateProcess permission. There is NO launched native Git root, no descendant process/Job/stdio EOF observation, no watchdog or uncertain-termination cleanup proof, and no MCP GitRead migration. Legacy runner is unchanged. GitHub [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is currently **closed**, but its described pipe-join bounded-wall-time acceptance has NOT been established by these no-child tests; closed/unmerged #352 is not evidence of resolution. [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN — Windows V1 NO-GO**, no release, no operator-PC or other project changes.


## CURRENT P0 — after session-only native owner #400 (28 October 2026)

[PR #400](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/400) tested HEAD `a949fbf1745de0f97e03284628967c30b2fa887e`, squash integrated `main=67e3604c198f83c16465863facdf2fa855f983d7`, identical tree `444a3175b7241d85d4e959de456f8fa4f1341ae0`. [PR CI #40658720479](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658720479) **SUCCESS 4/4 first attempt**, [native preparation #40658719448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658719448) SUCCESS, and independent [push/main #40659396121](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40659396121) **SUCCESS 4/4, attempt 1 on the exact integrated SHA**.

The short *session* authority commitment is now backed by a registered, empty Windows Job and immutable original-generation stop receipt; it is **not a launch authorization**. Next P0: design and test an independent **TaskLease/policy issuer fence** and durable native owner stop that remains observable across an OS launch in progress; then actual Job-associated suspended Git root, pre-resume checks, bounded full stdout/stderr EOF and descendant/uncertain/quiescence accounting; only then migrate GitRead MCP under existing sensitive-name policy. No native call, filesystem observation or unbounded wait under session or JobStore locks.

This is **NOT** a native launch grant or the final GitRead commit. TaskLeaseRegistry and policy epoch revocations remain independent; there is no CreateProcess/ResumeThread, actual Git process/root+descendant/stdio EOF, bounded guardian, termination uncertainty proof or production MCP GitRead switch. #352 was closed **without merge**, not accepted as proof; #355 behavioral acceptance remains unmet. #317/#323/#185/#319 remain OPEN. **Windows V1 NO-GO, no release**.


## CURRENT P0 — 28 October 2026, after #398

[PR #398](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/398) (exact tested HEAD `7456f3d96dc289eb5e278e3d9acbfce8970a6dcf`) merged into `main=8aa6fd205e7a264e7873e6af6b627ddb81207d9d`. Trees identical `8883d4c0ee6a3647b543799c29f9b573be93a028`. Exact-head [CI #40655468052](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468052) **SUCCESS 4/4**, [native preparation #40655468078](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468078) **SUCCESS**. Independent [push/main #40656160001](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40656160001) **SUCCESS 4/4 on exact merged SHA, first attempt**. First candidate's [CI #40655374591](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655374591) failed rustfmt, not #317 behavioral acceptance.

The Windows-only, internal `SessionRegistry::observe_original_owned_admission_under_close_lock` rechecks an owned admission's original registry, registration-generation signal, revoked bit and captured expiry under the **same mutex** as direct `revoke` and lifecycle `publish_revocation_before_drain`. The precreated, **non-executing** native GitRead custody now invokes it both before and after its independent lease/policy snapshot. Windows tests `original_owner_observation_orders_with_direct_and_lifecycle_close` and `original_owner_observation_rejects_foreign_registry_and_renewed_deadline` passed on exact HEAD. No OS IO or policy work is performed under that session lock.

**Next required engineering proof:** a short original-generation owner-commit event genuinely serialized with both direct and lifecycle session close, plus a separately proven TaskLease/policy fence; never perform Windows native creation, filesystem observation or indefinite waits while holding shared session/JobStore locks. Next, a real Job-associated process and independent stop/watchdog with physical root+Job+stdout/stderr EOF and uncertainty retention before any GitRead MCP migration.

**Nonclaims:** the mutex is released after this observation: it is not an owner **commit** or CreateProcess permit, nor a cross-registry TaskLease/policy epoch fence. Session or lease revoke may still race native creation. No real GitRead root, native child/descendant/stdio EOF, out-of-band stop, crash/uncertainty recovery or MCP switch is implemented; real #352/#355 acceptance remains RED. #317/#323/#185/#319 remain OPEN; **Windows V1 NO-GO**, no release or operator-PC access.


## CURRENT P0 PRIORITY — after independently verified #396 (28 October 2026)

Merged [#395](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/395) protects the **original** TaskLease witness against same-ID re-registration in a non-executing native owner; merged [#396](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/396) proves a genuine empty Windows Job stays empty when a valid Job-list `CreateProcessW` fails with `ERROR_FILE_NOT_FOUND`. Both are test-only. Exact independently verified `main=8ee1ad0b2c135af3d639ab753703c3f911dc4941` ([CI #40653860272](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653860272), 4/4 SUCCESS). See [handoff](docs/engineering/GITHUB_HANDOFF.md) for all test/workflow receipts.

**Next actual engineering gate:** shared, short, original-generation owner commit serialized against direct and lifecycle session revoke, with separate TaskLease/policy epoch revocation fences; then owned Windows native process startup, out-of-band watchdog/stop, root/Job/descendant+stdio EOF proof and uncertain/crash lifetime retention. Only after real #352/#355 acceptance may the MCP GitRead pathway migrate. #317/#323/#185/#319 remain OPEN, V1 NO-GO.

## 28 octobre 2026 — jalon Windows natif expérimental #393

La [PR #393](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/393) ajoute à `main=c81c1f5caf937695b3bbd9439f508ff9cb28f6f4` **uniquement** un test Windows effectif du Job attaché **pendant** `CreateProcessW` via `PROC_THREAD_ATTRIBUTE_JOB_LIST`. Le root système `cmd.exe` reste suspendu ; le test observe `IsProcessInJob`, un Job comptant un processus et ensuite sa terminaison réelle et le Job vide. CI PR [#40649262906](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262906) 4/4 SUCCESS + trois workflows natifs verts ; [CI indépendante push/main #40649847690](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649847690) **completed/SUCCESS 4/4, tentative 1 sur le SHA intégré exact**.

**Prochain P0 réel :** verrouillage d'engagement d'autorité de génération d'origine couvrant session, TaskLease, epoch politique, deadlines et pins de cibles, avec test déterministe commit-versus-revoke ; ensuite propriétaire Windows structuré (Job déjà associé, reprise conditionnée, watchdog hors appelant, stdout/stderr/EOF réel, descendants, quiescence et `TerminationUncertain`), et seulement ensuite raccord GitRead MCP sous règles de confidentialité identiques. Ce test ne prouve **ni** l'absence d'effet si l'hôte meurt au milieu de `CreateProcessW`, ni la révocation atomique, ni les acceptations #352/#355, ni une validation Windows 10/11. **#317/#323 restent OPEN, V1 NO-GO, pas de release.**

## PRIORITÉ P0 ACTUELLE — après #390/#391, 27 octobre 2026

Le contrôle de **génération originale** TaskLease #390 et le test de renommage de parent de dépôt #391 sont intégrés. Les protections préparatoires ont maintenant un vrai Job Windows **non exécutant**, des pipes, pins cible, quotas et permis originaux, et un snapshot d’autorité qui ne crée **aucune permission de lancement**. Ne pas dupliquer #391 : la garde parent est déjà prouvée par Windows CI. Prochain axe : démontrer une autorité **synchronisée à la frontière Win32 `CreateProcessW`**, avec révocation, policy epoch, délais monotones et identité Git, puis root suspendu/Job attaché avant exécution, vraie terminaison root, Job vide **après enfant** et EOF des deux pipes, descendants, watchdog, crash/reprise. Ne pas raccorder MCP tant que les acceptances #352/#355 restent RED et les restrictions de chemins sensibles ne sont pas vérifiées. #317/#323 OPEN ; V1 NO-GO. Checkpoint `main=c208348f0b63d4810db97f9a5284b325c5b64271` [push/main #40553879389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40553879389) SUCCESS 4/4, à revalider après cette PR documentaire.


## P0 after the candidate original-generation GitRead prelaunch snapshot (#390)

After original session/transport admission, counted private process reservation, genuine *unstarted* Win32 Job, stdio and target pins, the native custodian may now observe current typed GitRead policy plus original TaskLease registry *generation*. Next design and prove a **single synchronized final native launch authority** across lease/session/policy/epoch/deadline/revocation, pinned image, `CreateProcessW` and Job assignment-before-resume. Preserve the original owner and all resources through root exit + real Job-empty + actual EOF, including uncertain termination and host-loss recovery. Only after full native tests switch MCP `git_status` without reducing sensitive path filtering. #352/#355 RED, #317/#323 OPEN, V1 NO-GO.


## P0 after integrated native GitRead target pin custody (#388, 27 October 2026)

Already-admitted status can now keep **original-path handles** for the Git binary and readable canonical repository, together with its unstarted Job/pipes, session/transport admission and budgeted process record. Next: independent current TaskLease executable hash/policy/epoch/session/deadline revalidation **synchronized at the Win32 CreateProcess boundary**, with root Job assignment before resume. Then keep Job/stdio/executable/directory pins and all permits across uncertain termination until root, genuine Job-empty and actual EOF receipts, including host interruption and crash. Only after these proofs enable one MCP `git_status` path with original sensitive-name filtering. Do not assume #352/#355 accepted or Windows V1 releasable.


## Next P0 after no-spawn Win32 Job/pipes preparation (#387, 27 October 2026)

A real **unstarted** Windows kernel Job plus stdout/stderr pipes and closed
stdin can now be held with the original admitted Git status registration.
The next micro-increment must pin the exact Git executable and canonical
repository directory and provide synchronized, fail-closed final
session/TaskLease/policy/epoch/deadline authorization **at process creation**,
with native custody pre-established; do not expose a public spawn method
until that is proven. Then add root/Job-empty/real EOF and host-loss/
uncertain-retention proofs and only afterwards route one admitted MCP
`git_status` through the guardian, preserving sensitive path checks.

Original #352 and #355 real acceptance remains RED, #317/#323 OPEN and
Windows V1 NO-GO. No release, PC access, or bulk research merge.



## P0 after integrated registered native status accounting (#385, 27 October 2026)

The service can now **prepare** an exact lease-bound Git status and, on
Windows, **register process-store CPU/memory/output/process capacity** without
spawning. This is the last non-executing accounting boundary, not the Windows
owner itself. Next increment must create and register real kernel Job/root/
stdout/stderr/stdin custody and pinned executable/repository evidence **before**
any process creation; synchronize the final policy, session/TaskLease epoch,
revocation and monotonic deadline decision with the Win32 creation boundary.
Preserve owner and all counted permits under uncertain termination until true
Job-empty, root termination and appropriate actual EOF observations, including
caller abandonment and host interruption. Then connect the authorized MCP
`git_status` worker without second admission and retain sensitive-name denial.

Do not claim #352 real wall-clock or #355 real revoke-before-spawn success from
#385. Existing acceptance reds, #185, #319, confidentiality #176/#182 and
physical Win10/11 #323 remain independently blocking; no V1 release.


## REPRISE EN CHAT — 27 octobre 2026

[Prompt de reprise](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md)
et [passation/préconditions/preuves](docs/engineering/HANDOFF_WINDOWS_V1_CHAT_2026-10-27.md).
Après #380–#383 : prendre en charge le PreparedNativeGitStatus exact dans un
propriétaire Windows enregistré, avec les mêmes permis/TaskLease, quotas et
Job/root/pipes avant création, puis autorité finale synchronisée. Ne pas activer
le lanceur ni accumuler des helpers sans consommation réelle. Faire passer
ensuite originaux #355/#352 inchangés et les gates indépendants de V1.
Chat GitHub-only ; aucun accès PC ni tâche Codex/Work implicite.
#317/#323 ouvertes ; Windows V1 NO-GO.


## 27 October 2026 — after native reservation EOF correction (#383)

Before reusing private-worker reservations, preserve stream evidence and all
uncertain ownership; #383 fixes a reproduced synthetic-EOF state transition.
Next remains registered Windows guardian custody of #382's exact prepared value,
native budgets/Job/root/pipes before creation and synchronized final authority.
Then original #355/#352 unchanged and independent V1 gates.
[Evidence and limits](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
#317/#323 OPEN; Windows V1 NO-GO.


## 27 October 2026 — after service-bound status preparation (#382)

Next P0: register a service-owned Windows guardian taking the exact prepared
status value and its original worker permits/TaskLease, with Job/root/pipes and
native quotas before creation. Revalidate current identity/policy/revocation
and original absolute deadlines at a synchronized launch boundary.
Then prove original #355 and #352 unchanged, including real terminal/EOF and
crash recovery; separate #185/#319, sensitive-data and Win10/11 release gates
remain. [Preparation contract](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
#317/#323 OPEN, Windows V1 NO-GO.


## 27 October 2026 — native action deadline correction (#381)

Before native activation, preserve the original action horizon alongside
transport/session/TaskLease stop conditions; #381 corrects this concrete
preflight gap after a real Windows+Ubuntu test-first failure.
[Contract and red evidence](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
Next: registered service-owned guardian taking #380's same worker permits,
canonical Git TaskLease, Job/root/pipes/quotas before creation and synchronized
final launch authority. Then original unchanged #355 and #352.
#185/#319, sensitive metadata and actual authorized Win10/11/installer gates
remain separate. #317/#323 OPEN, Windows V1 NO-GO.


## 27 October 2026 — owned MCP worker candidate #380

Candidate #380 routes Git reads through a transferable owned session/transport
worker while preserving legacy authorization and sensitive metadata checks.
Next: service-owned registered native Job/root/pipes guardian consuming those
same permits plus canonical TaskLease; synchronized final spawn authority.
[Contract and test-first compiler evidence](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
Implementation `ba05382e353f401198dcb979f48701b8d442e05b` passed
[CI #40466899740](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466899740)
4/4 at attempt 1; final-head/integrated receipts belong to
[PR #380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/380). No native activation. #352/#355 RED,
#317/#323 OPEN, Windows V1 NO-GO.


## REPRISE P0 — 26 octobre 2026

Documents : [Passation canonique Windows V1 du 26 octobre](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-26.md) et [prompt complet à copier](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-26.md). Avant cette PR documentaire : `main=c058ccf3f9db18c7d67194d117af99cc3e0cea52`, [CI indépendante push/main #40358111878](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40358111878) **SUCCESS 4/4 au premier essai**. Cette PR fera avancer `main` : vérifier le nouveau SHA et la CI `push/main` après fusion. #369/#371/#373/#375/#377 sont des contrats GitRead **inertes** ; production MCP GitRead toujours legacy ; #327/#353/#358 recherches isolées, originaux #352/#355 **ROUGES** ; issues #317/#323/#185/#319 ouvertes ; Windows V1 NO-GO, aucun accès PC/Optic Desktop/REL.

1. Auditer `ReadonlyMcpServer::run_admitted_read`, `authorize_git_read`, `GitReadService` et le contrat `NativeGitReadOwnedPermits` pour UN vrai `git status` MCP, sans double admission ni permission ambiante.
2. Enregistrer Job/root/stdio/quotas et pin identité Git AVANT la création native, synchroniser stop+epoch+TaskLease+deadline juste avant le spawn et préserver `TerminationUncertain`.
3. Réussir **originaux inchangés** #355 (aucun Git réel post-revoke) et #352 (vraie fin stdin/stdout/stderr en temps borné), puis #185/#319, données sensibles, Win10/Win11 physiques, installateur offline/update/rollback #323. **V1 NO-GO.**

---

## 26 October 2026 — after trusted transfer seam #377

[PR #377](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/377) adds a registry-origin-checked handoff of the worker's existing counted session admission and transport permit into an inert GitRead native-owner contract. This is **not live MCP routing**.

1. **Next P0 single-handler wiring:** redesign a tightly scoped MCP `git_status` worker to hold transferable **owned**, not borrowed, session admission and the original request permit. Keep the legacy GitRead path unchanged until a service-owned native Job/root/pipes guardian can take custody of the same admission/lease trio without double-counting or premature drop. Preserve sensitive staged/untracked metadata protections.
2. **Next P0 actual native launch authority:** exact canonical repository and fixed opened/pinned/hashed Git executable, registered Job/guardian and quota reservations **before** process creation, immutable generation-bound session/lease/transport stop witnesses, synchronized last pre-spawn policy/epoch/revoke/deadline checks; `is_stop_required_at == false` is never independent spawn permission. Keep capacity on `TerminationUncertain`.
3. **Acceptance unmodified:** original #355 real post-revoke no Git spawn and #352 true total elapsed stdin/stdout/stderr EOF, root + Job-empty and host-loss. Research #327/#353/#358 stays isolated until integrated honestly; independent #185 AppContainer, #319 MCP first response, #176/#182 sensitive authorization and #323 actual Windows 10/11 install/update/rollback remain required. **#317/#323 OPEN, V1 NO-GO.**

---

## 26 October 2026 — after three-resource native admission contract #375

[PR #375](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/375) now packages an owned session admission, *real owned request transport permit* and exact TaskLease/revocation witness into one inert Rust contract. All three retain their original per-operation deadlines. No Git process is launched by the helper.

1. **P0 real single-tool integration:** route one genuinely admitted MCP `git_status` envelope and sensitive metadata/provenance checks into a service-owned native Windows Job/pipe guardian, transferring the current worker's owned transport permit + counted admission + lease without double-dropping reservations or altering unrelated GitRead legacy behavior.
2. **P0 true pre-spawn authority:** pin/hash the fixed Git executable and canonical repository from operator-owned state, register root/Job/pipes and quotas before child creation, and revalidate policy/epoch/revocation/expiry immediately at the last synchronized spawn boundary. Watcher `false` never licenses launch. Preserve TerminationUncertain capacity.
3. **P0 behavioral acceptance unchanged:** prove original [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) no real Git start after revoke, original [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) actual elapsed stdin/stdout/stderr/EOF, root + Job empty and host-loss recovery. Separate #185 AppContainer, #319 first MCP response, #176/#182 sensitive provenance, #323 installer/offline updates/rollback and real authorized Windows 10/11 tests remain required. **Windows V1 NO-GO; #317/#323 OPEN.**

---

## 26 October 2026 — after registry cleanup witness fix #373

[PR #373](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/373) closes one *registry cleanup* invalidation gap: physical TaskLease removal now publishes original-generation stop, even if the normal session lifecycle's earlier `revoke_session` is bypassed. It does **not** close #317's native-process ownership/race, prove a child has stopped, alter the live MCP GitRead path or change #352/#355.

1. Preserve existing tests and scoped permissions. Next real execution seam remains **one admitted MCP `git status`** with server-owned identity, exact registered scoped TaskLease, sensitive metadata/provenance approval and a pinned executable.
2. Before any native child exists, register its service-owned Job/root/stdio and cancellation/reservations, then revalidate session/lease policy, identity, expiry and revocation at the final synchronized spawn boundary. Do not mistake a watcher reading `false` for launch authorization.
3. Prove the **original unmodified** [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) post-revocation real Git no-start and [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) total wall-clock pipe/EOF gates, including root + Job-empty, crash/restart recovery and uncertain native ownership. #185, #319, #176/#182, real Windows 10/11 and #323 installer/offline update/rollback remain independent gates. **V1 NO-GO, #317/#323 OPEN.**

---

## Next P0 after admitted native GitRead preflight #371 — 25 October 2026

[PR #371](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/371) now binds **genuine owned session admission** to the exact registry-verified TaskLease policy witness, with original session/lease revocation and expiry. See [native preflight contract](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md). This is an inert Rust service prerequisite, **not live MCP wiring or a spawn permit**.

1. Next narrow slice: wire one *real MCP-admitted* `git status` ActionEnvelope through the trusted server application session, task registry, sensitive filename/provenance checks and canonical root into a **registered, retained native owner and Job before child creation**. Keep legacy GitRead behavior unchanged until the new path satisfies all authorization and behavioral acceptance.
2. Validate final pre-spawn generation, policy epoch, deadline, exact opened/hashed FixedTool Git identity and original-stop witnesses at a properly synchronized authority boundary. Negative watcher state alone is insufficient; maintain count/Job/pipe ownership under `TerminationUncertain`.
3. Meet **original unchanged** [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) real Git post-revocation no-spawn and [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) elapsed stdout/stderr/stdin + EOF tests on Windows/Ubuntu. Continue separate #185 AppContainer, #176/#182 sensitive provenance, #319 first MCP response and #323 Windows 10/11 physical/offline install/update/rollback work. Do not bulk-merge #353/#358. **V1 NO-GO, #317/#323 OPEN.**

---

## P0 next seam — 25 October 2026, after integrated registered-lease witness #369

[PR #369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/369) combined the genuine registry TaskLease and original-generation revocation watch with the existing typed GitRead policy preflight. See [inert contract and limitations](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md). This is **not** a native owner, true MCP admission, or permission to spawn; the legacy `GitReadService` still launches via `HardenedCommandRunner`.

1. **Next smallest tested integration:** make one real MCP `git status` path pass its *admitted* `ActionEnvelope`, sensitive metadata/provenance decision, exact active session and TaskLease, canonical worktree and opened/hash-verified Git executable into a service-owned native Job/reservation. Do not change all legacy GitRead endpoints as a shortcut.
2. **Final native boundary:** register all ownership, pipes, stop/watch and quotas before creating the process; revalidate authorization, deadline, execution identity and generation-bound revocation at the last safe pre-spawn point. Preserve uncertain ownership and capacity. A negative `is_revoked` bit from #369 is never sufficient launch authority.
3. **Unchanged behavioral gates:** original [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) actual post-revocation Git start and [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) bounded real stdout/stderr/stdin/EOF remain **RED**; do not modify their assertions, inflate delays, detach unsupervised pipes or release reservations without proof. Also require root + Job-empty + genuine EOF, crash/restart recovery and #185, #176/#182, #319, authorized Windows 10/11 physical tests and offline update/rollback #323. **WINDOWS V1 NO-GO; #317/#323 remain OPEN.**

---

## Immediate next-agent work order — 25 October 2026

Read [complete canonical handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-25.md) and [new-chat prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-25.md). Verify live refs before coding. Baseline before the docs PR: `main=7c72ecfb8886fb34be2cc7ef9f2e1070f73c08da`, [independent CI #40188090038](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40188090038) green 4/4; experimental #358 `f431c3ef…` with native evidence green on exact research SHA **not in main**.

1. **P0 #317 — trustworthy live MCP/native authorization seam.** Existing production typed scopes and `PolicyEngine::evaluate_native_git_read_preflight` are prerequisites only; live MCP `authorize_git_read` uses `task_lease: None` and `GitReadService` still calls `HardenedCommandRunner`. Prefer narrow test-first contract and adapter with real ActionEnvelope, sensitive Git authority, active registry-backed TaskLease, session admission, canonical repo, exact opened/hash-verified Git executable, one bound original revocation witness, pre-spawn native Job/owner and final stop check. Do NOT globally alter legacy GitRead permission before adapter is ready; do NOT wholesale merge research #353/#358.
2. **P0 acceptance #355 and #352.** Re-run existing original tests unmodified after actual candidate wiring: no real Git spawn after published revoke (specialized [#40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203) RED); true bounded stdout/stderr/**stdin** completion when descendants retain handles (specialized [#40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) RED). Never inflate timeouts, skip tests, fake EOF, leak pipes or release reservations early.
3. **P0 lifecycle/release #323.** Require root exit + native Job empty + both real EOFs and stdin writer completion; preserve TerminationUncertain, original Win32 errors, active admissions and quotas; external host-crash/guardian recovery, session close, sensitive reads/provenance #176/#182, #185 AppContainer, #319 first reply, authorized Windows 10/11 and offline installer/update/rollback acceptance. **Windows V1 NO-GO; #317/#323 OPEN.**

---


## Next P0 acceptance after native witness research #366 — 25 October 2026

- The latest **isolated** native owner study lives in [draft PR #358](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/358) at `f431c3efde2e5d9512eb1715ad3475196f18bf1d` after [research #366](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/366) was merged into #358 only. Its independently run CI is green: [general #40187179915](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179915), [native #40187179856](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179856), [private #40187179916](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179916), all first attempt. Production `main=288fa176251ac9038a8c6d1b8df74c6eac724d3c` is unchanged.
- The newest verified property is atomic registry verification of the *original* TaskLease + revoke witness, retained through native reservation. A later same-ID lease cannot make a revoked native owner spawn, and capacity is only released on its own never-spawned terminal receipt. This is **research evidence**, not MCP authorisation.
- **Next smallest production slice:** design a real MCP `ActionEnvelope`/sensitive-read/TaskLease authority adapter, with exact current session admission, canonical repository and verified Git executable, the #362 policy preflight, service-owned native Job and a final race-safe check immediately before kernel creation. Do not unconditionally demand a lease on the legacy GitRead runner before the adapter is complete. Reconcile #358 research with production #364 without duplicating native helpers.
- Run existing independent #355 real Git post-revoke gate and #352 stdout/stderr/stdin elapsed/EOF gate *unchanged*; then host crash/Job-empty/recovery, #185 Node/AppContainer and #323 Win10/11 installer/update/rollback. Green isolated owner tests never turn specialized RED acceptance green. **#317/#323 OPEN, V1 NO-GO.**

---

## Next P0 boundary — 25 October 2026 after #362

Code [#362](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/362) merged a *policy-only* `evaluate_native_git_read_preflight` on `main=739bdd5de3a86d72eaa251cb4f9f311e69b33e0e` with first-attempt full [PR CI #40182783889](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40182783889) 4/4. This checks GitRead action + exact TaskLease/session/epoch, root scope, FixedTool pin *scope* and finite watchdog envelope; it is **not** native authorization to spawn and is not called from live MCP yet. Independent [push/main #40183165673](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40183165673) retained separate #185 Windows Node/AppContainer RED; Ubuntu/dependency/release passed.

**Next smallest production step:** join the genuine MCP action/policy decision and active registry-backed TaskLease (including opened-and-hashed executable identity, canonical repository path and revoke watch) to a *service-owned* native reservation **before any process creation**, recheck immediately before spawn, preserve admissions/quotas on uncertainty. Prove #355 post-revoke no-spawn and #352 real wall-clock stdout/stderr **and stdin** independently; then prove root+Job-empty+EOF terminal, host-loss recovery, Win10/Win11 packaging/update/rollback. Research PRs #327/#352/#353/#355/#358 are not a bulk integration plan. **#317/#323 OPEN; Windows V1 NO-GO.**

---

## P0 sequencing — 25 October 2026 after five research PRs

**Verified production `main=d7d4d50adf95b6a3cf4a5df0ef06a6ba526d6810`**, independent [CI #40173338835](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40173338835) 4/4 green. Merged #356/#357 typed common GitRead command construction only; native owner is **NOT live**. #317/#323 remain open, V1 NO-GO.

1. **Behavioral acceptance RED:** [#352 dedicated CI #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) Windows/Ubuntu wall-clock ~1.6 s on 200 ms timeout for stdout/stderr/stdin; ordinary CI green does not supersede it. [#355 dedicated CI #40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203) Windows/Ubuntu Git spawned after published revoke; its ordinary Windows CI also red independently under #185 Node/AppContainer. Both must be corrected without suppressing/rerunning-to-green.
2. **Unmerged owners:** #353 head `b091dc28…` has staged Windows Job/pipes/watchdog/ledger, real typed Git status, counted admission, required GitRead capability+TaskLease/policy/executable and actual watchdog time-horizon revalidation, native terminal/EOF proof; [CI #40177026825](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177026825) and [native #40177026834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177026834) green. #358 head `59d1a331…` is **stacked onto #353, not main**; pins exact canonical GitRead repository to TaskLease in addition to generic Repository marker, [CI #40177299174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299174)/[native #40177299159](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299159) green on isolated head. #327 remains independent older research. No wholesale merge.
3. **Next smallest safe production seam:** bind real MCP GitRead policy/action/sensitive-read authorization, nonexpired scoped TaskLease, session/policy epoch, canonical root, exact Git exe, fixed typed builder, owned native admission/Job registered before child creation, and recheck under revocation immediately before spawn. If proof insufficient, first create narrow deterministic test/contract, not monolithic migration. Keep admission/quota/handles under native uncertainty.
4. **Later P0:** actual bounded EOF and cancellation for stdout/stderr **and stdin**, root+Job-empty receipts and guardian crash/restart reconciliation. Separate #176/#182, #185, #319, Windows 10/11/offline installer/update/rollback #323. Release remains NO-GO.

All earlier "latest" sections below are historical. [Detailed handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md), [complete successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md).

---


## Immediate engineering priorities — 25 October 2026

1. **P0 #317 — actual bounded completion:** [#352 dedicated acceptance CI #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) is intentionally RED on Windows and Ubuntu; descendant-retained stdout/stderr and stdin still cost ~1.6 s even though deadline is 200 ms and result is now `TimedOut` after #347. A future production fix must satisfy the actual <900ms test *without* weakening it, detached writers/readers, fake EOF, early resource releases or timeout inflation.
2. **P0 #317 — integrated trusted native owner:** [#353](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/353) has proof-of-concept Windows native Job/pipes/watchdog, admission/ledger/quota, pre-adoption cancellation and `GitRead` capability gate, full [CI #40153942601](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40153942601) and specialized [#40153942636](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40153942636) green. Still no actual `GitReadService` integration or full task lease, exact Git exe/repo provenance, stdin, service restart, persistent OS error.
3. **P0 #317 — avoid overclaiming:** [#350](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/350) fixes registered-job stop-before-drain only; [#349](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/349) preserves Windows native Job query error codes; [#347](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/347) fixes late-result classification, **not real pipe wait time**. Root exit, Job-empty and actual EOF are separate; retain session generation/admissions/quotas on uncertainty.
4. **Other independent gates:** GitRead sensitive authority/provenance #176/#182, Node/AppContainer #185 (preserve independent [main red #40084443275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40084443275)), first MCP response #319, host crash/recovery, Windows 10/11 field acceptance, installer/offline/release/rollback #323. Windows V1 **NO-GO**.

**Baseline:** `main=6a6457155758df3050e2aad81ee9446952d7eedd`, [push CI #40086602634](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40086602634) successful on exact main. At checkpoint, only PRs **#327/#352/#353** open, issues **#317/#323** open. All hashes are dated evidence and must be checked anew. See [latest handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) and [current successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md). Earlier roadmap sections are history.



## Next gates — 24 October 2026, after #349/#350

**Engineering code checkpoint:** `main=e0de79147e04daaea75cf59694d0e791b824fedc`. [PR #349](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/349) preserves genuine native Windows Job query errors; [PR #350](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/350) requests cancellation of **registered** session-owned process jobs before the admission-drain barrier, with a second sweep for late registration. Exact corrected-head full CI [#40085443930](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40085443930) 4/4 green; independent [push/main CI #40085754973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40085754973) **completed SUCCESS on `e0de7914` at first attempt, 4/4 jobs**.

1. **P0 #317 actual finite pipe/process lifetime:** replace direct-root-only `HardenedCommandRunner`'s blocking stdout/stderr/stdin joins with owned native/process-group supervision and cancellable bounded pipe EOF, holding admission/quotas until root and full Job/process tree are independently proven terminal. The prior #347 correction changes eventual results, not elapsed lifetime.
2. **P0 #317 late native registration and stop ownership:** prove process owner registered before any native spawn, stop observed during admission drain even if the job is not yet recorded, retained ownership on failed OS accounting, and true Job-empty/EOF receipts. Two cancellation sweeps (#350) are **not** sufficient for a guardian that holds an admission indefinitely without reacting to revoke. Never bulk-merge #327.
3. **P0 #317 durable and uncertain state:** preserve distinct OS observation phases, original Win32/HRESULT, no release of uncertain reservations, crash/guardian/restart reconciliation, bounded shutdown.
4. **P0 #176/#182** sensitive files and Git provenance; **P1 #185/#319** intermittent Node AppContainer and first MCP response; keep prior failed CI and compare distinct no-output vs post-network-attempt signatures.
5. **V1 #323 NO-GO:** real authorized Windows 10/11 acceptance, offline install/update/rollback and exact-integrated-main CI. No release tag or operator-PC command until explicitly authorized.

---

## Updated sequencing — 24 Oct 2026 after physical Windows 10 validation / Codex recovery procedure

**Release gate remains NO-GO:** #317 and #323 open. Production `main=dce94cc02f178d91b1eec4dc06134ab57df47ff6` after docs-only #343; confirm latest independent push/main CI [#40075855681](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40075855681) on its exact SHA. Earlier red Windows [#40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) (`TerminationUncertain` instead of `TimedOut`) must remain visible regardless of newer green research jobs. [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) is an independent unmerged owner experiment.

1. **P0 native uncertainty / owner authority (#317):** distinguish root wait, root exit + Job nonempty, query failure with preserved Win32 code, and proven root + Job-empty; demonstrate bounded negative observation, native owner retention, generation-bound `OwnedSessionAdmissionPermit` / `JobStore` quota, and out-of-band stop-before-draining admission. No automatic terminal publication or dropping owner when uncertain.
2. **P0 bounded pipes / stdin (#317):** design root+Job+all pipe owners as a single auditable lifetime. Invert the already demonstrated regression evidence from closed research [#339 stdout/stderr](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/339) and [#341 stdin/EOF](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/341) into *correctness* tests before a minimal reviewed production change. No detached readers/writers or timeout-only “clean” result.
3. **P0 owner loss / shutdown:** prove crash/guardian loss and restart reconciliation with true native processes and positive OS quiescence. Research #340 and physical Windows 10 #342 verified **normal paths**, not permanent observation failure. Preserve original failed main CI and all negative outcomes.
4. **P0 sensitive Git authorization (#176/#182):** repo/exe/provenance, task leases, secrets/aliases, deterministic GitRead policy after native owner contract proven. Do not wire the entirety of PR #327 into production.
5. **P1 startup / isolation (#185/#319):** real AppContainer/Node and first MCP response controls, kept separate from #317.
6. **V1 freeze (#323):** real Windows 10/11 acceptance, offline installer, update/rollback, exact-integrated-main CI and emergency procedures; V1 only GO after independently demonstrated security/availability gates.

**Operational (not production):** docs-only [PR #343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/343) added [Codex interruption recovery](docs/engineering/CODEX_INTERRUPTION_RECOVERY.md) to preserve checkpoints across interrupted assistant sessions. The procedure cannot override security blocks or survive same-disk failure by itself. Local snapshot reports are not remote CI evidence. Do not create competing handoff documents; use [this 24 Oct canonical handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) and [full successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md).

---


## Updated priority after ten hosted native crash scenarios — 24 Oct 2026

Latest researched and verified **unmerged** code: `research/317-failed-native-create-2026-10-24@b5a91dee`, exact [CI #40057931887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887) fully green with **ten** native externally supervised Windows scenarios. Production `main=16ca67a7` remained unchanged at this checkpoint; **#317/#323 OPEN, V1 NO-GO**. See [latest handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) for the research matrix, negative controls and semantics.

**Next engineering priority:** reproduce and characterize **descendant-held stdout/stderr pipe handles after parent exit**, bounded stdin closure/EOF and asynchronous stop-before-admission-drain under persistent native uncertainty. Keep the original guardian Job, generation-bound permit, JobStore quota and nonterminal status retained until independent native evidence; a timer cannot manufacture a terminal receipt. Prove host-loss/restart recovery or explicitly document why a persistent supervisor/ADR is required. Do not continue generating only near-identical 'suspended child/no-ACK' tests now that those controls are green. Only then extract one small independent production-safe increment from **current `main`** with exact-head Windows/Ubuntu gates; do not bulk merge research. GitRead task-lease/repo/exe and provenance (#176/#182), #185/#319, Windows 10/11 physical device and installer/release/rollback gates remain separate. GitHub-hosted only; Codex local blocked by controls is not a reason to bypass them.


## Current immediate sequencing — 24 October 2026

The [24 Oct canonical Windows V1 handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) supersedes the older 23 Oct research snapshots for the **current** work order. Production already has the *counted owned session admission* from #336, while research has separately proved JobStore commitment, kernel Job association at CreateProcessW, external synthetic creator-host death, combined stop-before-drain and **temporary injected** Win32 accounting uncertainty (latest exact-source [CI #40046563179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40046563179)). None of the combined native research is wired to GitRead in main.

**Next critical acceptance order:** (1) nonblocking stop publication distinct from terminal drain/reap with an accountable retained owner even under *persistently unavailable native observation*; (2) externally supervised Windows native descendant/stdio-EOF, creator/service death and quota/handle uncertainty tests; (3) only then one bounded production-owned read-only Git effect under approved lease/executable/repo provenance and security #176/#182; (4) CI #185/#319, Windows 10/11 devices, release/rollback acceptance. Keep issues #317/#323 OPEN and V1 NO-GO. No user-PC testing, new feature, broad research merge, or release by implication. Treat the 23 Oct sections below as history, and inspect GitHub live.

Reviewed: 2026-10-22. [STATUS.md](STATUS.md) owns current evidence; this page owns
the next delivery order. Earlier phase/H-track snapshots remain historical
planning inputs, not a second statement of current completion.

Execution mode for the current continuation: agent-owned isolated environment
and GitHub-hosted CI only; no operator-PC tests or runtime changes. See the
[handoff](docs/engineering/GITHUB_HANDOFF.md). This does not remove native Windows
acceptance gates or imply they can be established on Linux.

## Current integration priorities — 23 October 2026 after PR 334

Production `main` now includes the **read-only, generation-bound admitted stop
condition** (PR 334) in addition to PR 331's explicit revocation watcher.
The new witness observes the original admission expiry despite renewal; it is
**not** native process ownership, a launch permit or a completion receipt.
Use the [latest measured status](STATUS.md) instead of dated SHAs below.

The immediate next bounded increments are: (1) specify and test one immutable
session-generation-bound registered-owner commit under a shared close
linearization, with actual task/lease policy gates kept independent; (2) ensure
stop request reaches already admitted workers **before** any synchronous permit
drain, including late registration and uncertain Win32 observation; (3) prove
Windows process tree membership **at creation** and full stdout/stderr/stdin,
host crash, deadline and quota retention under an independent native
supervisor; (4) only then migrate one sealed read-only Git path with safe
publication/provenance. Keep legacy session/approval/lifecycle report contracts
until their callers are reviewed. Preserve the real concurrent reaper/ABA
negative control as a separate generation-isolation acceptance case, while
distinguishing forced handle reuse from supported provisioning.

The V1 decision remains **NO-GO** under issues 317/323. Also preserve
sensitive-content/provenance review (176/182), native Windows intermittency
(185/319), real Windows 10/11 operator-authorized testing, release rollback
and offline installer/PowerShell checks. Hosted green tests alone do not
satisfy this matrix. No GitHub ruleset or user-PC changes in this checkpoint.

## Current Windows V1 delivery order — 23 October 2026

[Read current handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-23.md) and [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). **Stabilization, not new features:** #317 remains the foremost Windows reliability blocker, but the new `ReadServiceRoot` research at `1c7dac9` has progressed past inert owner fixtures with real shared JobStore, native cleanup and seal/reap tests (CI #39888522519 + native #39888522200). Do **not** claim production repair: last-known `main=3fe3de0c` still runs HardenedCommandRunner, and PR #327 remains open with a real #185 red run. The new **host-crash-before-Job-assignment** reproduction leaves a suspended child until independent outer supervisor cleans it; revocation admission only protects submit; Drop may wait forever on persistent native observation errors.

**Next:** make an explicit architecture/revoke/shutdown responsibility decision; then prove child containment across host crash and stop-versus-resume linearization; then verified Git authorizer/pins/leases/deadlines and ONE read-only Git path, with #182 patch-provenance policy separately approved. #185/#319 native intermittency, real Windows10/11 support, packaging/recovery and release provenance remain gates. No Electron, remote-project scope or V1 release without owner consent.

## Current Windows V1 stabilization sequencing — handoff 22 October 2026

Owner directive: V1 is a future **release decision following stabilization of existing Rust/MCP code**, not an automatic result of green CI. [Current handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-22.md) is the continuity point; [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) holds the evidence-based V1 freeze/no-go matrix. The highest-priority release blocker is #317: a Windows service-owned Git command process tree, robust stdin/stdout/stderr cleanup, owner-retained uncertainty and session seal/reap must be proven on the *production* runner. The research branch has 13 supervised native cases, real SessionRegistry/Job tests and a **non-wired prototype ledger**, none merged to main. **Next:** review/prove independent native observation and stop (avoid callbacks under a global lock), then staged Windows owned process and session integration before any GitRead migration. No wholesale research merge.

After #317, resolve or fail-closed exclude unproven V1-reachable parts of #176/#182, #319 and #185; certify supported Windows 10/11/PowerShell combinations with operator-approved real-device tests, and prove reproducible release/rollback/CI. Out-of-scope remote H-01–H-05/#177 and Electron remain deferred with enforced exclusions. No V1 tag or feature unfreeze without operator approval.

## Windows V1 first — operator decision, 22 October 2026

Target Windows10/11; verify actual supported versions rather than inferring them
from hosted Windows Server. Prioritize demonstrated Windows command ownership317, keeping sensitive-read
176/182 gates explicit, then MCP startup319, AppContainer/Node185 and proven
recovery/authorization/resource and installation defects. Keep these distinct; doctor error-path diagnostics do not
explain natural startup latency. Linux functional support is post-V1. Do not start
new cgroup/pidfd/PDEATHSIG/confinement research unless shared-code regression
actually blocks Windows; retain existing Ubuntu CI as non-regression checks.
Rust/MCP remains core; Electron can resume only after the relevant foundation
gates and a delivery decision, never as a mandatory Rust runtime dependency.

## Temporary stabilization gate (operator directive, 2026-10-20)

**The feature roadmap below is paused, not cancelled.** Do not start new
features, optional integrations, GUI work, broad refactors or experimental
alpha-to-main merges. Prioritize reproducible defects in the existing
permission/isolation/recovery/process boundaries, reliable diagnostics for
intermittent hosted Windows gates, targeted regressions and documentation
accuracy. A corrective PR requires a demonstrated failure and exact-head
native checks where applicable. If no defect is demonstrated, do not create
a change merely to show activity. Feature work resumes only on a new operator
decision. This freeze does not relax any existing security or CI requirement.

## Stabilization exit criteria — objective gates, not a claim of zero bugs

Two **distinct** decisions are required:

1. **Resume bounded headless development** only after an exact-head Windows/Linux CI baseline and targeted safety checks for any newly exposed effect; the known Windows AppContainer/Node and MCP first-response flakes must be explained, corrected, or explicitly excluded from that narrowly permitted workflow **without weakening a security test**. The operator must explicitly authorize lifting the feature freeze for this limited scope.
2. **Enable real-project remote/autonomous work** only after deterministic classification/enforcement of sensitive filesystem content/list metadata ([#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)) and a matching GitRead authority decision ([#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182)); authenticated principal, session and host correlation proof (H-01–H-03), local approval/revocation contracts (H-04/H-05), process/namespace and recovery boundaries, and appropriately reproducible soak/fault-injection evidence. A green short CI is insufficient.

**Before either decision:** retain actual red-run evidence, avoid speculative timeout increases and redundant PRs, explicitly classify partial Windows power-loss/ancestor/race guarantees, and reconcile code review coverage and handoff docs with the exact `main` SHA. Existing documentation and PR status inconsistencies are a real continuity risk, not evidence of a new runtime defect. Owner approval is required before changing GitHub branch-protection requirements; the last GitHub query reported `main.protected=false` and no rulesets.

## Deferred feature delivery sequence (not currently authorized)

1. **Maintain coherent integration.** Repository consolidation and bounded
   diagnostic scopes are recorded; keep `main` and the generated-project alpha
   aligned through reviewed increments. Track compatibility fixes as delivery
   gates, close completed PRs and retire only preserved exact branch heads.
2. **Prototype optional Cargo preparation on disposable Windows.** Prove the
   fixed NUL/exact-profile contract, trusted local broker identity, revocation and
   interrupted recovery before presenting a concrete administrator change on a
   user's PC. Keep project code and the model unelevated.
3. **Complete an authorized writable build environment.** Define truthful storage
   enforcement, owned-tree stop/quiescence, crash evidence and artifact
   publication. Input-copy and post-exit collection ceilings are not disk quotas.
4. **Add locally selected project and dependency workflows.** Support empty or
   existing workspaces with explicit enrollment, permissions and restore/egress
   policy. Prefer a goal-driven Node or Rust application test over another fixed
   UI exercise once the relevant gates pass.
5. **Close GUI compatibility separately.** Investigate Electron's normal startup
   failure without silently disabling its sandbox. Do not block all headless
   Node/Rust progress on that investigation.
6. **Productize the operator experience.** Add a visual permissions/project/log
   interface over the same validated services. Conversation resume uses retained
   project/diagnostic state; it cannot recreate a host's hidden model state.

Detailed design references:

- [Product charter](docs/product/PRODUCT_CHARTER.md)
- [Scope and long-term requirements](docs/product/SCOPE_AND_ROADMAP.md)
- [Security invariants](docs/security/SECURITY_INVARIANTS.md)
- [Repository workflow](docs/engineering/REPOSITORY_WORKFLOW.md)

The [generated alpha's delivery index](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/alpha/generated-project-editing/docs/product/REAL_PROJECT_DELIVERY.md)
contains its adapter-specific operational boundaries. Historical phase and
reusable-approval plans remain useful references, but do not override the
current evidence and acceptance order above.
