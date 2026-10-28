# Native GitRead registered-lease witness — proposed contract (#317)

## 28 October 2026 — original lease stop before admission drainage

Baseline `main=eea5438873293243cd747b0f646ed903fb9f1282` integrates [#420](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/420); independent [push/main #40752232971](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40752232971) verified SUCCESS 4/4, identical tested/integrated trees.

The next targeted regression holds a genuine counted session admission during lifecycle revoke. Original session stop is already published before drainage, but the original TaskLease stop is currently published only afterward. A lease-only observer can therefore wait for a stop that depends on its own admission being released. The test-first candidate preserves that order to obtain behavioral RED.

Required correction: publish irreversible lease revoke flags before waiting for session admission drainage, retain the lease records/pins and counted resources, then repeat the lease sweep after drainage to catch registrations by already-admitted producers. Sum newly revoked counts from both sweeps without double counting; attempt all cleanup and propagate errors. This is stop publication, **not** destruction or physical process termination.

The real-registry test checks both original stop signals while the permit is held, failed early removal due to in-flight admission, revoke still waiting, a late registration caught after drain, exact revoke counts and eventual no-child reaping. No process is created; no policy/network/timeout relaxation. Final corrected-head and independent-main evidence must be checked before claiming integration. **Windows V1 NO-GO.**

See the new [current continuation](WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md) (from engineering pages: `WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md`) for the next host/policy authority boundary and exact remote-check procedure.


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

**Critical nonclaims:** no actual service-owned GitRead process, descendant-retained writers or bounded timeout with descendants, native owner/Job uncertainty recovery, shared session/TaskLease/policy/host commit/stop ordering or safe Win32-call stall handling. `HardenedCommandRunner` remains live for MCP GitRead with known potentially unbounded reader joins. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is administratively closed, not behavioral acceptance. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN: Windows V1 NO-GO**; #185 and #319 remain separate blockers. [Native authority audit](ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md) is still the required next technical design/proof. No release or PC changes.

## 28 October 2026 — 28 OCTOBER 2026, Windows Job resumed-root proof #406

**Verified integration:** [PR #406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/406) is **squash-merged**. Tested final HEAD `f4c7b8163d36434f5a26486625d4963c42a0dee8`, integrated `main=be59544537d5dc027b54e4bb5cd7f553f979d59d`, identical trees `f4d702915e429821bd658d6317014e0cd316241a`. [PR CI #40672893075](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893075): **4/4 jobs SUCCESS, first attempt**, exact final HEAD. Same-HEAD hosted Windows workflows [Owned Native Process Pipes #40672892946](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672892946), [Private Rust Compilation #40672893056](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893056), and [Private Project Preparation #40672893094](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893094): all SUCCESS. Independently triggered [push/main #40673602054](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40673602054) on the exact integrated SHA; **verify its completed/final four job conclusions live** before using it as post-merge proof.

**One test-only Windows file**, `crates/optic-bridge-windows/src/job_list_launch_tests.rs`: `job::job_list_launch_tests::resumed_root_exits_and_kernel_job_drains_after_job_list_creation ... ok` in hosted Windows Rust log. A fixed synthetic Windows `cmd.exe /D /Q /C exit /b 0` is created suspended, already Job-associated via `PROC_THREAD_ATTRIBUTE_JOB_LIST` at creation, then actually resumed; root exit code 0 and **separate physical Job-empty** observation are bounded. Original PR CI failed `rustfmt` on an earlier HEAD, corrected and preserved as a formatting failure, **not** a launch/liveness failure.

**NOT demonstrated:** no service-owned native GitRead launch/MCP switch; no descendants, stdout/stderr EOF, independent host/session/TaskLease/policy cross-authority final commit, Win32 blocking stop, uncertainty guardian, or legacy runner elapsed-time correction. Sessions and leases still issue independent receipts under different mutexes. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN**; [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) closed administratively, original pipe liveness not shown resolved; #185/#319 open. **Windows V1 remains NO-GO; no release.** Follow the concrete mutator/lock/host audit in [#323 native authority audit](ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md); do not confuse static policy epoch equality with a live cross-authority issuer. Previous dated sections are historical snapshots.

## 28 October 2026 — #404 verified session close between and after two receipts

[PR #404](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/404) : HEAD testé `cc7a77b3b5415c6b8d601c477c2373e39f84f7b6`, squash intégré `main=0d14502da39d90ccecd8f113981cd3bdb6520159`, arbre source/intégré identique `55bd4edc12e645ddc3e27a1cb799db932208cec0`. CI [PR #40666361354](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361354) **4/4 SUCCESS sur HEAD corrigé**, [native #40666361300](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361300) SUCCESS, et **CI indépendante push/main [#40667115232](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40667115232) `completed/success` 4/4 jobs, tentative 1, sur SHA intégré exact**.

**#404 ne modifie que les tests Windows** (`crates/optic-bridge-runtime/src/git/native_status_tests.rs`, +189 lignes) : `session_close_between_original_session_and_lease_receipts_denies_native_custody` et `dual_receipt_session_close_after_lease_commit_drains_and_stops` ressortent **`... ok`** dans les logs Windows. Le premier scénario prouve qu'une fermeture lifecycle de session *entre* les reçus session et TaskLease interdit de poursuivre malgré une TaskLease encore active. Le second prouve qu'une révocation directe *après* les deux reçus publie le stop de génération d'origine, attend la libération du permis compté, et conserve le Job Windows **vide** et les quotas jusqu'au drop. Aucun lancement Git.

Ces tests établissent uniquement stop observable et conservation d'ownership : le mutex session et le mutex TaskLease sont distincts, la deuxième lecture positive n'est pas une autorisation OS ; le lifecycle révoque la session avant ses leases. Aucune preuve d'arrêt après lancement réel.

**Limites :** les reçus session (#400) et TaskLease (#402) sont émis sous **deux mutex indépendants**, et ne constituent PAS un fence atomique pour policy epoch, host shutdown, `CreateProcessW`, `ResumeThread` ou publication. Aucune preuve après exécution Job vide/descendants, stdout/stderr EOF, watchdog borné ou récupération de terminaison incertaine. MCP GitRead reste dans la voie legacy `HardenedCommandRunner` (pipe joins non prouvés bornés). [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est **CLOSED administrativement**, sans preuve physique suffisante du pipe-join ; [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323), [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) **OPEN**. #352 fermée non fusionnée ; #355 recherche draft. **Windows V1 NO-GO, aucune release.**

[Prompt de reprise complet](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-28.md).


## 28 October 2026 — #402 original TaskLease close-lock receipt, no execution

[PR #402](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/402): exact tested HEAD `04a0c025202dbf2dfbd8cf43d3314c64260e6bee`, squash `main=fc53da251ff1c6b396fcdddc8092b9d18712cfd9`; identical tree `d287df495e1543e8d04e083651a34fd9ed641aa6`. Exact-head [CI #40663011191](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011191) **SUCCESS 4/4** and [specialized native #40663011174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011174) **SUCCESS**; independent [push/main #40663744586](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663744586) **SUCCESS 4/4 on exact merged SHA**.

The Windows-only, non-executing native GitRead candidate now retains **two separately issued** original-generation stop receipts: session (#400) and TaskLease (#402). The TaskLease receipt is minted under the same `TaskLeaseRegistry.leases` mutex as direct/session revoke, physical removal and registration, requires exact original registration-signal identity and preserved admission-time expiry. Four hosted Windows tests prove stop after revoke, `revoke_session` and `remove_session`, same-ID reuse and foreign registry rejection, post-renewal deadline, preserved real **empty** Job and counted permits. Foreign/reused registration fails closed with existing `Revoked` (no new public MCP error variant).

The TaskLease close lock orders only the original *lease generation* with `revoke`, `revoke_session`, `remove_session` and lease registration. The existing session close lock is **separate**. Receipt issuance retains original watches and deadlines for stop/cleanup, not native launch authority. After either lock releases, its authority can be revoked; a policy or host shutdown can also change independently. **No atomic cross-authority commit, no child creation, no terminal receipt** follows from successful tests.

**No-go:** sequential session and TaskLease receipts are **NOT** an atomic session/lease/policy/host effect fence or CreateProcess permission. There is NO launched native Git root, no descendant process/Job/stdio EOF observation, no watchdog or uncertain-termination cleanup proof, and no MCP GitRead migration. Legacy runner is unchanged. GitHub [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is currently **closed**, but its described pipe-join bounded-wall-time acceptance has NOT been established by these no-child tests; closed/unmerged #352 is not evidence of resolution. [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN — Windows V1 NO-GO**, no release, no operator-PC or other project changes.


## 28 October 2026 — #400 original-session commitment of a registered **unstarted** owner

[PR #400](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/400) tested HEAD `a949fbf1745de0f97e03284628967c30b2fa887e`, squash integrated `main=67e3604c198f83c16465863facdf2fa855f983d7`, identical tree `444a3175b7241d85d4e959de456f8fa4f1341ae0`. [PR CI #40658720479](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658720479) **SUCCESS 4/4 first attempt**, [native preparation #40658719448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658719448) SUCCESS, and independent [push/main #40659396121](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40659396121) **SUCCESS 4/4, attempt 1 on the exact integrated SHA**.

On Windows, the already-registered, **non-executing** native status owner may consume its precreated Job/pipes/pins into a **session-only** original-generation commit receipt issued under the same session mutex as direct and lifecycle revocation. The receipt captures the original stop watch and real private Job ID. Hosted Windows tests prove commitment preceding direct revoke retains original admission/transport/Job until drop and observes stop; direct or lifecycle close winning first prevents commitment. Both tests report `ok`.

The receipt is **session-only** and non-cloneable. It is minted under the same mutex as both direct and lifecycle close; the original watcher persists after that lock is released. There is no actual process-create call, no TaskLease/policy epoch serialization, no transferable spawn permission and no stdout/stderr terminal proof. An `Ok` receipt cannot by itself authorize any native effect.

This is **NOT** a native launch grant or the final GitRead commit. TaskLeaseRegistry and policy epoch revocations remain independent; there is no CreateProcess/ResumeThread, actual Git process/root+descendant/stdio EOF, bounded guardian, termination uncertainty proof or production MCP GitRead switch. #352 was closed **without merge**, not accepted as proof; #355 behavioral acceptance remains unmet. #317/#323/#185/#319 remain OPEN. **Windows V1 NO-GO, no release**.


## 28 October 2026 — #398 original session close-lock observation (NOT a commit ticket)

[PR #398](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/398) (exact tested HEAD `7456f3d96dc289eb5e278e3d9acbfce8970a6dcf`) merged into `main=8aa6fd205e7a264e7873e6af6b627ddb81207d9d`. Trees identical `8883d4c0ee6a3647b543799c29f9b573be93a028`. Exact-head [CI #40655468052](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468052) **SUCCESS 4/4**, [native preparation #40655468078](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468078) **SUCCESS**. Independent [push/main #40656160001](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40656160001) **SUCCESS 4/4 on exact merged SHA, first attempt**. First candidate's [CI #40655374591](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655374591) failed rustfmt, not #317 behavioral acceptance.

The Windows-only, internal `SessionRegistry::observe_original_owned_admission_under_close_lock` rechecks an owned admission's original registry, registration-generation signal, revoked bit and captured expiry under the **same mutex** as direct `revoke` and lifecycle `publish_revocation_before_drain`. The precreated, **non-executing** native GitRead custody now invokes it both before and after its independent lease/policy snapshot. Windows tests `original_owner_observation_orders_with_direct_and_lifecycle_close` and `original_owner_observation_rejects_foreign_registry_and_renewed_deadline` passed on exact HEAD. No OS IO or policy work is performed under that session lock.

A completed observation is temporary: as soon as the mutex releases, direct session revoke, lifecycle close, independent TaskLease revoke and policy changes can race. Repeating it before and after lease validation narrows the stale-session observation window, but does **not** give a linearized cross-registry commit or real native execution. Neither the original session nor lease watch returning `false` grants launch authority.

**Nonclaims:** the mutex is released after this observation: it is not an owner **commit** or CreateProcess permit, nor a cross-registry TaskLease/policy epoch fence. Session or lease revoke may still race native creation. No real GitRead root, native child/descendant/stdio EOF, out-of-band stop, crash/uncertainty recovery or MCP switch is implemented; real #352/#355 acceptance remains RED. #317/#323/#185/#319 remain OPEN; **Windows V1 NO-GO**, no release or operator-PC access.


## 28 October 2026 — latest Windows witness / failure-path evidence (#395, #396)

**Original-TaskLease generation retention #395:** [merged PR #395](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/395) test-only at HEAD `ec672e3e1320018f5d1a51d4b4de55a085d9bb52`, squash `4b32029b3adc2718ecec109bfae791fb9fd228f4`, identical tree `18b50fe128ecf7aa901201d2995c9d7fb1cbe755`; [PR CI #40651259497](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651259497) SUCCESS 4/4, [native preparation #40651259506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651259506) SUCCESS and [independent push/main #40651850790](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651850790) SUCCESS 4/4, all exact SHA. Windows test `prelaunch_snapshot_never_rebinds_after_original_lease_id_reuse` exercises revoke/remove/re-register of identical opaque TaskLease ID: the new lease may independently be valid, but the old nonexecuting native owner remains stopped and retains its original permits, quotas, and empty Job until safe no-child disposal. The first #395 candidate's format error on [#40651188973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651188973) was formatting only, not a behavioral reproduction.

**Physical #396 scope:** the research-only fixture configures a genuine empty Windows Job and `PROC_THREAD_ATTRIBUTE_JOB_LIST`, calls `CreateProcessW` for a deliberately absent image under `SystemRoot/System32`, verifies **Win32 ERROR_FILE_NOT_FOUND** (not an attribute/permission error) and a **still-empty kernel Job**. A scoped guard would terminate any unexpected suspended child. It never resumes a child or launches Git. Neither #395 nor #396 changes production, authorizes native GitRead, or proves a started process's terminal/pipe lifetime.

**Verified main after test-only #396:** `8ee1ad0b2c135af3d639ab753703c3f911dc4941`; independently green [push/main #40653860272](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653860272) **SUCCESS 4/4, attempt 1 on the exact integrated SHA**. #396 ([PR](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/396)) was tested at exact HEAD `ab927560f408ff3d21736ad5e817b5c0198d1424` ([CI #40653261422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261422) SUCCESS 4/4, attempt 1) with three successful specialized hosted Windows workflows [#40653261423](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261423), [#40653261517](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261517), [#40653261524](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261524). Squash HEAD/merge tree identical: `215295484f1fdcc4bbff780f94b1c8626b297eaf`. Windows log: `job::job_list_launch_tests::invalid_target_with_job_list_leaves_no_native_process ... ok`.

The original session and TaskLease witnesses in `PrecreatedNativeGitStatusCustody::observe_original_prelaunch_snapshot` remain **read-only, non-committing** observations; `false` for stop is not authority to start, resume or publish. A true synchronized owner-commit/revoke decision is still absent, as is a real native process owned through root+descendants/stdio termination. The negative `CreateProcessW` fixture proves only the missing-image failure path, not any later failure or crash.

**Still unproven (P0):** generation-bound owner commitment serialized against **both** direct and lifecycle session revoke; independent TaskLease/policy epoch fences; executable/repository authority at creation; real owned native GitRead root and descendant lifecycle; stdout/stderr EOF and stdin completion; bounded independent stop/watchdog; positive root-exit + Job-empty **after actual execution**; uncertain failure/crash capacity retention and safe result publication. Research acceptance #352/#355 remains RED. Issues #317/#323/#185/#319 are OPEN. **Windows 10/11 V1 remains NO-GO; no release, remote execution, Optic Desktop/REL work, operator-PC access, self-hosted CI or unrequested Codex/Work tasks.**


## 28 October 2026 — #393 Windows create-time Job membership proof (research test only)

On exact PR HEAD `14e73802b853c57b67c0b00bf169af6fd5a47793`, [#393](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/393) adds an isolated **kernel behavior test**: create a suspended, disposable `cmd.exe` using `PROC_THREAD_ATTRIBUTE_JOB_LIST` in `STARTUPINFOEXW`, assert `IsProcessInJob` and a real Job active-process count of one immediately upon `CreateProcessW` return, then request termination and observe root signaled plus Job active-process count zero under finite timeouts. No resume call, no live GitRead or MCP activation. The PR [CI #40649262906](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262906) passed all four jobs first attempt, as did specialized Windows workflows #40649262958/#40649262919/#40649262961. Squash integrated `main=c81c1f5caf937695b3bbd9439f508ff9cb28f6f4` with tree equality; separate [push/main #40649847690](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649847690) **completed/SUCCESS, attempt 1, all four jobs on exact integrated main SHA**.

**Nonclaims:** `CreateProcessW` internal crash window, Windows 10/11 deployment, session-commit-versus-revoke, policy/epoch and TaskLease generation locks, verified Git executable and repo during launch, resumed root, descendants retaining pipes, stdout/stderr EOF, `TerminationUncertain` or durable guardian/recovery are NOT proven by this fixture. The existing `observe_original_prelaunch_snapshot` remains a stop/identity observation, NOT launch authority. Keep #317/#323 open, #352/#355 acceptance unresolved and V1 NO-GO.

## Latest verification — #390 same-generation snapshot and #391 ancestor custody test (27 October 2026)

[Merged #390](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/390) adds `PrecreatedNativeGitStatusCustody::observe_original_prelaunch_snapshot`: an unstarted owner checks original `TaskLeaseRevocationWatch` record identity against current verified typed policy and pinned Git executable via `Arc::ptr_eq`, checks original session registry and original monotonic horizons, and retains original permits and empty Job/pipes/pins. **This is no synchronization with CreateProcessW and no spawn permit.** It refuses a foreign registry or same-ID replacement. Merge `main=7f663454039b294d84cd3fb2333cbbd73372f91a` passed [independent #40520286289](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40520286289), 4/4.

[Merged #391](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/391) added a **single physical Windows regression test**, not a production change: `pin_native_git_read_repository_root` already denies ancestor directory rename until the last pin closes. PR test HEAD `e7f684ae9c32b3d1026ef103d9832f8a7767a535`, [CI #40526579598](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40526579598) 4/4 SUCCESS, three native workflows SUCCESS, integrated `main=c208348f0b63d4810db97f9a5284b325c5b64271` [push/main #40553879389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40553879389) SUCCESS 4/4. No new pin behavior was needed.

Future authority must make a **synchronized creation-time** decision covering the already retained session, TaskLease generation, executable identity/content, canonical repository, policy epoch, monotonic action+transport deadline and revocation. The original Job/root/pipes/quotas/admissions must outlive uncertain termination until physical root-exit + Job empty **after execution** + actual stdout/stderr EOF. Only then MCP GitRead may switch while retaining sensitive-name policy. #352/#355 real acceptance remains RED, #317/#323 OPEN, Windows V1 NO-GO. After the present documentation merge verify its own push/main CI.


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
