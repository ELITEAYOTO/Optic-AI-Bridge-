Warning: truncated output (original token count: 48880)
Total output lines: 1188

# Current project status

## 29 October 2026 — original session horizon during stdio initialization

PR #424 bounds MCP initialization by the remaining original session lifetime, including time already spent in startup. Pre-initialize traffic does not renew that horizon; local session renewal still begins only after initialization. Already-expired or late-ready initialization fails closed and follows the existing original-session cleanup, which publishes session/lease stop before waiting for counted admissions. This does not bound admission drainage or fix the legacy process/pipe joins.

Real RMCP duplex regressions reproduced behavioral RED on Windows and Ubuntu at 14a9d1c3722fcc7312f54acf08181cbf8c10baa1, CI #40848166754: idle initialization outlived the horizon and expiry did not publish stop. They cover default/supervised idle initialization and retained admission during cleanup. The correction keeps the original deadline, error path, other-session independence and resource ownership. Exact-head and independent-main validation receipts are recorded in #424 and #323; this description alone is not CI acceptance.

No native GitRead launch or global host/policy revocation fence is added. Early setup errors, abrupt task/process interruption and physical child/Job/EOF proof remain separate requirements. Windows V1 stays NO-GO; no release. Local work remains lightweight; full validation uses hosted Windows/Ubuntu.

## 29 October 2026 — original stdio session close

