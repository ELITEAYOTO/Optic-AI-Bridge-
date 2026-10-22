# Optic AI Bridge — Registre des preuves #317, Astra/Codex et reprises
_Date : 22 octobre 2026. Archive de lecture destinée au prochain chat. Référence source : **branche de recherche**, pas `main`._

## I. Les trois travaux indépendants à ne pas confondre

| Filière | Lieu | Résultat | Intégration |
| --- | --- | --- | --- |
| Correctif raw Git diff modes #318 | PR `#318`, `49fe3efd34c9ffe9c10c1c5945b37da2c6a74028` | CI #39585600089 **failure** Windows premier `initialize` MCP 30 s | **Non fusionné** |
| Diagnostic démarrage #319 / campagne Codex | `bootstrap/issue-319-process-lease-phases`, `49b64e213e144be4ba5a608616183add8e9d47a1` | Normal CI #39613682202 **success**. Repro #317 initial ; collecte #319 améliorée. Assemblage recherche #39613681993 **failure** car fenêtre d'attente CI indépendante expirée; artefact tout de même présent. | **Recherche seulement** |
| Ownership #317 / campagne Astra puis ce chat | `bootstrap/issue-317-owned-cleanup-study`, `7b36d971022f6cd732a01b7440b09022e17e277d` **avant passation docs** | Prototype de nettoyage + tests GitIntegration / révocation / saturation / CAS, CI exacte #39696149147 **success** | **Non fusionné**, plusieurs incréments expérimentaux à séparer avant PR |

## II. Chronologie probante de #317

### A. Campagne Codex initiale — blocage réel des pipes du runner inchangé

Le `HardenedCommandRunner` teste le délai du processus enfant **direct**, puis rejoint des lecteurs/writer de pipes sans garantie de retour borné si un descendant conserve un descripteur ouvert. La campagne a démontré sur Ubuntu et Windows un délai de 200 ms et des retours ~1 011–1 217 ms après libération externe. Un superviseur natif/indépendant garde la propriété des fixtures. **Conclusions :** liveness defect reproductible ; pas de correctif de production ; ne pas assimiler aux #319 (30 s MCP) ou #185 (10 s AppContainer).

