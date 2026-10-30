# Windows V1 — native GitRead authority and launch race audit (#323)

## 30 October — failed watchdog completion with an actual suspended root

Extend the existing private Job-list fixture with explicit scenario variants,
preserving the original and preterminated-empty-Job cases. Two new cases create
the same fixed system cmd.exe CREATE_SUSPENDED/CREATE_NO_WINDOW with kernel
Job-list membership at creation, inherit no handles and never call ResumeThread.
The original Job still has its existing one-process/256MiB/20%CPU limits.
Before any failed stop, require actual membership, active1 and original root
handle WAIT_TIMEOUT at zero wait.

The actual NativeJobDeadline receives a QUERY-only noninheritable duplicate of
that original Job, leaving the original full handle/ACL unchanged. Its real
worker must finish after original stop or25ms expiry and retain ACCESS_DENIED.
After joining/dropping that failed worker, require the same error receipt, both
original/query-only kernel counts1 and the same original root handle still
unsignaled. Thus a finished failed watchdog demonstrably is not root exit or
Job-empty while actual native work exists. No mocked API or invalid handle.

Only THEN use the trusted fixture's full original Job termination, require root
handle signal within the existing5s wait and separately query kernel active0
within the existing5s bound. Failure RAII remains a last-resort cleanup and
cannot supply the passing observations. This is explicit external-fixture
cleanup after failure, not implemented runtime uncertainty recovery. No root is
resumed, no descendant or pipe EOF is observed, and no counted runtime owner is
attached to this synthetic root. The earlier no-child runtime capacity proof
and this real-root proof remain separate; do not compose them into live MCP
acceptance. No product launch API, grant, timeout, retry, security protection or
release/native Git activation changes. Local Clippy compiles all tests; actual
new process execution is hosted Windows only. Require both new cases and both
original cases in exact-head/main Windows logs, all applicable checks, review,
authorized squash and independent-main receipt. Windows V1 NO-GO.

## 30 October — propagate the exact termination receipt and exercise native denial

LeaseObservedNativeGitStatusCustody now forwards the retained original Job
watchdog result: None is pending/unarmed/cancelled without an attempt; Ok or
Err(HRESULT) describes the completed native call only. It neither consumes nor
releases its registered record, original session admission, transport, pins,
Job or streams. Existing original session/lease-reused-ID/Job-stop and absolute
service-clock expiry fixtures additionally require actual native success.
The stop fixture checks transport and registered Job counts remain1 before
owner drop and return0 afterward. No live MCP native launcher is enabled.

A new Windows no-child test duplicates a real bounded empty original Job with
only JOB_OBJECT_QUERY (0x0004), noninheritable, without DUPLICATE_SAME_ACCESS.
The real NativeJobDeadline duplicates THAT restricted handle, observes an
already-published stop or its own25ms deadline, calls TerminateJobObject, returns
and publishes the actual ACCESS_DENIED HRESULT. The receipt must remain Err
after worker drop/join. Separate kernel queries through both original and
query-only handles prove the same empty Job remains usable. This is a native
access-right denial, not a mocked API, invalid handle, ACL change or product
configuration. Microsoft's [Job Object Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/procthread/job-object-security-and-access-rights)
requires JOB_OBJECT_TERMINATE for termination; the fixture deliberately withholds
that right only from its own extra duplicate. Production duplication is unchanged.

First local compilation failed because JOB_OBJECT_QUERY is exported under an
unselected SystemServices binding feature. Preserve the log; the fixture uses
the documented local mask without adding a feature/dependency. Five Windows
focused tests pass after correction; hosted exact-head evidence still required.

