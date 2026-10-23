# Optic AI Bridge — Passation canonique de stabilisation Windows V1 (23 octobre 2026)

> **État documentaire constaté avant cette PR.** Vérifier GitHub en direct au prochain chat : les SHA, PR et CI datés ici ne sont PAS des ordres de checkout/reset/merge. Les contrôles post-merge éventuels de cette PR devront être consignés dans le compte-rendu GitHub.

## 1. Objectif et cadre opérateur

Le propriétaire souhaite avancer progressivement vers **une V1 locale Windows 10/11 du socle Rust/MCP existant**. C'est une **phase de stabilisation et de sûreté**, pas de nouvelle fonctionnalité ni un gel V1 annoncé. Le résultat recherché est l'absence de défaut bloquant connu atteignable, avec preuves reproductibles et risques résiduels explicitement acceptés, pas une promesse impossible de « zéro bug ».

Doctrine : **l'IA demande ; la politique autorise ; l'isolation contient**. Préserver `TransportGuard`, `ActionEnvelope` et `Effect` typés, permissions et leases révocables, scopes de sessions, plafonds communs, durée de vie monotone, isolation Windows, autorité Git et refus déterministe. Ne pas remettre à la disposition d'un assistant des processus ou PID arbitraires ; Windows Job Object n'est pas à lui seul un sandbox.

L'utilisateur délègue les décisions de recherche/correction courantes, mais **les changements de contrat de sécurité (#182), de protection GitHub, les tests sur le PC utilisateur, toute nouvelle intégration autorisée ou la release V1 requièrent un consentement spécifique**. Codex a réalisé une campagne locale Windows 10 précédemment autorisée, sur des worktrees E: séparés ; ne pas interpréter cette autorisation historique comme un accès permanent du nouveau chat ou de tous les outils. Par défaut : GitHub et ses runners hébergés, ou environnement propre isolé ; pas de self-hosted runner, aucune installation ni intervention sur le PC par un autre agent sans une nouvelle instruction explicite. Ne pas tuer les processus préexistants de l'utilisateur, ne pas toucher aux anciennes archives ni aux quarantaines.

## 2. État GitHub vérifié avant passation documentaire