Sources : [rapport Codex](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/49b64e213e144be4ba5a608616183add8e9d47a1/docs/research/CODEX_STABILIZATION_2026-10-21.md); [#39613682202](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39613682202) normal green, [#39613681993](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39613681993) assemblage rouge. Le rouge de l'assembleur n'est **pas** un échec natif du runner; conserver `PROVENANCE.json` et le résultat effectif.

### B. Astra — expérimentation d'un owner retenu, non intégrable telle quelle

Astra a relevé les sorties d'erreur qui abandonnent des handles avant `join`, et l'absence de registre de réservation dans le runner `Copy`. Une solution Windows Job / Unix group seule ne suffit pas aux invariants.

- Première série 16 Linux réussis ; un essai Windows a mis en évidence une assertion trop immédiate sur Job vide, corrigée vers confirmation **bornée**.
- Ensuite 16 Windows réussis ; scénario Linux avec descendant `setsid` gardant les pipes a démontré que `kill`+joins ne prouve pas `waitpid` ; rouge conservé, puis correction expérimentale.
- Une erreur de fixture supplémentaire a été révélée : Python lui-même avait stdout/stderr sur `/dev/null`, donc le cas `escape-both` n'héritait pas des pipes attendus. Correction ciblée dans `crates/optic-bridge-runtime/examples/issue317_pipe_fixture.rs` au commit [`06d688b6`](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/commit/06d688b6b05642fc749ab6137188fac9466fa640).
- Reprise et validation : [recherche #39688523435](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39688523435), **17 Linux + 16 Windows réussis** ; [CI normale #39688523364](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39688523364), **success** sur le SHA exact `06d688b6b05642fc749ab6137188fac9466fa640`.
- Dossier de conception originel : [`ISSUE317_OWNED_CLEANUP_2026-10-21.md`](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/06d688b6b05642fc749ab6137188fac9466fa640/docs/research/ISSUE317_OWNED_CLEANUP_2026-10-21.md). Les déclarations anciennes « no production code changed » sont **historiques**, car plus tard un correctif expérimental GitIntegration a été apporté dans cette même branche.

Astra a déclaré avoir épuisé la fenêtre Codex, passant de **98 %** mesurés au début à **35 %, 18 %, puis 1 %** et à la limite. L'interface indiquait alors une reprise possible à **04 h 37**. **Aucune valeur de quota ou disponibilité actuelle n'est certifiée par le dépôt.** Lui demander de lire ce registre et de revérifier GitHub avant sa prochaine campagne, plutôt que de reprendre son ancienne branche sur un vieux commit.

### C. Tests de capacité et révocation après Astra

| Preuve | SHA de code testé | Workflow exact, résultat |
| --- | --- | --- |
| Régression `RequestPermit` déplacé dans `spawn_blocking` (capacité conservée après abandon du waiter; succès/erreur libèrent) | `3a5bb8cf2e07528e774eca74479c18020e96bf7f` | [CI #39690755337](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39690755337), **success** Windows/Ubuntu/dépendances |
| Test synthétique : l'admission session et le slot demeurent jusqu'à fin worker, même si caller annulé ; révocation attend l'admission et refuse de nouvelles admissions | `f2f4422156c96e52f55026b8631ccd5a75b6bbfd` | [CI #39691887518](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39691887518), **success** Windows/Ubuntu/dépendances |
| Test synthétique : `TransportGuard.max_concurrent_requests=1` et worker Git tenant le slot => `session_cancel` répond `optic.server_busy` **sans révoquer** ; réussit après libération | `afcee4e479cc71566187c9be42b786280cdf057a` | [CI #39693485443](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39693485443), **success** Windows/Ubuntu/dépendances |
| Fixture native `git update-ref --no-deref` réellement exécuté puis child tenu > deadline; `TimedOut` avec ref avancée vs témoin `TimedOut` sans mutation | `7b36d971022f6cd732a01b7440b09022e17e277d` | [CI #39696149147](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39696149147), **success** Windows/Ubuntu/dépendances, tentative 1 |

**Ne pas interpréter les tests synthétiques comme des solutions intégrées.** Le correctif local `RequestPermit` dans `git_integrate_tools.rs` est réellement une modification de **code de production sur la branche de recherche**, et non une modification fusionnée de `main`. Le reste est principalement instrumentation/fixture.

### D. Cas négatifs et échecs conservés

- `#39681209526` : première CI expérimentale Windows rouge, assertion prématurée Job vide.
- `#39681675221`, `#39682198589` : prototypes Unix / reaping / pipes ont exposé et documenté des erreurs de fixture et d'ownership.
- `#39690055005`, `#39690638363`, `#39691820761`, `#39693355080` : premiers commits de régression Git ont fait ressortir formatage/compilation test-only; certains workflows annulés par concurrence après un nouveau commit. Ces essais **ne sont pas verts**.
- CAS Git : `#39695739428` rustfmt, `#39695823221` compilation `String` vs `&String`, `#39696029105` rustfmt; ces CI ont été supplantées/annulées, **ne pas les réécrire comme réussies**. `#39696149147` fournit la preuve terminale au nouveau SHA.
- PR #318 `#39585600089` : échec Windows 30 s sur exact SHA PR, toujours bloquant malgré les autres recherches vertes.

## III. Signatures exactes et code à auditer

- **Runner** : `crates/optic-bridge-runtime/src/hardened_command.rs`, `run_inner`, `join()` stdout/stderr/stdin, kill/wait/try_wait, sorties anticipées. Runner de `main` toujours inchangé.
- **GitIntegration** : `crates/optic-bridge-runtime/src/git/integrate.rs`, `update-ref` CAS en `integrate_fast_forward`, `map_runner_error(TimedOut -> CommandTimedOut)`, `PostUpdateVerificationUncertain` uniquement après succès de la commande et échec d'observation.
- **Mapping MCP** : `crates/optic-bridge-mcp/src/git_integrate_tools.rs`, `run_git_integration`, `RequestPermit` maintenant détenu par le worker sur branche d'étude, `map_git_integration_error(CommandTimedOut -> optic.request_timeout)`.
- **Admission/revoke** : `crates/optic-bridge-runtime/src/session/registry.rs`, `SessionRegistry::revoke` pose le drapeau puis attend `active_admissions==0`; `crates/optic-bridge-runtime/src/session/lifecycle.rs`, `SessionLifecycleManager::revoke` ne stoppe les jobs `ProcessManager` qu'**après** le drain. `seal_quiescent`/`try_reap` ignorent actuellement un éventuel registre spécifique de processus Git (inexistant).
- **Annulation MCP** : `crates/optic-bridge-mcp/src/process_tools.rs::session_cancel` tente une capacité TransportGuard avant `lifecycle.revoke`; sous saturation = `optic.server_busy` observé dans test synthétique.
- **Recovery au démarrage** : `crates/optic-bridge-app/src/main.rs`, `GitIntegrationService::recover_owned_worktrees` exécuté avant un `SessionHandle` opérationnel ; prévoir `HostStartup` distinct de `Session(...)`.
- **Fixtures** : `crates/optic-bridge-runtime/examples/issue317_pipe_fixture.rs`, `issue317_git_ref_timeout.rs`; `scripts/research/issue317_owner_study.py` et ses workflows GitHub hébergés.
- **Sécurité** : `docs/security/SECURITY_INVARIANTS.md`, particulièrement INV-015 (Git CAS exact), INV-021/022/023/025 (propriété des jobs, plafonds, terminaison incertaine, CPU avant exécution). Aucun pid-kill arbitraire, aucun thread détaché sans ownership, aucune approche Unix sous-reaper globale non prouvée.

## IV. Limites à ne pas gommer

- **Prouvé** : blocage du runner lorsque descendant tient pipes; propriétaire de fixture peut préserver état `Uncertain`; transport permit libéré trop tôt avant correctif puis retenu après; annulation MCP saturée; vrai Git CAS déjà commis malgré runner `TimedOut`.
- **Non prouvé** : intégration d'un owner process natif dans le Bridge; vraie terminaison Unix avec échappement sous le modèle de sécurité applicatif; propagation de `GitRefOutcomeUncertain` dans `GitIntegrationService` en condition naturelle; déblocage de #318/#319; confinement en sortie réseau sous #185 rouge; sécurité du remote workspace.
- **Par choix** : pas de PR nouvelle tant que #318 ouverte, pas de migration silencieuse des erreurs `optic.request_timeout`, pas de changement de plafond/délai/autorisation pour verdir.
- **Obligation** : Windows natif et Linux exact-head et preuve de nettoyage après fault injection; conserver journaux rouges et SHAs du premier échec, vérifier l'absence de fuite/admission anticipée, ne jamais dire « 100 % stable ».

## V. Passage de relais à Astra et nouveau chat

1. Vérifier dans GitHub le **SHA actuel**, la CI et la différence avec les checkpoints inscrits ici. Le commit documentaire ultérieur n'est pas une nouvelle preuve runtime.
2. Reconnaître qu'Astra **n'a pas terminé** sa correction native #317 : il a délibérément arrêté avant le code de production et son quota a expiré.
3. Lui faire lire `docs/research/ISSUE317_OWNED_CLEANUP_2026-10-21.md`, ce registre, puis les commentaires de l'issue #317, surtout [contrat de nettoyage](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6329018999) et [preuve CAS](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6329260125).
4. Reprendre d'abord par un test d'**ownership Rust isolé**, avec autorité Session et HostStartup, signal de stop hors capacité saturable, états `RetainedUncertain` non relâchés, gate `seal_quiescent`/reap; instrumentation native exact-head. Ne pas installer un owner global spéculatif dans `main`.
5. Étudier séparément si les changements GitIntegration doivent être proposés en petite PR revue après #318, **sur une branche propre depuis main**, et non via fusion de la branche de recherche.
6. Budget Astra/Codex si une nouvelle fenêtre apparaît : mesurer réellement, arrêter les nouvelles expériences aux alentours de 20 %, garder 10–15 % pour rapport et vérification; ne jamais atteindre zéro par choix. Le quota affiché précédemment ne prouve rien pour une nouvelle session.

**Références originales :** [issue #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317), [PR #318](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/318), [rapport diagnostic/Codex #319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/49b64e213e144be4ba5a608616183add8e9d47a1/docs/research/CODEX_STABILIZATION_2026-10-21.md), [dernière CI de code #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39696149147).
