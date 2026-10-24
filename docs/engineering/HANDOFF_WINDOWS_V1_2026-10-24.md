# Optic AI Bridge — Passation canonique Windows V1 (24 octobre 2026)

> **Point de reprise vérifié sur GitHub le 24 octobre 2026.** Les SHA et les chiffres ci-dessous sont des checkpoints datés, jamais des ordres de réinitialisation. Reconsulter GitHub à chaque reprise et après fusion de cette PR documentaire. Les validations de recherche ne valent pas intégration de production.

## 1. Objectif, périmètre et autorité

Dépôt officiel : https://github.com/ELITEAYOTO/Optic-AI-Bridge-

Objectif : **stabiliser le socle Rust/MCP actuel pour une future V1 locale Windows 10/11**, avec garanties de sûreté, tests Windows natifs et Linux, preuves CI sur le SHA exact, architecture cohérente et dette technique explicite. La V1 **n'est pas autorisée**. Les travaux de fonctionnalités, Electron/UI, nouvelles capacités de mutation, exposition distante de vrais projets, extension Linux, packages/releases restent gelés.

Doctrine : **l'IA demande ; la politique autorise ; l'isolation contient**. Respecter TransportGuard, ActionEnvelope + Effect typés, SessionHandle, tâche/lease, scopes, budgets, deadlines monotones, revue de sécurité, provenance Git, protections des ressources et erreurs fail-closed. Un Job Object ne constitue pas un sandbox. Ne jamais exposer l'arrêt par PID arbitraire.

L'agent peut organiser les investigations et correctifs limités sur GitHub. **Ne pas prendre l'autorisation historique Codex comme une autorisation permanente de manipuler l'ordinateur de l'opérateur.** Les opérations sur son PC, permissions, modifications sensibles de politique, activation de fonctionnalités, protection GitHub, release/tag V1 nécessitent son approbation spécifique. Runners GitHub hébergés par défaut ; pas de self-hosted runners. Préserver les archives, worktrees et travaux existants.

## 2. État exact constaté avant la nouvelle PR documentaire

