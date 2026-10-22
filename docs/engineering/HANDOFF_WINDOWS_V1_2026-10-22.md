# Optic AI Bridge — Passation technique canonique pour le prochain chat
_Date du constat : 22 octobre 2026. Checkpoint GitHub, pas état live garanti après rédaction._

**Lecture prioritaire.** Ce document indique l'état du chantier à la coupure, ses preuves et la méthode de reprise. Commencer par interroger GitHub avant toute modification. Les SHA mentionnés sont des observations, pas des consignes de reset ni des autorisations de fusion.

## 1. Directive du propriétaire et périmètre

**Objectif final :** progresser vers une première V1 Windows de l'existant Rust/MCP, fiable, sûre, cohérente, documentée, maintenable et correctement testée. **Étape actuelle : stabilisation de ce qui est déjà développé** ; pas de gel V1 tant que les portes de sortie pertinentes ne sont pas franchies. Ne pas promettre un produit « 100 % sans bugs » : rechercher l'absence de défaut bloquant connu atteignable, et documenter les risques résiduels.

**Autonomie :** l'agent choisit les investigations et corrections étroites, une par une, et peut créer des branches/PR vérifiables selon les règles du dépôt ; ne doit pas inventer de tests accomplis ni demander une autorisation pour chaque détail technique. Solliciter le propriétaire seulement pour de vraies décisions produit/sécurité, pour agir sur son PC, pour modifier les protections GitHub ou pour publier une release.

**Interdits toujours actifs :** aucune installation, exécution, serveur, tunnel, élévation, permission ni test sur le PC utilisateur ; pas de self-hosted runner. GitHub et ses runners hébergés, ou environnement isolé propre à l'agent, seulement. Pas d'Electron/GUI, ni nouvelles fonctionnalités, ni fusion globale de la branche alpha, ni nouveau transport distant vers vrais projets, ni reprise automatique de la roadmap fonctionnelle, ni relâchement des délais, sandbox, quotas, permissions, politiques réseau ou gardes de sécurité. Pas de tag/release V1. Le Rust core ne dépend pas obligatoirement de Node, Python, Docker, Electron ou d'un modèle hébergé.

Invariable de conception : **l'IA demande, la politique autorise, l'isolation contient**. Respecter TransportGuard, ActionEnvelope, les effets typés, scopes, sessions/leases monotones, capacités expirables/révocables, préconditions exactes, identité propriétaire et absence de privilèges ambiants. Un Job Object Windows gère des processus et des ressources : ce n'est pas un sandbox complet.

## 2. État GitHub exact vérifié avant cette passation