| Élément | Preuve / constat |
| --- | --- |
| Dépôt unique | [ELITEAYOTO/Optic-AI-Bridge-](https://github.com/ELITEAYOTO/Optic-AI-Bridge-) |
| `main` avant la PR de passation | `3fe3de0c258b2f8fc4328d6c405c8bf05cb22e3d`; `protected=false` |
| PR ouverte | **[#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327)**, source `codex/317-prepared-read-owner` SHA `ef3208d787baa0c6433abfa0b4aba687d8bc6bb7`, cible la **branche research bootstrap**, pas main |
| PR #327 / CI source | [#39845778580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39845778580) **échec Windows #185**, première tentative conservée ; workflows natifs ciblés verts. Ne pas interpréter la CI d'une autre branche comme sa réparation |
| Recherche principale #317 | `codex/317-service-retention`, head `1c7dac973c8f41f27b303811041b54031f7272ae` |
| CI exacte `1c7dac9` | [générale #39888522519](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522519) et [native #39888522200](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522200) : toutes deux `completed/success`, attempt 1 |
| Dernière suite workspace Windows | **608 passed, 0 failed, 25 ignored**. L'ancien rapport `c047a05` comptait **24 ignored** ; ne pas copier ce nombre au dernier SHA |
| Dernière suite workspace Ubuntu | 463 passed, 0 failed, 0 ignored |
| Branche recherche #182 | `research/issue-182-git-provenance-contract` ; preuve exacte [#39865183807](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39865183807) sur `b4868b39`, docs recherche mises à jour à `db32111d` |
| Issue V1 | [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) ouverte, **NO-GO** |
| Défaut Git natif | [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) ouvert : `HardenedCommandRunner` production sur main toujours risqué |
| Autres issues critiques | #176 / #182 (lectures sensibles/Git), #185 (AppContainer/Node intermittent), #319 (MCP première réponse) : distinctes, ouvertes au dernier contrôle |

**Cette passation documentaire ne résout aucune de ces issues.** N'ajouter ni tag V1 ni release ni changement de protection branch sans approbation.

## 3. Archives et preuves Codex examinées

Le propriétaire a fourni `optic-bridge-317-admission-shutdown-2026-10-23.zip` (ZIP local de la conversation, pas nécessairement présent dans GitHub). Le dossier contient **28 charges utiles + MANIFEST.json**, toutes les tailles et empreintes SHA-256 vérifiées, archive ZIP saine. Les quatre sources principales incluses correspondent exactement aux blobs publiés sur le commit `1c7dac9`.

Fichiers de l'archive : `REPORT.md`, `HANDOFF.md`, `NEXT_STEPS.md`, `VALIDATIONS.json`, `HOSTED_EVIDENCE.json`, `CLOSURE.json`, `audit.patch`, reproducteurs et traces de la fenêtre crash avant/après assignation ; copies de `read_service.rs`, `read_service_tests.rs`, `prepared_read_command.rs` et [contrat détaillé de recherche](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/1c7dac973c8f41f27b303811041b54031f7272ae/docs/research/ISSUE317_SERVICE_ADMISSION_SHUTDOWN_2026-10-23.md).

Deux nouveaux commits Codex, sans PR ni fusion :
- [`f91dc3542b647bdb0182437dffc4b6b16ae14d10`](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/commit/f91dc3542b647bdb0182437dffc4b6b16ae14d10) : admissions et destruction du root auditées avec tests sans permis supplémentaire artificiel ; [CI #39888064014](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888064014) + [native #39888064013](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888064013), verts sur SHA exact.
- [`1c7dac973c8f41f27b303811041b54031f7272ae`](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/commit/1c7dac973c8f41f27b303811041b54031f7272ae) : vrai test crash hôte avant/ après affectation Job, neutralisation des creation flags externes, docs/CI ; [CI #39888522519](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522519) et [native #39888522200](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522200) verts.

Six groupes natifs distincts sous superviseur extérieur Windows :
1. Service réel : abandon du reply, panique d'observation synthétique, quota du JobStore partagé, refus intersession, incertitude qui retient l'owner et bloque seal/reap, révocation sans permis factice, attente de Drop puis reprise et join.
2. Révocation pendant création suspendue, contrôle des flags externes `u32::MAX` et de la non-reprise après stop avant contrôle.
3. Matrice de huit états : normal, nonzero, output, both-output, stdout, stderr, timeout, stdin-null.
4. Deux scénarios préstart : stop et deadline, sans enfant lancé.
5. Arrêt du *seul* hôte synthétique juste AVANT `AssignProcessToJobObject` : 2 processus dans le Job extérieur → 1 enfant suspendu survit, puis cleanup extérieur prouvé.
6. Contrôle hôte arrêté APRÈS l'assignation : 2 → 0, puis Job extérieur vide.

**Interprétation cruciale :** un workflow réussi pour le cas 5 signifie que **le défaut de sûreté a bien été reproduit et que le superviseur a nettoyé**, pas que l'enfant ne peut plus survivre au crash. Aucune panne de l'hôte réel ni fuite de vrai secret n'a été reproduite. Les études sont des preuves expérimentales, pas des garanties de production.

## 4. #317 — état précis du service et nouvelles limites

Sources de référence :
- `crates/optic-bridge-runtime/src/process/read_service.rs`
- `crates/optic-bridge-runtime/src/process/read_service_tests.rs`
- `crates/optic-bridge-windows/src/prepared_read_command.rs`
- `crates/optic-bridge-runtime/src/session/registry.rs`
- `crates/optic-bridge-runtime/src/session/lifecycle.rs`
- `crates/optic-bridge-runtime/src/process.rs`
- `crates/optic-bridge-runtime/src/hardened_command.rs`.

**Résultats favorables (recherche seulement) :** `ReadServiceRoot` possède des `JoinHandle` durables indépendants du caller. Un `Driver` conserve Job/Child/pipes et le record de quotas partagé avec `ProcessManager`. Un arrêt explicite atteint le watchdog via atomiques hors verrou du registre global. Une observation fausse/erreur/panique synthétique conserve l'owner, les réservations et le statut `TerminationUncertain`. `SessionLifecycleManager` consulte le JobStore commun ; il ne scelle ni ne reap une session dont le record est actif/incertain. Les tests natifs Windows sont positifs sur ces contrats.

**Trois grandes limites démontrées :**

### A. Admission de session ≠ opération native complète

`ReadServiceRoot::submit` acquiert un `SessionAdmissionPermit` pour la durée de **submit/réservation seulement**. Le worker ne possède pas ce permis sur toute sa durée. Nouveau test contrôlé : submit retourne, revoke retourne aussi sans bloquer, le record existe encore ; seal/reap refusent la fin de session ; worker libéré ensuite voit le stop et ne crée pas l'enfant. Ce résultat caractérise le contrat actuel, ne garantit pas qu'une autorisation Git synchrone produit n'aura pas été affaiblie.

Deux options à arbitrer (aucune acceptée) : **permis réel service-owned couvrant jusqu'à terminaison et finalisation**, stop pré-drain + état `RevokePending` non bloquant si incertitude ; **ou** redéfinir officiellement le contrat revoke comme fermeture d'admission + demande d'arrêt, puis exiger un seal avant toute réutilisation/nettoyage. Examiner tous les consommateurs avant de choisir. Transférer simplement le permis au worker peut déplacer le hang de Drop vers `revoke()`. Un watcher atomique avant reprise n'est pas une linéarisation avec la fermeture des admissions.

### B. `Drop` peut bloquer sans borne

`try_shutdown` demande stop, puis retourne count/pending ; le root doit survivre. `Drop` rejoint les threads : un échec persistant de l'observation OS peut empêcher cette fin, tout en retenant les ressources. Le test démontre une attente pendant une fenêtre injectée puis une sortie lorsque l'observation redevient disponible ; il ne prouve pas une vraie panne permanente du noyau. Le contrat doit être décidé : superviseur applicatif conservant réellement le root et refusant de nouveau travail, process owner externe indépendant, ou arrêt terminal reconnu comme incertain. Ne jamais « corriger » avec `detach`, `mem::forget`, suppression de la réservation ou simple timeout autour de join.

### C. Fenêtre de crash avant assignation au Job

La préparation native fait spawn **suspendu**, puis `AssignProcessToJobObject`, puis recheck et `ResumeThread`. Un arrêt brutal de l'hôte **après spawn mais avant assignation** peut laisser survivre un enfant suspendu. La supervision extérieure a démontré exactement cela, a ensuite nettoyé entièrement, et a fourni un contrôle après assignation. Cela contredit la garantie trop forte « toute création suspendue est déjà crash-safe ». Examiner association au Job à la création (compatibilité Win10/11 et versions ciblées) ou propriétaire du lancement dans un service/processus indépendant. Ne pas tenir un verrou d'autorité autour d'un appel Win32 bloquant sans analyse de risque.

**Autres gates #317 :** race entre dernier check de stop et `ResumeThread`, plafonds/leases et autorité réels, horloge monotone commune, accès à un exécutable Git épinglé et cwd/repo canonique, construction interne d'une Command depuis une action typée, environnement sans secrets/config externes inattendus, garde post-revoke de résultat. Le test `creation_flags(u32::MAX)` montre l'écrasement des flags dans le scénario contrôlé ; ce n'est PAS une preuve que toute option opaque `Command` d'un demandeur serait sûre. Garder tout `AuthorizedRead` sans constructeur de production jusqu'à correction/acceptation.

**Production inchangée :** `HardenedCommandRunner::run_inner` conserve des joins stdout/stderr/stdin potentiellement non bornés après fin du direct-child, notamment lorsqu'un descendant retient un pipe. Aucun chemin GitRead réel n'utilise `ReadServiceRoot`. Ne pas clore #317 ni basculer GitRead en production.

## 5. #182 et autres gates V1

La recherche `research/issue-182-git-provenance-contract` possède [ADR-0036 PROPOSÉE](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/db32111d349cfcccab1dbe5cbbb161ea4b51371c/docs/decisions/ADR-0036-GIT-DIFF-PROVENANCE-V1-PROPOSAL.md) et [matrice de validation](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/db32111d349cfcccab1dbe5cbbb161ea4b51371c/docs/research/ISSUE182_GIT_PATCH_PROVENANCE_ACCEPTANCE_2026-10-22.md). [CI Windows #39865183807](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39865183807) **reproduit** la révélation d'un faux secret dans GitDiff après disparition d'un alias hardlink et remplacement par un fichier ordinaire : le blob indexé subsiste. `fs_read` peut autoriser le nouveau fichier tandis que staged/unstaged GitDiff contient encore l'ancien marqueur. **Vert = comportement problématique reproductible, pas correctif.** Le contrat de la future V1 restreinte pourrait être fail-closed pour les patches sans preuve suffisante de source/autorité ; **changement produit/sécurité en attente d'approbation**. Ne jamais ajouter implicitement ReadSensitive ou prétendre que GitRead = FileRead.

- #185 : intermittence AppContainer/Node après reprise du processus, ancien rouge #327 conservé. Pas de retry-to-green, hausse de délai ou sandbox affaibli.
- #319 : premier échange MCP parfois ~30 s ; distinct du BOM PowerShell 5.1 corrigé #321.
- #176 : limites de lecture sensible, aliases et provenance ; coordonner avec #182.
- #323 : profils Windows 10/11 réels, packaging, rollback, recovery, limites, source exacte CI après merge, provenance release, approbation explicite du gel.
- #171–175 / #177 : accès distant et autonomie hors V1 locale proposée ; exclusion doit être effective dans le runtime. Electron/GUI différés ; CI Ubuntu toujours obligatoire.

## 6. Suite technique préférée et méthode

**Prochaine mission : auditer/décider l'autorité d'une opération et le contrat de fermeture AVANT toute migration MCP.**
1. Formaliser l'état `RevokePending` et `ShutdownPending` et le détenteur réel de l'admission/owner, avec preuve qu'un échec persistant ne libère ni handles ni quotas et n'empêche pas sans stratégie la fermeture de l'hôte. Comparer deux designs ; soumettre la décision sécurité/produit pertinente.
2. Explorer la frontière crash-before-assign et la course final-check/ResumeThread par un plan Windows 10/11, un processus externe/test supervisé et aucun OS-process kill arbitraire. Si correctif tentable, isoler un patch strictement prouvé ; sinon documenter le NO-GO.
3. Après choix de contrat, authorizer GitRead validé (session/task/lease, clock monotone, deadline absolue, pin exécutables/repo/cwd, env contrôlé, token non constructible par client). Puis **une seule lecture Git read-only** avec preuve native du temps, pipes, arrêt, résultats et refus post-revoke. Ne pas mélanger #182 ni mutation Git/CAS dans la même PR.
4. Une branche technique à la fois : une PR ciblée vers sa bonne base, exact SHA CI native Windows, Ubuntu et dependency policy, review, squash, indépendance post-merge sur SHA intégré. **Ne pas fusionner en bloc les branches `bootstrap`, `codex` ou `research`** ; conserver #327 ouverte tant qu'elle a le rouge #185. Une PR documentaire de passation, si nécessaire, est une exception de portée *docs only*, explicitée et clôturée avant nouveau chantier.
5. Premier run rouge toujours conservé, aucun retry d'un même SHA juste pour obtenir vert. Pour tests pouvant bloquer : enfant et hôte synthétiques, Job externe créé avant effet, deadline extérieure, process tree empty confirmé, traces assainies, aucune action sur projets réels ou processus de tiers.
6. Mettre à jour `STATUS.md`, `ROADMAP.md`, `CHANGELOG.md`, `docs/engineering/GITHUB_HANDOFF.md`, ADR et #317/#323 avec SHA exact et limitation à chaque vraie étape. Séparer résultats prouvés, inconnus, causes de CI rouges et risques.
7. **Codex** : quota rapporté 38 % à la fin de la campagne (variable ; lui faire mesurer). Le réserver aux véritables tests Windows 10 et aux expériences natives impossibles à prouver sur GitHub hébergé ; ChatGPT peut piloter revues, GitHub, sécurité #182 et documentation. Codex ne travaille pas en arrière-plan sans nouvelle requête utilisateur. Échanger archives ZIP assainies + SHA + manifeste et éviter changements simultanés des mêmes branches. Permissions anciennes de tests locaux propres à la session de Codex, pas transmissibles à un autre agent.

## 7. Lecture prioritaire du successeur

1. Lire entièrement `AGENTS.md`, `AI_HANDOFF.md`, `STATUS.md`, `ROADMAP.md`, `docs/engineering/GITHUB_HANDOFF.md`, `docs/engineering/HANDOFF_WINDOWS_V1_2026-10-23.md`, `docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-23.md`, `PROJECT_CONTINUITY_PROMPT.md`, `CONTRIBUTING.md`, `docs/engineering/REPOSITORY_WORKFLOW.md`, `docs/engineering/RELIABILITY_DEBT.md`, `docs/security/SECURITY_INVARIANTS.md`, puis ADR-0035/0036 et documents issus des branches recherche.
2. Re-vérifier GitHub en direct : main, branches, #327, reviews, CI source exacte, issues #317/#323/#182/#176/#319/#185, protections et droits, **y compris cette PR docs potentiellement fusionnée**.
3. Lire véritable code de `ReadServiceRoot` et `PreparedReadCommand` au commit `1c7dac9` ; `main` ne l'a pas.
4. Démarrer par un audit de décision **revoke / admission / crash / shutdown**, pas par la migration prématurée GitRead ou un nouveau mock. Ne pas fermer une issue ni lever un gate sans une preuve sur `main`.