Limits: only empty Jobs are used. No active root survives a failed stop, no
uncertain process accounting/recovery, worker panic/stalled native-call bound,
shared host/policy/session/lease create-resume fence, descendant EOF or installed
Windows matrix is proven. A completed worker may hold Err; a successful receipt
still cannot authorize future creation (#439) or stand in for physical terminal
proof. This increment exposes the distinction to the actual inert runtime owner;
it does not make uncertain native Git execution safe. V1 NO-GO, no release.

## 30 October — distinguish watchdog completion from native termination result

The existing watchdog discarded the Windows result of TerminateJobObject.
Retain one immutable result after that exact original-Job call returns, using
OnceLock shared with the worker. The custody accessor returns None until a
result is published, Some(Ok(())) after native success or Some(Err(HRESULT))
after failure. No error is replaced with a successful stop receipt. This also
preserves unknown/pending state if the call has not returned; it supplies no
bound on a stalled Win32 call. Drop still cancels and joins before Job closure.

The existing real empty-Job deadline test now waits separately for worker
completion and requires actual native success. A new original-stop test requires
success without expiry, and a cancellation test joins an unexpired no-stop
watchdog and requires NO result. These are hosted Windows no-child fixtures;
no injected termination-failure execution or live Git/root/descendant proof is
claimed. Local formatting/diff checks are not hosted behavioral evidence.

Only observability changes: polling, original stop flags, immutable horizon,
termination invocation, ownership, limits and cleanup order remain unchanged.
The AppContainer watchdog shares the recording mechanism but its behavior and
acceptance are unchanged. HRESULT success is not kernel Job-empty, root signal,
stream EOF or an authority fence; #439 explicitly shows later creation remains
possible on a terminated retained Job. This does not implement uncertain-stop
recovery, a service-wide issuer or future create/resume permission. Native Git
remains disabled, Windows V1 NO-GO. Require exact-head applicable hosted CI,
review, authorized squash and independent push/main before claiming integration.

## 30 October — empty Job termination does not seal later creation (#439)

Before treating a native stop/empty receipt as a launch/revoke boundary, test its
actual kernel scope. [TerminateJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject)
terminates currently associated processes; its API contract is not a shared
application authority issuer. The new fixture observes this sequential history:

1. Create the original bounded Job (one process,256MiB,20% CPU), observe active0,
   successfully terminate that empty Job, and observe active0 again.
2. Keep its original handle and limits unchanged. Create only the fixed system
   cmd.exe with CREATE_SUSPENDED/CREATE_NO_WINDOW and the original kernel
   PROC_THREAD_ATTRIBUTE_JOB_LIST; inherit no handles and never call ResumeThread.
3. Require IsProcessInJob true, original Job active1 and retained root handle
   WAIT_TIMEOUT at zero wait. Thus prior successful termination did not prohibit
   this later suspended creation or immediately terminate its root.
4. Terminate the original Job again, require root handle signaled within the
   existing5s wait and separately query kernel active0 within the existing5s bound.
   Original RAII failure cleanup remains; it cannot supply a passing observation.

The original no-pretermination test and this mode use one private helper. First
exact source2ba610ea926e035404af0854384fc5d83b0f35f8,
[CI41063470571](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41063470571)
SUCCESS4/4 attempt1, Windows job124088771167 explicitly passes BOTH tests. Same-head
[Owned Native Process Pipes41063470344](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41063470344),
[Private Rust Compilation41063470166](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41063470166),
and [Private Project Preparation41063470401](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41063470401)
also pass. These workflows validate their own applicable compiled-source scopes;
the explicit new-case execution is established by the ordinary Windows test log.
Final exact-head and independent-main receipts remain required in #439/#323.

Design implication: stop publication/termination/Job-empty alone cannot substitute
for an issuer that controls subsequent create/resume intentions and retains
ownership of in-flight Win32 work. This fixture does NOT exercise a concurrently
blocked CreateProcessW/ResumeThread, an armed/expired original watchdog, session/
TaskLease/policy/host ordering, a resumed root, descendants, stream EOF, native
uncertainty or installed Windows10/11. It is not evidence of an enabled product
bypass; no service launch API or live MCP GitRead is changed. No new retry loop,
lock-across-Win32 scheme, deadline reset or guardian is supplied. Keep those
separate design/proof obligations and all earlier failures open. Windows V1 NO-GO.

## 28 October 2026 — original lease stop before admission drainage

Baseline `main=eea5438873293243cd747b0f646ed903fb9f1282` integrates [#420](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/420); independent [push/main #40752232971](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40752232971) verified SUCCESS 4/4, identical tested/integrated trees.

The next targeted regression holds a genuine counted session admission during lifecycle revoke. Original session stop is already published before drainage, but the original TaskLease stop is currently published only afterward. A lease-only observer can therefore wait for a stop that depends on its own admission being released. The test-first candidate preserves that order to obtain behavioral RED.

Required correction: publish irreversible lease revoke flags before waiting for session admission drainage, retain the lease records/pins and counted resources, then repeat the lease sweep after drainage to catch registrations by already-admitted producers. Sum newly revoked counts from both sweeps without double counting; attempt all cleanup and propagate errors. This is stop publication, **not** destruction or physical process termination.

The real-registry test checks both original stop signals while the permit is held, failed early removal due to in-flight admission, revoke still waiting, a late registration caught after drain, exact revoke counts and eventual no-child reaping. No process is created; no policy/network/timeout relaxation. Final corrected-head and independent-main evidence must be checked before claiming integration. **Windows V1 NO-GO.**

See the new [current continuation](WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md) (from engineering pages: `WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md`) for the next host/policy authority boundary and exact remote-check procedure.



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



### Preserved first-head Windows failure and validation strengthening

First HEAD `4e57f565205b35ecdf23725bb36b19ed9ec9688b`: [CI #40746209363](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40746209363) failed its original Windows workspace test in `capability_free_appcontainer_denies_node_loopback_connection`: TimedOut, exit 1, 10053 ms, no Node stdout/stderr and no observed connection attempt. The later three built-in serialized diagnostic comparisons passed with launcher phases and actual NETWORK_ATTEMPT/NETWORK_DENIED; they do **not** clear the original failure or establish its cause (#185 remains open).

The ordinary workspace test now enables the existing fixed launcher phase opt-in on Windows, so a future **original cold/parallel** failure contains phase evidence, not only later comparisons. The specialized Windows workflow also runs the actual TaskLease clock and inert native status test groups before unrelated Node proofs, so an earlier workspace abort cannot leave the new Windows tests unexecuted. No timeout, scheduling, network grant, sandbox, assertion or test acceptance is weakened. No unchanged-SHA retry was requested. A green later HEAD is evidence for that source only, never a #185 fix. Root cause remains unproved and Windows V1 NO-GO.

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

## 28 October 2026 — #410 physical descendant-held pipe/Job proof (NO-GO)

**Merged research-only test:** [PR #410](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/410) at `main=20ed7887715601e260cc6f0fe9e73c18a1b3170e`. Exact final PR HEAD `99ba2e5d773bb82ad0ac6b3f39142055b1a3cb70` passed [CI #40679470136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40679470136) **completed/success, 4/4 jobs**. Windows job `122786478971` logged `job::job_list_launch_tests::descendant_keeps_both_pipes_open_after_root_exit_until_job_termination ... ok`. Three exact-head hosted Windows workflows [Owned Native Process Pipes #40679470081](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40679470081), [Private Rust Compilation #40679470074](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40679470074) and [Private Project Preparation #40679470063](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40679470063) passed. Independent post-merge [push/main CI #40680226256](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40680226256) was **in progress when this checkpoint was written**; recheck its final exact-SHA, four-job outcome.

**Physical scope:** a fixed `cmd.exe` root starts suspended already inside a private kernel Job, with both `PROC_THREAD_ATTRIBUTE_JOB_LIST` and a three-handle stdio inheritance list. It starts one fixed sleeping Windows PowerShell descendant using `START /B` and exits with code 0 while Job accounting still sees a live descendant and **neither stdout nor stderr has physical EOF**. Explicit `TerminateJobObject` then results in independently observed bounded Job-empty, stdout EOF and stderr EOF. No user project, actual GitRead, network, production launch owner or MCP change; one Rust Windows test only. The first candidate CI failed rustfmt; it was corrected before the successful final HEAD.

**Still blocked (P0):** the MCP `git_status` implementation continues using `HardenedCommandRunner` with potentially unbounded blocking reader joins. No single synchronized session + original TaskLease + policy epoch + host shutdown native launch/revoke issuer exists; no production Win32 guardian, fail-closed owner-loss recovery, real native Git status/descendant budget/timeout acceptance, or validated Windows 10/11 device campaign exists. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) stays **OPEN — Windows V1 NO-GO**; #185 and #319 remain separate blockers. The next engineering PR must target genuine native ownership/authority, not activate the service from a synthetic witness. No releases, other repositories, self-hosted runners or user-PC operations.

**Source inspection: 28 October 2026.** Baseline: `main=be59544537d5dc027b54e4bb5cd7f553f979d59d` after [research test PR #406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/406). This is a **design/audit checkpoint, not a merged native spawn authority contract**. Recheck code and exact-head CI before implementing any launcher. All new ideas below are requirements to prove, not existing behavior.

## Further physical evidence #408 — host/fence still missing

**Integrated technical test:** [PR #408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/408) tested exact HEAD `e346a0b6f219ca55a360b292879b4f16468e8c01` with [CI #40675433145](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433145) **completed/SUCCESS, 4/4 jobs, attempt 1**. Hosted Windows job `122772868039` logged `job::job_list_launch_tests::resumed_job_root_produces_two_physical_pipe_eofs ... ok`. The three specialized hosted Windows workflows [#40675433176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433176), [#40675433141](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433141), [#40675433144](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433144) also succeeded on the exact HEAD. Squash-integrated `main=fb10d7e98664a6b4deda2c8b509bdf1048672618`, tree unchanged `b65ff73a7351bac620256fb923457062cc4bfed0`. Independent post-merge [push/main CI #40676169755](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40676169755) was triggered for this precise SHA; **check all four final jobs live**, do not equate the PR checks to that independent CI.

**Exactly one Rust Windows test source**, no production GitRead or MCP changes. A fixed system `cmd.exe /D /Q /C` echo fixture is created **suspended** with kernel `PROC_THREAD_ATTRIBUTE_JOB_LIST` AND explicit inherited-only `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, `NUL` stdin and separate bounded pipes. The test verifies Job membership before `ResumeThread`, real root exit code `0`, separate kernel Job-empty, **physical EOF of BOTH stdout and stderr**, and distinct child output markers. The original candidate's first CI failed formatting only; corrected on final HEAD. No user-controlled project, real Git, external network, or production owner is involved.

**Critical nonclaims:** no actual service-owned GitRead process, descendant-retained writers or bounded timeout with descendants, native owner/Job uncertainty recovery, shared session/TaskLease/policy/host commit/stop ordering or safe Win32-call stall handling. `HardenedCommandRunner` remains live for MCP GitRead with known potentially unbounded reader joins. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is administratively closed, not behavioral acceptance. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN: Windows V1 NO-GO**; #185 and #319 remain separate blockers. [Native authority audit](ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md) is still the required next technical design/proof. No release or PC changes.

## What is physically proved

- [#393](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/393) established that a Windows synthetic root is already in its configured kernel Job at `CreateProcessW` return while **suspended, never resumed**.
- [#396](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/396) established that `CreateProcessW` for a missing image returns `ERROR_FILE_NOT_FOUND` with no active process in the Job.
- [#406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/406) now adds **one synthetic resumed root**: system `cmd.exe /D /Q /C exit /b 0`, no handle inheritance or user input, Job-list assignment at creation, membership checked before `ResumeThread`, real bounded root wait and exit code 0, separately observed kernel Job empty within a 5-second bound. Exact-head [CI #40672893075](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893075) 4/4 SUCCESS; hosted Windows log shows `job::job_list_launch_tests::resumed_root_exits_and_kernel_job_drains_after_job_list_creation ... ok`. Same-head Windows [Owned Native Process Pipes #40672892946](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672892946), [Private Rust Compilation #40672893056](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893056) and [Private Project Preparation #40672893094](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893094) SUCCESS.
- #406 is **not** a native GitRead, a process descendant, a pipe EOF, a guardian lifetime, or a runtime revoke/launch fence. Its first CI HEAD failed **rustfmt only**; the corrected final head is `f4c7b8163d36434f5a26486625d4963c42a0dee8`. Squash `main=be59544537d5dc027b54e4bb5cd7f553f979d59d`, identical tree `f4d702915e429821bd658d6317014e0cd316241a`.

## Current mutation and lock map (observed, not a complete formal proof)

| Authority / owner | Observed mutation / observation | What remains unfenced |
| --- | --- | --- |
| Session | `crates/optic-bridge-runtime/src/session/registry.rs`: `sessions: Mutex<...>`; `revoke` and `publish_revocation_before_drain` set `record.revoked` and original `Arc<AtomicBool>` stop bit. `commit_registered_native_owner_session` mints a **session-only** receipt under this mutex; `revoke` waits for counted admission drain through a condvar. | This mutex is **released** before any OS call. A successful receipt does not prohibit a later session close before process creation or resume. |
| TaskLease | `crates/optic-bridge-runtime/src/task_lease_registry.rs`: **different** `leases: Mutex<...>`; `revoke`, `revoke_session`, `remove_session` publish stop for the exact original generation. `commit_original_native_read_lease` mints a **lease-only** receipt under the same lease mutex; preserves admission-time expiry and original watch identity against ID reuse/ABA. | There is no shared commit lock with sessions, policy/host or `CreateProcessW`. Two sequential receipts are not one atomic launcher permit. |
| Typed policy / epoch | `crates/optic-bridge-policy/src/lib.rs`: `PolicyEngine` is stateless, checking envelope/session/lease `policy_epoch` equality and scopes; native preflight is an observation. `crates/optic-bridge-app/src/main.rs` loads a policy epoch from configuration into the provisioned session. PolicyChange effects are denied by the policy engine. | Source inspection did **not** identify a live mutable issuer/fence whose epoch changes atomically with native launch and shutdown. **Do not assert that an in-process policy mutation currently exists**; define how future reload/changed policy invalidates an outstanding native owner instead of assuming static equality is a live revocation. |
| Host / transport loss | `crates/optic-bridge-app/src/main.rs`: ordinary service completion and returned MCP initialize/wait errors invoke `SessionLifecycleManager::revoke` on the original session with `spawn_blocking`, independently of renewal. `session/lifecycle.rs` publishes session and lease stop, requests process cancellation, drains admission, then repeats lease/process sweeps. | Future abortion and abrupt OS exit are outside this returned-result boundary; legacy admitted IO may still prevent drainage. No independent native `HostStartup`/service-root launch issuer is held by the inert native GitRead owner; transport exit is not an atomic CreateProcess/ResumeThread boundary or physical termination proof. |
| Job owner / stop | `crates/optic-bridge-runtime/src/process.rs`: private JobStore registration and finite quotas precede native custody; `cancel_session` sets per-job `stop_requested` under the JobStore and JobState mutexes. `NativeProcessReservation::drop` records `TerminationUncertain` if `may_have_run`, not successful EOF/termination. | A stop **flag** is not a kernel `TerminateJobObject` receipt. The proposed real native guardian must retain Job, permits, pins and quotas across OS calls, errors, caller abandonment and uncertainty until physical termination/stream proof. |
| Native candidate | `git/native_status.rs`: `PreparedNativeGitStatus` → `RegisteredNativeGitStatus` → `PrecreatedNativeGitStatusCustody` → `SessionCommittedNativeGitStatusCustody` → `LeaseObservedNativeGitStatusCustody`; no native spawn method. `observe_original_prelaunch_snapshot` repeats session lock observations around a lease/policy snapshot. | No final issuer, `CreateProcessW`, `ResumeThread`, guardian or MCP switch consumes this candidate. `git_status` still takes the legacy `HardenedCommandRunner` path. |

## P0 correctness conditions for the next **technical** PR

1. Specify a **single linearization/ownership contract** for the actual irreversible effect, including session, exact original TaskLease, policy epoch, host shutdown, deadline, target identity, Job record, and private guardian. Choose and document what it means for revoke vs. launch to "win". A later positive snapshot cannot redeem an earlier generation.
2. Never hold SessionRegistry, TaskLeaseRegistry, JobStore or proposed issuer mutex across disk I/O, waiting, `CreateProcessW`, `ResumeThread`, pipe reads or termination. Avoid lock-order cycles: lifecycle operates session → request Job stop → session drain → lease/approval cleanup → second Job sweep, but these are **separate** critical sections.
3. For a native process created **suspended with Job-at-create**, require a trusted owner to retain both process/thread handles and a durable stop latch through creation, final checks, resume or forced terminate. Revocation must remain publishable even if a Win32 call stalls. A process created after revoke must never run or be published as authorized. The stronger guarantee must be demonstrated with barriers and real OS observation, not inferred from two independent receipt `Ok` results.
4. Fail closed on failed/partial native creation and owner loss. Once an effect **may** have run, do not drop native Job/permits or mark streams EOF without physical evidence. Explicitly state the recovery path for uncertain ownership and a bounded watchdog independent of the caller future.
5. Prove real native Git `status` in a disposable repository, its descendants if any, independent **root exit vs Job-empty** predicates, separately observed **stdout and stderr EOF** (and stdin shutdown), real wall-clock bound even when a descendant retains a writer, timeout/stop after host/session/lease revoke, quota retention and failures. Run on hosted Windows; later platform signoff requires owner-approved Windows 10/11 real-device acceptance.
6. Only after the above passes may an explicit reviewed PR migrate MCP `git_status` away from `HardenedCommandRunner`; preserve current sensitive-read denials, capability/lease scopes, tool command shape and budgets.

## Nonclaims, disposition and release gate

[#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN — Windows V1 NO-GO**. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is **closed administratively**, yet the legacy production pipe-reader join elapsed-time defect is not thereby proven fixed; [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) is closed/unmerged and [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) remains research. [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) and [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) remain separate acceptance blockers. No release, no GitRead production switch, no user PC, Desktop, REL or self-hosted runner work is authorized by this audit.