| Objet | État vérifié |
| --- | --- |
| Branche de production main | **100c43b53c795be5678c8b4f2c0af4353ca5b196** |
| CI de main sur ce SHA | [#40027279676](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40027279676), completed/success, attempt 1 |
| PR de production la plus récente sur #317 | [#336](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/336) fusionnée par squash à **100c43b5** |
| PR ouverte | **[#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327)** ; head **ef3208d787baa0c6433abfa0b4aba687d8bc6bb7** ; **recherche**, ne pas modifier/fusionner par défaut |
| Issues de sûreté | [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317), [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176), [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) **ouvertes** |
| Issues de fiabilité CI | [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) (AppContainer/Node), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) (première réponse MCP) **ouvertes** |
| Gate de V1 | [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **ouverte / NO-GO** |
| Configuration branche | main.protected = false constaté ; **ne pas modifier** sans accord |
| Dernière recherche #317 | research/317-native-observation-uncertain-2026-10-24 = **8202c9c633bf797be9ea69f9ce994b5ec5da9b86** |
| Preuve sur le SHA de recherche | [CI #40046563179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40046563179) : Windows 348 passed / 0 failed / 11 ignored ; Ubuntu 270 passed / 0 failed ; fmt et Clippy strict ; test incertitude explicite 1/1 ; attempt 1 |

**Attention :** la présente PR documentaire peut faire avancer main ; revalider head, merge, full CI et CI push/main du commit squash. Ne jamais réutiliser automatiquement les SHA ci-dessus pour écraser des changements ultérieurs.

## 3. Ce qui est intégré en production

Des primitives fines, pas un runner natif prêt pour la V1 :

- PR #331 / #334 : suivi de révocation lié à la génération d'admission, échéance initiale monotone capturée et read-only stop-watch (bien distinguer les détails par PR dans STATUS et la documentation du dépôt).
- [PR #336](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/336) : OwnedSessionAdmissionPermit non clonable, conservant Arc<SessionRegistry>, même compteur de vraies admissions que le permis emprunté, retour de revoke bloqué jusqu'au Drop, stop-watch sur la **génération et échéance d'origine**. Le permis seul n'autorise pas le lancement d'un processus et n'est pas une preuve de quiescence.
- PR #332 : mise à jour de sécurité Tokio (historique, relire Cargo.lock avant toute nouvelle modification).
- Le **vrai GitRead de production** dépend encore de HardenedCommandRunner ; la fenêtre ancienne création suspendue → AssignProcessToJobObject et les risques de descendants conservant des pipes restent sans correction complète dans ce runner.
- Le contrat courant SessionLifecycleManager::revoke publie la fermeture puis attend le drainage des admissions avant certaines étapes de cancellation. Un worker natif doit donc observer un stop hors verrou **avant** cette attente ; transfert naïf de permis => blocage potentiel.

CI verte de main prouve les changements intégrés au SHA, **pas** la réussite de la recherche ni l'absence de bugs.

## 4. Étapes de recherche validées, toutes HORS PRODUCTION

Ces branches forment un historique expérimental : **ne jamais les fusionner en bloc vers main**.

| Branche / SHA | Preuve | Démonstration strictement délimitée |
| --- | --- | --- |
| codex/317-native-registration-authority / **bcad195208bc5ecb0222fbfe5cd0595d07cb5938** | [générale #39954689300](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39954689300), 22 cas natifs | Autorité de session / propriété Windows dans un prototype Codex ; non câblé au Git réel ; Drop incertain toujours problématique |
| research/317-native-owner-generation-commit-2026-10-23 / **b6653f700af6028882e74232214001024822a7f8** | [#40025477442](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40025477442) | Réservation JobStore + engagement unique sous verrou de révocation, sans spawn |
| research/317-owned-native-retention-2026-10-23 / **555d32ff9a20e23a4ffb10efca4daf7dff2c589b** | [#40030518687](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40030518687) | Le worker indépendant retient permis possédé + vraie entrée JobStore ; **pas** de Job Object kernel créé par LimitedJobObject::new |
| research/317-kernel-job-create-time-2026-10-24 / **669b3ad19ea78cf6169d17e4bad4d198f3250893** | [#40032180130](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40032180130) | Véritable OwnedJob créé avant CreateProcessW ; PROC_THREAD_ATTRIBUTE_JOB_LIST ; host-kill supervisé de l'extérieur : Job externe 1 → 2 → 0 **avant sa propre fermeture** ; enfant suspendu jamais exécuté |
| research/317-owned-native-kernel-tree-2026-10-24 / **75024b0efea65c5595a3737838041aae197fecfa** | [#40042298153](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40042298153) | Composition permis possédé + JobStore + kernel Job à la création ; signal stop pendant l'enfant vivant, puis preuve arrêt root + Job vide avant drainage |
| research/317-native-observation-uncertain-2026-10-24 / **8202c9c633bf797be9ea69f9ce994b5ec5da9b86** | [#40046563179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40046563179) | Injecte **artificiellement** une erreur de lecture Job après arrêt du root ; conserve admission, Job, réservation et statut non terminal jusqu'à une lecture kernel positive de Job vide |

Dernier document détaillé : [ISSUE317_NATIVE_OBSERVATION_UNCERTAINTY_2026-10-24.md](../research/ISSUE317_NATIVE_OBSERVATION_UNCERTAINTY_2026-10-24.md) **sur la branche de recherche**. Les preuves et limites sont dans les commentaires datés de [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317), notamment [validation finale](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6377736151).

**Ne pas surinterpréter les tests :**
- Une entrée JobStore n'est **pas** un Job Object Win32 ; LimitedJobObject::new prépare une configuration, OwnedJob::create crée réellement le handle.
- Un succès de TerminateJobObject, la fin du root ou EOF des pipes ne prouvent **pas** l'absence de descendants.
- Une erreur Win32 **injectée et récupérable** n'est **pas** une défaillance native persistante observée ; le test précédent ne prouve pas un shutdown borné.
- Le statut non terminal Running pendant la panne injectée prouve l'absence de fausse publication, pas un contrat public définitif pour l'incertitude.
- La preuve extérieure de crash du kernel Job isolé ne prouve pas encore un **service-root** conservant conjointement lease, session, quota, JobStore et flux après crash.
- Toutes les suites sont limitées à des fixtures synthétiques ; aucune autorisation d'exposer de vrais projets ni de publier une V1.

## 5. Architecture de sécurité et emplacement des fichiers

- **crates/optic-bridge-runtime/src/session/registry.rs** : admissions/compteurs/génération et stop ; **session/lifecycle.rs** : fermeture et drainage ; **process.rs** : JobStore, réservations privées, result/finish ; **hardened_command.rs** : chemin de commande Git de production.
- **crates/optic-bridge-windows/src/job.rs** : Job Objects natifs existants ; les fixtures nouvelles se trouvent sur les branches de recherche dans **issue317_owned_kernel_fixture.rs**.
- **crates/optic-bridge-runtime/src/session/registry_owned_native_kernel_tests.rs** : scénarios de composition sur recherche ; **.github/workflows/issue317-owned-native-kernel-tree.yml** : CI spécialisée, renommée/conservée sur la branche d'incertitude.
- Le runtime impose **#![forbid(unsafe_code)]**. Ne pas l'assouplir : tout Win32 unsafe doit rester dans optic-bridge-windows, à travers une petite interface sûre, derrière une **feature de recherche non activée par défaut**. Pas de nouvelle API publique de lancement arbitraire.
- ADR-0035 / 0036 : **proposées, non adoptées** ; question de responsabilité native et de provenance Git. Vérifier les fichiers/revisions réels avant une décision produit.

## 6. Prochain chantier conseillé (dans l'ordre)

**A. Contrat d'arrêt non bloquant et propriétaire persistant.** Concevoir proprement « demander l'arrêt / publier la révocation » séparément de « observer l'arrêt complet / récupérer quotas ». Une observation OS durablement défaillante doit maintenir le Job et le propriétaire/lease en quarantaine sous responsabilité d'un service/guardian existant ; **ne pas simuler une terminaison ou libérer l'admission pour gagner du temps**. Un simple timeout suivi de Drop/forget/detach est dangereux. Étudier les cycles de vie de service, arrêt de host et reprise, la liveness de revoke, les verrous SessionRegistry ↔ JobStore, panics/abandon, propriété hors thread et crash.

**B. Preuves Windows natives supervisées.** Injection persistante des erreurs (pas uniquement un fail puis retry vert), descendant qui conserve stdout/stderr, EOF des deux pipes, fermeture stdin, arrêt lors de creation/resume, disparition de l'hôte, logs de terminaison incertaine, quota JobStore et nettoyage extérieur. Jobs/PIDs/handles fixes dans environnement synthétique, supervision indépendante, délais finis, aucun processus sur PC utilisateur.

**C. Petite intégration de production seulement après preuve.** Extraire des éléments sûrs depuis une branche de recherche vers une **nouvelle branche issue du main courant**. Une seule PR limitée à un contrat vérifiable, review, tests Windows+Ubuntu stricts sur HEAD exact, puis squash et CI indépendante sur main. Pas de fusion d'une stack entière, pas de raccourci CI.

**D. GitRead réel en dernier.** Une seule commande Git en lecture seule, avec sealed executable/repo, task-lease active, politique provenance #182, permissions, budgets, gestion des flux et publication fail-closed. Garder distinctes les préoccupations #176 (données sensibles), #185, #319 et récupération du host.

## 7. Méthode GitHub et Codex

1. Début de chaque tranche : inspecter live main, PRs, checks, issue owners, tree, AGENTS/STATUS/ROADMAP et le SHA de branche ; si le dépôt a avancé, réconcilier plutôt que remplacer.
2. Brancher depuis le bon ancêtre ; recherche isolée ou petite PR production, en conservant les journaux de première défaillance. Formater avant commit si un environnement isolé peut compiler ; sinon appliquer le diff exact de rustfmt des CI, ne pas inventer un run local.
3. Preuves sur **SHA exact** : rustfmt, strict Clippy, tests complets Windows et Ubuntu, scénarios natifs explicites ; si PR squash : contrôle **indépendant push/main**. Vérifier tests réellement exécutés, pas seulement job vert. AppContainer #185 et MCP #319 peuvent flaker : ne jamais augmenter timeout/skip tests ou retry-to-green pour déguiser.
4. GitHub peut auto-fermer des issues si une description de PR utilise des mots clés de fermeture, même négatifs : ne pas utiliser ces mots clés avec numéros d'issues. Garder #317/#323 ouverts et vérifier après fusion.
5. **Codex est disponible via l'opérateur** : si une investigation lourde Windows locale est objectivement utile, rédiger un prompt détaillé que l'opérateur peut copier à Codex. Celui-ci peut exploiter son PC **uniquement après instruction explicite de l'opérateur**, avec isolation et bornes. Décrire contexte/branche/SHA exacts, worktree séparé, tests limités à des fixtures, aucune suppression/installation/protection ni fermeture de processus non possédés, journaux + métriques + ZIP optionnel, CI/PR distinctes, critères GO/NO-GO. Privilégier d'abord les runners GitHub hébergés ; ne pas consommer son quota pour des recherches documentaires faisables par GitHub. Le nouvel assistant ne doit jamais prétendre avoir pris le contrôle de Codex ni de la machine sans outil/autorisations.
6. L'utilisateur délègue les décisions techniques quotidiennes et apprécie des livraisons réelles. Fournir des bilans avec SHAs, commits, diff, suites, limites et étapes restantes. Pas de fausses promesses de travail futur en arrière-plan.

## 8. Ressources à lire lors du transfert

Dans cet ordre : **AGENTS.md**, **AI_HANDOFF.md**, **STATUS.md** (nouvel encadré), **ROADMAP.md**, **docs/engineering/GITHUB_HANDOFF.md**, **ce handoff**, [NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md), **PROJECT_CONTINUITY_PROMPT.md**, **docs/engineering/REPOSITORY_WORKFLOW.md**, **docs/engineering/RELIABILITY_DEBT.md**, **docs/security/SECURITY_INVARIANTS.md**, issues #317/#323/#176/#182/#185/#319, les docs de recherche pointées au tableau et leurs workflows exact-SHA.

**Fin de passation :** V1 NO-GO ; pas de nouvelles features, pas de release, pas d'accès distant réel. Le chantier suivant reste une recherche critique sur la responsabilité durable et l'arrêt/incertitude natifs.
