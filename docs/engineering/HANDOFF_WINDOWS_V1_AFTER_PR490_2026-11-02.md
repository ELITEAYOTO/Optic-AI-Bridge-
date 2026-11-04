# Optic AI Bridge — passation Windows V1 apres PR #490 (2 novembre 2026)

> DOCUMENT DE REPRISE LE PLUS RECENT au moment de sa creation. Les anciens titres « actuels » du README, AGENTS et des handoffs d'octobre/1er novembre sont historiques. Ce document enregistre le **main avant la PR documentaire**. Interroger GitHub avant toute prochaine modification et apres son squash.

## Dernier point de securite prouve

- Depot : https://github.com/ELITEAYOTO/Optic-AI-Bridge-
- **main avant mise a jour documentaire** : `c69ef87155a1dc1a58520f8cfe11384db2c1ff5f`.
- Arbre Git : `e9fcb448710e516f6048ea51be644af8531038ab`.
- [PR #490](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/490) MERGED. Exact HEAD teste : `9cbb319be38292690bd654b2ad2f6141f53ec63a`, identite d'arbre verifiee.
- [CI HEAD #41518213511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41518213511) : 4/4 SUCCESS tentative 1 (Windows, Ubuntu, dependencies, release guard). [Late-pipe #41518213507](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41518213507) SUCCESS ; [Private Project Preparation #41518213592](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41518213592) SUCCESS.
- [CI independante push/main #41519267054](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41519267054) : **4/4 SUCCESS tentative 1**, `event=push` sur main ci-dessus. Les trois nouveaux tests ont effectivement tourne et affiche `ok` sur runner Windows HEAD et post-squash.
- Tests locaux Windows apres #490 : `fmt --check` OK ; Clippy strict workspace all-targets/all-features OK ; tests workspace `exit 0` (runtime **448 passed / 11 ignored**, MCP **60 passed**, Windows **70 passed / 4 ignored**, autres suites OK). Journal SHA-256 `598FB7F45EADA68FF75811BB95B3544D21D3D1FF1BB174A722A9CE4B7AE8E8A1`.
- Recus : [PR #490](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/490#issuecomment-6587800904) ; [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6587813334).

**[Issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN : Windows 10/11 V1 NO-GO.** Ne PAS publier V1, installateur, release, tag ou activer GitRead natif tant que les verrous de securite ne sont pas prouves.

## Corrections recentes : faits et limites

- **#484** : protocole d'architecture propose, *non implemente*, [ISSUE317_NATIVE_LAUNCH_REVOKE_PROTOCOL_2026-11-02.md](ISSUE317_NATIVE_LAUNCH_REVOKE_PROTOCOL_2026-11-02.md). Separer stop publie, ledger d'appels natifs scelle+draine, et completude physique du Job/EOF.
- **#485** : drainage du compteur d'appels natifs **inerte** : mutex contendu ne peut plus bloquer apres borne absolue. Windows RED puis GREEN. Ce compteur n'autorise aucun vrai lancement Win32.
- **#488** : tests Windows d'une racine Git reelle suspendue dans son Job et d'arret avant reprise ; [PR #488](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/488) validee. N'est pas un branchement MCP/autorite native.
- **#489** : [PR #489](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/489) : MCP recontrole `admission.stop_watch()` sur la **generation de session originale** apres `authorize_git_read`, avant operation. RED : GitReadService entrait dans l'operation apres revoke ; GREEN ensuite. [push/main #41513052188](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41513052188) 4/4 SUCCESS.
- **#490** : MCP transmet la watch originale et l'horloge a une **copie ephemere** de `GitReadService` ; `HardenedCommandRunner::run_with_stop` observe l'arret **juste avant chaque `Command::spawn`**. `status`, `diff`, `log` et sous-processus successifs couverts. RED comportementale : vrai Git, exit 101 (stop non lu) ; GREEN apres correction. Le premier subprocessus log peut reussir et le second doit refuser apres stop. **Ceci n'est PAS un verrou atomique Win32.**

PR de recherche #327/#353/#358 non fusionnees ; #355 research negative de Git apres revoke (reverifier live). #471, opt-in sensible Git diff, DRAFT, ne pas fusionner sans arbitrage.

## Chemin actif, code inerte et P0

**Chemin MCP actif :**
`crates/optic-bridge-mcp/src/git_tools.rs` -> `ReadonlyMcpServer::run_git_read_checked` -> admission originale/politique -> service ephemere -> `crates/optic-bridge-runtime/src/git/read.rs` -> `run_spec` -> `HardenedCommandRunner::run_with_stop` -> `std::process::Command::spawn` (`hardened_command.rs`).

**Le dernier controle reste un snapshot.** Revocation possible immediatement apres ou pendant `CreateProcessW`; un appel deja entre peut creer un enfant. Attentes, joins, pipes et descendants ne sont pas universellement interruptibles.

**Preparation native Windows non activee :**
- `git/native_status.rs` : `PreparedNativeGitStatus` -> `RegisteredNativeGitStatus` -> `PrecreatedNativeGitStatusCustody` -> `SessionCommittedNativeGitStatusCustody` -> `LeaseObservedNativeGitStatusCustody` ; admission/lease/clock/pinned targets/Job/watchdog **sans lancer Git en production**.
- `git/native_inflight_calls.rs` : tickets Create/Resume, stop, borne absolue, poison/Unknown et drainage ; **inerte**.
- `session/registry.rs` + `task_lease_registry.rs` : revoke direct/lifecycle, observation conjointe session lock -> lease try-lock ; snapshot seulement.
- `optic-bridge-windows/src/job.rs` + `job_list_launch_tests.rs` : Job kernel, racine suspendue et fixtures, deux EOF physiques ; pas de permission native MCP.

**P0 technique** : issuer **service-owned, non-cloneable, lie a l'original** (session, TaskLease, policy epoch, host, repo/exe epingles, action/quotas, deadline absolue, Job). Inscrire et compter de vrais appels `CreateProcessW` / `ResumeThread` avec toutes les revoke directes/lifecycle, **sans mutex de registre sur appel OS bloquant**. Stop prompt ; appels in-flight comptabilises jusqu'au retour ; processeur cree suspendu lie au Job a la creation ; stop interdit reprise tardive. Sceller/drainer appels, conserver ownership jusqu'a **root exit + Job empty + EOF stdout + EOF stderr + no future native call**. Poison/timeout/crash = Unknown, jamais succes fictif. Rendre [test #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) GREEN via vrai MCP sans suppression/affaiblissement. Reference [protocole](ISSUE317_NATIVE_LAUNCH_REVOKE_PROTOCOL_2026-11-02.md).

**Autres gates** : #176/#182 sensitive read, #185 Node/AppContainer, #319 initialize MCP intermittent. Vision long terme profils/permissions, Git, compilation, generation projets, tests isoles/VM ; **pas de developpement produit premature**, eviter pourtant de verrouiller l'architecture.

## Methode contractuelle (UNE PR complete)

1. Interroger GitHub main SHA/tree, CI `event=push`, issues/PR, code et AGENTS **live** ; ne jamais croire a la permanence d'un SHA de ce fichier.
2. Une PR **complete** a la fois dans un **nouveau LAB isole**. Ne pas toucher Bridge quotidien, Desktop, REL, anciens LAB sales, secrets.
3. RED comportementale **executee** avant correction quand faisable ; format/compilation/zero-test/cancel ne sont PAS des RED. Fixtures bornes et nettoyage RAII, pas de rerun-to-green.
4. Correction minimale ; `cargo +1.99.0 fmt --all -- --check`, Clippy `--locked --offline --workspace --all-targets --all-features -- -D warnings`, tests workspace `--test-threads=1`, hosted Windows/Ubuntu exact-head, workflows specialises et preuves schema8 si applicables.
5. Auto-revue seulement **COMMENT**, jamais APPROVE ; squash attendu sur HEAD exact+base inchangee, verifier arbres Git identiques ; **CI independante push/main 4/4 premiere tentative** sur SHA integre, recus PR et #323.
6. Pas de runner auto-heberge, relachement permissions/timeout/tests, release/installation, activation native non prouvee. Nettoyer uniquement caches regenerables de LAB apres archive verifiee.

## Premier mouvement du prochain chat

Lire [prompt de reprise](NEXT_CHAT_PROMPT_WINDOWS_V1_AFTER_PR490_2026-11-02.md), ce document et protocole issuer. Lister les appareils autorises Remote Desktop Commander, interroger `gh` live, lire l'etat de #323 et #355. Selectionner le prochain P0 vers un **vrai issuer Create/Resume/revoke**, pas une nouvelle serie de tests sur ledger inerte. Si connecteur indisponible, ne jamais inventer des modifications effectuees.

Ce fichier est un jalon date, non une autorisation V1.