| Source | Observation du 22-10-2026 |
| --- | --- |
| Dépôt unique | [ELITEAYOTO/Optic-AI-Bridge-](https://github.com/ELITEAYOTO/Optic-AI-Bridge-) |
| main avant éventuelle PR documentaire | `82ad1234f184591e0f2c80c86296a65a883ecc20` |
| Protection de main | `protected=false` ; changement nécessite validation du propriétaire |
| PR ouvertes au contrôle | 0 |
| Branche de recherche #317 | `bootstrap/issue-317-windows-rust-owner` |
| Tête de recherche | `33c95e00aa384774ccac6ccbab24b8f54fa5ab11` |
| CI exacte sur cette tête | [#39748924515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748924515), réussie en première tentative sur ce SHA |
| CI sur main après #322 | [#39724872707](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39724872707) ; PowerShell 5.1 [#39724872591](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39724872591), succès confirmés au checkpoint antérieur |
| Référence globale V1 | [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) — ouverte, NO-GO |
| Priorité actuelle | [issue #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) — ouverte, défaut de production non corrigé |

**Important :** la PR de passation/documentation ouverte après ce checkpoint peut déplacer main. Vérifier son état, ses checks, la fusion éventuelle et la CI indépendante sur le nouveau SHA avant de conclure quoi que ce soit. Ne pas traiter les anciens en-têtes historiques de STATUS, ROADMAP, GITHUB_HANDOFF, PROJECT_CONTINUITY_PROMPT comme des états live.

Issues ouvertes confirmées au contrôle : #323, #319, #317, #185, #182, #177, #176, #175, #174, #173, #172, #171. Une issue ouverte n'est pas nécessairement un obstacle à la **V1 locale strictement excluante** : la portée doit être décidée et les exclusions effectivement garanties dans le runtime.

## 3. Ce qui est intégré à main et ce qui ne l'est PAS

**Intégré / corrections antérieures :**
- PR #320 — vérification du commit exact intégré à main et des preuves CI avant publication d'un tag/prérelease ; n'autorise pas une release.
- PR #321 — correction BOM UTF-8 au début du JSON MCP sous Windows PowerShell 5.1 ; cas Unicode, octets et vrai Bridge vérifiés. **Ne résout pas** les intermittences de première réponse #319.
- PR #322 — intégrée avant le checkpoint main `82ad1234` ; consulter son diff/CI pour la description exacte, ne pas inférer d'autres corrections.
- Historique PR #308–#314 — classification de chemins sensibles, analyse de statut/diff Git, protection de hardlinks, vérification des chemins/portées. Les trous de lecture/provenance #176/#182 ne sont PAS clos.

**Recherche exclusivement sur la branche bootstrap, jamais fusionnée à main :**
- ADR-0035 **PROPOSED**, pas acceptée : `docs/decisions/ADR-0035-WINDOWS-GIT-OWNER-STAGING.md`.
- 13 scénarios natifs Windows dédiés sur `command_owner_probe.rs`, accompagnés du superviseur Python d'essai : CI native [#39735459098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39735459098), `complete=true`, `cases=13`, `outer_job_empty_confirmed=true` pour leur périmètre.
- Preuves ciblées : enfant/descendant retenu, contrôle Job/sorties, writer stdin sous contre-pression, erreur d'affectation Win32 provoquée, ressource et capacité conservées sans preuve, propriétaires Session/HostStartup distincts et ordonnancement arrêt-avant-drain dans des fixtures.
- Régressions **du vrai SessionRegistry** sous Windows : signal de révocation avant drainage, demandeur qui abandonne sa réponse, interdiction de nouvelle admission, course watcher/admission/revoke avant reprise et non-interférence d'une autre session. CI exactes [#39737228296](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39737228296) et [#39738348645](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39738348645).
- Test d'intégration réel SessionRegistry + **LimitedJobObject Windows** sur enfant synthétique, Job terminé + stdout/stderr EOF avant libération du permis, abandon de réponse du demandeur. CI [#39742348738](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39742348738) ; test nommé `session_git_owner_integration::real_session_revoke_stops_a_contained_windows_child_after_caller_abandons_reply`. Ce n'est **pas un appel Git réel**, ni la preuve du service complet.
- Dernière tranche : module Rust privé `#[cfg(windows)]` `crates/optic-bridge-runtime/src/git/owner_registry.rs` dans la branche recherche, compilé **mais non câblé au runner Git**. Deux nouveaux tests Windows ont passé [#39748924515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748924515). Contrat de recherche : `docs/research/ISSUE317_OWNER_REGISTRY_SLICE_2026-10-22.md`. Checkpoint détaillé [commentaire #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6336561736).

**Ne jamais écrire** « 13 tests de production prouvent #317 corrigé ». Les 13 sont des fixtures de recherche, les preuves supplémentaires sont des tests de registre/intégration. Aucune migration GitRead, aucune correction effective de HardenedCommandRunner, aucune décision release n'en découle.

## 4. Défaut #317 : source, liveness et modèle cible

Source réelle sur main : `crates/optic-bridge-runtime/src/hardened_command.rs`, `HardenedCommandRunner::run_inner`. Actuellement `std::process::Command`, `child.try_wait()` ou `kill()/wait()` contrôlés par timeout pour l'enfant DIRECT, mais les threads `stdout_reader.join()`, `stderr_reader.join()`, éventuellement `input_writer.join()` peuvent attendre **sans borne** après la sortie de ce processus. Un descendant ayant hérité d'un handle de pipe peut empêcher EOF et dépasser le délai annoncé. Un timeout autour de `join` qui abandonnerait les threads n'est PAS une correction sûre.

L'architecture souhaitée doit fournir :
1. **Propriétaire de service persistant** indépendant du caller/future MCP, créé avant tout effet natif ; identifiant opaque minté par le serveur, séparation d'autorité `Session`/véritable `HostStartup`, jamais une API pour tuer n'importe quel PID.
2. Admission de session/lease/capacité vérifiée, réservation globale et par session bornée **avant spawn**, validation du binaire/cwd/env/scopes et état de révocation jusqu'au point d'effet. Aucun permis côté client n'autorise le processus.
3. Sur Windows : Job Object borné (CPU, mémoire, nombre de processus), création suspendue, assignment avant resume, zéro breakaway ; handles, enfant, stdin, stdout/stderr restent attachés à l'owner.
4. Stop fiable hors des capacités ordinaires de transport et avant attente du drainage des admissions ; ne pas bloquer ce chemin par un verrou global détenu pendant une lecture native potentiellement bloquante.
5. `TerminationUncertain` conservant **l'owner et toutes les réservations** tant que sortie du processus, Job vide et fin de TOUTES les opérations stdin/stdout/stderr ne sont pas prouvées. Pas de detach silencieux ni réemploi prématuré du slot.
6. Comptabiliser les propriétaires Git dans `SessionLifecycleManager::revoke`, `seal_quiescent` et `try_reap` (session irrécupérable/non scellée si owner incertain). Le `SessionRegistry::revoke` existant signale déjà `revocation_signal` AVANT le drainage ; `watch_active` fournit un flag `Arc<AtomicBool>` authentifié sur Windows ; c'est un signal, PAS une preuve d'arrêt natif.
7. Traiter distinctement les erreurs Win32 de création/assignation/reprise/attente/terminaison/I-O, la destruction du service, les races fermeture/reprise, la rétention des handles et la récupération après crash.
8. Migrer **un chemin GitRead lecture seule** seulement après intégration et preuves. GitIntegrate/update-ref CAS est plus délicat : le ref peut avoir été commité malgré un timeout. **`GitRefOutcomeUncertain` est distinct de `TerminationUncertain`**, exige journal d'effet et réconciliation avant tout retry.

**Attention sur le dernier module propriétaire expérimental :** son generic `GitOwnerRegistry<T>` utilise un payload inert dans les tests (chaîne), des plafonds de test non issus de l'autorisation réelle, `#[allow(dead_code)]` et un callback `reap_if_proven` exécuté SOUS mutex global. Un callback native bloquant pourrait verrouiller la commande stop et produire un deadlock. Première revue à faire : protocole owner/observer par opération qui ne monopolise pas le registre global, sans détacher la responsabilité de cleanup. `confirm_no_spawn` n'est sûr qu'après preuve qu'aucun spawn ne s'est produit ; un simple indicateur logique ne suffit pas en cas d'erreur Win32 incertaine. `reserve_host_startup` devra être bornée par une authentification interne réelle. Cette interface est un **candidat**, pas un contrat accepté.

Fichiers pour l'audit : `crates/optic-bridge-runtime/src/hardened_command.rs`, `src/git/read.rs`, `src/git/integrate.rs`, `src/session/registry.rs`, `src/session/lifecycle.rs`, `src/process.rs`, `crates/optic-bridge-windows/src/job.rs` et `output_pipe.rs`. Rechercher aussi les appels HostStartup, la gestion de stdin et de l'exécutable Git et les invariants de sécurité.

## 5. Autres portes de sortie V1

- **#176 / #182** — cohérence des autorisations de lecture sensible FileRead/fs_list/GitRead : secrets dans fichiers à noms ordinaires, hardlinks/autres alias, contenu Git index/historique, étiquettes branch.head/branch.upstream, provenance et changements concurrentiels. Les protections actuelles sont partielles ; aucune fuite concrète supplémentaire n'est automatiquement démontrée. S'il existe des chemins non prouvés, les refuser à la V1 jusqu'à contrat vérifié. Distant vers vrai projet NON autorisé.
- **#319** — première réponse MCP ~30s parfois intermittente. Le BOM #321 est indépendant ; collecter des preuves au bon stade et corriger une cause avérée, pas augmenter arbitrairement un délai.
- **#185** — intermittence Node/AppContainer native Windows ~10s. Ne pas affaiblir le test négatif réseau/sandbox ; diagnostiquer une occurrence naturelle ou exclure proprement un profil non fiable.
- **#323** — tableau transversal de gel V1 : provenance de builds, intégrité du tag et exact-main CI, pouvoirs release minimaux, stockage/récupération, leases/permissions, tests Windows natifs et Windows 10/11 RÉELS, PowerShell 5.1 et pwsh, installation/retrait/mise à jour, scénarios de charge/fautes, docs de retour arrière. Checkboxes non cochées sans preuve sur main intégré.
- **#171–#175, #177** — travaux d'auth/session/approbation et Streamable HTTP distants **hors périmètre proposé V1 locale** tant qu'ils sont clairement exclus au runtime. Electron/GUI et confinement Linux fonctionnel aussi post-V1 ; **maintenir la CI Ubuntu partagée**.
- Protection GitHub `main.protected=false` au constat ; pas de changement sans aval explicite. La publication bénéficie déjà d'une garde de provenance #320, mais les scopes de `contents:write` en phase build restent à examiner.

## 6. Validation et discipline de PR

1. Relire dans cet ordre : `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`, `ROADMAP.md`, `docs/engineering/GITHUB_HANDOFF.md`, `PROJECT_CONTINUITY_PROMPT.md`, `CONTRIBUTING.md`, `docs/engineering/REPOSITORY_WORKFLOW.md`, `docs/engineering/RELIABILITY_DEBT.md`, `docs/security/SECURITY_INVARIANTS.md`, puis ADR-0034, ADR-0035/recherche et sources de #317.
2. **Re-vérifier live** main SHA, branches, commits, PR ouvertes, reviews, issues, runs/policy, SHA exact des checks ; ne pas deviner l'état à partir d'un prompt ou de ce document.
3. Préférer **un incrément cohérent et prouvé**, une seule PR active au plus, depuis le vrai main quand une tranche est prête. Ne pas fusionner en bloc la branche `bootstrap/issue-317-windows-rust-owner`. Conserver l'historique des premières CI rouges ; préciser pourquoi elles ont échoué. Ne pas faire de retry du même SHA pour faire disparaître un échec de fond.
4. Tests synthétiques, bornés, sans réseau ni secrets, exclusivement runners GitHub hébergés (Windows réellement natif pour Win32) et environnement propre de l'agent. Chaque expérience risquant un blocage a un superviseur externe indépendant, un Job englobant, des deadlines courtes et une preuve de nettoyage, même lorsque le test échoue. La CI Ubuntu et dépendances doit rester verte.
5. PR : code ciblé + tests négatifs/régression + contrat/ADR pertinent ; revue des invariants, état exact de source et des CI. Squash/revue selon la politique du repo si prêt, puis **CI indépendante après fusion** sur le SHA exact de main. Ne déclarer un problème résolu qu'une fois ce dernier contrôle effectué.
6. Mettre à jour STATUS / ROADMAP / CHANGELOG / GITHUB_HANDOFF de manière sélective ; les anciens checkpoints sont historiques. Toute modification de production ou de portée sécurité requiert la doc d'architecture et une preuve. On peut choisir de ne créer aucune PR si l'hypothèse n'est pas démontrable.

## 7. Prochaine mission proposée au successeur (ordre concret)

**P0 — poursuivre #317 sans commencer de nouvelles fonctionnalités.**
- Auditer précisément `GitOwnerRegistry<T>` de la branche recherche : mutex/callback bloquant, admission/revoke atomique, vraie propriété de l'owner et des ressources, insuffisances `confirm_no_spawn`, `HostStartup`, plafonds réels, possibilités de perte d'owner lors d'un drop.
- Proposer et prouver un protocole arrêt/observation **non bloquant pour le contrôle**, avec test déterministe de contention et fail-closed en cas d'erreur. Ne pas simuler la terminalité du Job en production ; utiliser ensuite un vrai payload Win32.
- Introduire une tranche suffisamment limitée reliant ce registre au vrai `LimitedJobObject` et aux handles pipes/stdin, avec tests supervisés Windows. La preuve doit survivre à un abandon de requête et retenir le slot quand la fin n'est pas observée.
- Relier ensuite le signal de révocation réel et le comptage des owners au cycle `revoke -> stop -> drain -> seal/reap` ; ne pas introduire de deadlock ni de faux statut quiescent. Garder l'identité HostStartup indépendante.
- Seulement après ces preuves : migrer **un** chemin GitRead en lecture seule. Mesurer le comportement avec descendants qui conservent les pipes, limites sortie et stdin, course de révocation. L'écriture Git/CAS est un chantier distinct.
- Mettre à jour #317 et #323 avec exact SHAs/runs, ce qui est réellement prouvé et le prochain défaut. Ne pas cocher la V1 avant la correction sur main.

**Point d'arrêt pour le nouveau chat :** commencer par vérification GitHub live et analyse critique du ledger sous mutex avant tout nouveau commit. Ce document remplace les vieilles instructions « huit scénarios », mais pas les règles AGENTS/AI_HANDOFF ni l'ADR proposée.

## 8. Sources de preuves rapides

- [CI recherche ledger #39748924515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748924515), exact `33c95e00`, Windows/Ubuntu/dépendances réussis.
- [Compte rendu #317 ledger](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6336561736).
- [CI intégration vraie session + vrai Job #39742348738](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39742348738), exact `8366a876`.
- [13 cas natifs #39735459098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39735459098).
- [Contrat de gel V1](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/3017164be4a39a5937934768a11417522a7a7dab/docs/research/WINDOWS_V1_FREEZE_READINESS_2026-10-22.md), recherche uniquement.
- [ADR-0035](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/33c95e00aa384774ccac6ccbab24b8f54fa5ab11/docs/decisions/ADR-0035-WINDOWS-GIT-OWNER-STAGING.md), statut PROPOSED.
- [Gates V1 #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323).
- First-red CI `2b380a06`: [#39748694120](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748694120), échec rustfmt sur Ubuntu puis remplacement par un SHA corrigé, ne pas l'appeler vert.

**Consigne finale :** ne pas confondre test vert, interface non câblée, correction sur main et qualité V1. Le but est de renforcer l'existant jusqu'à des critères de sortie réels, jamais d'accélérer artificiellement une publication.
