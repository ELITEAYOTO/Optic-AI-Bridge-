# PROMPT DE REPRISE ULTRA-COMPLET — OPTIC AI BRIDGE — 23 OCTOBRE 2026

Tu prends la direction technique **autonome mais disciplinée** de la stabilisation d'Optic AI Bridge : https://github.com/ELITEAYOTO/Optic-AI-Bridge- . Nous poursuivons vers une **V1 locale Windows 10/11**, pas une reprise des nouvelles fonctionnalités. Le but est un Rust/MCP fiable, sûr, maintenable, avec preuves natives et CI exact-source, sans dette technique excessive. Je t'autorise à décider les étapes techniques courantes ; demande mon aval pour changer les contrats de sécurité, scopes, politique GitHub, tests/actions sur mon PC ou publier une V1.

## Reprise OBLIGATOIRE et ordre de lecture

Commence par vérifier **GitHub en direct** (SHA main, PR/reviews, branches, issues et CI), puis lis dans l'ordre :
`AGENTS.md`, `AI_HANDOFF.md`, `STATUS.md`, `ROADMAP.md`,
`docs/engineering/GITHUB_HANDOFF.md`,
`docs/engineering/HANDOFF_WINDOWS_V1_2026-10-23.md`,
`docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-23.md`,
`PROJECT_CONTINUITY_PROMPT.md`, `CONTRIBUTING.md`,
`docs/engineering/REPOSITORY_WORKFLOW.md`,
`docs/engineering/RELIABILITY_DEBT.md`,
`docs/security/SECURITY_INVARIANTS.md`, et les ADR-0035/0036 en recherche.
L'ancien handoff du 22 octobre est historique, pas un état actuel. **Aucun SHA historique n'est un ordre de reset.**

## État connu avant PR documentaire (re-vérifier)