#421 is integrated at `efb3fc28a47fba136ea912381fc9d1cc5fe1d08c`, independent
[main CI #40754598941](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40754598941) SUCCESS 4/4.
The next PR closes the original session on ordinary service EOF and returned MCP
initialize errors, independently of renewal. Local Windows regression and binary
tests pass; exact-head hosted and independent post-merge CI remain delivery gates.
See [current scope and limits](docs/engineering/WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md).
**Windows V1 NO-GO**; no physical Git termination or global host/policy fence claimed.

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

## 28 October 2026 — #408 real stdout/stderr EOF on fixed Windows root

**Integrated technical test:** [PR #408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/408) tested exact HEAD `e346a0b6f219ca55a360b292879b4f16468e8c01` with [CI #40675433145](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433145) **completed/SUCCESS, 4/4 jobs, attempt 1**. Hosted Windows job `122772868039` logged `job::job_list_launch_tests::resumed_job_root_produces_two_physical_pipe_eofs ... ok`. The three specialized hosted Windows workflows [#40675433176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433176), [#40675433141](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433141), [#40675433144](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433144) also succeeded on the exact HEAD. Squash-integrated `main=fb10d7e98664a6b4deda2c8b509bdf1048672618`, tree unchanged `b65ff73a7351bac620256fb923457062cc4bfed0`. Independent post-merge [push/main CI #40676169755](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40676169755) was triggered for this precise SHA; **check all four final jobs live**, do not equate the PR checks to that independent CI.

**Exactly one Rust Windows test source**, no production GitRead or MCP changes. A fixed system `cmd.exe /D /Q /C` echo fixture is created **suspended** with kernel `PROC_THREAD_ATTRIBUTE_JOB_LIST` AND explicit inherited-only `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, `NUL` stdin and separate bounded pipes. The test verifies Job membership before `ResumeThread`, real root exit code `0`, separate kernel Job-empty, **physical EOF of BOTH stdout and stderr**, and distinct child output markers. The original candidate's first CI failed formatting only; corrected on final HEAD. No user-controlled project, real Git, external network, or production owner is involved.

**Critical nonclaims:** no actual service-owned GitRead process, descendant-retained writers or bounded timeout with descendants, native owner/Job uncertainty recovery, shared session/TaskLease/policy/host commit/stop ordering or safe Win32-call stall handling. `HardenedCommandRunner` remains live for MCP GitRead with known potentially unbounded reader joins. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is administratively closed, not behavioral acceptance. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN: Windows V1 NO-GO**; #185 and #319 remain separate blockers. [Native authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md) is still the required next technical design/proof. No release or PC changes.

## LATEST VERIFIED STATUS — 28 OCTOBER 2026, Windows Job resumed-root proof #406

**Verified integration:** [PR #406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/406) is **squash-merged**. Tested final HEAD `f4c7b8163d36434f5a26486625d4963c42a0dee8`, integrated `main=be59544537d5dc027b54e4bb5cd7f553f979d59d`, identical trees `f4d702915e429821bd658d6317014e0cd316241a`. [PR CI #40672893075](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893075): **4/4 jobs SUCCESS, first attempt**, exact final HEAD. Same-HEAD hosted Windows workflows [Owned Native Process Pipes #40672892946](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672892946), [Private Rust Compilation #40672893056](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893056), and [Private Project Preparation #40672893094](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893094): all SUCCESS. Independently triggered [push/main #40673602054](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40673602054) on the exact integrated SHA; **verify its completed/final four job conclusions live** before using it as post-merge proof.

**One test-only Windows file**, `crates/optic-bridge-windows/src/job_list_launch_tests.rs`: `job::job_list_launch_tests::resumed_root_exits_and_kernel_job_drains_after_job_list_creation ... ok` in hosted Windows Rust log. A fixed synthetic Windows `cmd.exe /D /Q /C exit /b 0` is created suspended, already Job-associated via `PROC_THREAD_ATTRIBUTE_JOB_LIST` at creation, then actually resumed; root exit code 0 and **separate physical Job-empty** observation are bounded. Original PR CI failed `rustfmt` on an earlier HEAD, corrected and preserved as a formatting failure, **not** a launch/liveness failure.

**NOT demonstrated:** no service-owned native GitRead launch/MCP switch; no descendants, stdout/stderr EOF, independent host/session/TaskLease/policy cross-authority final commit, Win32 blocking stop, uncertainty guardian, or legacy runner elapsed-time correction. Sessions and leases still issue independent receipts under different mutexes. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN**; [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) closed administratively, original pipe liveness not shown resolved; #185/#319 open. **Windows V1 remains NO-GO; no release.** Follow the concrete mutator/lock/host audit in [#323 native authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md); do not confuse static policy epoch equality with a live cross-authority issuer. Previous dated sections are historical snapshots.

## LATEST VERIFIED STATUS — 28 OCTOBER 2026, dual-receipt close #404

[PR #404](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/404) : HEAD testé `cc7a77b3b5415c6b8d601c477c2373e39f84f7b6`, squash intégré `main=0d14502da39d90ccecd8f113981cd3bdb6520159`, arbre source/intégré identique `55bd4edc12e645ddc3e27a1cb799db932208cec0`. CI [PR #40666361354](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361354) **4/4 SUCCESS sur HEAD corrigé**, [native #40666361300](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361300) SUCCESS, et **CI indépendante push/main [#40667115232](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40667115232) `completed/success` 4/4 jobs, tentative 1, sur SHA intégré exact**.

**#404 ne modifie que les tests Windows** (`crates/optic-bridge-runtime/src/git/native_status_tests.rs`, +189 lignes) : `session_close_between_original_session_and_lease_receipts_denies_native_custody` et `dual_receipt_session_close_after_lease_commit_drains_and_stops` ressortent **`... ok`** dans les logs Windows. Le premier scénario prouve qu'une fermeture lifecycle de session *entre* les reçus session et TaskLease interdit de poursuivre malgré une TaskLease encore active. Le second prouve qu'une révocation directe *après* les deux reçus publie le stop de génération d'origine, attend la libération du permis compté, et conserve le Job Windows **vide** et les quotas jusqu'au drop. Aucun lancement Git.

**Limites :** les reçus session (#400) et TaskLease (#402) sont émis sous **deux mutex indépendants**, et ne constituent PAS un fence atomique pour policy epoch, host shutdown, `CreateProcessW`, `ResumeThread` ou publication. Aucune preuve après exécution Job vide/descendants, stdout/stderr EOF, watchdog borné ou récupération de terminaison incertaine. MCP GitRead reste dans la voie legacy `HardenedCommandRunner` (pipe joins non prouvés bornés). [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est **CLOSED administrativement**, sans preuve physique suffisante du pipe-join ; [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323), [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) **OPEN**. #352 fermée non fusionnée ; #355 recherche draft. **Windows V1 NO-GO, aucune release.**

[Première CI de #404 #40666050161](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666050161) : échec `rustfmt` seul sur premier SHA, corrigé ; Windows première tentative annulé par supersession. **Pas** un échec comportemental natif.

**P0 ensuite :** auditer les vrais chemins mutateurs session/TaskLease/policy epoch/host shutdown et JobStore, prouver un protocole d'engagement/révocation au plus près de Windows Job-at-create avec arrêt hors verrou, puis processus réel/descendants/stdout/stderr EOF. Pas de nouveau snapshot présenté comme lancement autorisé. [Prompt complet daté 28/10](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-28.md). La PR documentaire crée un SHA ultérieur : **revérifier le nouveau main et sa CI indépendante après fusion**.


## CURRENT VERIFIED STATUS — 28 OCTOBER 2026, TaskLease generation stage #402

[PR #402](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/402): exact tested HEAD `04a0c025202dbf2dfbd8cf43d3314c64260e6bee`, squash `main=fc53da251ff1c6b396fcdddc8092b9d18712cfd9`; identical tree `d287df495e1543e8d04e083651a34fd9ed641aa6`. Exact-head [CI #40663011191](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011191) **SUCCESS 4/4** and [specialized native #40663011174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011174) **SUCCESS**; independent [push/main #40663744586](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663744586) **SUCCESS 4/4 on exact merged SHA**.

The Windows-only, non-executing native GitRead candidate now retains **two separately issued** original-generation stop receipts: session (#400) and TaskLease (#402). The TaskLease receipt is minted under the same `TaskLeaseRegistry.leases` mutex as direct/session revoke, physical removal and registration, requires exact original registration-signal identity and preserved admission-time expiry. Four hosted Windows tests prove stop after revoke, `revoke_session` and `remove_session`, same-ID reuse and foreign registry rejection, post-renewal deadline, preserved real **empty** Job and counted permits. Foreign/reused registration fails closed with existing `Revoked` (no new public MCP error variant).

**No-go:** sequential session and TaskLease receipts are **NOT** an atomic session/lease/policy/host effect fence or CreateProcess permission. There is NO launched native Git root, no descendant process/Job/stdio EOF observation, no watchdog or uncertain-termination cleanup proof, and no MCP GitRead migration. Legacy runner is unchanged. GitHub [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is currently **closed**, but its described pipe-join bounded-wall-time acceptance has NOT been established by these no-child tests; closed/unmerged #352 is not evidence of resolution. [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN — Windows V1 NO-GO**, no release, no operator-PC or other project changes.

Earlier dated sections below are historical; the final main SHA is `fc53da251ff1c6b396fcdddc8092b9d18712cfd9` until a later **verified** integration supersedes it.


## CURRENT VERIFIED CHECKPOINT — 28 October 2026, session-only owner commitment #400

[PR #400](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/400) tested HEAD `a949fbf1745de0f97e03284628967c30b2fa887e`, squash integrated `main=67e3604c198f83c16465863facdf2fa855f983d7`, identical tree `444a3175b7241d85d4e959de456f8fa4f1341ae0`. [PR CI #40658720479](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658720479) **SUCCESS 4/4 first attempt**, [native preparation #40658719448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658719448) SUCCESS, and independent [push/main #40659396121](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40659396121) **SUCCESS 4/4, attempt 1 on the exact integrated SHA**.

On Windows, the already-registered, **non-executing** native status owner may consume its precreated Job/pipes/pins into a **session-only** original-generation commit receipt issued under the same session mutex as direct and lifecycle revocation. The receipt captures the original stop watch and real private Job ID. Hosted Windows tests prove commitment preceding direct revoke retains original admission/transport/Job until drop and observes stop; direct or lifecycle close winning first prevents commitment. Both tests report `ok`.

This is **NOT** a native launch grant or the final GitRead commit. TaskLeaseRegistry and policy epoch revocations remain independent; there is no CreateProcess/ResumeThread, actual Git process/root+descendant/stdio EOF, bounded guardian, termination uncertainty proof or production MCP GitRead switch. #352 was closed **without merge**, not accepted as proof; #355 behavioral acceptance remains unmet. #317/#323/#185/#319 remain OPEN. **Windows V1 NO-GO, no release**.

Older dated STATUS sections below remain historical checkpoints. The full evidence and next lock-order work is in [GitHub handoff](docs/engineering/GITHUB_HANDOFF.md).


## CURRENT VERIFIED CHECKPOINT — 28 OCTOBER 2026, original session close observation #398

[PR #398](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/398) (exact tested HEAD `7456f3d96dc289eb5e278e3d9acbfce8970a6dcf`) merged into `main=8aa6fd205e7a264e7873e6af6b627ddb81207d9d`. Trees identical `8883d4c0ee6a3647b543799c29f9b573be93a028`. Exact-head [CI #40655468052](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468052) **SUCCESS 4/4**, [native preparation #40655468078](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468078) **SUCCESS**. Independent [push/main #40656160001](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40656160001) **SUCCESS 4/4 on exact merged SHA, first attempt**. First candidate's [CI #40655374591](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655374591) failed rustfmt, not #317 behavioral acceptance.

The Windows-only, internal `SessionRegistry::observe_original_owned_admission_under_close_lock` rechecks an owned admission's original registry, registration-generation signal, revoked bit and captured expiry under the **same mutex** as direct `revoke` and lifecycle `publish_revocation_before_drain`. The precreated, **non-executing** native GitRead custody now invokes it both before and after its independent lease/policy snapshot. Windows tests `original_owner_observation_orders_with_direct_and_lifecycle_close` and `original_owner_observation_rejects_foreign_registry_and_renewed_deadline` passed on exact HEAD. No OS IO or policy work is performed under that session lock.

**Nonclaims:** the mutex is released after this observation: it is not an owner **commit** or CreateProcess permit, nor a cross-registry TaskLease/policy epoch fence. Session or lease revoke may still race native creation. No real GitRead root, native child/descendant/stdio EOF, out-of-band stop, crash/uncertainty recovery or MCP switch is implemented; real #352/#355 acceptance remains RED. #317/#323/#185/#319 remain OPEN; **Windows V1 NO-GO**, no release or operator-PC access.

Prior dated STATUS sections are historical. [Issue #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) retains source-specific proofs and remaining blockers.


## CURRENT VERIFIED CHECKPOINT — 28 OCTOBER 2026, #394/#395/#396 integrated

**Exact verified main:** `8ee1ad0b2c135af3d639ab753703c3f911dc4941` with independent [push/main #40653860272](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653860272) SUCCESS 4/4, attempt 1. [#394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/394) updated prior documentation; test-only [#395](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/395) preserved the original native TaskLease revocation through physical removal and reuse of its opaque ID ([PR CI #40651259497](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651259497) 4/4 and independent main #40651850790 4/4). Test-only [#396](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/396) proved a real kernel Job stays empty after expected `CreateProcessW` `ERROR_FILE_NOT_FOUND` with a valid Job-list attribute ([PR CI #40653261422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261422) SUCCESS 4/4; three specialized Windows workflows SUCCESS). Exact HEAD, tree and workflow receipts are retained in [GitHub handoff](docs/engineering/GITHUB_HANDOFF.md) and [native witness contract](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).

**No production GitRead migration or synchronized revoke/commit**: both tests cover an unstarted owner/Job and do not prove native child-tree/stdio EOF, crash/uncertainty or the #352/#355 real acceptance requirements. Issues #317/#323/#185/#319 OPEN; **Windows V1 NO-GO**, no release. Older dated sections below are history.

## Contrôle réel Windows #317 — 28 octobre 2026, PR #393 intégrée (recherche test uniquement)

**Baseline vérifiée avant travail :** `main=204f8619ff14c86fb3991f752eb3efed77ab2706`, [CI indépendante #40556021484](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40556021484) SUCCESS 4/4. [PR #393 fusionnée](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/393) depuis HEAD exact `14e73802b853c57b67c0b00bf169af6fd5a47793` vers `main=c81c1f5caf937695b3bbd9439f508ff9cb28f6f4`. Arbre HEAD et squash identique : `12c27901f8ec3f6dd4b8f6abfbfae44aaa16f987`. [CI PR #40649262906](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262906) SUCCESS 4/4 tentative 1, plus trois workflows natifs #40649262958/#40649262919/#40649262961 SUCCESS. **CI indépendante après fusion : [#40649847690](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649847690), **completed/SUCCESS, tentative 1, quatre jobs individuellement SUCCESS sur le SHA intégré exact**.**

**Portée limitée :** nouveau test Windows `job::job_list_launch_tests::suspended_root_is_in_configured_job_at_createprocess_return` dans `crates/optic-bridge-windows/src/job_list_launch_tests.rs`, enregistré via `job.rs`. Avec `PROC_THREAD_ATTRIBUTE_JOB_LIST`, il crée un `cmd.exe` système suspendu, sans le reprendre, vérifie `IsProcessInJob` et le compte noyau `ActiveProcesses=1`, puis observe root terminé et Job vide après terminaison. Le test réel est passé dans le job Windows de la CI PR. **Aucun code de lancement produit, aucun GitRead MCP, aucune TaskLease/politique ou interface client modifiés.** Premiers RED : rustfmt [#40648989661](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40648989661), puis import windows-rs E0432 [#40649081510](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649081510), **pas** reproduction de #317.

**Toujours non démontré :** interruption pendant `CreateProcessW`, sérialisation des révocations session/TaskLease/policy epoch à l'engagement, réveil du processus, propriétaire et watchdog réels, descendants conservant stdout/stderr, EOF des flux, conservation sous `TerminationUncertain`, reprise, voie MCP réelle, tests d'acceptation #352/#355 et Windows 10/11 de destination. **#317/#323/#185/#319 OUVERTES ; Windows V1 NO-GO.** Les sections datées ci-dessous conservent leur valeur historique et ne représentent pas le dernier SHA.

## Dernière passation vérifiée — 27 octobre 2026, après PR #391 fusionnée

**Périmètre : stabilisation locale Windows 10/11 V1, GitHub-only, NO-GO.** `main=c208348f0b63d4810db97f9a5284b325c5b64271` après [PR #391 fusionnée](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/391) (HEAD `e7f684ae9c32b3d1026ef103d9832f8a7767a535`, arbres Git identiques `da6ff9bcbeb63ffb3a64ffc590187ae4d2be4765`). [CI PR #40526579598](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40526579598) SUCCESS **4/4 tentative 1** et trois workflows natifs #40526579328, #40526579323, #40526579729 SUCCESS. [CI indépendante push/main #40553879389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40553879389) **completed/SUCCESS, tentative 1, quatre jobs individuellement SUCCESS** sur le SHA intégré exact. La **présente PR documentaire** fera avancer ce SHA : revalider son HEAD, sa CI puis l’indépendant push/main.

**#391 ne modifie AUCUNE production** : un seul test Windows ajouté à `crates/optic-bridge-windows/src/directory.rs`. Il constate que la garde actuelle `pin_native_git_read_repository_root` interdit déjà le renommage d’un répertoire **parent** du dépôt pendant la détention du pin, puis autorise le renommage après relâchement. L’hypothèse d’une lacune n’a **pas** été reproduite. Ne pas inventer de correctif ni prétendre qu’un lancement de Git a été testé.

**#390 est intégrée** sur le checkpoint précédent `main=7f663454039b294d84cd3fb2333cbbd73372f91a`, [push/main #40520286289](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40520286289) SUCCESS 4/4. Son observation `observe_original_prelaunch_snapshot` vérifie politique GitRead, identité exécutable épinglée, registre de session initial et **même génération réelle TaskLease** (Arc du témoin de révocation, non pas son ID réutilisable). Elle reste **non exécutante** et **non synchronisée** à la création Win32.

**État effectif** : GitRead MCP actif emploie encore `HardenedCommandRunner` et `task_lease: None` sur l’ancienne voie. Les tranches intégrées #382/#385/#387/#388/#390 préparent l’action Git fixée, retiennent admission MCP session/transport, scope/TaskLease/epoch/deadline, enregistrent quotas et Job Win32 **vide**, pipes stdout/stderr avec writers parents + stdin `NUL`, pins binaire Git et dépôt canonique et un instantané du registre TaskLease d’origine. Aucun enfant créé, aucun `CreateProcessW` sécurisé intégré, aucune preuve native root-exit/Job-empty-après-exécution/EOF, aucun véritable watchdog/crash recovery ni raccord MCP natif.

**P0 :** architecturer et prouver le contrôle **synchronisé** révocation/politique/TaskLease/session/deadline/identité Git à la création native Windows, puis assurer attachement Job avant exécution, conservation des propriétaires/quotas/admissions sous `TerminationUncertain`, preuves positives root terminé + Job vide + **EOF réel** et traitement des descendants, avant toute activation du `git_status` MCP avec chemins sensibles inchangés. Les acceptations de recherche [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) (finitude temporelle/stdio) et [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) (révocation/création) restent **RED pour la V1**. #353/#358/#327 recherche isolée, jamais fusion en bloc. #317, #323, #185, #319 **OPEN** ; V1 **NO-GO**, aucune release ni accès au PC.

Le nouveau **[prompt complet à copier](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md)** est canonique ; la [passation technique](docs/engineering/HANDOFF_WINDOWS_V1_CHAT_2026-10-27.md) et [AGENTS.md](AGENTS.md) démarrent par ce checkpoint. Les sections datées suivantes sont historiques, leurs mentions « draft » ou anciennes bases ne reflètent plus le `main` courant.



## 27 October 2026 — unstarted native GitRead original-lease generation snapshot (#390 draft)

[PR #390](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/390) adds a **read-only, non-launching observation** to already-precreated Windows GitRead custody. It rechecks current TaskLease policy and pinned executable content identity through the registry, compares the **same underlying revocation record** against the original admitted TaskLease witness (`Arc::ptr_eq`, not merely the reusable opaque ID), confirms the original session registry, and rechecks the captured monotonic stop predicate before and after. The true empty Job, pinned executable/repository, stdio and original session/transport/process permits stay owned throughout. A foreign registry with the same lease ID, wrong session registry, and revoked original generation must fail closed, leaving permits held until the known-no-child custody is explicitly dropped.

**RED evidence:** test-first SHA `589388e1caae7213eb96ddb70f7e7ebd5f4def56`, [CI #40517792327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40517792327), showed Windows compiler E0433/E0599 (missing `NativeGitStatusSnapshotError` and method) plus independent Ubuntu rustfmt differences. This is *not* original runtime behavior #352/#355. Implementation `968d279326e33cc27626c4352fd7b17ef0ef7a00`; follow-up rustfmt `f4a7eb5b4d72926fea612c07d2143a734ac1f2ed`, [CI #40518650462](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40518650462), must be verified before the new documentation HEAD can be accepted.

**Still absent:** the snapshot is NOT synchronized with `CreateProcessW`; it is NOT approval to spawn. No root creation/Job assignment/resume, real EOF/Job-empty-after-run, watchdog/restart recovery, live MCP `git_status` switch, original #352/#355 acceptance or Windows 10/11 freeze proof. Keep #317/#323 OPEN and Windows V1 **NO-GO**; no release.


## Integration receipt — 27 October 2026, post #388

**Confirmed integration after PR #388 (27 October 2026):** `main=720a38833705e661049841cfd39852e770687643`. PR HEAD `0b29e833a53195f199b5d3b53f808091d020bdc5` passed [CI #40513121389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40513121389), first attempt, four jobs individually SUCCESS and all three native-specialized workflows successful. The reviewed HEAD and squash tree are identical (`1d79045f8ac60db9dd2dc2ffe6e23b72af0811f9`). Independent [push/main #40514100157](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40514100157) on that exact integrated SHA completed SUCCESS, attempt 1, all four Windows/Ubuntu/dependencies/release jobs successful.

**Scope remains pre-spawn only:** original Git executable and repository root opened and pinned by verified Windows handles, with the already counted admissions, TaskLease witness, unstarted Job and stdio. No child, final synchronized `CreateProcess` authority, native terminal/EOF proof, MCP GitRead switch, physical Windows 10/11 acceptance or release. Keep #317/#323 OPEN, #352/#355 original acceptance RED, and Windows V1 NO-GO. This documentary commit itself changes the HEAD; revalidate live `main`/`push/main` separately after any merge.


## 27 October 2026 — integrated original-target pins for Windows native Git status (#388)

[Merged PR #388](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/388) extends the already **unstarted** and counted #387 native status custody with a pinned canonical executable handle and a special READ/LIST-access root-directory handle that denies Windows DELETE-sharing rename of the repository root itself. Both opened-handle final paths must match the original service command's canonical executable/root; no caller-supplied path, argv or renewed TaskLease is accepted. The original session/transport permits and process quotas remain retained with the empty Job and its stdio handles. Existing zero-access directory-chain callers are unchanged.

**Behavioral red, preserved:** [CI #40510524394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40510524394) test-first SHA `968adc2f58a5b8c012a6455fdef2b8f053a6873c`: Windows **371 passed / 2 failed / 11 ignored** (repository renamed during unstarted custody; already-moved repository admitted); Ubuntu rustfmt separately RED. [CI #40511257834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40511257834) first pin implementation SHA `132f988131b202ad1ed5cf9122ba0c3f26945648`: Windows **372 passed / 1 failed / 11 ignored**, showing that the general zero-access directory pin did not block root rename. That finding led to the GitRead-specific READ/LIST pin on `382857c0269b7173c24bef8eb948d08ad0e558ae`: [CI #40512075888](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075888) completed SUCCESS, attempt 1; [native lifecycle #40512075880](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075880), [native Rust IR #40512075887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075887), [native preparation #40512075891](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075891) all SUCCESS on this SHA. A later documentation HEAD still requires its own CI.

**Explicit limits:** no new verified content hash at the CreateProcess boundary, no synchronized final session/TaskLease/policy/epoch/deadline check, no child/root assignment/resume, no watchdog/real EOF/Job-empty-after-child, and no live MCP Git status switch. Original #352/#355 acceptance stays RED; #317/#323 OPEN, Windows V1 **NO-GO**. No release.


## 27 October 2026 — real, unstarted native Git status Job custody (#387)

[Draft PR #387](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/387) adds a **no-spawn**
step after the integrated #385 registered native status. On Windows, the
original `RegisteredNativeGitStatus` consumes its already counted MCP session
admission, transport permit, registered TaskLease witness, action horizon and
process-store reservation into `PrecreatedNativeGitStatusCustody`. The
platform `PrecreatedNativeReadCustody` constructs a **real kernel Job Object**
with the existing process/memory/CPU ceilings and kill-on-final-handle-close,
two owned stdout/stderr anonymous pipes with parent writers still open, and an
opened `NUL` stdin. A second original-generation stop check occurs after
precreation. An expired/revoked admission fails closed without a child; only
the **known-unstarted** owner may use ordinary RAII cleanup.

This is **not** the native process owner or the live MCP Git status path:
there is no CreateProcess method, no root, assignment, resume, pinned Git
identity, synchronous final policy/revocation/epoch gate, positive post-run
Job-empty/root-exit/EOF evidence, or host-loss recovery. A zero process count
on a never-started Job and idle pipe with no EOF prove only precreation.
Live MCP `git_status` still uses `HardenedCommandRunner` and `task_lease: None`.
The original real behavioral acceptance #352/#355 stays RED. #317/#323
OPEN, Windows V1 NO-GO; #353/#358 remain separate research.

**Test-first** `35223758a95a9d6874b8bae42e8a0e789cd6aa2d`,
[CI #40506640216](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40506640216):
Windows compiler E0433/E0599 absent interface and Ubuntu rustfmt, **not**
native runtime failures. The first implementation `cce80c41998a75d7b2172a33875c2cde63d71daf`
has [format RED #40506898619](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40506898619)
while its native lifecycle, private Rust compilation and native preparation
checks passed. For…18880 tokens truncated…ks. Functional Linux delivery
is post-V1; existing Ubuntu Rust/dependency checks remain required regression guards.

The #318 change rejects non-octal raw Git modes and bounds Windows doctor stderr
capture after EOF/timeout using a sole-consumer native pipe snapshot. Tests preserve
primary failures, cap bytes/messages, reject unknown labels and supervise all fixture
processes. No startup deadline, full executable identity/pin, resource/permission,
AppContainer or network boundary is relaxed. Exact integration status and source
checks are in [PR318](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/318);
the [native proof](docs/research/WINDOWS_DOCTOR_PIPE_LIFETIME_2026-10-22.md)
distinguishes demonstrated error-path failures from startup hypotheses.

**Still open:**319 natural first MCP30s timeout (cause not established),185 separate
Node/AppContainer10s failure,317 demonstrated generic command pipe-join overrun
(no production Rust ownership correction),176/182 sensitive/provenance boundaries.
These remain V1/release gates where applicable. An unrelated Git/parser correction
and a green current CI do not certify startup reliability or real-workspace safety.
No feature unfreeze, Electron, generalized Cargo, alpha merge or user-PC execution.

Older checkpoints below describe their dates, not current branch heads.

## Historical verified checkpoint — #314 (2026-10-21)

**Verified source of truth:** `main=7bc17c16a52383bf6fb7ba68dc4b9348b259be9a`, 0 open PRs. [PR #314](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314) is squash-merged: a hard-linked file can bear both a classified sensitive name (`.env`) and an innocent-looking alias (`README.txt`); `fs_read` now requires existing matching `ReadSensitiveScope` in addition to ordinary FileRead for multi-linked opened native identities. This is intentionally conservative for harmless hard links. Native Windows/Linux regression tests passed on accepted source SHA `fbf7c9f40bbae18d4017f237b8783a19964620c8` in [CI #39576724127](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39576724127); three specialized native workflows also passed. Independent [push CI #39577461324](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39577461324) passed **first attempt on the exact squash SHA**, Windows Rust, Linux Rust and dependencies. Earlier rustfmt failure on a superseded PR head remains preserved. [Complete proof](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314#issuecomment-6312779381).

**Residual trust boundaries:** #176 (arbitrary secrets in ordinary files, native ancestor identity/races), #182 (Git index/commit content provenance and branch/ref metadata), and #185 (intermittent 10-second AppContainer Node timeout of unknown cause) remain **OPEN**. A separate ~30-second first MCP response flake remains unproven. [Targeted GitRead review](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182#issuecomment-6312922336) confirmed existing status/diff protections and recorded that status `# branch.head` / `# branch.upstream` values are returned as Git metadata without separate sensitivity classification; this does **not** prove credential leakage. Do not change the public metadata policy without an explicit contract and synthetic regression.

**Execution:** pre-alpha, feature work frozen, no real-project remote access, no alpha/generated-project-editing merge, no user-PC or self-hosted execution, no privilege/sandbox/timeout relaxation. Source head must always be rechecked live. All later candidate checkpoints below are dated history, not unmerged current work.

## Verified Policy B path-floor integration — after #310 (2026-10-21)

**Source of truth verified on GitHub:** `main=bd2915c3a9833c1da5a8fc5005445d1884260d4f`; [#310](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/310) is squash-merged; zero PRs are open at this checkpoint. Independent [main CI #39552523257](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39552523257) succeeded **first attempt** at the exact merge SHA on native Windows, Ubuntu and dependency policy. #310's accepted head `154d7ea6ab6b10aa967ab0a2f2034f2f06400fe9` passed [source CI #39551993433](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993433) and [native preparation #39551993151](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993151) first attempt. Earlier red PR CI attempts remain evidence of corrected rustfmt and verified Git raw-protocol test failures; do not overwrite them.

**Integrated security path floor:** #308 adds shared Rust-core sensitive-name classification and the FileRead / fs_list additional scoped authority checks. #309 parses Git porcelain-v2 `-z` metadata and denies unknown or unauthorized protected names. #310 produces `git diff --raw -z --patch` in a single hardened process, validates NUL-delimited raw changed paths **and the extra NUL before the patch**, denies unverifiable streams, and requires active typed FileRead (ReadContent + ReadSensitive) authority for classified protected patch contents while read admission remains held. No new default grants, remote transport, feature expansion or weaker containment. See [ADR-0034](docs/decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md).

**Still blocked:** lexical filename rules cannot prove ordinary-named contents are secret-free (#176); Git metadata and diff coverage remain subject to concurrent repository changes, unproven alternate path identity and other provenance/authority guarantees (#182). Windows Node/AppContainer timing #185, separate MCP first-response intermittency, crash recovery atomicity/persistence, ancestor races, stress/fault tests and incomplete code audit remain. #176 and #182 are OPEN; #176 was accidentally auto-closed during #310 merge by a GitHub closing keyword in prose and explicitly reopened with evidence. No real-workspace remote exposure or feature unfreeze. Older sections marked *candidate* below are **historical pre-merge records** and are superseded by this verified checkpoint; do not reapply earlier base SHAs.

## Git diff path-verified output boundary — candidate (2026-10-21)

Live GitHub before branching: `main=ccaa7c05af27cfaab02d5dcb93369f60a0c01f53`; [#309](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/309) squash-merged; independent [post-merge CI #39549267903](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39549267903) green first attempt (native Windows/Linux/dependencies); zero open PRs. New **candidate** `security/git-diff-raw-output-authority`: not integrated until exact-PR-SHA CI and independent main CI succeed.

Proven #182 gap: unlike the now-protected `git_status`, `git_diff` returned raw staged/worktree patch bytes without classifying any changed path. Candidate correction: one hardened Git invocation `--raw -z --patch` with raw changed-path verification and strict rejection of malformed/unknown/native-unrepresentable outputs. No preflight/patch race from two separate Git calls. For protected patch paths, check an additional active typed FileRead effect (both content and sensitive scopes) **inside session admission**, then release only the patch suffix. Normal source patches stay available. Unit/MCP regressions target unscoped/path-scoped/dir-scoped/staged protected patches, ordinary code, malformed raw and resource budgets.

**Limit:** Git diff source metadata and patch come from the same Git diff command, but worktree snapshots are not atomic against arbitrary concurrent edits. Lexical path rules do not detect secrets in normal-looking file contents. #176/#182 remain OPEN; no remote real-project deployment or restart of new features.

## Git status sensitive-path classification — candidate (2026-10-21)

Live baseline for this increment: `main=bfba4b97a531440de08012ca50106797d995646b`, [#308](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/308) squash-merged, independent [post-merge CI #39546153987](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39546153987) green first attempt, zero open PRs. Operator-approved Policy B remains binding, with no remote exposure to real workspaces.

**Candidate `security/git-status-sensitive-paths` (unmerged until exact-head CI):** validate every path in the returned Git porcelain-v2 `-z` status output, including rename original names, before returning status bytes. Reuse the Rust-core classifier and require an active matching sensitive read lease in the admitted MCP worker; refuse malformed/unknown/unrepresentable records. Native Windows/Linux tests cover sensitive staged and untracked files, mixed normal content, explicit grants, prefix mismatches and invalid output. No new CLI, transport, grant, override or runtime privilege.

**Open:** #182 remains blocked by the *independently unclassified raw `git_diff`*; harmless-looking files can carry secrets and #176 remains incomplete. Do not close either issue or enable real-workspace remote access. Inspect GitHub CI and final SHA before considering this candidate merged.

## Sensitive-read authority — policy B approved, first scoped fix under CI review (2026-10-21)

After merged #307, live `main=3d9a0f78304578af53fd23b9fed7d08603348e0b`, zero open PRs, and post-merge hosted Windows/Linux/dependency CI #39542505029 passed first attempt. The operator has now approved **Policy B**: shared deterministic classification of sensitive path/content across FileRead, FileSearch and GitRead, with fail-closed handling of unverifiable outputs. **This paragraph describes a candidate increment on `security/sensitive-file-authority-scope`, not a verified merge.**

First narrow corrective implementation: pure conservative sensitive-path classifier in core, enforced by `PolicyEngine` for file reads (ordinary content and matching sensitive scopes both required), and by `fs_list` before releasing sensitive directory names (whole affected page denied without additional sensitive grant). Unit and MCP regressions cover path ancestry, case, absent/mismatched sensitive leases, ordinary content and explicit scoped access. Exact-head hosted Windows/Linux acceptance and independent main CI are **not yet claimed**.

**Important limitation:** lexical name detection is not secret-content inspection. `GitRead` status/diff remain outside this increment and can independently disclose names or secret-bearing patches. Both #176 and #182 remain **open blockers** to real-project remote/autonomous exposure. Do not infer safe remote access, close these tickets, or restart features. Next correction must treat Git output records and unverifiable outputs fail-closed under the **same** classifier (ADR-0034) with exact SHA CI. No broad grants, deadline changes, PC execution or alpha merge.

## Successor handoff snapshot — 2026-10-21, after merged #306

GitHub live before this documentation handoff: `main=dadaf48539788235b689dd4e09adcd2fb94904f3`, **zero PRs open**, [PR #306](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/306) squash-merged (governance and stabilization exit-gates documentation). [PR #306 source CI #39540522392](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540522392) passed first attempt, and independent [main CI #39540940136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540940136) passed first attempt on exact merge SHA (hosted Rust Windows, Rust Linux, dependencies). This is **dated evidence, not an instruction to restore a commit**. Verify live GitHub for changes after this handoff's own PR.

**Next step:** security source audit of the `fs_read`/`fs_list` vs `GitRead`/status/diff data-exposure contract, [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) and [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182), with **no sensitive classifier yet proven** and no permission broadening. Other open reliability boundaries remain [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) Windows 10s Node/AppContainer, distinct MCP ~30s initial response, recovery atomicity/ancestor races/power-loss durability, soak/fault tests and incomplete exhaustive code audit. Administrative issues #180/#181/#183 have code-level regressions/protections recorded and need acceptance reconciliation, not automatic recreation or closure.

**Operating rule:** one PR at a time only after proving a defect, tests+docs together, exact SHA CI and independent post-fusion main verification; no operator PC, self-hosted runner, new features, release or changing GitHub branch protections without approval. Full future-chat instructions live in [PROJECT_CONTINUITY_PROMPT.md](PROJECT_CONTINUITY_PROMPT.md) and [GITHUB_HANDOFF.md](docs/engineering/GITHUB_HANDOFF.md); old candidate headings below are archive records.


## Verified integration checkpoint — 2026-10-21 (before this documentation update)

At the **live GitHub check**, `main` was `43d3a4aeedee04dd03e895e27e81d3bc5af6652c`, with **zero open PRs**. Independent push [CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed **first attempt** on this exact commit (hosted Windows Rust, Linux Rust, dependency policy). This is a dated observation, **not** an instruction to reset or a claim about later commits.

The recent stabilization work is **already merged**:

| PR | Main merge SHA | Narrow effect and verified integration |
| --- | --- | --- |
| [#301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) | `05b5a928` | Preflight all unpublished recovery artifacts before deleting; [main CI #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) passed first attempt. Source CI #39529419994 needed an unchanged-SHA Windows job retry after the known 10s Node/AppContainer failure. |
| [#302](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/302) | `c8c1110f` | Documentary continuity; [main CI #39530925055](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530925055) passed first attempt. |
| [#303](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/303) | `9c6c9109` | Canonical directory scope before `fs_list` authorization; [main CI #39534614408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39534614408) passed first attempt. |
| [#304](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/304) | `9d90485f` | Reserve canonical `.git` control paths from generic mutation; [main CI #39536541153](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39536541153) passed first attempt. |
| [#305](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/305) | `43d3a4ae` | Fit the final `git_log` response **including** newly added continuation cursor; [main CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed first attempt. |

**Not closed by green CI:** Windows Node/AppContainer 10-second no-output intermittency ([#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185)); separate ~30-second MCP first-response intermittency; recovery multi-delete non-atomicity, ancestor namespace races and Windows power-loss durability; sensitive-file read classification ([#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)) and GitRead sensitivity composition ([#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182)). #180/#181/#183 still appear **open administratively** despite already-integrated scoped corrections/tests; reconcile their *remaining acceptance criteria*, do not repeat the implementation or silently close them.

The core is **pre-alpha**. The feature freeze and GitHub-hosted-only restrictions remain. See [stabilization exit gates](ROADMAP.md), [current handoff](docs/engineering/GITHUB_HANDOFF.md) and [reliability debt](docs/engineering/RELIABILITY_DEBT.md). The older sections below are **dated historical reports**, including wording such as “candidate” written before those PRs merged.


## Git-log response size & pagination — bounded regression

A deterministic `git_log` response-fitting flaw existed when the final history page had `next_cursor: None`: trimming entries to meet the structured response-byte limit then **added** a cursor containing the snapshot HEAD and new offset without checking the resulting serialized size. A too-large MCP response could be emitted despite the configured output budget. The narrow fix repeatedly refits after cursor insertion, reducing the page while preserving at least one advancing entry; if no entry plus cursor fits, it fails closed with `optic.response_too_large`. Cursor arithmetic now rejects overflow rather than saturating into a non-advancing token.

Focused tests construct an end-of-history page with a carefully selected byte budget to prove rejection when the cursor cannot fit, acceptance when it can and preservation of a complete cursor-free last page. GitRead capabilities, HEAD snapshot pinning, Git subprocess sandbox and resource limits are unchanged. **Verify the actual PR, exact-head CI, and independent `main` CI before claiming this change is integrated.** No new feature, user-PC activity or CI bypass.


## Git-control mutation authority boundary — stabilization candidate (2026-10-21)

The [security audit #180](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/180) identified a real authority-composition gap: generic FileWrite/FileDelete with whole-workspace or prefix grants could address `.git` control metadata, despite dedicated GitIntegrate authority. The scoped candidate reserves every exact `.git` path component (case-insensitively, including linked-worktree `.git` files) from generic transactional write/delete/patch preparation, **after canonical target observation**; generic directory creation rejects the same namespace. It does not expand session/lease rights or change the Git integration runtime. Ordinary dotfiles such as `.gitignore` remain regular project files.

Cross-platform preparation tests, a Unix internal alias test, native Windows end-to-end authorized mutation and directory tests, and explicit MCP denial-code tests are included. **Do not describe as delivered until exact-head GitHub-hosted Windows/Linux/dependency/native-preparation CI and post-merge main checks succeed.** Parent path swaps, hardlink alias identity and power-loss durability remain independent issues. Never infer that sensitive-content reads or remote exposure are solved by this change. No new features, permission expansion, operator-PC activity or self-hosted CI.


## Scoped fs_list canonical-target authorization — security stabilization (2026-10-21)

Source review of the existing local MCP `fs_list` found a concrete lexical/canonical scope mismatch: policy evaluated a requested directory alias before `BoundedFileSystem::list` followed the alias to a different workspace subtree. Under prefix-scoped `SearchMetadataScope`, this permits metadata enumeration outside the granted prefix. The bounded correction resolves the **canonical directory target before policy evaluation** and lists that resolved canonical root; returned paths identify the actual directory. A Unix symlink regression distinguishes restricted-prefix denial from whole-workspace allowed canonical listing; a portable root/normal-directory regression runs on hosted Windows and Linux.

The change does not grant new permissions, create a transport, add a sensitive-content classifier, pin ancestor directory handles, or eliminate concurrent namespace replacement races. Review the exact source PR and native CI before claiming delivery. [Security issue #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) remains open for its separate sensitive-read classifier and remaining path/race boundaries; #301 recovery and Node #185 remain independent. Feature freeze remains in force.


## Stabilization: unpublished recovery evidence — PR #301 (2026-10-21)

[PR #301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) was squash-merged into `main` at `05b5a928c862601926e8b0cc11540ce020caf271`. Its scoped fix preflights **all** unpublished `*.prepared.tmp` entries with existing native opened-file, single-link and no-reparse checks before the first deletion, and rechecks each immediately before removal. The new ordered-fixture regression proves a later hard-linked unsafe entry cannot erase earlier unrelated evidence; repair permits normal cleanup. Windows and Linux logs explicitly record `later_aliased_unpublished_entry_preserves_all_earlier_evidence ... ok` on accepted source `089e64a2b6c95dc0705eeaaa22f90194dfb7be66`.

[Source CI #39529419994](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529419994) passed after **attempt 2**, not on first attempt. Initial Windows attempt failed in the already-known native Node/AppContainer 10s timeout (no captured launcher/Node marker); three serial same-head diagnostics passed and remain evidence in [issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185#issuecomment-6306039365). Linux and dependency checks passed on the original attempt; [native preparation #39529420240](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529420240) passed first attempt. A failed-Windows-job-only rerun passed the full native suite and smokes on unchanged SHA. Independent [main CI #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) **passed first attempt on exact squash SHA** `05b5a928c862601926e8b0cc11540ce020caf271` (hosted Windows Rust, Linux Rust and dependency policy).

**Limit:** This closes one demonstrable early-cleanup-on-later-unsafe-entry defect. It does **not** guarantee atomicity of multiple deletions if a later actual deletion fails, concurrent ancestor/path identity, Unix unlink race safety or Windows power-loss directory persistence. Node/AppContainer (~10s) and separate MCP first response (~30s) intermittencies remain open. Feature freeze and GitHub-only execution restrictions unchanged.



## Stabilisation Windows — PR #299 fusionnée le 21 octobre 2026

À l'observation GitHub après la fusion : `main=6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea`, aucune PR ouverte. La [PR #299](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/299) a été fusionnée par squash depuis la tête exacte `aca382535f24b6f6b1c26c2313302d315099ed4e`. Sa [CI source #39526324369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324369) a réussi **du premier coup** (Windows Rust natif, Linux Rust, dépendances), de même que la [préparation native #39526324493](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324493). La [CI indépendante post-fusion #39526677098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526677098) a **réussi dès la première tentative**, sur le SHA exact `6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea` (Windows Rust, Linux Rust et dépendances). Les docs sont un relevé daté, jamais la tête GitHub live.

**Incrément réel :** jalons monotones opt-in, à libellés fixes, écrits sur `stderr` par le lanceur d'isolation Windows : entrée, requête, entrées validées, profil, grants, enfant créé suspendu, token vérifié, reprise, fin. `OPTIC_LAUNCHER_PHASES=1` est transmis uniquement en tant que constante opt-in à ce lanceur, dans la preuve Windows existante et les répétitions diagnostiques déclenchées lors d'un échec. Le test natif vérifie la présence des phases et conserve le refus réseau effectif, la sortie attendue, les limites de 10 secondes et les règles d'isolation.

**Exemple validé :** `target_created_suspended=14 ms`, `target_resumed=14 ms`, `target_exited=361 ms` à partir de l'entrée du lanceur. Le parent a séparément observé `launch_returned=13 ms`, `terminal_observed=407 ms` ; ces horloges ont des origines différentes. Ces données prouvent le fonctionnement de l'instrumentation sur un essai **réussi**, pas la cause des expirations précédentes. Les intermittences Windows AppContainer/Node de ~10s et MCP première réponse de ~30s restent **ouvertes**. Le nettoyage multi-fichiers, les courses d'ancêtres et la persistance après coupure restent non validés. Aucun correctif fonctionnel de ces défauts n'est revendiqué. Le gel des nouvelles fonctions, l'interdiction de toute intervention sur le PC et la distinction `main`/alpha restent en vigueur.

Voir [la dette](docs/engineering/RELIABILITY_DEBT.md), [l'issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) et [la reprise](docs/engineering/GITHUB_HANDOFF.md). Les sections antérieures restent des archives datées.


## Point de reprise canonique — stabilisation vérifiée le 21 octobre 2026

**Observation avant cette mise à jour documentaire** — `main` = `83329d48789c20b1fc235d3096c3199dc9dc5176`; **aucune PR ouverte**. La [PR #297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) est fusionnée. Sa [CI indépendante post-fusion #39514832814](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514832814) a réussi **dès la première tentative**, sur le commit exact : Rust Windows natif, Rust Linux et politique des dépendances. La préparation native sur la tête source exacte a également réussi. Vérifier de nouveau l'état GitHub : ce point de contrôle n'est pas une tête à réinitialiser.

**Livré sur main (preuves précises dans les PR, ne pas répéter ces incréments)** :

| PR | Fusion (SHA abrégé) | Résultat réellement intégré |
| --- | --- | --- |
| [#293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/293) | `de973989` | Prévalidation coordonnée des journaux et fichiers de staging avant nettoyage des preuves temporaires |
| [#294](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/294) | `da3ab6f1` | Marqueurs de temps MCP du processus enfant, strictement filtrés dans le diagnostic Windows |
| [#295](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/295) | `615e6219` | Mesures distinctes des étapes Git, session, lecture et leases de processus |
| [#296](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/296) | `72966f92` | Mesures opt-in résolution de chemin / ouverture épinglée / lecture et hachage d'exécutable |
| [#297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) | `83329d48` | Refus doublon/capacité **avant** la capture d'identité ; seconde vérification sous verrou avant insertion, tests négatifs et concurrents Windows/Linux |

La CI indépendante sur `main` a réussi dès la première tentative pour #293, #295, #296 et #297. **Pour #294, la CI post-fusion n'a réussi qu'à la deuxième tentative**, après une répétition du timeout natif Node/AppContainer de 10 secondes sur Windows ; l'échec de première tentative reste une preuve, pas un résultat à masquer.

**Mesures et portée exacte** : sur une exécution Windows de #296, `path_ms=0`, `pin_ms=0`, `hash_ms=4869` (temps de **lecture bornée et BLAKE3**, pas le calcul BLAKE3 isolé). Le correctif #297 évite ce travail coûteux **uniquement si l'inscription du lease est déjà irrecevable** ; il ne supprime pas et ne met pas en cache la vérification d'intégrité d'une inscription valide.

**Risques ouverts et indépendants** : intermittence Windows Node/AppContainer (~10 s sans premier marqueur du script malgré retour rapide du lancement), intermittence MCP de première réponse (~30 s historiquement), nettoyage multi-fichiers non atomique et risques d'identité des répertoires ancêtres / persistance sous coupure électrique. Les preuves Cargo préparées n'autorisent pas un Cargo général ; Electron avec sa sandbox normale reste non validé. L'alpha repose encore sur des protections/authentifications non considérées comme prêtes pour la production. Le projet reste **pré-alpha, sans release de production**.

**Directive de l'opérateur en vigueur** : gel des nouvelles fonctionnalités, Electron, Cargo produit, interface graphique, fusion globale de l'alpha et refactorings non justifiés. Corriger seulement un défaut démontré, avec tests ciblés ; ne pas fabriquer une PR lorsqu'aucun défaut n'est trouvé. Aucun build/test/installateur/permission sur le PC de l'opérateur, aucun runner auto-hébergé ni release, aucune suppression massive ; GitHub et ses runners hébergés uniquement. Préserver la sécurité des identités épinglées, leases/permissions, budgets, préconditions et preuves de récupération.

Voir [le relais des agents](docs/engineering/GITHUB_HANDOFF.md), [la dette de fiabilité](docs/engineering/RELIABILITY_DEBT.md) et [le prompt de reprise](PROJECT_CONTINUITY_PROMPT.md).

## Historical checkpoints — dated evidence superseded by the canonical section above

## Canonical verified stabilization checkpoint — 2026-10-20

At live inspection, `main` is `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`, **zero open PRs**, and the independent post-merge [CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) completed successfully **on the exact main SHA, first attempt**, with hosted Windows Rust, Linux Rust and dependency-policy jobs all green.

- [PR #290](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/290) is **merged**, at `0c2a125537d020e3a3aff4124d12745d7a8c5c4c`. Its Windows Node/AppContainer test now records monotonic launch/terminal timings and child script-entry markers. Exact-source [CI #39392262806](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392262806) and independent [post-merge #39392829136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392829136) passed. This **does not resolve** the intermittent 10-second Node timeout or distinct 30-second MCP first-response timeout.
- [PR #291](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/291) is **merged**, at `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`. A Windows recovery scan now validates **all** existing staging artifacts before deleting any, then revalidates each through the existing exclusive native delete handle. Exact-source [CI #39393849343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849343), [native preparation #39393849287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849287) and post-merge #39394389914 passed. The negative Windows regression `corrupt_later_staging_does_not_erase_earlier_verified_staging` passed. The second deletion pass is **not multi-file atomic**; ancestor namespace races and sudden-power-loss persistence are not proved.

**Operator delivery policy remains stabilization-only:** no new functionality, broad refactor, experimental alpha merge, weakened CI, permissions or timeouts. Continue only with evidenced defects, targeted tests, and accurate documentation. No tests, builds or permissions changes on the operator's PC, no self-hosted runners, release or mass deletion. Every historical checkpoint below is dated evidence, **not** a live head or pending assignment; verify GitHub before further changes.

## Stabilization-only baseline — 2026-10-20

**Temporary delivery policy (operator decision, 2026-10-20): stabilization only.** Pause new features, product expansion, experimental merges, architectural reshuffles and broad refactors. Work exclusively on verified defects, security and recovery boundaries, flaky-test root causes, focused regressions, and accurate status/debt documents. Do not create code changes merely to keep PRs moving; when no defect can be demonstrated, record the evidence and stop. Preserve all existing permissions, isolation, deadlines, budgets and CI gates. Resuming feature development requires fresh operator direction.

Verified GitHub `main` at this checkpoint: `b1edc83438571867bd164ea0b2b9653304eed752`. [PR #288](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/288) was squash-merged at `b1edc83438571867bd164ea0b2b9653304eed752` from source `60206dcadbf2efb57800ca93f0bd226acf16f6fb`. Its hosted [CI #39388280621](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280621) passed Windows Rust, Linux Rust and dependency policy; [native preparation #39388280521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280521) passed too. Independent [post-merge CI #39388749406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388749406) succeeded on the same merge SHA, first attempt (Windows, Linux, dependencies). GitHub showed zero open PRs at verification.

**Delivered behavior:** `MutationRecoveryJournal::inspect_pending` validates all stable journal records before deleting any well-formed unpublished `*.prepared.tmp` files. A Windows/Linux regression preserves both the temporary and a later corrupt stable record on a failed scan, then checks normal cleanup after resolving corruption. Remaining boundaries include non-atomic multi-temp deletion, parent namespace/identity races, Windows directory persistence and unexplained Windows Node/AppContainer and MCP-first-response intermittent timeouts. These are risks for targeted investigation, not permission to claim fixes without proof.

## Archived proposal checkpoint — PR #288 (now integrated)

The preceding candidate wording was written before PR #288 passed native CI and merged. Its delivery is confirmed in the stabilization baseline above; do not implement it again.

## Linux journal directory sync integrated — PR #287 (2026-10-20)

[PR #287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/287) was squash-merged as `0ca2305e7f01d5263330b67c4a0bb0520d452c8a` from final head `66161c9dee9e1a565b074d1cf105cf8961a712c2`. Hosted [PR CI #39386566516](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566516) and [native preparation #39386566570](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566570) passed; independent [main CI #39387133506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39387133506) passed Windows Rust, Linux Rust and dependency policy on its first attempt. Linux GNU syncs the open recovery directory after publishing the stable prepared journal; failed sync retains a published record for recovery. Windows durability and sudden-power-loss guarantees remain distinct and unproven.

## Atomic no-replace journal publication — integrated by #286 (2026-10-20)

[PR #286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/286) was squash-merged as `67c65a5d78a2921cee826184a2ad8bc573e4fb88` (exact head `7007ad3d202e42f642cece5f8df131035806ff35`). Hosted Windows/Linux Rust, dependency policy and the three native checks passed on that head; independent [post-merge CI #39385303625](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39385303625) passed Windows/Linux/dependencies on first attempt. The stable journal publication uses native Windows `MoveFileExW` without replace/copy flags and Linux GNU `renameat2(RENAME_NOREPLACE)`; competing publishers and existing destination evidence tests passed. Other OS targets fail closed. Directory pinning and complete power-loss durability remain separate.

## State-root creation preflight — pending focused PR (2026-10-20)

GitHub main `adeed077215c2103991254e4b5e2c64fa2d632de` and its [post-merge CI #39381445867](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39381445867) were checked: hosted Windows Rust, Linux Rust, and dependency policy succeeded on first attempt, with no open PRs before this increment. A separate targeted branch `fix/recovery-state-root-preflight` addresses a concrete setup side effect: `MutationRecoveryJournal::open` previously called `create_dir_all` before rejecting a requested state path inside the workspace. This could create nested directories in the workspace despite a rejected state-root request. The candidate first canonicalizes the closest existing ancestor and fails closed on an in-workspace ancestor *before* creating a directory, while retaining the post-create canonical check. Tests cover both rejected absent in-workspace state roots without side effects and accepted absent external state roots. This is **not integrated until exact-head hosted CI and merge**. Concurrent ancestor substitutions and symlink races remain outside this limited check; no general directory pinning or power-loss guarantee is claimed.


## Windows recovery staging cleanup — scoped source correction, 2026-10-20

At resumed inspection, `main` was `d11d27c8efb1048f504d96024e4015ec60fd1535`, with no open PRs and post-merge hosted [CI #39376986248](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376986248) passing all Windows Rust, Linux Rust and dependency checks on attempt 1.

[PR #284](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) isolates a Windows-only recovery staging-cleanup defect: the original code read/validated bytes, closed that file and unlinked a separately resolved name. Valid matching bytes in a staging hard link to an outside file were accepted without detecting the second link. The change reuses the existing exclusive no-reparse, single-link native delete handle, verifies the opened length and hash, and deletes via that **same handle**. A negative native Windows test requires both hard links and the journal evidence to survive failed recovery. This paragraph describes PR scope, not its merge/CI outcome; inspect the [PR's latest head/checks](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) before claiming delivery. Parent-directory pinning, concurrent ancestor swaps and power-loss guarantees remain unproven. The experimental alpha and Windows Node intermittency are unchanged.


## Recovery journal append alias hardening integrated — 2026-10-20

[PR #282](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/282) was squash-merged into `main` as `bf79b320b4e7aa04a02c87fe448d589296bc8bb2` from final source head `c837cead816fb59c5d3a94b8d9b5fe1926987644`. [CI #39375905332](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905332) passed hosted Windows Rust (including native AppContainer and MCP smokes), Linux Rust and dependency policy; hosted [native preparation #39375905331](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905331) also passed. The original head failed Clippy Windows due to an unnecessary `mut`, corrected without waiving checks. No PRs remained open after merging. The distinct [post-merge main CI #39376336498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376336498) was in progress at documentation time; check its actual final result.

Journal `append` no longer writes to hard-linked external evidence: Windows uses the existing exclusive no-reparse single-link open handle, seeks to EOF and syncs; Unix validates opened device/inode/link count against pre-open metadata. The size ceiling is rechecked on the opened file, and a cross-platform regression preserves evidence bytes when the file has an external hard link. No journal schema, authority or state transition changes. **This is not a solution** to Unix concurrent namespace/size races, Windows parent-directory pinning, staged artifact cleanup or general power-loss durability.

Windows Node/AppContainer 10s loopback and MCP first-response 30s intermittencies remain unresolved despite successful latest CI checks. Earlier failed attempts and exact-head retries remain evidence, not accepted causal fixes.

## Windows MCP startup timing evidence integrated — 2026-10-20

Fresh GitHub verification: `main` was `0f857b81a0715b2fcb7cb8f1e2a4250bf9221fa6`, with no open PRs; post-merge [CI 39370890039](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39370890039) passed on its second attempt. Its first attempt still contains a failed 30-second TEMP-only Node MCP initialization smoke; the separate AppContainer Node loopback 10-second intermittency also remains unresolved. These are not proven journal regressions.

[PR #280](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/280) was squash-merged as `8b6ccdc2270f8fa130df5d182d728d07466df908` from exact head `ce15d036332d314e7f951042285d4f442d0219a7`, which passed [hosted CI 39373512931](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373512931) (Rust Windows, Rust Linux and dependency policy). The separate post-merge [main CI 39373992973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373992973) was still in progress at this checkpoint. The scoped change provides opt-in monotonic startup phase timestamps on the bridge's stderr and host-side process-start/initialize timings for the existing GitHub-hosted Windows TEMP-only doctor. Static phase labels contain no path, credential, permissions or MCP payload. Ordinary bridge launches are unchanged unless `OPTIC_STARTUP_TIMING=1` is explicitly selected by the host. The diagnostic does **not** change 10s/30s timeouts, grant/isolation rules, CI acceptance conditions or the Rust core's optional language dependencies. The Windows TEMP-only doctor and Rust opt-in predicate test passed on the exact PR head. Its host-side timings on one passing Windows run were process-started=7ms, initialize-sent=30ms, initialize-received=4823ms (each from the doctor stopwatch). A successful diagnostic run does **not** prove or repair the cause of earlier timeouts.


## Recovery journal read and retirement hardening integrated — 2026-10-20

- [PR #277](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/277)
  was squash-merged into `main` as
  `c2282ff5eba85d2a7480dd353db638a0623bd06d`.
  Its final source head `825bcdb963d80ac8419df887e8b8c307aa497caf`
  passed hosted Windows Rust, Linux Rust, dependency policy
  ([CI 39368379416](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379416))
  and native preparation
  ([run 39368379422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379422)).
  An earlier head failed rustfmt and was corrected; canceled intermediate runs
  are not counted as passing. The independent post-merge `main` CI
  [39368741554](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368741554)
  completed successfully on exact post-merge `main` (Windows Rust, Linux Rust,
  dependency policy).
- Read hardening rejects hard-link journal aliases on both hosted platforms:
  Windows uses the existing exclusive no-reparse, single-link read primitive;
  Unix validates the opened descriptor's identity and link count against
  pathname metadata. This is not a complete race/power-loss guarantee.
- [PR #278](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/278)
  squash-merged as `b15fe7636dcb31033d71ecdfea9ed51bdc786a2b`
  from exact final head `9778b497320d82db493bfbe4c02ff13d113c8ad5`.
  Hosted [CI 39369331802](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331802)
  passed Windows Rust, Linux Rust and dependency policy; hosted
  [native preparation 39369331816](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331816)
  passed. An earlier PR head failed rustfmt and was corrected, not waived.
  Windows journal retirement now uses an exclusive validated handle for
  deletion. Unix rejects aliased evidence before the existing pathname unlink;
  a concurrent pathname replacement remains possible. The separate post-merge
  `main` [CI 39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540)
  initially failed its Windows Rust test on the **known intermittent**
  `capability_free_appcontainer_denies_node_loopback_connection`: owned Node
  returned `TimedOut` after 10 seconds with no output. Three immediate
  serial diagnostics passed. The original first attempt is preserved and
  the failed-Windows-job-only second attempt on the identical `main` SHA
  **passed** all applicable Windows tests and smokes. Linux/dependency jobs
  passed on the original attempt. CI #39369706540 is therefore completed
  successfully on attempt 2, with the original failure retained as evidence.
  There were zero open PRs immediately after the merge.
- Next priority after this recurrent failure: isolate native AppContainer/Node
  startup and scheduling with bounded diagnostics **without** increasing the
  timeout or disabling the network-isolation proof. A separate future domain
  review is journal appends and their opened-file identity, followed by staged
  artifact cleanup or directory pinning, with demonstrable defects and tests.
  Append, staging cleanup, power-loss durability and Unix pathname deletion
  races are not covered by #277/#278. Experimental alpha remains separate.
  No operator-PC or self-hosted execution occurred.

## Final GitHub-verified continuation checkpoint — 2026-10-20

- At this checkpoint, `main` = `e222ff912ac784eb7ae29d09e74ae1780f8a051b` (squash merge
  [PR #275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/275)).
  GitHub reported **zero open pull requests**.
- The exact post-merge `main` CI
  [run 39364949397](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39364949397)
  completed successfully: GitHub-hosted Windows Rust, Linux Rust and dependency
  policy all succeeded. This confirms the state of that commit, **not** an
  unconditional guarantee for future branch heads.
- Post-merge `main` CI for #274
  [run 39364345041](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39364345041)
  also completed successfully. #273 post-merge run 39363288153 succeeded.
- Source/test outcome remains narrowly scoped: six unchanged Rust source files
  reorganized under `src/mutation/`; the recovery parser now rejects complete
  blank journal records, retaining evidence while tolerating torn final lines.
  The first-attempt AppContainer/Node loopback timeout on #274 remains a separate
  unresolved intermittent failure despite a successful exact-head rerun.
- `alpha/generated-project-editing` remains a separate experimental branch;
  no full alpha-to-main merge, release, branch bulk-deletion, local operator-PC
  test, installer, tunnel or self-hosted run occurred.

**Next bounded review:** mutation recovery journal open/read/retire boundaries
and evidence retention, or a dedicated investigation of Windows Node/AppContainer
cold-start timing. Read [GITHUB_HANDOFF](docs/engineering/GITHUB_HANDOFF.md)
and [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md) before changing code.

## Mutation and recovery stabilization — 2026-10-20

- [PR #273](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/273)
  merged into `main` as `58ebc5c1a4f0fd015b26c04f8cd83cf4255d5782`.
  Six unchanged Rust source blobs were relocated into `src/mutation/`;
  their original crate-root module names, public exports and inline tests
  remain unchanged. Exact source head `9758492` passed all four applicable
  hosted Windows/Linux Rust, dependency and native-preparation checks;
  post-merge `main` CI 39363288153 passed.
- [PR #274](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/274)
  merged as `58a72a202bbdb57da425fba461a7b1a97b2f9d92`.
  The recovery journal parser now fails closed on a complete empty journal
  record while still allowing a nonterminated torn final record. Regression
  tests cover interior/trailing blank records, retained journal evidence and
  torn-tail classification. The first source head `8d51d19` failed rustfmt
  and was corrected. Exact final head `283c9e0` passed Linux, dependency
  policy and native preparation; Windows test failure on the first CI
  attempt was an AppContainer/Node loopback probe timeout, **not** a parser
  failure. Three serialized diagnostics passed on the same source, and a
  failed-Windows-job-only rerun passed the entire applicable Windows job.
  Both attempts are retained in CI run 39363540900. The cause of that
  intermittent native test remains unknown and is tracked in
  [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md).
  Post-merge `main` CI run 39364345041 subsequently completed successfully
  on hosted Windows/Linux and dependency policy.

No operator-PC execution, install, permission change, tunnel or self-hosted
runner. These changes do not establish general Cargo/Electron support,
durable disk quotas, power-loss ACID recovery or a complete source audit.

## Latest reviewed stabilization — 2026-10-20

- [PR #270](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/270)
  merged as `636a8b5e917fd731dbd8566e118958f141f67542`.
  Four session-owned modules were moved byte-identically into
  `crates/optic-bridge-runtime/src/session/`. Crate-root module names,
  test namespaces and public exports did not change. Final source head
  `4d5ff0a` passed GitHub-hosted Windows/Linux Rust, dependency policy
  and native preparation checks; post-merge main CI run 39360825623 passed.
- [PR #271](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/271)
  merged as `8610c8dc2558695842412c5a0fce993d2420d6fb`.
  Unexpected stdout/stderr Tokio drain task join failures are now treated
  as incomplete output, not silently discarded. Two synthetic regressions
  cover task panic and intentionally aborted drains after
  `TerminationUncertain`. Final source head `fb9ffdb` passed four hosted
  Windows/Linux Rust, dependency and native preparation checks. The initial
  `bed3ca9` head failed rustfmt and was corrected. Separate post-merge
  main CI run 39361585398 passed.
  
No tests/builds/installers/permissions/tunnels were executed on the operator's
PC; no self-hosted runners. These increments do not prove normal Cargo,
sandboxed Electron, general disk quotas, universal process logging durability
or completion of an exhaustive source audit.

## Offline Cargo research integration — 2026-10-20

PR255 is merged as `f1e72f8`. It completes the fixed research fixture and optional
preparation design, not general Cargo or administrator setup. At final head
`6bbcb64`, research [run 39339860948](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39339860948)
passed prepared job118358440446, including unrelated-profile denial, known-output
cleanup and explicit revoke. Independent normal job118358440142 retained NUL
error5 / exit101. Standard direct-Rust CI is unchanged. See the
[research owner](docs/research/PRIVATE_OFFLINE_CARGO_NUL_2026-10-19.md).


Reviewed: 2026-10-20. This page describes source and measured acceptance, not a
production release. Detailed historical notes are retained in
[the exact previous status snapshot](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/e1b82888dcf9c079740777be27341ca943758859/STATUS.md).

Current continuation is [GitHub-only](docs/engineering/GITHUB_HANDOFF.md): no
operator-PC testing or runtime changes. The [new-chat prompt](PROJECT_CONTINUITY_PROMPT.md)
and [agent entry point](AGENTS.md) do not require old chat memory or local files.

## Current structural delivery and Windows smoke evidence

[PR #268](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/268)
merged to `main` as `d79d60b88b60b2e95c85039c2d31427d0954a865`.
The ten prior `src/git_*.rs` modules are now under `src/git/`, with
identical content hashes and unchanged crate-root module names and
public exports. Exact PR head `b2b5207` passed four hosted checks:
Windows/Linux Rust, native preparation and dependency policy. This is a
mechanical organization improvement, not new behavior or an audit of all files.
Post-merge `main` CI run 39358651176 passed; the subsequent #269
post-merge main CI 39359157209 also passed.

Earlier [run 39356540735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39356540735)
on `main` initially failed the Windows TEMP-only Node MCP doctor smoke
after a 30-second response timeout with the bridge still alive. Re-running
only the failed Windows job at the exact same SHA succeeded, without a
source or workflow change. Root cause is not proven; the failure remains
recorded in [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md).

## Merged and validated

PR266 merged into main as `5a8157e`: a targeted process stdout/stderr
reader-error correction no longer treats a read failure as clean EOF.
A distinct private read-failure bit marks output incomplete, stops a running
owned job through the existing bounded confirmation path, and converts an
otherwise successful exit to `Failed` after pipe drains complete. Timed-out,
stopped and termination-uncertain jobs retain their specific status. Two
synthetic async-reader regressions were added. Final PR head `4e4c712`
passed hosted Windows/Linux Rust, dependency policy and native preparation
checks; the prior revision failed rustfmt and was corrected. This is not a
general guarantee that every process log is durable or fully collected.
[PR #266](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/266).
The separate main push CI run 39356012838 was still executing at the
documentation checkpoint; check its live result.

PR265 merged into main as `a9d9a31`: the `process.rs` test body
was moved unchanged into `process/tests.rs`; production logic remained
byte-identical, with 27 tests preserved. Exact head `f65f69b` passed four
hosted Windows/Linux Rust, dependency-policy and native-preparation checks.
This is structural code quality, not a runtime security fix. The separate
process-output diagnostic issue and its regression work are tracked by
[CODE_QUALITY_REVIEW](docs/engineering/CODE_QUALITY_REVIEW.md); an unmerged
fix is not counted as shipped behavior.

PR263 merged into main as `bcb5e8a` after exact head `2d8a410` passed four applicable hosted checks (Windows/Linux Rust, dependency policy, native preparation). It extracts unchanged inline project-input test logic into a child test module and introduces the scoped [code quality register](docs/engineering/CODE_QUALITY_REVIEW.md); it is **not** an exhaustive file audit or a behavioral improvement. [PR #263](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/263).

PR262 merged into main as `bca2eae` after its exact head `14b978b` passed all six applicable hosted checks (Windows/Linux Rust, dependency policy, native preparation, lifecycle and direct Rust). It adds a host-only, version-pinned retirement API for strictly closed private task evidence, with no model-visible deletion, automatic replay or operator workflow. This does **not** solve general retention UX or eliminate unresolved recovery capacity. [PR #262](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/262). No operator-PC execution. A separate source-organization review is tracked in [the quality register](docs/engineering/CODE_QUALITY_REVIEW.md); extracted tests require their own exact-head CI proof.


PR259/`97f4098` adds lossless Windows path boundary checks and a native
regression for a reproduced alias read across distinct non-Unicode roots.
Git path comparisons/arguments and operator-policy placement receive the same
representation audit. See [ADR-0033](docs/decisions/ADR-0033-LOSSLESS-NATIVE-PATH-BOUNDARIES.md)
and the [reviewed debt register](docs/engineering/RELIABILITY_DEBT.md); this is not
a claim that every filesystem or compatibility gate is closed. Exact
CI39338661674 and preparation39338661374 passed before merge.

| Track | Latest delivered increment | Evidence |
| --- | --- | --- |
| Main core | Native path hardening #259; fixed offline Cargo research #255 | #259 merged `97f4098`; #255 head `6bbcb64` passed CI 39339859057, preparation 39339859114, pipes 39339859071 and direct Rust 39339859055; merged `f1e72f8`. Research normal failure remains separate. |
| Generated FS alpha | Completed Electron diagnosis #250; hardened-core ancestry alignment #260 | #260 head `a4091bc` passed CI 39340735293, adapter/package 39340735266, preparation 39340735187, pipes 39340735200 and direct Rust 39340735265; merged `5245e3d`. Cargo/Electron research baselines remain red. |
| Previously verified alpha package | Source CI merge `ac76efd6ea833e46dfced7bee3148af968754566`, artifact 12229445645 | 85 archive entries and 80 source files verified; native packaged 32 KiB HTTP editing and Node fail/fix/check/test passed before the no-PC continuation. |

The alpha ZIP SHA256 is
`3563bb8e9666455e6559cf6e30362b99849eb437fe91769fc1b6691c2113bc00`.
Its source identity remains fixed; a documentation change does not relabel it.
It uses generated projects, shared connection authority and public dummy auth.
The older `b2b34a47…` package predates #259 and is historical. Actions artifacts
can expire; inspect availability in [run 39340735266](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39340735266)
instead of assuming a local ZIP is accessible to a new agent. No production
release or new PC test is implied by this provenance update.

The core includes scoped/versioned filesystem mutation, separately authorized
Git operations, bounded process lifecycle and private-service foundations.
Not every internal service is exposed as a general MCP project/build tool.

## Compatibility evidence and remaining limits

- **Electron #250:** bounded diagnostic scope merged into alpha as `e62ea95`.
  At head `62f8177`, ordinary CI/adapter/preparation/pipes/Rust passed. The
  staging rerun job118357480984 passed both diagnostic controls and observed
  normal exit `0x80000003` / primary result11 before app startup. The separately selected outer-only diagnostic succeeds; no automatic
  sandbox downgrade or public GUI permission follows. The baseline research
  staging workflow remains red, including run39340735239 on #260; its controls
  and separately selected outer-only comparison do not change normal support.
- **Cargo #255:** ordinary capability-free execution fails opening NUL, exit 101.
  At source `21ac09b`, disposable CI 39291321987/job 118192403768 passed the prepared
  exact-profile comparison, actual checks/tests, output cleanup and explicit
  revoke with restored denial. This is not ordinary Windows Cargo acceptance.
- **Optional Windows preparation:** development direction approved; the proposed
  contract is [in main](docs/research/OPTIONAL_WINDOWS_NUL_PREPARATION.md). No local privileged helper or device permission change
  has been applied. Authentication, lease provenance and interrupted recovery
  remain implementation gates.
- **Real projects:** writable output containment, dependency restore, secrets,
  enrollment and artifact publication require their own proven contracts.
- **FS failures:** retained local evidence cannot expose FS backend logs or
  restore hidden model state; unknown writes must be observed before retry.

## Repository consolidation

41 historical PRs were closed with explicit disposition. Their work remains in
alpha ancestry, exact GitHub pull refs or published archive tags. The later #250
and #255 integration completes their bounded measurement scopes; unresolved
compatibility work remains in this status and the debt register, rather than
indefinitely open diagnostic carriers.

244 historical remote branches were removed after explicit operator approval,
exact-head/preservation revalidation and backup hash verification. The four
reference branches and all preservation refs were verified unchanged at that
retirement checkpoint. After later integrations #250/#255/#260, the pre-handoff
inventory was zero open PRs and two branches: `main` and
`alpha/generated-project-editing`; completed short-lived branches were retired.
This dated inventory must be refreshed live by the next agent. Exact historical
heads, approval and observed results are in the
[consolidation record](docs/engineering/REPOSITORY_CONSOLIDATION_2026-10-20.md).

Next delivery steps are owned by [ROADMAP.md](ROADMAP.md). Core and alpha remain
separate until their applicable release gates are satisfied.
