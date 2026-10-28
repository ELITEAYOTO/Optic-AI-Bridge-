# GitHub-only project handoff

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

**Critical nonclaims:** no actual service-owned GitRead process, descendant-retained writers or bounded timeout with descendants, native owner/Job uncertainty recovery, shared session/TaskLease/policy/host commit/stop ordering or safe Win32-call stall handling. `HardenedCommandRunner` remains live for MCP GitRead with known potentially unbounded reader joins. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is administratively closed, not behavioral acceptance. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN: Windows V1 NO-GO**; #185 and #319 remain separate blockers. [Native authority audit](ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md) is still the required next technical design/proof. No release or PC changes.

## LATEST VERIFIED HANDOFF — 28 OCTOBER 2026, Windows Job resumed-root proof #406

**Verified integration:** [PR #406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/406) is **squash-merged**. Tested final HEAD `f4c7b8163d36434f5a26486625d4963c42a0dee8`, integrated `main=be59544537d5dc027b54e4bb5cd7f553f979d59d`, identical trees `f4d702915e429821bd658d6317014e0cd316241a`. [PR CI #40672893075](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893075): **4/4 jobs SUCCESS, first attempt**, exact final HEAD. Same-HEAD hosted Windows workflows [Owned Native Process Pipes #40672892946](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672892946), [Private Rust Compilation #40672893056](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893056), and [Private Project Preparation #40672893094](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893094): all SUCCESS. Independently triggered [push/main #40673602054](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40673602054) on the exact integrated SHA; **verify its completed/final four job conclusions live** before using it as post-merge proof.

**One test-only Windows file**, `crates/optic-bridge-windows/src/job_list_launch_tests.rs`: `job::job_list_launch_tests::resumed_root_exits_and_kernel_job_drains_after_job_list_creation ... ok` in hosted Windows Rust log. A fixed synthetic Windows `cmd.exe /D /Q /C exit /b 0` is created suspended, already Job-associated via `PROC_THREAD_ATTRIBUTE_JOB_LIST` at creation, then actually resumed; root exit code 0 and **separate physical Job-empty** observation are bounded. Original PR CI failed `rustfmt` on an earlier HEAD, corrected and preserved as a formatting failure, **not** a launch/liveness failure.

**NOT demonstrated:** no service-owned native GitRead launch/MCP switch; no descendants, stdout/stderr EOF, independent host/session/TaskLease/policy cross-authority final commit, Win32 blocking stop, uncertainty guardian, or legacy runner elapsed-time correction. Sessions and leases still issue independent receipts under different mutexes. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN**; [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) closed administratively, original pipe liveness not shown resolved; #185/#319 open. **Windows V1 remains NO-GO; no release.** Follow the concrete mutator/lock/host audit in [#323 native authority audit](ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md); do not confuse static policy epoch equality with a live cross-authority issuer. Previous dated sections are historical snapshots.

## LATEST VERIFIED HANDOFF — 28 OCTOBER 2026, after #404

[PR #404](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/404) : HEAD testé `cc7a77b3b5415c6b8d601c477c2373e39f84f7b6`, squash intégré `main=0d14502da39d90ccecd8f113981cd3bdb6520159`, arbre source/intégré identique `55bd4edc12e645ddc3e27a1cb799db932208cec0`. CI [PR #40666361354](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361354) **4/4 SUCCESS sur HEAD corrigé**, [native #40666361300](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361300) SUCCESS, et **CI indépendante push/main [#40667115232](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40667115232) `completed/success` 4/4 jobs, tentative 1, sur SHA intégré exact**.

**#404 ne modifie que les tests Windows** (`crates/optic-bridge-runtime/src/git/native_status_tests.rs`, +189 lignes) : `session_close_between_original_session_and_lease_receipts_denies_native_custody` et `dual_receipt_session_close_after_lease_commit_drains_and_stops` ressortent **`... ok`** dans les logs Windows. Le premier scénario prouve qu'une fermeture lifecycle de session *entre* les reçus session et TaskLease interdit de poursuivre malgré une TaskLease encore active. Le second prouve qu'une révocation directe *après* les deux reçus publie le stop de génération d'origine, attend la libération du permis compté, et conserve le Job Windows **vide** et les quotas jusqu'au drop. Aucun lancement Git.

[Première CI de #404 #40666050161](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666050161) : échec `rustfmt` seul sur premier SHA, corrigé ; Windows première tentative annulé par supersession. **Pas** un échec comportemental natif.

**Architecture :** le lifecycle publie le stop session avant drain alors que la TaskLease peut rester active. Les nouveaux tests démontrent l'ordre et la durée de vie des permis d'origine, non pas un lancement. `LeaseObservedNativeGitStatusCustody` n'est pas un ticket de création : le Job réel est vide parce qu'aucun child n'a été créé. Un statut `Failed` « no-child » n'est pas une preuve de terminaison OS d'un enfant.

**Limites :** les reçus session (#400) et TaskLease (#402) sont émis sous **deux mutex indépendants**, et ne constituent PAS un fence atomique pour policy epoch, host shutdown, `CreateProcessW`, `ResumeThread` ou publication. Aucune preuve après exécution Job vide/descendants, stdout/stderr EOF, watchdog borné ou récupération de terminaison incertaine. MCP GitRead reste dans la voie legacy `HardenedCommandRunner` (pipe joins non prouvés bornés). [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est **CLOSED administrativement**, sans preuve physique suffisante du pipe-join ; [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323), [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) **OPEN**. #352 fermée non fusionnée ; #355 recherche draft. **Windows V1 NO-GO, aucune release.**

**Reprise :** consulter [prompt complet Windows V1 du 28/10](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-28.md), vérifier main live + 4 jobs indépendants + issue #323 ; auditer issuer policy/host et liveness avant lancement natif. Aucune intervention sur PC, Desktop/REL ou releases. Ce checkpoint décrit le main **avant** cette PR de documentation ; revérifier SHA et CI indépendante après fusion.


## LATEST VERIFIED GITHUB HANDOFF — 28 OCTOBER 2026, #402

[PR #402](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/402): exact tested HEAD `04a0c025202dbf2dfbd8cf43d3314c64260e6bee`, squash `main=fc53da251ff1c6b396fcdddc8092b9d18712cfd9`; identical tree `d287df495e1543e8d04e083651a34fd9ed641aa6`. Exact-head [CI #40663011191](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011191) **SUCCESS 4/4** and [specialized native #40663011174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011174) **SUCCESS**; independent [push/main #40663744586](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663744586) **SUCCESS 4/4 on exact merged SHA**.

The Windows-only, non-executing native GitRead candidate now retains **two separately issued** original-generation stop receipts: session (#400) and TaskLease (#402). The TaskLease receipt is minted under the same `TaskLeaseRegistry.leases` mutex as direct/session revoke, physical removal and registration, requires exact original registration-signal identity and preserved admission-time expiry. Four hosted Windows tests prove stop after revoke, `revoke_session` and `remove_session`, same-ID reuse and foreign registry rejection, post-renewal deadline, preserved real **empty** Job and counted permits. Foreign/reused registration fails closed with existing `Revoked` (no new public MCP error variant).

The first hosted [CI #40662644378](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40662644378) detected rustfmt and a Windows MCP exhaustive-match failure from a proposed additional public error variant. The change was revised to reuse the existing fail-closed `Revoked` error; no MCP/public error changes were merged. Intermediate superseded runs were cancelled. No first failure reproduced #317/#355 liveness symptoms.

**Source architecture:** `TaskLeaseRegistry::commit_original_native_read_lease` takes the original `TaskLeaseRevocationWatch` under the existing lease mutex. Original `NativeGitReadLeaseWitness` is the only accepted source for this internal read. `SessionCommittedNativeGitStatusCustody::commit_original_lease` consumes the already registered, pinned, **unstarted** native Job/transport/session owner into `LeaseObservedNativeGitStatusCustody` with two stop witnesses. Never perform file hash, OS, policy callback or waiting operations under the TaskLease mutex; it only checks registry record identity/revoked/session/expiry. A positive receipt is **not a launch ticket** and can be stale after unlocking. Actual launch safety will require a separate, demonstrable issuer/fence across independently mutable policy and host authorities; do not fake atomicity by comparing two sequential receipts.

**No-go:** sequential session and TaskLease receipts are **NOT** an atomic session/lease/policy/host effect fence or CreateProcess permission. There is NO launched native Git root, no descendant process/Job/stdio EOF observation, no watchdog or uncertain-termination cleanup proof, and no MCP GitRead migration. Legacy runner is unchanged. GitHub [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is currently **closed**, but its described pipe-join bounded-wall-time acceptance has NOT been established by these no-child tests; closed/unmerged #352 is not evidence of resolution. [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN — Windows V1 NO-GO**, no release, no operator-PC or other project changes.

**Operational handoff:** inspect latest `main`, exact PR and independently green push/main CI before any change. Continue small scoped PRs, preserve first red attempts and test logs, validate Windows/Ubuntu and specialized hosted suites, verify merge tree equality. No releases, user PC, self-hosted runners or edits to Optic Desktop/REL.


## CURRENT VERIFIED HANDOFF — 28 October 2026, #400

[PR #400](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/400) tested HEAD `a949fbf1745de0f97e03284628967c30b2fa887e`, squash integrated `main=67e3604c198f83c16465863facdf2fa855f983d7`, identical tree `444a3175b7241d85d4e959de456f8fa4f1341ae0`. [PR CI #40658720479](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658720479) **SUCCESS 4/4 first attempt**, [native preparation #40658719448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658719448) SUCCESS, and independent [push/main #40659396121](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40659396121) **SUCCESS 4/4, attempt 1 on the exact integrated SHA**.

On Windows, the already-registered, **non-executing** native status owner may consume its precreated Job/pipes/pins into a **session-only** original-generation commit receipt issued under the same session mutex as direct and lifecycle revocation. The receipt captures the original stop watch and real private Job ID. Hosted Windows tests prove commitment preceding direct revoke retains original admission/transport/Job until drop and observes stop; direct or lifecycle close winning first prevents commitment. Both tests report `ok`.

The first test SHA encountered hosted rustfmt and Windows `-D warnings` dead-code diagnostics ([initial CI #40658301013](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658301013)); corrected with formatting and narrowly scoped, documented allowances on intentionally inert research methods, not global lint suppression. Superseded runs were cancelled; these were not reproductions of #317/#352.

**Boundary audit:** `SessionRegistry::commit_registered_native_owner_session` checks exact original registry and per-registration revocation `Arc`, unrevoked session and captured deadline *inside the shared session close mutex*, and emits a private, noncloneable receipt for the private Job ID. The only service path to it consumes a precreated, already accounted `PrecreatedNativeGitStatusCustody` after typed lease/policy snapshot. The original Job, stdout/stderr pipes, executable/repository pins, session and transport permits remain owned. Drop before any child is created safely records a no-child failure; it is **not** terminal proof of a process that ran. The session close can race after receipt issuance and must be observed through its original stop signal. No cross-registry atomicity is inferred.

This is **NOT** a native launch grant or the final GitRead commit. TaskLeaseRegistry and policy epoch revocations remain independent; there is no CreateProcess/ResumeThread, actual Git process/root+descendant/stdio EOF, bounded guardian, termination uncertainty proof or production MCP GitRead switch. #352 was closed **without merge**, not accepted as proof; #355 behavioral acceptance remains unmet. #317/#323/#185/#319 remain OPEN. **Windows V1 NO-GO, no release**.

**Next actions:** audit `TaskLeaseRegistry.leases`, policy epoch and independent shutdown mutation paths, lock-order/liveness under stalled Windows calls, final sealed issuer/fence and out-of-band stop before invoking a real `CreateProcessW`. Keep #355 acceptance and original pipe-join elapsed-time defect open as behavioral blockers; do not close #317 or #323. No Optic Desktop/REL, user-PC, self-hosted runner, release or Git mutation scope. Review exact-head and independently verified integrated `main` CI for every subsequent patch.


## LATEST VERIFIED GITHUB-ONLY HANDOFF — 28 October 2026, #398

[PR #398](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/398) (exact tested HEAD `7456f3d96dc289eb5e278e3d9acbfce8970a6dcf`) merged into `main=8aa6fd205e7a264e7873e6af6b627ddb81207d9d`. Trees identical `8883d4c0ee6a3647b543799c29f9b573be93a028`. Exact-head [CI #40655468052](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468052) **SUCCESS 4/4**, [native preparation #40655468078](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468078) **SUCCESS**. Independent [push/main #40656160001](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40656160001) **SUCCESS 4/4 on exact merged SHA, first attempt**. First candidate's [CI #40655374591](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655374591) failed rustfmt, not #317 behavioral acceptance.

The Windows-only, internal `SessionRegistry::observe_original_owned_admission_under_close_lock` rechecks an owned admission's original registry, registration-generation signal, revoked bit and captured expiry under the **same mutex** as direct `revoke` and lifecycle `publish_revocation_before_drain`. The precreated, **non-executing** native GitRead custody now invokes it both before and after its independent lease/policy snapshot. Windows tests `original_owner_observation_orders_with_direct_and_lifecycle_close` and `original_owner_observation_rejects_foreign_registry_and_renewed_deadline` passed on exact HEAD. No OS IO or policy work is performed under that session lock.

**Lock/authority decision still outstanding:** `SessionRegistry.sessions` closes direct and lifecycle admission atomically *only for its own generation*, while `TaskLeaseRegistry.leases` and policy epochs have **independent** mutation and revocation points. A pair of valid snapshots cannot be silently treated as an atomic commit. No new public revoke return semantics, process launch or release setting is authorized. Preserve owner-held permits, process quota and kernel Job until *positive* terminal OS and pipe proof; failed observation must remain uncertain, not free capacity.

**Nonclaims:** the mutex is released after this observation: it is not an owner **commit** or CreateProcess permit, nor a cross-registry TaskLease/policy epoch fence. Session or lease revoke may still race native creation. No real GitRead root, native child/descendant/stdio EOF, out-of-band stop, crash/uncertainty recovery or MCP switch is implemented; real #352/#355 acceptance remains RED. #317/#323/#185/#319 remain OPEN; **Windows V1 NO-GO**, no release or operator-PC access.

Before any further code: inspect current `main`, exact SHA/first attempts, existing #327/#352/#353/#355/#358 research branches and accepted source/CI. Continue with small reviewed PRs, Windows/Ubuntu and hosted native checks, and independently verify every squash on exact integrated `main`.


## CURRENT VERIFIED HANDOFF — 28 October 2026, #394/#395/#396

**Verified main after test-only #396:** `8ee1ad0b2c135af3d639ab753703c3f911dc4941`; independently green [push/main #40653860272](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653860272) **SUCCESS 4/4, attempt 1 on the exact integrated SHA**. #396 ([PR](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/396)) was tested at exact HEAD `ab927560f408ff3d21736ad5e817b5c0198d1424` ([CI #40653261422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261422) SUCCESS 4/4, attempt 1) with three successful specialized hosted Windows workflows [#40653261423](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261423), [#40653261517](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261517), [#40653261524](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261524). Squash HEAD/merge tree identical: `215295484f1fdcc4bbff780f94b1c8626b297eaf`. Windows log: `job::job_list_launch_tests::invalid_target_with_job_list_leaves_no_native_process ... ok`.

**Original-TaskLease generation retention #395:** [merged PR #395](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/395) test-only at HEAD `ec672e3e1320018f5d1a51d4b4de55a085d9bb52`, squash `4b32029b3adc2718ecec109bfae791fb9fd228f4`, identical tree `18b50fe128ecf7aa901201d2995c9d7fb1cbe755`; [PR CI #40651259497](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651259497) SUCCESS 4/4, [native preparation #40651259506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651259506) SUCCESS and [independent push/main #40651850790](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651850790) SUCCESS 4/4, all exact SHA. Windows test `prelaunch_snapshot_never_rebinds_after_original_lease_id_reuse` exercises revoke/remove/re-register of identical opaque TaskLease ID: the new lease may independently be valid, but the old nonexecuting native owner remains stopped and retains its original permits, quotas, and empty Job until safe no-child disposal. The first #395 candidate's format error on [#40651188973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651188973) was formatting only, not a behavioral reproduction.

**Physical #396 scope:** the research-only fixture configures a genuine empty Windows Job and `PROC_THREAD_ATTRIBUTE_JOB_LIST`, calls `CreateProcessW` for a deliberately absent image under `SystemRoot/System32`, verifies **Win32 ERROR_FILE_NOT_FOUND** (not an attribute/permission error) and a **still-empty kernel Job**. A scoped guard would terminate any unexpected suspended child. It never resumes a child or launches Git. Neither #395 nor #396 changes production, authorizes native GitRead, or proves a started process's terminal/pipe lifetime.

**Repository workflow:** one scoped PR at a time, exact head SHA, first RED preserved, real Windows and Linux CI, specialized Windows workflows where applicable, merge only after review/green exact-head, compare source and squash trees, then verify independent push/main on exact integrated SHA. The #317 evidence is also preserved in issue comments; do not treat these research-only tests as proof of the missing real production launcher.

**Still unproven (P0):** generation-bound owner commitment serialized against **both** direct and lifecycle session revoke; independent TaskLease/policy epoch fences; executable/repository authority at creation; real owned native GitRead root and descendant lifecycle; stdout/stderr EOF and stdin completion; bounded independent stop/watchdog; positive root-exit + Job-empty **after actual execution**; uncertain failure/crash capacity retention and safe result publication. Research acceptance #352/#355 remains RED. Issues #317/#323/#185/#319 are OPEN. **Windows 10/11 V1 remains NO-GO; no release, remote execution, Optic Desktop/REL work, operator-PC access, self-hosted CI or unrequested Codex/Work tasks.**

**Next engineering study:** enumerate and verify the shared session close/commit linearization for *direct registry* and lifecycle revoke, maintain original counted permits and independent process/Job quotas through uncertain termination, and identify a nonblocking stop path before moving ownership into a live native GitRead worker. Never hold a session/JobStore mutex over Windows native process creation, blocking IO or host-memory observation. Preserve the original disabled #352/#355 behavioral acceptance gates.


## CURRENT CHECKPOINT — 28 October 2026, merged research-only #393

Verified preceding baseline `main=204f8619ff14c86fb3991f752eb3efed77ab2706`, independent push/main CI #40556021484 SUCCESS 4/4. [#393](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/393) squash merged at `main=c81c1f5caf937695b3bbd9439f508ff9cb28f6f4`; approved exact HEAD `14e73802b853c57b67c0b00bf169af6fd5a47793`, identical head/merge tree `12c27901f8ec3f6dd4b8f6abfbfae44aaa16f987`, [PR CI #40649262906](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262906) SUCCESS 4/4 first attempt, and three dedicated native workflows (#40649262958, #40649262919, #40649262961) all SUCCESS. Separate [push/main #40649847690](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649847690) is independently **completed/SUCCESS, first attempt, all four jobs individually successful on the exact integrated main SHA**.

**What changed:** `crates/optic-bridge-windows/src/job_list_launch_tests.rs` and test-only registration in `job.rs`. Real hosted Windows research fixture creates one suspended `cmd.exe` with a preexisting Job inserted in `PROC_THREAD_ATTRIBUTE_JOB_LIST`. It observes immediate kernel membership, root exit after termination and an empty Job. No resume, no production Git process, no policy/MCP change, and no proof of CreateProcess-internal crash safety, concurrent authority close, descendant EOF, result publication or recovery. Preserve first rustfmt RED #40648989661 and compiler E0432 RED #40649081510 as **nonbehavioral**. Next: authority generation-bound synchronized commit/revoke gate and a truly owned Job/root/pipe lifecycle with physical EOF under `TerminationUncertain`. #352/#355 acceptance still unmet; #317/#323/#185/#319 OPEN; Windows V1 NO-GO, no release. GitHub-only, no operator PC, self-hosted runners, Desktop or REL.

## CURRENT VERIFIED CHECKPOINT — post #391 merged, 27 October 2026

Main observed at `c208348f0b63d4810db97f9a5284b325c5b64271` after [merged #391](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/391). Exact tested PR SHA `e7f684ae9c32b3d1026ef103d9832f8a7767a535`, tree `da6ff9bcbeb63ffb3a64ffc590187ae4d2be4765` equals squash tree. [PR CI #40526579598](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40526579598) SUCCESS 4/4, attempt 1, native workflows #40526579328/#40526579323/#40526579729 all SUCCESS. [Independent push/main #40553879389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40553879389) confirmed completed/SUCCESS first attempt with each of four jobs completed/success. The present docs PR will change main again: check its exact HEAD and fresh independent push CI.

#391 is a regression **test only**. Its genuine Windows result disproved an assumed ancestor rename gap: pin already prevents parent rename while custody remains held. The original-generation snapshot #390 was previously merged as `7f663454039b294d84cd3fb2333cbbd73372f91a` with independent [#40520286289](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40520286289) success 4/4.

Current native Git status work holds original session/transport admissions and TaskLease witness, process quotas, true **unstarted** Job/stdio handles and canonical target pins. Its same-generation lease/policy/executable snapshot remains **non-atomic** with CreateProcess and **non-executing**. Live MCP uses legacy HardenedCommandRunner, task_lease None. No real native child, root+Job+EOF terminal evidence, watchdog/crash proof, or MCP switchover. The original research behavioral requirements #352/#355 remain RED. #317/#323/#185/#319 OPEN, Windows V1 NO-GO. Do not merge research #353/#358/#327 wholesale.

GitHub-only classic chat: no operator computer access (not even read only), no Work/Codex/automations/self-hosted runners/Optic Desktop/REL. Read the **[consolidated successor prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md)** and current handoff first. On each PR use test-first when meaningful, exact SHA, four CI jobs and specialized workflows, squash expected_head_sha after proof, compare trees, independent push/main success. Avoid auto-closing #317 with commit titles or keywords.


## 27 October 2026 — draft #390 original-generation prelaunch snapshot

Baseline verified after [#389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/389): `main=2702e8f9de9982a7654bb09559e18865b8ebe763`, independent [push/main #40516482577](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40516482577) SUCCESS 4/4, attempt 1. [Draft #390](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/390) introduces a no-spawn snapshot verifying that the refreshed policy/identity TaskLease is the same original revocation registration; an independent same-ID replacement or foreign session registry is rejected. Test-first [CI #40517792327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40517792327) is compiler/format red only. Inspect exact final PR HEAD CI and native workflows before merge; after merge verify separate independent push/main, never assume prior SHA results transfer. Native process/Job-root/EOF and MCP integration remain unproven. #317/#323 OPEN, #352/#355 acceptance RED, Windows V1 NO-GO.


## Verified GitHub integration after #388 — 27 October 2026

**Confirmed integration after PR #388 (27 October 2026):** `main=720a38833705e661049841cfd39852e770687643`. PR HEAD `0b29e833a53195f199b5d3b53f808091d020bdc5` passed [CI #40513121389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40513121389), first attempt, four jobs individually SUCCESS and all three native-specialized workflows successful. The reviewed HEAD and squash tree are identical (`1d79045f8ac60db9dd2dc2ffe6e23b72af0811f9`). Independent [push/main #40514100157](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40514100157) on that exact integrated SHA completed SUCCESS, attempt 1, all four Windows/Ubuntu/dependencies/release jobs successful.

**Scope remains pre-spawn only:** original Git executable and repository root opened and pinned by verified Windows handles, with the already counted admissions, TaskLease witness, unstarted Job and stdio. No child, final synchronized `CreateProcess` authority, native terminal/EOF proof, MCP GitRead switch, physical Windows 10/11 acceptance or release. Keep #317/#323 OPEN, #352/#355 original acceptance RED, and Windows V1 NO-GO. This documentary commit itself changes the HEAD; revalidate live `main`/`push/main` separately after any merge.


## 27 October 2026 — integrated #388 GitRead target pin checkpoint

[PR #388](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/388) extends #387 non-executing real native Job custody with path-handle pins for the **original** trusted Git executable and canonical repository. GitRead uses a stronger READ/LIST directory-root handle, because [test-first #40510524394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40510524394) was Windows behaviorally red (2 missing protections) and [first pin #40511257834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40511257834) still red (root rename permitted). Code SHA `382857c0269b7173c24bef8eb948d08ad0e558ae` [CI #40512075888](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075888) success first attempt; 3 native workflows success. Verify later PR/doc HEAD and independent push/main afresh. No Git child nor MCP switch, no sync CreateProcess authority or terminal receipts. #317/#323 OPEN; #352/#355 RED; Windows V1 NO-GO; no operator PC, Work, Codex or release.


## POST-#385 INTEGRATION CHECKPOINT — 27 October 2026

Most recent **independently verified** `main=e5017c48de5d4d6433f92e3117d63f1677aecb59`
after [merged PR #385](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/385):
[push/main #40503510236](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40503510236)
completed SUCCESS, first attempt, all 4 jobs (Windows Rust, Ubuntu Rust,
dependency policy and release evidence guard). Verified PR HEAD
`b8c0db14c966979170dbb8e26fd620964fcc68fa` passed [PR CI #40502910932](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40502910932)
4/4 plus [native-preparation #40502910914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40502910914).
Initial test-first interface compilation RED
[#40501489580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501489580)
was E0433/E0599 and rustfmt, **not native runtime acceptance**.

What changed: exact lease-bound prepared `git status` can now be registered
**without execution** against shared process/CPU/memory/output capacity,
retaining the original MCP session and transport reservations, action horizon
and TaskLease generation watch. The registered `LimitedJobObject` is a
**configuration**, NOT an opened Windows kernel Job. There is **no live
native GitRead MCP route**, no true Job/root/pipes/EOF, no synchronous final
launch gate, no physical crash-proof termination. Live `git_status` continues
to use the legacy runner and `task_lease: None`. No bulk merge #353/#358.
Original behavioral acceptance #352/#355 remains RED; #185/#319 remain
separate blockers. #317 was briefly auto-closed by the squash at 12:24:27 UTC,
**reopened at 12:25:27 UTC** and documented in
[issue #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6441832785).
#317/#323 OPEN; Windows V1 **NO-GO**.

P0 next: real service-owned Windows guardian must adopt the same registered
value, pre-create and own Win32 Job/root/stdout/stderr/stdin, verify pinned Git
identity/canonical repo and synchronize revocation/policy/epoch/monotonic
deadline against CreateProcess. Retain admission, transport and quota under
uncertain termination until positive root, Job-empty and true EOF. Only then
wire one MCP Git status with original sensitive-path guards. Original #352
and #355 must pass unchanged, plus host loss and physical Win10/11 acceptance.
Use GitHub-hosted CI only; no operator PC access/Work/Codex/automation and
no release. **Always verify the live main SHA and independent push CI anew,
as a later documentation merge will itself advance main.**


## REPRISE ACTIVE — CHAT CLASSIQUE — 27 octobre 2026

Lire la [passation Windows V1 pour Chat classique](HANDOFF_WINDOWS_V1_CHAT_2026-10-27.md)
et le [prompt complet à copier](NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md).
Reprise demandée dans un **Chat classique, GitHub uniquement** ; ne pas créer de
tâche Codex/Work ou accéder au PC sans demande explicite. Les entrées datées
ci-dessous sont historiques pour leurs références ; les invariants restent
obligatoires.

Avant cette PR documentaire : `main=75947c9b4fb72fab2f3d45f34f5b750f09bc31b2`,
[CI indépendante #40477277322](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40477277322)
SUCCESS 4/4, tentative 1. #380–#383 sont intégrées, mais GitRead utilise encore
le runner legacy, `task_lease: None` et aucun propriétaire Git natif actif.
Vérifier le nouveau main et sa CI push après fusion de cette documentation.
#352/#355 ROUGES ; #317/#323/#185/#319 ouvertes ; Windows V1 NO-GO.
Aucun accès PC, Optic Desktop ou REL.

---

## CURRENT — 25 October 2026 full successor handoff

**Authoritative continuation now lives at [HANDOFF_WINDOWS_V1_2026-10-25.md](HANDOFF_WINDOWS_V1_2026-10-25.md), together with [NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-25.md](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-25.md).** Live-check branches and CI before action. Pre-documentation checkpoint: production `main=7c72ecfb8886fb34be2cc7ef9f2e1070f73c08da` [CI #40188090038](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40188090038) 4/4 green first attempt, native research #358 HEAD `f431c3ef…` stacked on #353, integrated research CI [#40187179915](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179915) general 4/4 / [#40187179856](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179856) native 2/2 / [#40187179916](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179916) preparation green. This docs PR will advance main; its exact push CI must be confirmed independently. #352/#355 acceptance RED, #185 intermittent, #317/#323 OPEN, V1 NO-GO; no operator PC or other repositories.

---


## Newest GitHub continuation — 25 October 2026, post #358

Integrated `main=d7d4d50adf95b6a3cf4a5df0ef06a6ba526d6810` [push/main CI #40173338835](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40173338835) success 4/4; code #356/#357 merged typed GitRead builder only, live `HardenedCommandRunner` remains. Open #327/#352/#353/#355/#358; specialized #352 Windows/Ubuntu [CI #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) RED wall-clock stdio, specialized #355 [CI #40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203) RED post-revocation Git spawn, independent #185 Windows CI red in #355 normal run. Research #353 [CI #40177026825](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177026825)/[native #40177026834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177026834) green on `b091dc28…`, #358 **stacked onto #353** [CI #40177299174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299174)/[native #40177299159](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299159) green on `59d1a331…`. A stacked commit's CI is not an integrated production proof. #317/#323 OPEN, Windows V1 NO-GO. Full details: [technical Windows handoff](HANDOFF_WINDOWS_V1_2026-10-24.md), [complete prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md). Permanent [Codex recovery protocol](CODEX_INTERRUPTION_RECOVERY.md) is binding but no user PC access without specific permission. Older checkpoints below are historical.

---


## GitHub live continuation checkpoint — 25 October 2026

**Canonical latest handoff:** [Windows V1 engineering summary](HANDOFF_WINDOWS_V1_2026-10-24.md) and [full successor prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md). `main=6a6457155758df3050e2aad81ee9446952d7eedd` after docs #351, [push/main CI #40086602634](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40086602634) completed SUCCESS, attempt 1, four jobs. Earlier code PRs #347 (late result classification **not** bound joins), #349 (Win32 query errors preserved) and #350 (stop-before-drain for registered jobs) are integrated; older #185 red [main #40084443275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40084443275) and original `TerminationUncertain` red [#40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) remain historical evidence. [Open #352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) dedicated [CI #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) is behaviorally RED on Windows/Ubuntu (~1.6s vs 200ms runner timeout; actual <900ms acceptance), though normal CI green. [Open #353](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/353) staged native pre-spawn Job/EOF/session ledger, `GitRead` capability check and no-spawn preparation reclaim: [specialized #40153942636](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40153942636) and [full #40153942601](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40153942601) SUCCESS at head `261e9df9…`, still **NOT** connected to production `GitReadService`. Open research #327 remains separate, issues #317/#323 OPEN, V1 NO-GO. GitHub SHA/status changes are expected: verify anew, do not merge research wholesale. [Recovery protocol](CODEX_INTERRUPTION_RECOVERY.md) still applies.



## Newest GitHub-only checkpoint — 24 October 2026 after Codex interruption recovery

**Read first:** [Windows V1 handoff after Codex recovery](HANDOFF_WINDOWS_V1_2026-10-24.md), [updated complete successor prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md), [agent recovery policy](CODEX_INTERRUPTION_RECOVERY.md) and `AGENTS.md`. This is the latest continuation, superseding earlier “ten crash schedules” history. GitHub can change after this checkpoint: always query it live.

Documentation-only [#343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/343) merged as `main=dce94cc02f178d91b1eec4dc06134ab57df47ff6` after updated strict PR-head [CI #40075523748](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40075523748) passed Windows, Ubuntu, dependency and release jobs. Verify independent push/main [CI #40075855681](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40075855681) on that SHA separately, not by extrapolation. Before #343, `main=cf5a6c3` [push CI #40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) had a real Windows `TerminationUncertain` failure; never reclassify this red as green.

Only [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) remains independent code research PR; #339, #340, #341, #342 are **closed without merge**, with verified research CI/physical test evidence described in the new handoff. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) and [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) are OPEN, **Windows V1 NO-GO**. Production still uses root-deadline-only `HardenedCommandRunner` for GitRead; #339 reproduced unbounded stdout/stderr join; #341 reproduced stdin writer join after root; #340/#342 found no natural cause for `TerminationUncertain` in successful scenarios. None establish service-crash recovery or a guardian native owner.

Local Codex checkpoint recovery is governed by [CODEX_INTERRUPTION_RECOVERY.md](CODEX_INTERRUPTION_RECOVERY.md): external snapshots, SHA-256 and independent Git reconciliation, fail-closed snapshot pointers, no policy bypass. Local snapshot `0013-policy-published-ci` / 55 checksums is **operator-reported** and must not be treated as remote proof; a GitHub-only agent must mark inaccessible local disk as unverified. Follow canonical worktree/branch/CI discipline and leave Optic Desktop and REL alone. Main/source SHAs are historical checkpoints, never destructive checkout commands.

---


## Most recent verified GitHub takeover — 24 October 2026, ten native scenarios

**Canonical files to read first:** [updated Windows V1 handoff](HANDOFF_WINDOWS_V1_2026-10-24.md) and [complete successor prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md). Before this documentation-only PR, GitHub `main=16ca67a75fd49c5681a0672773ffc268c3635f6b`, [CI #40047950688](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40047950688) green. Only PR #327 open; issues #317/#323 open, V1 NO-GO. **Latest code evidence unmerged**: `research/317-failed-native-create-2026-10-24@b5a91dee`, [CI #40057931887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887) Windows 355 pass/13 ignored, Ubuntu 270 pass, ten external synthetic crash schedules and Windows workspace green. Documentation `4bef362d`, [CI #40058237376](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40058237376) was still in progress before this PR; re-check. Research is **not** a production GitRead guardian, and persistent native observation faults, descendant-held stdout/stderr + stdin EOF, host recovery and bounded service shutdown remain unproved. No broad research merges, user-PC execution, security-control workaround, release or V1 assertion. The earlier 24 Oct and prior sections below are **historical** rather than current reference SHAs; check GitHub anew after documentation merge.


## Current canonical takeover — 24 October 2026

**Read [24 Oct Windows V1 handoff](HANDOFF_WINDOWS_V1_2026-10-24.md) and [24 Oct complete next-chat prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md) first.** Live state before this docs PR: main **100c43b53c795be5678c8b4f2c0af4353ca5b196** ([exact push CI #40027279676](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40027279676) succeeded); PR **#327** open, #317/#323 open and **V1 NO-GO**. Last research **8202c9c633bf797be9ea69f9ce994b5ec5da9b86**, [Windows 348/Ubuntu 270 CI #40046563179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40046563179) green (single injected Win32 accounting fault, then positive recovery) is **test-only and unwired**. Next: separate stop request vs quiescence/drain, prove persistent uncertainty, descendant-held stdio/EOF and externally supervised service-host crash; never free quotas/handles on guessed timeout. Verify SHA and post-merge CI anew. All older handoff entries below are historical checkpoints.

## Canonical engineering takeover — 23 October 2026

Current source of truth: [HANDOFF_WINDOWS_V1_2026-10-23.md](HANDOFF_WINDOWS_V1_2026-10-23.md) and [NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-23.md](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-23.md). Before this docs-only change, `main=3fe3de0c`, research [PR #327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) open with [CI red #39845778580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39845778580) #185; research Codex `codex/317-service-retention=1c7dac9`, exact-source [CI #39888522519](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522519) and [native #39888522200](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522200) successful. **Actual findings:** native child can survive synthetic host death in the spawn-suspended/assign gap (recovered by independent supervisor), submit admission does not cover worker lifetime, root Drop may block on persistent OS observation error. Neither #317 nor #182 is fixed in production. Keep ADR-0035/0036 proposed, no bulk research merges or V1 tag. Old handoff sections below are historical dated checkpoints. Re-check GitHub, including any merge/CI of this documentation PR.


## Canonical next-chat checkpoint — Windows V1 / issue #317, 22 Oct 2026

**Begin with [HANDOFF_WINDOWS_V1_2026-10-22.md](HANDOFF_WINDOWS_V1_2026-10-22.md)**, then the [standalone takeover prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-22.md). The historical sections below are **not** current refs. At the verification preceding this docs-only PR, main=`82ad1234f184591e0f2c80c86296a65a883ecc20`, no open PRs, main unprotected; research `bootstrap/issue-317-windows-rust-owner`=`33c95e00aa384774ccac6ccbab24b8f54fa5ab11`; [CI #39748924515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748924515) green on that exact research SHA. No production #317 correction: the 13 native Job fixtures, real SessionRegistry/Job tests and private unwired registry do **not** justify merging the whole research branch, changing ADR-0035 from PROPOSED or claiming V1 readiness. First review the registry's global mutex around native observation and how it might block out-of-band stop. #317, #176, #182, #319, #185 and V1 gate #323 still require decisions/proofs. Preserve GitHub-only, Windows hosted CI, no operator PC, release, security weakening or Electron. A documentation PR/merge may have advanced main after the listed snapshot.


## Current handoff — Windows V1, after PRs #320/#321 (2026-10-22)

`main=65aa5db0dbaad24894f2cf4b050d5ac0bd61eac9` and zero PRs open were verified. Both #320 (exact-main-CI prerelease provenance gate) and #321 (Windows PowerShell 5.1 no-BOM MCP stdin) are squash-integrated. Independent same-source runs [CI #39722724572](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724572), [pwsh #39722724496](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724496) and [Desktop5.1 #39722724477](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724477) passed attempt 1. Read [release gate](RELEASE_SOURCE_GATE.md), [PowerShell proof](WINDOWS_POWERSHELL_ACCEPTANCE.md) and [current status](../../STATUS.md). No release and no Windows10/11 device certification.

**Next #317 Windows:** native Rust test-only owner slice at research head `d295d92ec813612a3a919b56143ed23dcbb89485` (eight supervised cases, #39718651623; normal CI #39718651583) is **not merged**. The [research-only audit](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/d295d92ec813612a3a919b56143ed23dcbb89485/docs/research/WINDOWS_V1_OWNER_AUDIT_2026-10-22.md) and [ADR-0035 proposal](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/d295d92ec813612a3a919b56143ed23dcbb89485/docs/decisions/ADR-0035-WINDOWS-GIT-OWNER-STAGING.md) are not yet on main. Test stdin, native OS error/handle cleanup and fail-closed retained resources before a long-lived Session/HostStartup owner service, trusted stop-before-drain and seal/reap wiring. Separate Git CAS effect uncertainty. Do not bulk merge research.

Remaining issues #317, #176/#182 (real-project remote sensitive/provenance), #319 (natural 30s first response) and #185 (separate AppContainer flake) remain open. The #321 framing fix does **not** explain #319. `main.protected=false` and zero rulesets at last audit; settings require approval. Release build's broader `contents:write` needs separate review. No feature/Electron work, user-PC execution, self-hosted runner, Linux confinement research, remote access to real projects or relaxation of limits. Retain Ubuntu CI and first red attempts. Full [2026-10-22 Codex handoff](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319#issuecomment-6332848895). Historical sections below are not current live state.

## Historical successor checkpoint — after #314 and GitRead review (2026-10-21)

At review, `main=7bc17c16a52383bf6fb7ba68dc4b9348b259be9a`, **0 open PRs**. [PR #314](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314) is squash-merged. Accepted source `fbf7c9f40bbae18d4017f237b8783a19964620c8` passed [CI #39576724127](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39576724127) (native Windows, Ubuntu, dependencies) and three specialized native Windows workflows; independent [main CI #39577461324](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39577461324) succeeded first attempt at the exact squash SHA. [Verification comment](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314#issuecomment-6312779381). First superseded PR-head rustfmt failure was retained, not retried to conceal it.

**Delivered:** `fs_read` conservatively requires a normal FileRead grant **plus matching ReadSensitive scope** when the opened file has multiple hard-link names. The number of links is read from the opened native file handle/descriptor, not an independently re-resolved pathname. Harmless hard links may be denied without extra authority. Previous #308–#313 protections remain integrated.

**Read-only GitRead audit on this SHA:** [issue #182 checkpoint](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182#issuecomment-6312922336) confirms raw Git status path validation, checked changed paths and patch framing in a single Git diff invocation, and lease checks for classified content. It also isolates a policy question: `# branch.head` and `# branch.upstream` are returned as Git status metadata but are not workspace paths, so no sensitivity contract currently classifies their values. This is not a reproduced secret leak; do not silently remap branch names to file authority or break public output without a policy decision and regression.

**Open risks:** #176 arbitrary ordinary-name secrets/native aliases and races, #182 Git index/commit blob provenance/branch metadata, #185 intermittent AppContainer Node 10s no-output timing, distinct first MCP 30s timeout, recovery cleanup/ancestor/power-loss and long soak limitations. Existing CI green does not close them. Keep feature freeze, synthetic-only research, one PR at a time, exact-head hosted Windows/Linux CI, independent main CI, no user-PC or self-hosted tests, no relaxed timeouts/sandbox/grants, no alpha merge.

Older successor checkpoints below are historical. Verify GitHub live before next action and prefer no code change to speculative security modifications.

## Verified successor checkpoint — after Policy B #310 (2026-10-21)

Latest observed `main=bd2915c3a9833c1da5a8fc5005445d1884260d4f`, zero open PRs. [PR #310](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/310) was squash-merged; [independent main CI #39552523257](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39552523257) passed **first attempt on the exact merge SHA**, hosted native Windows, Ubuntu and dependency checks. Accepted source `154d7ea6ab6b10aa967ab0a2f2034f2f06400fe9` passed [CI #39551993433](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993433) and [native prep #39551993151](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993151), both first attempt. Initial red PR attempts were corrected without weakening policy; see [PR #310 verification comment](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/310#issuecomment-6309177355).

**Merged, not candidates:** #308 deterministic shared filename-sensitive floor for FileRead/fs_list, #309 strict porcelain-v2 Git status protected-path checks, #310 strict one-invocation Git diff raw-path + patch framing with extra NUL separator and active FileRead dual-scope gate for protected contents. Existing Git isolation and session admission/revoke, bounds and defaults are preserved. See [ADR-0034](../decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md). Sections below describing these as future candidates are historical only; never reset `main` to their dated SHAs.

**Gates still active:** #176 (ordinary-looking source/config files can contain secrets; name patterns do not inspect arbitrary content, filesystem ancestor identity remains hard); #182 (cross-tool exposure contract, concurrency and alternate names/provenance/branch metadata). These issues remain OPEN; #176 was inadvertently marked completed by GitHub during #310 squash after a closing keyword appeared inside negative prose, then explicitly reopened with an explanatory issue comment. Avoid GitHub auto-closing wording in future PR bodies. Real-workspace remote access is not authorized. Other non-closed stabilization tracks: #185 natural Windows Node/AppContainer no-output ~10s, distinct ~30s first MCP response, recovery multi-file cleanup/ancestor races/Windows power-loss persistence, missing stress/fault-soak and code audit. No new Electron, Cargo integration, alpha-wide merge, remote transport or release.

**Process:** recheck live main/PRs/CI; one PR at a time; bounded proven defect or documented debt; hosted Windows/Linux exact-head validation and independent post-merge CI; no user-PC execution, permission expansion, protection edits or self-hosted runners. Keep first failed CI attempts available; do not manufacture a PR when no real issue is demonstrated.

## Next Policy B step — Git diff candidate after verified #309 (2026-10-21)

Verified before branch creation: `main=ccaa7c05af27cfaab02d5dcb93369f60a0c01f53`, zero open PRs; merged #309 protects Git porcelain-v2 status including sensitive rename/untracked names, independent push [CI #39549267903](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39549267903) green first attempt on exact squash SHA. New **unmerged** candidate `security/git-diff-raw-output-authority` targets distinct #182 bypass: unclassified `git_diff` patch contents.

Intended diff strategy: produce the machine-readable `--raw -z` changed-file prelude and `--patch` bytes in *one* existing hardened Git subprocess, validate each path with the shared classifier, refuse unparseable/unknown path records, and require a matching active FileRead (ReadContent + ReadSensitive) lease for protected contents, inside session admission. Return the original bounded patch without internal raw prefix. Test Windows/Linux staged additions, worktree edits, path- and directory-scoped diffs, ordinary source content and adversarial malformed byte streams. Keep no-ext-diff/no-textconv/no-renames, exact executable, output ceilings, revoke draining and no permissions by default.

**Do not infer integration until GitHub verifies accepted source SHA and independent post-merge main.** Git-diff content is not automatically harmless when its filename looks ordinary; Git's paired output does not promise atomic snapshots across concurrent file changes. #176/#182 remain OPEN, no real-project remote use. Don't expand features or relax isolation/CI.

## Follow-up after verified #308 — Git status candidate (2026-10-21)

Reverified `main=bfba4b97a531440de08012ca50106797d995646b`, zero open PRs, independent [#308 post-squash CI #39546153987](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39546153987) passed first attempt (native Windows, Ubuntu, dependency policy). **New candidate only:** `security/git-status-sensitive-paths`. Do not infer merge until exact-head CI and independent main checks.

Source confirmed: `git_status` was returning Git porcelain-v2 bytes without checking protected tracked/untracked/renamed pathnames despite #308's classifier. Proposed narrow fix: parse complete NUL-delimited status records in the hardened runtime, refuse malformed/unrepresentable/unknown output, and check all sensitive names against a matching active `ReadSensitiveScope` before releasing bytes while session admission is held. Test both rename names, staged/untracked names, prefix mismatch, normal status and Windows/Unix behavior. See [ADR-0034](../decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md).

**Major remaining #182 bypass:** `git_diff` still emits unclassified raw patch bytes; do not treat status correction as GitRead completion. Whole-repo and path-scoped Git diffs need an independent, trustworthy output-boundary design that cannot be bypassed by untracked/renamed/symlink/ancestor/native-path cases or repository races; deny unverifiable output. #176 content classifier and Windows/recovery/soak limitations also remain. No operator-PC activity, new features or security grant expansion.

## Approved Policy B — first narrow correction candidate (2026-10-21)

Live baseline before branching: `main=3d9a0f78304578af53fd23b9fed7d08603348e0b` after merged #307, zero open PRs, independent push CI #39542505029 green on first attempt (Windows native/Linux/dependencies). The operator **approved Policy B**: a shared deterministic sensitive classifier for FileRead, FileSearch and GitRead, rejecting unverifiable outputs. The first scoped branch `security/sensitive-file-authority-scope` adds a shared pure core lexical classifier, file-read dual-scope enforcement and `fs_list` sensitive-name page denial with targeted unit/MCP regressions; acceptance depends on exact-head hosted CI and post-merge main CI. See [ADR-0034](../decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md). Do not assume this candidate is merged: inspect GitHub live.

Remaining critical gap: the classifier is a path-name **floor**, not exhaustive content-secret detection, and **GitRead currently remains unfiltered**. #176 and #182 stay open; real-workspace remote/autonomous exposure is prohibited. The next individually demonstrated change should target complete Git status/diff output sensitivity, fail closed on unknown records, preserve existing Git containment and ordinary behavior where proven, with Windows/Linux fixture coverage. No speculative permission broadening or feature development. Other open Windows, recovery/race, soak and code-audit debts are unchanged.

## Current successor handoff — after merged #306 (2026-10-21)

**Verified live base before this handoff update:** `main=dadaf48539788235b689dd4e09adcd2fb94904f3`, zero open PRs, [docs-only PR #306](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/306) squash-merged. Its [source CI #39540522392](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540522392) and independent [main CI #39540940136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540940136) **both succeeded first attempt on their exact SHAs** (Windows Rust, Linux Rust and dependency policy). A new handoff-only PR may have changed main after this snapshot: **requery live GitHub**, do not reset to this SHA.

The operator wants the **existing repo stabilized**, not new features. One PR at a time, scoped correction + regression + docs, no artificial PR. Never use the operator PC or self-hosted runners. No Electron GUI, generalized Cargo, production remote MCP, alpha-wide merge, release, permission broadening or disabled protection. Follow `AGENTS.md`, `AI_HANDOFF.md`, `STATUS.md`, `ROADMAP.md`, `RELIABILITY_DEBT.md` and [the full reusable prompt](../../PROJECT_CONTINUITY_PROMPT.md) for instructions and invariants.

**Immediate next investigation:** read-only inspection of the current sensitive-read authority composition, [issue #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) and [issue #182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182), across `ReadSensitiveScope`, default `ReadAuthoritySpec::workspace_all_non_sensitive()`, `fs_read`/`fs_list`, `GitRead`, `git_status`, `git_diff`, `git_log`, canonical path resolution, policy and lease checks. **Do not assert a functioning sensitive-file classifier:** it has not been implemented/proved, and Git status/diff can independently reveal sensitive material. Decide a deterministic cross-capability contract *before* code changes or real workspace remote exposure; a trust-model choice may require the operator. The existing canonical-`fs_list` and `.git` mutation fixes are already in main (#303/#304).

**Next outstanding reliability gates:** natural native Windows Node/AppContainer 10s startup intermittent [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) with original red evidence, separate MCP ~30s first response, non-atomic multi-delete recovery after a real delete fails, directory ancestor races and Windows crash/power-loss persistence, incomplete 1h/4h fault-soak and file-level audit. Existing #180/#181/#183 issues remain open despite prior targeted safeguards—reconcile acceptance evidence before creating duplicates or closing issues.

**Already integrated:** #293–#297, #299, #301–#306 (source/merge and CI evidence in STATUS); particularly #301 preserve recovery evidence, #303 canonical listing policy, #304 Git control namespace, #305 cursor-aware Git-log response, #306 documentation reconciliation. Green first-pass CI is **not** a certificate of production correctness. `main` was unprotected and the ruleset listing returned none at last observation; owner approval is needed before changing settings. Treat historical sections below as dated accounts, not active tasks.


## Latest verified handoff — 2026-10-21 after merged PR #305

**Previously verified live `main`:** `43d3a4aeedee04dd03e895e27e81d3bc5af6652c`; **zero open PRs**. Independent [CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed first attempt **on the exact merge SHA**, hosted Windows Rust, Linux Rust, dependencies. Requery GitHub before acting: this is a timestamped checkpoint, never a reset target.

**Do not redo merged increments.** [#301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) preserved unpublished recovery evidence (source Windows first attempt exposed the separate known timeout; independent main CI passed). [#302](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/302) delivered earlier handoff docs. [#303](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/303) authorized canonical `fs_list` directory scopes. [#304](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/304) protected canonical `.git` control namespace against generic mutations. [#305](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/305) refit bounded `git_log` output after insertion of a continuation cursor. All five were merged; their independent main CI results are linked in [STATUS.md](../../STATUS.md).

**Actual remaining stabilization investigations:** (1) natural Windows AppContainer/Node 10s no-output failure [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), *not fixed by opt-in timings*; (2) distinct 30s first MCP response timeout; (3) non-atomic recovery cleanup, ancestor races and unproven power-loss persistence; (4) sensitive read classification [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) plus GitRead content/status sensitivity [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182). These last two need a written policy/product decision **before remote access to a real workspace**, not an improvised broad grant. Cargo and sandboxed Electron remain deferred, not production support. Issues #180/#181/#183 are administratively open although code/tests contain scoped fixes; reconcile criteria with evidence before closure, and avoid speculative duplicates.

**Completion discipline:** preserve one PR at a time (fix + focused regression + docs), exact-head native Windows tests for Windows claims, the first failed CI attempts, review and post-squash independent main CI. The [quality audit](CODE_QUALITY_REVIEW.md) is partial. No user-PC commands, self-hosted runners, releases, new features, permission expansion or whole-alpha merge.

**Governance observation:** GitHub branch lookup indicated `main.protected=false` and ruleset listing returned no rules at the above checkpoint. This is *not* authorization to change branch protections. Operator approval is needed for any protection-policy change; continue enforcing exact-head CI and review in the meantime.

Below are **historical checkpoints**. Their “active PR”, “candidate”, and old main SHAs describe *that date* only.


## Focused Git-log response-budget regression

Source review of `ReadonlyMcpServer::fit_git_log_response` identified that adding a continuation cursor **after** clipping an end-of-history page could grow serialized JSON beyond `max_response_bytes`. The scoped correction refits the **final** entries+cursor combination and rejects a page if no entry plus cursor can fit. It also checks offset addition instead of using saturating addition. Regressions cover insufficient budget, sufficient budget with exactly one advancing entry, and a complete page that stays cursor-free.

The correction belongs to a single GitHub PR with docs and tests; live GitHub source SHA and CI **must** be rechecked before merge. No new commands/capabilities, policy changes, timeout increases, MCP transport additions or operator PC access. The active Linux/Windows test results and the post-merge `main` verification, once known, are authoritative in the PR/Actions history. Keep Windows Node/AppContainer and MCP first-response timeout investigations separate, and preserve their evidence.


## Active single-PR stabilization: reserve .git metadata (2026-10-21)

At the start of this increment GitHub `main=9c6c91096590f9f005dabc51253b9b5f8b1abcca`, zero PRs open and [independent #303 post-merge CI #39534614408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39534614408) passed on attempt 1. [Issue #180](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/180) documents an actual separation-of-authorities defect: generic FileWrite/FileDelete could target Git-owned `.git` data without GitIntegrate authority.

The focused branch implements an explicit reserved Git-control namespace at transactional **canonical** mutation preparation and directory creation, rejecting any exact case-insensitive `.git` segment while leaving `.gitignore` etc. usable. Tests cover existing/absent `.git` file content, linked worktree pointer `.git` file, alias into Git metadata (Unix), Windows authenticated WorkspaceAll write/delete and directory denial, and stable MCP invalid-request error. Do not treat the branch as merged until exact-head native Windows/Linux and policy CI confirms it; integrate documentation **in the same PR**; afterward check independent main CI on the squash SHA. Keep first failed attempts.

Scope limits: no expanded Git integration ability, no sensitive-read classifier, no remote transport, no general Cargo or Electron. Concurrent directory replacement races, alternate hardlinks and power-loss persistence are not globally solved. Continue one PR at a time, preserve the Windows Node/AppContainer #185 evidence and use hosted GitHub runners only.


## Security stabilization candidate: canonical fs_list scope — 2026-10-21

Source audit identified a **real** `fs_list` authority mismatch: an alias path inside an allowed `SearchMetadataScope::Prefix` could resolve to a different workspace directory, because policy had used the lexical alias while the bounded filesystem resolved the actual target only after authorization. The scoped candidate adds `BoundedFileSystem::canonical_list_root`, evaluates `Effect::FileSearch` on that canonical directory, and enumerates/returns canonical-path entries. Regressions cover a Unix in-workspace symlink out of a permitted prefix, whole-workspace canonical listing, and an ordinary directory on native Windows/Linux.

This is **not** a fix for simultaneous directory swaps (there is no lifetime-pinned directory handle), unclassified sensitive-file reads, or remote transport safety. [Security audit #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) retains those independent requirements. Verify the actual PR, exact source head, all CI results and independent `main` status live before claiming integration. Use one PR at a time; no operator-PC execution or runner self-hosting, no GUI/Cargo features. The earlier #301/#302 integration and preserved Windows #185 first-failure evidence remain canonical.


## Most recent recovery correction — 2026-10-21, after PR #301

**Live at post-merge inspection:** `main=05b5a928c862601926e8b0cc11540ce020caf271`, zero open PRs. [PR #301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) squash-merged from source `089e64a2b6c95dc0705eeaaa22f90194dfb7be66`. [Source CI #39529419994](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529419994) passed **on attempt 2** after an unrelated but significant initial native Windows Node/AppContainer timeout; [native preparation #39529420240](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529420240) passed first attempt. The **exact same new recovery regression** passed on Linux and on the Windows second attempt. [Independent post-merge main run #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) **passed first attempt** on exact squash SHA `05b5a928c862601926e8b0cc11540ce020caf271`, Windows Rust, Linux Rust and dependency policy.

**Concrete source bug corrected:** `MutationRecoveryJournal::discard_unpublished` formerly deleted sorted `*.prepared.tmp` files one at a time. A valid first file disappeared before a later dangerous hard-link alias failed the deletion check. Now the whole list is preflighted via existing identity/link/regular-file checks before any delete; a second pass reopens/revalidates each handle immediately before deleting. A cross-platform negative regression preserves earlier temporary bytes, aliased bytes, and the external hard-linked evidence after rejection, then repairs and confirms successful cleanup. These are scoped preflight-order guarantees, **not** transactional multi-file atomicity or race/durability proof.

**New real Windows failure evidence from the #301 source CI first attempt:** parent outer-helper launch returned in 9ms, then 10s `TimedOut` without captured stderr or Node marker. On the same SHA, diagnostic #1 reached inner `target_resumed=14ms` but only `target_exited=8952ms`, normal network-denied exit 42. Two later diagnostics exited ~360ms from helper entry. See [issue #185 diagnostic entry](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185#issuecomment-6306039365). No cause established; do not infer loader/scheduling fault as fact, weaken the 10s bound, or treat retries as a fix. MCP first-response 30s issue is distinct.

**Next allowed step:** recheck live branch, PRs and exact-head post-merge CI, then investigate the next *demonstrable* isolation/recovery/security defect. Do not repeat #299/#301 or create speculative PRs. Stabilization only, GitHub-hosted Windows for Windows claims, no operator PC, self-hosted runners, release, new features or alpha-wide merge. Historical checkpoints below are not reset targets.



## Handoff verified through PR #299 — 2026-10-21

The first GitHub inspection after [PR #299](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/299) squash merge found `main=6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea`, **zero open PRs**. The accepted source `aca382535f24b6f6b1c26c2313302d315099ed4e` passed [CI #39526324369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324369) on its **first attempt** (native Windows Rust, Linux Rust, dependency policy) and native-preparation [#39526324493](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324493) on attempt 1. **Independent main CI:** [#39526677098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526677098) **passed on the first attempt** on exact merge SHA `6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea` (hosted Windows Rust, Linux Rust, dependency policy). Do not reset to the old #297/#298 checkpoint.

**Delivered:** exact-value opt-in `OPTIC_LAUNCHER_PHASES=1` allows fixed-label, monotonic stderr timestamps inside the outer Windows isolation launcher without printing paths, arguments, SID, handles or credentials; the existing native test and diagnostic-on-failure CI step enable the flag, no new runner workflow. Observed in a successful Node/AppContainer loopback denial proof: relative to launcher entry, `entered=0ms`, `request_read=0ms`, `profile_created=10ms`, `target_created_suspended=14ms`, `token_verified=14ms`, `target_resumed=14ms`, `target_exited=361ms`. Node emitted `NODE_PROBE_STARTED` and `NETWORK_ATTEMPT`, normal denied `exit=42`, no accepted connection. Parent `launch_returned_ms=13`, `terminal_observed_ms=407` uses a separate clock origin. An earlier failure after only 14-17ms *outer* launch has no proof of inner `CreateProcessW`/resume completion: do not equate launch return with Node execution.

**Still open:** native Windows Node/AppContainer intermittent 10s no-child-output failure [issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185); separate MCP TEMP-only 30s first response; recovery multi-delete non-atomicity, ancestor races and Windows power-loss durability. No root cause/fix claimed from a passing instrumented run. The next useful step is to correlate a naturally occurring failed Windows diagnostic with the last captured **launcher** phase, always preserving initial red failures. If the first marker is absent, investigate outer helper spawn/stdio/entry; if only `entered`, request delivery; if `profile_created` absent, profile setup; if `target_resumed` present but no Node marker, investigate loader/init/forwarding. A shared stderr marker is observational data, not an unforgeable attestation. Avoid synthetic CI loops without a hypothesis. Do not weaken timeout, network denial, native sandbox, or required tests.

**Operator direction unchanged:** stabilization only. No Electron GUI, general Cargo, alpha merge, refactor for appearance, operator PC execution, self-hosted runners, release, privileged/permission change, or evidence deletion. Use GitHub-hosted Windows for Windows claims, preserve exact-source and independent post-merge evidence, squash only after applicable checks/reviews. Reconfirm GitHub live state and read `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`, `ROADMAP.md`, this handoff, `CONTRIBUTING.md`, `REPOSITORY_WORKFLOW.md`, `RELIABILITY_DEBT.md`, `CODE_QUALITY_REVIEW.md`, `PROJECT_CONTINUITY_PROMPT.md` before selecting work.


## Canonical agent takeover — verified 2026-10-21, after PR #297

**Last independently observed GitHub state before this documentation PR**: `main` = `83329d48789c20b1fc235d3096c3199dc9dc5176`, **0 open PRs**. [#297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) is merged by squash. Independent [post-merge CI #39514832814](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514832814) **passed on attempt 1 on that exact SHA**, covering hosted native Windows Rust, Linux Rust and dependency policy. The accepted #297 source head was `30ed0d6c6d28df18351a7165dcdd63d2616cf7f7`; [PR CI #39514352067](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514352067) and [native preparation #39514352140](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514352140) passed on it first attempt. Windows and Linux logs explicitly record both new regressions passing. An earlier draft head failed *rustfmt*; the failure remains in Actions history. Do not treat the branch names or candidate paragraphs below as open work.

**Integrated, not merely proposed:** [#293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/293) (`de973989`, journal/staging cross-phase evidence retention); [#294](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/294) (`da3ab6f1`, bounded successful Windows MCP child startup phase reporting); [#295](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/295) (`615e6219`, bounded Git/session/read/process-lease timing); [#296](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/296) (`72966f92`, opt-in path/pin/read+hash identity capture timing); [#297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) (`83329d48`, preflight duplicate and capacity checks before expensive identity capture **plus race-safe revalidation under insertion lock**). Source and independent main CI are successful for all; **#294 post-merge needed attempt 2** after repeatable first-attempt Windows 10s Node/AppContainer failure. Preserve that failed attempt and serial diagnostic failures; a passing rerun does not establish a fix.

**Concrete measurement:** On a passing native Windows #296 source CI, capture consumed `path_ms=0 pin_ms=0 hash_ms=4869 total_ms=4869`, with process-leases provisioned ~4871ms after read authorities. Here `hash_ms` includes reading 64KiB chunks and BLAKE3; it does **not** isolate algorithm CPU cost from filesystem/disk/cold-cache/host effects. No hash cache, bypass or faster production startup has been delivered. #297 eliminates this expense only for duplicate/exhausted **rejected** lease registrations; valid ones still require pinned identity.

**Remaining verified/unresolved boundaries:** (1) native Windows Node/AppContainer loopback gate sporadically times out at ~10s, sometimes with no `NODE_PROBE_STARTED` marker despite rapid parent-side start return; (2) separate MCP first-response 30s intermittency; (3) recovery cleanup can partially remove multiple files if a later deletion fails, and ancestor namespace races/Windows directory power-loss guarantees are not proven; (4) ordinary Cargo and normal Electron sandbox acceptance remain separate incomplete research gates; production credentials/auth are not established. Do not infer production readiness from CI green.

**Current operator authority**: *stabilization-only feature freeze*. No optional GUI/Electron/Cargo product/new integration, architecture expansion, global refactor or whole-alpha merge. No operator-PC build/test/install/tunnel/permission change, no self-hosted CI, no release or mass deletion. Core Rust independent of Electron/Node/Python. Security rule: AI proposes, deterministic policy authorizes, native isolation contains. Never skip executable pin/identity check; preserve task/session ownership, expiring revocable grants, monotonic deadlines, bounded queues/logs/resources, typed effects/ActionEnvelope, exact filesystem preconditions, recoverable evidence and default-deny network.

**Resume procedure**:
1. Read `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`, `ROADMAP.md`, this file, `CONTRIBUTING.md`, `REPOSITORY_WORKFLOW.md`, `RELIABILITY_DEBT.md`, `CODE_QUALITY_REVIEW.md` and `PROJECT_CONTINUITY_PROMPT.md`.
2. Use the GitHub connector **live**: verify default branch SHA, PRs, reviews, branch protections, latest and first-attempt CI. Dated notes are not a live state or a reset instruction.
3. Select **one demonstrated** reliability/safety defect in the existing code. Good candidates: distinct Node/AppContainer and MCP intermittent diagnostics, bounded recovery identity/persistence failure, or an actual uncovered registry/resource bug. A measurement alone is not a bug. Do not run CI unnecessarily.
4. If a fix is justified, choose a short branch, minimal diff, meaningful negative regression, hosted Windows for Windows-specific guarantees, exact-source checks and reviews. Merge squash only after gates; independently verify post-merge main. Preserve failures, and checkpoint reproducible evidence in GitHub.
5. If no defect is shown, record the scoped no-finding without an artificial PR. End the session with exact SHAs, CI attempts, open limitations and a next actionable step.

**Historical material below** records observations made *before* their later merges. It is archived context, not an active candidate queue; [STATUS.md](../../STATUS.md) and GitHub live state outrank any stale heading.

## Historical handoff checkpoints — dated evidence, not active PR candidates

## Canonical current handoff — verified after PR #291 (2026-10-20)

GitHub `main` was independently confirmed at `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`, with **zero open PRs**. [Post-merge CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) completed successfully on that **exact SHA**, first attempt: hosted Windows Rust, Linux Rust and dependency policy. This supersedes candidate/pending language in the dated history below.

- [PR #290](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/290): merged as `0c2a125537d020e3a3aff4124d12745d7a8c5c4c` from `f7b4207395b495f5a70aaa87d441fb075e991b94`. Hosted [source CI #39392262806](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392262806) and independent [main CI #39392829136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392829136) passed. Windows Node/AppContainer test-only monotonic parent timing and `NODE_PROBE_STARTED` marker are integrated, **not a causal fix**. Historical first-attempt 10-second timeout with no output and the separate 30-second MCP timeout both remain unresolved.
- [PR #291](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/291): merged as `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02` from `cdb8407bd22bb91babd9fbe3386da10443d458c1`. Hosted [source CI #39393849343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849343), [native preparation #39393849287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849287) and independent [main CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) all passed. Windows `JournaledMutationService::cleanup_recovered_staging` now preflights all staged artifacts before any deletion and verifies each again through its exclusive native handle before delete. Two-journal Windows regression proves earlier valid staging and both journals remain when later staged bytes mismatch; repair allows normal cleanup. Multi-file deletion **remains non-atomic** if a later removal fails; ancestor namespace races and power-loss durability are still open.
- No feature expansion, new scopes, changed timeouts, disabled tests, local-PC execution, self-hosted runners, releases, archive deletion or whole-alpha integration. The connector has no individual merged-branch deletion action; no mass cleanup attempted.

**Next allowed step:** after reconfirming live `main` and CI, inspect one *demonstrable* remaining reliability/security defect or correlate a naturally occurring Windows Node/MCP failure using existing phase diagnostics. Fix only with a negative regression and exact-head hosted native checks. If no defect is proven, record the finding and stop. Read older handoff entries below as historical observations even where they once said “candidate”.

## Current operator handoff — stabilization freeze (2026-10-20)

**Temporary delivery policy (operator decision, 2026-10-20): stabilization only.** Pause new features, product expansion, experimental merges, architectural reshuffles and broad refactors. Work exclusively on verified defects, security and recovery boundaries, flaky-test root causes, focused regressions, and accurate status/debt documents. Do not create code changes merely to keep PRs moving; when no defect can be demonstrated, record the evidence and stop. Preserve all existing permissions, isolation, deadlines, budgets and CI gates. Resuming feature development requires fresh operator direction.

Live GitHub `main` at this checkpoint: `b1edc83438571867bd164ea0b2b9653304eed752`. [PR #288](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/288) was squash-merged at `b1edc83438571867bd164ea0b2b9653304eed752` from source `60206dcadbf2efb57800ca93f0bd226acf16f6fb`. Its hosted [CI #39388280621](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280621) passed Windows Rust, Linux Rust and dependency policy; [native preparation #39388280521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280521) passed too. Independent [post-merge CI #39388749406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388749406) succeeded on the same merge SHA, first attempt (Windows, Linux, dependencies). GitHub showed zero open PRs at verification. No open PRs at this checkpoint.

#288 completes the bounded first-phase recovery-scan ordering fix: stable records are classified before unpublished temp cleanup, tested on hosted Windows and Linux; it is **not a multi-file atomic-delete guarantee**. Do not repeat #277–#288. The checked-in old headings labelled “candidate” are historical checkpoints, not live status or unfinished assignments.

**Next permitted work:** examine one existing defect or the recorded Windows Node/AppContainer / MCP first-response intermittencies. Keep an evidence-first threshold: identify a reproducible failure (or bounded log-based evidence), define a negative regression, correct the smallest existing boundary, and run the exact-head hosted Windows/Linux checks. If the cause is unknown, retain debt and avoid speculative fixes, timeout increases, disabled checks or new features. No PC actions, self-hosted runners, releases, archive/evidence deletion, massive branch cleanup, or whole-alpha integration.

## Historical handoff — #288 proposal, already integrated

The previously pending `fix/recovery-inspect-preserve-temp-on-failure` branch was delivered by merged PR #288; source, merge SHA and post-merge CI are recorded above. Remaining cleanup atomicity and directory-identity limitations are still open.

## Integrated Linux directory sync — PR #287 (2026-10-20)

[PR #287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/287) was squash-merged as `0ca2305e7f01d5263330b67c4a0bb0520d452c8a` from final head `66161c9dee9e1a565b074d1cf105cf8961a712c2`. Hosted [PR CI #39386566516](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566516) and [native preparation #39386566570](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566570) passed; independent [main CI #39387133506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39387133506) passed Windows Rust, Linux Rust and dependency policy on its first attempt. This fixes one Linux directory-entry persistence barrier after no-replace journal publication; it does not certify sudden-power-loss durability or Windows directory flush.

## Integrated no-clobber journal publication — PR #286 (2026-10-20)

[PR #286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/286) was squash-merged as `67c65a5d78a2921cee826184a2ad8bc573e4fb88` (exact head `7007ad3d202e42f642cece5f8df131035806ff35`). Hosted Windows/Linux Rust, dependency policy and the three native checks passed on that head; independent [post-merge CI #39385303625](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39385303625) passed Windows/Linux/dependencies on first attempt. The source code and proof establish no replacement of an existing journal during publication on hosted Windows/Linux GNU; they do not establish directory flush or power-loss durability on Windows, nor full ancestor identity guarantees.

## Current scoped state-root setup preflight (2026-10-20)

On GitHub `main` was `adeed077215c2103991254e4b5e2c64fa2d632de`, with zero open PRs. [Post-merge CI #39381445867](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39381445867) finished green on exact main: Windows Rust, Linux Rust, dependency policy, attempt 1. Work branch `fix/recovery-state-root-preflight` corrects an actual side-effect-before-denial in `MutationRecoveryJournal::open`: when the chosen state-root did not yet exist within a workspace, `create_dir_all` previously created it before detecting and rejecting the workspace location. The candidate probes the canonical nearest existing ancestor first, retains the post-create canonical check, and tests refusal without project directory creation plus valid external directory creation. This is a **pending targeted PR** until exact-head CI and squash are observed; refresh live GitHub state.

Parent-directory races between preflight and creation are still possible. No blanket security or durability claims. #277/#278/#282/#284 journal/staging hardening is already merged; do not redo. The Windows AppContainer/Node and MCP first-response intermittencies remain separate unexplained reliability debt. Next after validating this increment: narrow examination of journal publication no-clobber semantics or root identity, without speculative broad native changes. Preserve alpha ancestry and all evidence refs. No operator-PC tests, self-hosted runners, releases or mass deletions.


## Scoped Windows staging recovery evidence — 2026-10-20

GitHub bootstrap: `main` `d11d27c8efb1048f504d96024e4015ec60fd1535`, zero open PRs, and [post-merge CI #39376986248](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376986248) success on first attempt for Windows Rust, Linux Rust and dependency policy. The current work is [PR #284](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) on `fix/recovery-staging-handle-delete`. **Its eventual merge and checks are not pre-asserted**: re-open GitHub and validate exact head, reviews, hosted Windows/Linux and post-merge `main` before declaring success.

The defect was demonstrated in source: Windows `cleanup_recovered_staging` accepted a staging hard link if bytes matched the journal, closed the validation file handle then independently deleted the pathname. The targeted replacement uses the existing native exclusive, non-reparse, single-hard-link delete opener; verifies bounded content and deletes with the same live handle. One native regression asserts hard-linked external evidence, staging bytes and durable journal all survive a rejected recovery. No change to mutation contracts, new rights or byte ceilings.

Remain explicit: the root/parent directory is not pinned by this increment; full ancestor-reparse or namespace replacement resistance and power-loss durability are not proved. Separate Windows Node/AppContainer/MCP cold-start intermittencies remain active debt. Next follow-up should investigate only a demonstrable flaw in remaining temporary journal publication or root identity, not another broad refactor. No PC execution, self-hosted runner, release or mass branch cleanup. Preserve main/alpha separation.


## Current GitHub continuation — journal append integrated, 2026-10-20

Fresh verified `main`: `bf79b320b4e7aa04a02c87fe448d589296bc8bb2`. [PR #282](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/282) was squash-merged from final head `c837cead816fb59c5d3a94b8d9b5fe1926987644`. That head passed all hosted Windows Rust, Linux Rust and dependency policy checks in [CI #39375905332](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905332), plus Windows [native preparation #39375905331](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905331). Its first head failed a Windows-only Clippy `unused_mut` warning; the final head corrected it without suppressing Clippy. The post-merge [main run #39376336498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376336498) had started on that SHA and was still executing when this note was drafted. **Verify live status**, not this time-stamped text, before describing post-merge CI as passed. GitHub showed zero open PRs after #282.

The scoped fix to `MutationRecoveryJournal::append` rejects hard-link aliases before appending, using a checked native exclusive Windows state-file handle or Unix opened file identity/link check; length is bounded against the actual opened file and the existing sync-before-effect contract is unchanged. The regression proves both outside and in-journal bytes unchanged after a rejected `mark_committing`. Do not redo #277 (read), #278 (retire), #280 (MCP timing), or #282 (append). Parent-directory identity, Unix concurrent path/size races, Windows staged-artifact cleanup and power-loss durability remain separate unproven bounds.

**Next focused choice:** inspect the distinct Windows staged-artifact cleanup / opened-handle identity path for a demonstrated flaw with a small native regression, or investigate an actual timed-out AppContainer/Node or MCP first-response run using the opt-in #280 diagnostic markers. Do not invent the root cause or weaken timeouts. Keep `main` and `alpha/generated-project-editing` distinct. No tests, builds, installs, servers, tunnels or permission changes on the operator's PC; hosted CI only, no self-hosted runners, release or bulk deletion.

## Integrated startup timing probe and follow-up — 2026-10-20

A new chat confirmed `main` `0f857b81a0715b2fcb7cb8f1e2a4250bf9221fa6`, no PRs open before work and [post-merge CI 39370890039](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39370890039) completed successfully on attempt 2. Windows attempt 1 of that CI failed in the existing TEMP-only MCP doctor with a 30-second response timeout and an alive bridge, while Linux and dependency checks passed; the initial failure is retained. The separately recurring AppContainer/Node loopback `TimedOut` (10 seconds, no output) also remains a separate investigation.

[PR #280](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/280) squash-merged as `8b6ccdc2270f8fa130df5d182d728d07466df908` from exact final head `ce15d036332d314e7f951042285d4f442d0219a7`. [CI 39373512931](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373512931) passed Windows/Linux Rust and dependency policy. The new opt-in smoke produced process-started=7ms, initialize-sent=30ms, initialize-received=4823ms on a passing runner (elapsed from doctor launch). A previous head's rustfmt failure was corrected, not waived. Separate post-merge [main CI 39373992973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373992973) was still in progress when written; check actual conclusion before claiming post-merge success. The bounded diagnostics increment adds a static opt-in startup clock (`OPTIC_STARTUP_TIMING=1`) to the Rust bridge's stderr and measures spawn/initialize handoff with monotonic time in the existing hosted Windows doctor. The scoped CI step passes `-StartupTiming` only to its child process. There is no additional timeout, fallback, capability, secret disclosure, stderr-to-stdout redirect, network grant, test skip, runner type, or operator-PC action. The phase markers separate pre-main OS startup from argument/policy/server/transport preparation and MCP handshake. If the failure repeats, correlate marker last-seen with host-side timestamps; do **not** infer causation from timing alone.

**Next agent action:** Verify `main` and post-merge run 39373992973, then investigate an actual failed Windows timing trace when available. First check whether the stalled stage was before Rust entry, during configuration, before MCP service creation, or during the first response. Do not interpret the 4.8s passing sample as proof of a defect, or reduce isolation/extend watchdogs. Investigate the 10s AppContainer Node-loopback intermittency separately from 30s MCP first response, then return to journal append hard-link boundaries as a different increment. Preserve the alpha branch, existing evidence and historic failed attempts.


## Current handoff: recovery journal read and retirement integrated — 2026-10-20

At live inspection, `main` became
`c2282ff5eba85d2a7480dd353db638a0623bd06d` after
[PR #277](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/277)
was squash-merged. Exact source `825bcdb963d80ac8419df887e8b8c307aa497caf`
passed four hosted checks in
[CI 39368379416](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379416)
and [native preparation 39368379422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379422).
The distinct post-merge `main` CI [39368741554](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368741554)
completed successfully on exact post-merge `main` (Windows, Linux and dependency policy).

[PR #278](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/278)
was also squash-merged into `main` as
`b15fe7636dcb31033d71ecdfea9ed51bdc786a2b`
from exact head `9778b497320d82db493bfbe4c02ff13d113c8ad5`.
Its [CI 39369331802](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331802)
passed hosted Windows/Linux Rust and dependency policy;
[native preparation 39369331816](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331816)
passed. The independent `main` CI
[39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540)
failed Windows Rust Test on attempt 1 for the known intermittent native
AppContainer/Node network probe (`TimedOut` at 10s, no output); three serial
reruns of the single diagnostic passed. The original failure remains retained
and the failed-Windows-job-only second attempt on exact `main` SHA `b15fe76`
**passed** its native tests and smokes. Linux/dependency passed on the first
attempt. CI 39369706540 completed successfully on attempt 2, but its original
failure remains relevant and recorded; the unknown cause is not fixed.
The original
#278 head failed rustfmt, corrected on a new exact head without waiver.
There were zero open PRs following the observed merge. Previous handoff
checkpoints below are historical and must not be treated as reset targets.
Branch retirement tooling is unavailable through this connector; the integrated
work branches remain without bulk cleanup.

**Next specific review:** prioritize the now-repeated Windows
`capability_free_appcontainer_denies_node_loopback_connection` failure
(run 39363540900 and again 39369706540 attempt 1). Inspect bounded isolation
launch/scheduling/start-of-script timestamps and deterministic diagnostics;
do not extend the 10s watchdog, disable the native gate or interpret diagnostic
passes as a production fix. In a separate subsequent increment, review the
journal append path in
`crates/optic-bridge-runtime/src/mutation/recovery.rs`: it checks pathname
metadata then opens for append separately, without the equivalent alias/
opened-file identity protection now used for reading or retirement.
Demonstrate a defect with a hard-link fixture, then isolate the correction with
native tests. Do not bundle this with Windows staged-artifact cleanup, directory
identity, process-smoke intermittency or research compatibility gates.

Unresolved: the separate journal append and staging-artifact cleanup boundaries,
Unix pathname deletion race, directory pinning, power-loss durability, and the
Windows Node/AppContainer timeout. Keep alpha branch and archive/proof refs
untouched. No PC testing, self-hosted runner, release or broad cleanup.


Reviewed: 2026-10-20. Repository:
[ELITEAYOTO/Optic-AI-Bridge-](https://github.com/ELITEAYOTO/Optic-AI-Bridge-).
This document makes continuation independent of the previous chat and its local
files. [STATUS](../../STATUS.md) owns measured delivery; [ROADMAP](../../ROADMAP.md)
owns acceptance order; contracts and ADRs remain normative in their own scopes.
## Latest verified handoff checkpoint — 2026-10-20

At the time of this final handoff verification, `main` was
`e222ff912ac784eb7ae29d09e74ae1780f8a051b`; there were **zero open PRs**. Documentation reconciliation
[PR #275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/275)
had been squash-merged, and exact-`main` CI
[run 39364949397](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39364949397)
was **successful** (hosted Windows Rust, Linux Rust and dependency policy).
The preceding #274 post-merge run 39364345041 also completed successfully.
Verify GitHub again in a new chat; do not treat this dated SHA as a reset target.

The physical mutation-domain move and blank-journal-record correction (#273/#274)
are **finished**. Do not redo them. The first-attempt Windows Node/AppContainer
timeout in the original #274 CI (run 39363540900) remains an open reliability
finding despite same-head success on rerun; preserve both attempts. Cargo normal,
Electron sandboxed, disk quota enforcement, real power-loss ACID and production
auth in the generated-project alpha remain **unproven**.

**Next specific work candidate:** inspect mutation recovery journal file-open,
read and retirement boundary for a demonstrable safety or evidence-preservation
defect with regression tests, or isolate the Windows AppContainer cold-start
timeout in a separate PR. Choose only one bounded increment with native checks;
do not manufacture a bug, expand capabilities or perform a mass refactor.

## Current operator direction

Continue development, hardening and documentation through GitHub using the
connector and the new agent's own isolated environment. **No tests or runtime
changes on the operator's PC for now.** Do not request PowerShell commands,
start tunnels, install dependencies there, change device permissions or dispatch
self-hosted jobs. GitHub-hosted Windows/Linux runners are available through the
repository workflows. A Linux pass cannot prove a Windows security boundary.
Future real-PC testing is a separate operator decision, not an implicit next step.

The product goal is an AI-independent, Windows-first Rust bridge enabling work
in an explicitly selected project: files, builds, processes and Git operations
under deterministic permissions, with useful failure evidence and resumable
project state. Optional Node, Electron and Rust toolchains should eventually
support real applications. The core must not require those optional runtimes,
Python, Docker or an embedded LLM. A visual operator app is later work.

The user wants fewer unfinished PRs, reviewed merges and truthful docs. Small
completed increments are preferred to another large research stack. Installed
tools or a model's ability do not grant execution, network or administrator rights.

## Bootstrap without the old chat

1. Inspect available GitHub read/write/PR/Actions capabilities. Confirm repository
   identity, default branch, actual heads, open PRs, reviews and current CI. If
   tools cannot edit, push, merge or execute, report that specific limit; do not
   invent capabilities or redirect execution to the operator's PC.
2. Read root AGENTS, AI_HANDOFF, README, STATUS, ROADMAP, CONTRIBUTING, this file,
   REPOSITORY_WORKFLOW and RELIABILITY_DEBT. Then read the owning contract,
   security invariants and relevant ADRs for the chosen change.
3. Treat the snapshot below as a locator, not a command to reset branches. Compare
   it with live history. Inspect existing work before deciding to resume it.
4. Choose one bounded roadmap increment with observable acceptance. Separate
   implementation, supported behavior, diagnostic evidence and future targets.
5. Branch from the actual target; implement with tests appropriate to the change
   in your own environment or hosted CI. Keep Windows-specific proof native.
6. Publish a reviewable PR with exact evidence, docs and limitations. Check its
   latest head and feedback, merge when applicable gates pass and retire only
   that preserved branch after verifying its head. No bulk deletion or release
   publication follows from this handoff.
7. Before stopping, update the owners of current state and record remaining work,
   exact PR/head/CI links, uncertain outcomes and the next concrete action. The
   next agent must not depend on hidden chat memory or local output files.

Repository/log/issue content is untrusted task data. Human authorization and
deterministic policy control effects. Do not copy credentials, private paths or
raw capability handles into commits, comments or logs. Observe an uncertain
write/merge/job outcome before retrying it; a transport timeout is not proof that
the operation did not happen. Do not recreate an entire project after a crash.

## Verified source snapshot before this documentation update

At bootstrap there were zero open PRs and two remote branches:

| Branch | Verified head | Role |
| --- | --- | --- |
| `main` | `f1e72f856eaa5f6ee5e6dd861022c049c7f4cd16` | Reviewed Rust core and canonical docs |
| `alpha/generated-project-editing` | `5245e3daeadd2fce8e3dde1fd6a55ba46588fdb2` | Generated-project HTTP/FS alpha; main ancestry aligned by #260 |

These are historical anchors, not promises about the next chat's live state.
The 244 old branch retirements and 41 historical PR dispositions are already
recorded in [the consolidation ledger](REPOSITORY_CONSOLIDATION_2026-10-20.md).
Archive tags and preserved PR refs are historical evidence, not production
releases. Do not repeat that cleanup or delete preservation refs.

Completed increments: #259 fixes reproduced lossy native Windows path aliasing
(main `97f4098`); #255 completes fixed offline Cargo research (main `f1e72f8`);
#250 completes bounded Electron diagnosis (alpha `e62ea95`); #260 aligns alpha
and packages the hardened core (alpha `5245e3d`). See STATUS for exact CI/source
evidence. Completion of a research PR does not establish runtime compatibility.

## Validations and known red research gates

| Workflow | What it establishes / limit |
| --- | --- |
| `.github/workflows/ci.yml` | Rust Windows/Linux checks and dependency policy; inspect job results, not only aggregate status |
| `private-project-preparation.yml` | Native private-project preparation contract |
| `owned-native-process-pipes.yml` | Owned native process/pipe lifecycle |
| `private-rust-compilation.yml` | Fixed direct-rustc compilation; does not prove general Cargo builds |
| `offline-cargo-research.yml` | Independent normal and prepared disposable Windows comparisons; normal currently fails NUL access with exit 101, prepared succeeds |
| `private-application-staging.yml` (alpha) | Node staging plus Electron controls; normal Electron fails before app startup (`0x80000003`), explicit outer-only diagnostic succeeds |
| `real-link-admission.yml` (alpha) | SDK/generated adapter acceptance and source-identified alpha bundle |

Inspect workflow source, event/path filters, permissions and actual runner labels
before triggering it. Never run the NUL research grant outside its disposable
hosted fixture, change a self-hosted machine, silently disable Electron isolation,
add `continue-on-error` to hide unsupported behavior or relabel a comparison as
ordinary support. Distinguish an unchanged, documented research baseline from a
new regression; preserve evidence when retrying a proven infrastructure failure.

The latest previously verified package is an experimental Actions artifact, not
a permanent release. Artifact `12229445645`, run `39340735266`, source merge
`ac76efd6ea833e46dfced7bee3148af968754566`, PR260 head `a4091bc`.
Payload ZIP SHA256:
`3563bb8e9666455e6559cf6e30362b99849eb437fe91769fc1b6691c2113bc00`.
Its 85 entries/80 source files and packaged HTTP/Node fail-fix checks were
previously verified. Actions artifacts expire; inspect availability and rebuild
with provenance if needed. Do not assume the new chat can access old local ZIPs.

## Scope that remains open

- General writable builds need actual containment, storage enforcement, owned
  stop/quiescence, crash evidence and artifact publication. Input and post-exit
  byte limits are not runtime disk quotas.
- Optional Cargo preparation needs trusted broker/lease provenance, explicit
  operator preview/approval, revoke and interrupted recovery. The user approved
  developing that option, not applying administrator changes to their PC.
- Dependency restore/package scripts/egress/secrets need scoped authorization.
  General project enrollment and normal Electron startup remain unproven.
- The FS adapter uses generated projects, shared authority and public dummy
  authentication. It is not production authentication or conversation isolation.
  Host conversation IDs are not authority. FS backend logs and hidden model
  state cannot be reconstructed by Optic.
- Small paginated reads were a workaround for observed host truncation, not a
  universal 1024-byte security rule. Preserve bounded output and version/offset/
  EOF metadata; measure total serialized responses before increasing budgets.
- Native path hardening is scoped; Windows case/identity semantics and other
  remaining findings live in [RELIABILITY_DEBT](RELIABILITY_DEBT.md).

Prioritize the roadmap's next provable contract, preferably enabling headless
real-project work without waiting for Electron. Do not expand into every runtime
at once. Useful cross-cutting improvements include clear bounded diagnostics,
version-safe resume, efficient tool responses and reliable shutdown; implement
only after inspecting current code/tests so closed defects are not reopened.

## Reporting and completion

Report what changed, exact PR/commit and CI evidence, what remains unsupported
and the next action. State explicitly which validations were not run and why.
Use GitHub links rather than personal filesystem paths. Never say “all tests
pass” when normal Cargo/Electron research is red. Documentation-only work needs
accurate links/state, not a claim of runtime verification. Keep historical
audits dated and annotate their scope instead of rewriting measured history.

## Latest verified quality cycle — 2026-10-20

This is a **newer checkpoint** than the bootstrap snapshot above. GitHub
confirmed `main` at `5a8157ea2fff0b19c4b47e075eeefa457947efc1`,
zero open PRs after completing:

- [PR #265](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/265):
  merged as `a9d9a31b4e1635c35ac53c8225a4c5b4348aeb05`. Moved 1,115
  lines of tests from `process.rs` into `process/tests.rs` without
  altering the production prefix. Final head `f65f69b` passed four hosted
  checks (Windows/Linux Rust, dependency policy, native preparation);
  post-merge main CI run 39355110640 passed.
- [PR #266](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/266):
  merged as `5a8157ea2fff0b19c4b47e075eeefa457947efc1`. Corrected silent
  process stdout/stderr read errors: incomplete output is flagged, and an
  otherwise successful exit becomes `Failed`; existing owned stop,
  deadline and termination-uncertain handling remain distinct. Two
  synthetic failure/EOF tests were added. Exact head `4e4c712` passed
  four hosted checks. Previous head failed rustfmt and was corrected.
  Post-merge main run 39356012838 was still in progress at documentation
  creation; verify its actual result rather than inventing a pass.

Both operations used hosted Windows/Linux runners only; no user-PC
process, install, permission change, tunnel or self-hosted runner. The
normal offline Cargo and sandboxed Electron research gates remain red.
The separate generated-project alpha remains unmerged into main and is
not production authentication. None of this establishes general writable
builds or runtime disk quota enforcement.

[CODE_QUALITY_REVIEW](CODE_QUALITY_REVIEW.md) now tracks the two scoped
source reviews; this is **not** a full review of all 212 baseline files.
Next review target: `process/output.rs` panic/JoinHandle observability,
the remaining spawn/launcher/lock paths, then operator CLI parsing and
session lifecycle. Only add fixes backed by specific tests and native
Windows evidence; avoid unrelated reorganizations.

The connected GitHub actions available in this chat do not include a
delete-branch operation. Previously merged short-lived branch heads
remain until individually verified and retired; preserve archive tags,
historical proof refs and `alpha/generated-project-editing`. Do not start
another mass cleanup. New agent should inspect current heads/checks/PRs,
especially the post-merge run and any later changes, before proceeding.

## Latest runtime Git layout consolidation — 2026-10-20

This supersedes the earlier source snapshot. At inspection `main` was
`24a40e1` with no PRs, and the Windows job in [CI 39356540735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39356540735)
had failed only the TEMP-only Node MCP doctor smoke after 30 seconds with the
bridge alive. An exact-source failed-Windows-job rerun (job 118412524278)
succeeded. This remains an unresolved intermittent failure; do not dismiss
or weaken that acceptance.

[PR #268](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/268)
has **merged into main** at
`d79d60b88b60b2e95c85039c2d31427d0954a865`. The source head
`b2b5207834efe27b131aa1e7d1b46372e55194c0` passed all four
applicable GitHub-hosted checks (Windows/Linux Rust, dependency policy and
private native preparation). Ten `src/git_*.rs` files moved to
`src/git/{blob_batch,change_set,integrate,integration_authority,merge_commit,merge_plan,read,tree_manifest,tree_write,worktree}.rs`
with identical file blob SHAs. `lib.rs` uses explicit `#[path]` to preserve
all previous `git_*` module names, public reexports and private test
namespaces. No authorization, resource ceilings, contracts or functionality
were altered. Post-merge `main` CI run 39358651176 must be inspected
separately.

Next action: verify actual post-merge CI, then continue **one domain at a
time** (session and lease lifecycle or file mutation/recovery) only if module
dependencies and tests justify it. A second priority is investigating the
Windows MCP first-response intermittency with deterministic, bounded
diagnostics; no workaround is approved. The file-by-file audit is ongoing,
not complete. The separate generated-project alpha remains isolated,
no broad Cargo or sandboxed Electron support was proven, and no runtime disk
quota is claimed.

Do not touch the operator's PC, request PowerShell, change local permissions,
start tunnels or use self-hosted runners. The connected GitHub action surface
does not provide deletion of old integrated branches: retain them until a
specific verified retirement capability is available, and never delete
archive/proof references or conduct another mass cleanup.

## Latest completed stabilization cycle — 2026-10-20

The previous session/Git layout work is now superseded by these exact
GitHub observations:

- [PR #270](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/270)
  merged on `main` as `636a8b5e917fd731dbd8566e118958f141f67542`
  from source `4d5ff0a4372c465b7e65b7d1bd581c0d44fe9f3b`.
  The four session-domain Rust source blobs were unchanged; paths and
  `lib.rs` declarations only. Four hosted checks passed on exact PR head;
  post-merge main CI 39360825623 passed.
- [PR #271](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/271)
  merged as `8610c8dc2558695842412c5a0fce993d2420d6fb`
  from source `fb9ffdb8b6489b09653e3a8ea5ed6c71634b08dc`.
  A failed stdout/stderr Tokio drain task join now sets the existing
  incomplete-output signal and closes the stream, preventing clean
  `Exited` reporting for incomplete collection. Deliberately aborted
  drains following `TerminationUncertain` retain their original uncertainty
  semantics. Two synthetic tests were added. Exact source passed four hosted
  checks; initial head `bed3ca9` failed formatting and was corrected.
  Post-merge main run 39361585398 was still in progress when this handoff
  was drafted and must be checked live.

The source inspection of session/lease admission/revoke/renew is **scoped**;
no exhaustive per-file or platform security audit was completed. Follow
[CODE_QUALITY_REVIEW](CODE_QUALITY_REVIEW.md) and keep known research
Cargo/Electron and Windows MCP TEMP-only intermittency evidence intact.
No PC tests, installation, permission change, tunnel or self-hosted runner
was used.

**Next action:** verify actual latest main post-merge CI. Then select one
bounded code-quality increment (e.g. process monitor shutdown/cancellation,
session lock-order tests, or mutation/recovery layout) based on a demonstrated
problem or structural benefit. Do not group many unrelated moves. Keep
`alpha/generated-project-editing` isolated and its ancestry intact.
No branch-deletion action is available through this connector; only
individually verified integrated work branches may later be retired, not tags,
archive/proof refs or a bulk set.

## Mutation/recovery stabilization checkpoint — 2026-10-20

This section **supersedes previous in-progress observations**. The latest
verified `main` immediately after the merges was
`58a72a202bbdb57da425fba461a7b1a97b2f9d92`, zero open PRs.

1. [PR #273](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/273):
   source `97584927ba438efdd843b7bd4ad488d3c321e43f`, squash
   `58ebc5c1a4f0fd015b26c04f8cd83cf4255d5782`. Six exactly unchanged
   mutation/recovery Rust blobs moved under `src/mutation/`, with module
   identities and exports preserved by six explicit `#[path]` declarations.
   Four hosted Windows/Linux checks passed; post-merge main CI 39363288153
   succeeded. No semantic file/journal change.
2. [PR #274](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/274):
   exact final source `283c9e0e699a50470fab2183dc9945b9d4559c23`, squash
   `58a72a202bbdb57da425fba461a7b1a97b2f9d92`. The recovery parser
   rejects completed blank newline-delimited records (`CorruptJournal`)
   while preserving the contractual torn-final-line recovery. Regression
   tests cover corrupt interior/trailing blank lines, evidence retention and
   permitted torn tail. The initial `8d51d19` head failed `rustfmt`, fixed
   without silencing checks. Linux, dependency and native preparation
   passed. Native Windows run 39363540900 first attempt failed a separate
   Node/AppContainer loopback probe because the child `TimedOut`; three
   serial diagnostics passed, and an exact-source failed-Windows-job-only
   rerun passed. **This is an unresolved intermittent CI failure**, not
   fixed compatibility. Preserve the first attempt and study the native
   startup path separately.
3. Post-merge `main` CI run 39364345041 later completed successfully.
   The following documentation reconciliation PR #275 was merged at
   `e222ff912ac784eb7ae29d09e74ae1780f8a051b`; exact-`main` run 39364949397 passed. See the
   latest verified checkpoint at the top; re-check live state after this date.

Follow [CODE_QUALITY_REVIEW](CODE_QUALITY_REVIEW.md) for reviewed files and
[RELIABILITY_DEBT](RELIABILITY_DEBT.md) for unresolved native timing evidence.
Next priority: focused source/test review of mutation-journal file-open and
retirement boundaries or the Windows Node/AppContainer cold-start timing,
with one regression-backed increment at a time. Do not rewrite all mutation
services just to reduce length; their contracts and recovery state machine
remain normative. No all-files audit is complete.

No operator-PC work, self-hosted CI, broad Cargo/Electron compatibility
claim, physical disk-quota claim, alpha merge, new release or mass branch
retirement was authorized. The connected GitHub action surface exposed
no delete-branch operation; preserve archive/proof refs and verify exact
integrated heads before any future individual retirement.