- `main` = `3fe3de0c258b2f8fc4328d6c405c8bf05cb22e3d`, `protected=false`. Si le présent handoff a été fusionné sur main, SHA nouveau à constater et CI indépendante obligatoire.
- Seule PR technique ouverte : [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327), head `ef3208d787baa0c6433abfa0b4aba687d8bc6bb7`, vers `bootstrap/issue-317-windows-rust-owner` ; [CI générale #39845778580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39845778580) rouge avec intermittence AppContainer/Node #185. Tests spécialisés natifs verts. Ne pas remplacer ce rouge par un run vert d'une autre branche.
- Recherche récente : `codex/317-service-retention`, SHA `1c7dac973c8f41f27b303811041b54031f7272ae` ; PR **non créée**, **pas intégré dans main**. Composant `ReadServiceRoot` expérimental, vérifications `[CI #39888522519](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522519)` et `[native #39888522200](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522200)` vertes first attempt. Windows 608 passed, 0 failed, **25 ignored** ; Ubuntu 463 passed, 0 failed.
- Étape antérieure `f91dc3542b647bdb0182437dffc4b6b16ae14d10` CI [#39888064014](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888064014) + native [#39888064013](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888064013) vertes.
- Archive Codex `optic-bridge-317-admission-shutdown-2026-10-23.zip` : manifeste **28/28 fichiers de charge utile vérifiés**, empreintes des principaux fichiers sources conformes aux blobs du SHA final ; rapport `REPORT.md`, `HOSTED_EVIDENCE.json`, `CLOSURE.json`, `audit.patch`. Fichier ZIP fourni dans le chat antérieur, **pas automatiquement présent dans le prochain chat**. Reprendre les sources/preuves GitHub sans inventer un lien sandbox persistant.
- Research #182 : `research/issue-182-git-provenance-contract`, documents ADR-0036 PROPOSED sur `db32111d`, test indépendant sur `b4868b39` [CI #39865183807](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39865183807). Il reproduit une **divulgation de faux marqueur** par GitDiff via index même après remplacement du hardlink. C'est une preuve du problème, **PAS sa correction**.
- V1 [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **NO-GO** ; #317, #176, #182, #185, #319 ouverts au dernier contrôle.

## Vision produit et interdits

**L'IA demande ; la politique autorise ; l'isolation contient.** Maintenir TransportGuard, ActionEnvelope, effets typés, droits session/task, scopes, lease monotone, quotas, sandbox, provenance, présupposés explicites. Pas de privilège ambiant ; Job Object n'est pas un sandbox. Ne pas ajouter Electron/GUI, transport distant, cgroup Linux, nouvelles intégrations ni tâches hors stabilisation. Rust core ne doit pas exiger Node, Python, Docker ni LLM externe. CI Ubuntu obligatoire comme non-régression. Ne pas lever V1 juste parce qu'une CI est verte. Pas de release/tag ni changement de branch protection sans mon approbation.

**Attention aux autorisations du PC :** Codex avait reçu une autorisation spécifique pour tester localement sous Windows 10 (worktrees sur E:, sans écraser les anciennes archives). Ceci ne donne PAS automatiquement au nouveau chat, au modèle ni aux outils une autorisation générale d'accéder au PC, de tuer des processus, d'installer ou de modifier des réglages. Pour des tests natifs, utiliser GitHub hébergé par défaut ; si tu veux qu'un Codex autorisé refasse une campagne locale, propose un prompt explicite au propriétaire. Ne pas promettre du travail asynchrone. Ne pas utiliser de self-hosted runner.

## #317 — dernière progression ET vrais bloqueurs

Code expérimental au SHA `1c7dac9` :
`crates/optic-bridge-runtime/src/process/read_service.rs` et `read_service_tests.rs`,
`crates/optic-bridge-windows/src/prepared_read_command.rs`,
`docs/research/ISSUE317_SERVICE_ADMISSION_SHUTDOWN_2026-10-23.md`.
Production main utilise toujours `crates/optic-bridge-runtime/src/hardened_command.rs`, dont les joins stdin/stdout/stderr peuvent dépasser le timeout si les descendants retiennent les pipes.

**Validé en recherche :** root de service garde les JoinHandles / objets natifs au-delà du caller ; quotas partagés avec ProcessManager, stop hors verrous, record `Running/TerminationUncertain` conservé jusqu'à preuve native, seal/reap refusent avec owner actif, stdout/stderr plafonnés, stdin null pour premier prototype ; panic synthétique récupérée sans libérer quota, suites Windows natives supervisées.

**Démontré LIMITES :**
1. `submit` tient son `SessionAdmissionPermit` seulement pendant réservation/soumission, pas toute vie du worker. Un test **sans permit supplémentaire factice** montre que revoke peut revenir alors que l'owner reste actif ; seal/reap le bloquent malgré tout. Décider entre admission à durée totale avec pending-revoke non bloquant ou contrat async « stop puis seal obligatoire ». Transfert naïf du permit au worker peut faire bloquer `revoke()` indéfiniment !
2. `ReadServiceRoot::Drop` join les workers : défaut d'observation persistant entraîne attente potentiellement infinie. Test synthétique prouve attente pendant l'erreur puis sortie après récupération ; **pas de reproduction réelle d'un kernel OS irrécupérable**. `try_shutdown` peut signaler pending/busy seulement si un propriétaire réel continue de vivre. Ne pas prétendre une sortie bornée sans politique d'owner persistant.
3. **Crash-before-assign réel reproduit** : Windows spawn suspendu, l'hôte synthetic meurt avant `AssignProcessToJobObject` ; l'enfant suspendu survit (Job externe 2→1), mais un test après assignation voit 2→0. Le Job extérieur nettoie tout. Ce positif de reproduction **n'est pas une correction**. Étudier association au Job atomique à la création ou une autorité d'exécution indépendante, compatible Windows ciblé.
4. Course stop/revoke entre dernier test du flag et `ResumeThread` non prouvée linéarisée ; watchdog arrête après assignation, mais **ne prouve pas zéro instruction exécutée après révocation**.
5. Aucun vrai authorizer `AuthorizedRead`, pas de GitRead de production ni de task-lease absolu/deadline/Clock commun, identité du binaire Git/cwd/repo épinglée et environnement contrôlé, pas de garantie sur une Command arbitraire. Test `creation_flags(u32::MAX)` établit seulement remplacement des flags dans ce scénario.

**La PR #327 et les candidats `codex/317-*` restent expérimentaux. Ne fusionner aucun dépôt de recherche en bloc dans main, ne déclarer #317 clos. ADR-0035 est PROPOSED.**

## Travail à reprendre en priorité

**Première tranche utile : contrat technique des responsabilités avant code GitRead.**
- Auditer les chemins `SessionRegistry::revoke`, `SessionLifecycleManager::revoke/seal_quiescent/try_reap`, `ProcessManager/JobStore` et les passages d'autorité `submit/worker/Drop/try_shutdown`. Identifier toutes les opérations qui exigent encore un résultat synchrone et une admission jusqu'à complétion.
- Proposer un design linéarisé de `admit → reserve → spawn-suspended → assign → recheck/resume → terminal proof → finalization` face à `revoke → stop-pre-drain → pending → seal/reap`. Respecter l'accès indépendant du stop et éviter de transformer un hang Drop en hang revoke. Ajouter tests déterministes des races réelles, sans faux permit.
- Comparer et documenter options d'owner natif crash-before-assign, supervision externe/association dès création, compatibilité Win10/11. Tester avec un superviseur *indépendant déjà propriétaire* avant premier effet de fixture. Si aucune solution prouvée, continuer NO-GO sans prétendre la sûreté.
- Une fois décisions acceptées et tests réussis : construire `AuthorizedRead` côté service uniquement, avec `Effect::GitRead`, session+task leases/scope, monotonic deadlines, binaire/cwd/repo pins, env contrôlé et commande interne à arguments fixes ; puis migrer **une seule lecture Git en mode read-only**. Ne pas mélanger #182 (provenance patch) ni mutations Git CAS.
- La difficulté #182 doit être traitée comme contrat séparé : ADR-0036 propose de refuser de façon conservative les patches dont l'autorité/provenance n'est pas démontrée, en attente de validation produit. `fs_read` refusait alias, `git_diff` retournait le faux secret via index même après remplacement. Ne PAS élargir GitRead silencieusement, ne PAS inférer l'autorité Git depuis le chemin actuel.
- #185 CI Node AppContainer et #319 premier MCP restent à diagnostiquer sans hausse arbitraire de timeout, désactivation d'assertions ni retry-to-green.

## Méthode de développement non négociable

Vérifie d'abord l'état GitHub et lis les fichiers canoniques/ADR/risques, puis choisis **un** défaut précis. Une seule PR technique à la fois, petit diff, pas de refonte speculative, mise à jour ADR si un invariant change. Utiliser branche/worktree dédié, préserver anciens essais/commits et première CI rouge, pas de force push inattendu. Preuve exacte source : `cargo fmt`, `cargo clippy --locked ... -D warnings`, cargo tests Windows natifs via GitHub Windows, Ubuntu, policy dependency ; pour tests à risque : Job superviseur indépendant, délais explicites, cleanup/Job empty même en échec, pas de vrais secrets, pas de tunnels ni PID arbitrary kill. Vérifier sur SHA exact du HEAD candidat et indépendamment sur `main` intégré après une éventuelle PR revue/squash. La PR #327 reste OPEN/CI rouge #185 ; toute nouvelle PR documentaire doit rester docs-only et être fusionnée/fermée avant nouvelle PR technique, sinon préserver la règle une-PR à la fois.

**Codex :** quota final rapporté 38 % (variable), re-mesurer avant de suggérer; utiliser là où un PC Windows autorisé prouve un risque indisponible sur hosted runner. Ne pas déléguer le même fichier/branche à ChatGPT et Codex simultanément. Demander des ZIP assainis avec `MANIFEST.json` SHA-256, rapports, scripts reproductibles, runs et SHA, sans quarantaine opaque, processus privés ou secrets. Les essais locaux antérieurs ne sont pas un droit général de connexion. **Pour coordonner :** ChatGPT pilote audits/règles/doc/code GitHub ; Codex fait tests Win10 et corrections autorisées sur branches indépendantes ; choisir un seul responsable d'une PR ; faire inspecter l'archive et les commits par l'autre avant promotion.

## Conditions de succès et rapport final

Pour chaque incrément, indiquer code modifié et non modifié, SHA main/branche/PR, statut vrai GitHub, CI exacte et tentatives, Windows10 local vs runner Windows Server, droits/risques, preuves manquantes, impact produit, décisions à confirmer. Pas de « 100 % clean » ni V1 tag sans critères [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) réellement satisfaits.

**Ta première action :** lire et vérifier l'intégralité de la nouvelle passation datée 23 octobre, puis les sources `1c7dac9` et ses preuves ; proposer/corriger la première tranche qui résout admission-vs-revoke et crash/shutdown, plutôt que démarrer une migration GitRead ou dupliquer des mocks. Travaille proprement, avec initiative, sans m'interroger pour chaque détail technique.
