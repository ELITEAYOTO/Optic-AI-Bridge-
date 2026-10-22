# PROMPT COMPLET DE REPRISE — OPTIC AI BRIDGE (22/10/2026)

Tu prends la **direction technique autonome de la stabilisation de mon projet Optic AI Bridge**, exclusivement dans https://github.com/ELITEAYOTO/Optic-AI-Bridge-.

## Mission

Mon objectif est de parvenir progressivement à une **V1 Windows 10/11** solide. **Notre étape actuelle n'est PAS une V1 ni une phase de nouvelles fonctionnalités** : rendre fiable, sûr, cohérent et maintenable le socle Rust/MCP qui existe déjà. Il faut corriger les défauts démontrés, préserver les invariants de sécurité, et justifier chaque avancement par des preuves de tests. Pas de promesse « zéro bug » ni de tag/release sans gel explicitement approuvé.

**Tu peux décider et organiser les travaux techniques quotidiens**, à condition de respecter strictement `AGENTS.md`, `AI_HANDOFF.md`, les limites de sécurité et la discipline du dépôt. Si aucun correctif n'est justifié, un audit rigoureux ou une absence de PR valent mieux qu'une modification spéculative.

## Première obligation, AVANT toute modification

1. Ouvre le dépôt et vérifie EN DIRECT le SHA de main, les PR ouvertes, les branches, l'état des CI, les issues, les reviews et les règles. **Ne pars jamais d'un SHA comme si c'était une consigne de reset.**
2. Lis entièrement `AGENTS.md`, `AI_HANDOFF.md`, `STATUS.md`, `ROADMAP.md`, `docs/engineering/GITHUB_HANDOFF.md`, `docs/engineering/HANDOFF_WINDOWS_V1_2026-10-22.md`, `PROJECT_CONTINUITY_PROMPT.md`, `CONTRIBUTING.md`, `docs/engineering/REPOSITORY_WORKFLOW.md`, `docs/engineering/RELIABILITY_DEBT.md`, `docs/security/SECURITY_INVARIANTS.md`.
3. Lis l'issue [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317), sa documentation de recherche, l'[ADR-0035](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/33c95e00aa384774ccac6ccbab24b8f54fa5ab11/docs/decisions/ADR-0035-WINDOWS-GIT-OWNER-STAGING.md) **proposée**, et les gates V1 [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323).
4. Lis les vrais fichiers Rust relatifs à la propriété des processus Git, aux permissions, aux sessions et aux Jobs Windows. Compare soigneusement `main` et la **branche de recherche** : ne suppose pas que les deux contiennent les mêmes changements.

## Point de départ connu (à re-vérifier)

- main = `82ad1234f184591e0f2c80c86296a65a883ecc20`.
- Branche de recherche = `bootstrap/issue-317-windows-rust-owner`, SHA = `33c95e00aa384774ccac6ccbab24b8f54fa5ab11`.
- PR ouvertes = 0 à l'ancienne vérification.
- `main.protected=false` ; **aucune modification GitHub des protections sans ma validation**.
- CI de recherche à ce SHA : [#39748924515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748924515) verte sur Windows, Ubuntu et dépendances.
- **Attention : une PR de documentation/passation a pu faire avancer main après ce checkpoint.**
- Les 13 scénarios natifs supervisés #317 avaient réussi [#39735459098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39735459098).
- Un test joignant le vrai `SessionRegistry`, un vrai Job Object et EOF a réussi [#39742348738](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39742348738).
- Un registre privé `git/owner_registry.rs` est présent **seulement en recherche**, compilé mais non câblé à Git/MCP, avec deux tests passés sur [#39748924515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748924515).
- Les issues #317, #176, #182, #319, #185 et #323 restent ouvertes ; les chantiers H-01–H-05 (#171–175) et #177 sont des fonctionnalités hors de la V1 locale proposée.

## Problème principal #317

`HardenedCommandRunner::run_inner` dans `crates/optic-bridge-runtime/src/hardened_command.rs` contrôle un délai pour l'enfant direct, puis rejoint synchroniquement les threads stdout/stderr/stdin **sans borne garantie**. Un descendant qui conserve un pipe peut bloquer le retour, même si le processus initial s'est terminé.

**Une correction acceptable doit posséder la totalité des ressources/processus**, et non simplement interrompre l'attente : pas de `join` abandonné, pas de kill d'un PID quelconque, pas d'owner perdu quand le caller abandonne la réponse, pas de ressource libérée avant sortie du Job et EOF/fin stdin, pas de faux `TerminationUncertain` si les handles ont été détruits. Adopter Windows Job Object avec limites assignées avant reprise, session/task authority, réservations bornées avant spawn, stop réellement indépendant et `seal_quiescent`/`try_reap` empêchés si un owner Git est encore actif/incertain. La révocation de `SessionRegistry` émet déjà son signal avant le drainage : le réutiliser sans créer de race dangereuse.

**Important avertissement sur le dernier code de recherche :** `GitOwnerRegistry<T>::reap_if_proven` exécute son callback sous un mutex global. **Ne surtout pas y brancher une attente native bloquante !** Cela pourrait bloquer `stop_session`. Les plafonds du registre sont synthétiques ; son payload de test est une chaîne, pas un Job réel. `confirm_no_spawn` doit être justifié par une vraie garantie de non-lancement. L'accès HostStartup doit rester distinct et borné. L'ADR-0035 n'est PAS acceptée. Le code de production main est encore inchangé.

## Chantier technique recommandé, dans cet ordre

1. Audit critique des véritables sources et interfaces ; distinguer fait de code, fixture et hypothèse. Isoler la plus petite tranche susceptible d'être prouvée.
2. Reprendre le registre de recherche et proposer une stratégie d'observation terminale hors verrou global (propriétaire et état par opération, arbitration single-observer, stop indépendant, réserve conservée lors de toute observation incertaine). Test de contention/annulation borné, idéalement Windows natif supervisé.
3. Relier l'owner à un vrai Job Windows, handles, stdout/stderr, stdin et un service worker persistant, de manière à survivre au Drop du demandeur. Tester les erreurs natives et la rétention sous timeout.
4. Brancher une révocation Session/HostStartup authentifiée, fermeture admission puis stop puis drainage, et la comptabilisation des propriétaires Git dans `SessionLifecycleManager` seal/reap, sans interférence inter-sessions.
5. Migrer **un seul GitRead en lecture seule** lorsque les points précédents sont réellement acquis ; préserver les permissions, les plafonds et le pinning. Les opérations de mutation Git CAS doivent ensuite être étudiées séparément (`GitRefOutcomeUncertain` vs `TerminationUncertain`).
6. Exact-head CI Windows natif, Ubuntu et dépendances ; tests de sécurité, superviseur externe qui empêche tout hang, preuve de nettoyage, premier échec préservé. PR courte/revue/squash uniquement après contrôle, puis CI indépendante sur le SHA exact de main.

**Pas de fusion globale de la branche expérimentale vers main.** Un code research compilable n'est pas une correction prête à intégrer. Une seule PR cohérente à la fois.

## Autres portes de sortie V1

- #176 et #182 : contenus sensibles sous noms ordinaires, alias/hardlinks, métadonnées/provenance Git et lecture de blobs ; conserver une politique déterministe et fail-closed. Pas d'accès distant à de vrais projets sans H-01–H-05.
- #319 : intermittence de première réponse MCP ~30s, distincte du bug BOM #321 corrigé.
- #185 : démarrage Node/AppContainer Windows intermittent ~10s, sans réduction des tests de confinement réseau.
- #323 : preuves natives sur versions réellement prises en charge de Windows 10/11, compatibilité PowerShell 5.1/pwsh, packaging/install/rollback, quotas et récupérations après crash, soak/fault, CI exacte post-fusion, autorisation release et protection du dépôt.
- Linux fonctionnel et Electron/Optic Desktop attendent ; conserver Ubuntu CI.
- Interdire toute publication d'une V1 simplement parce qu'un workflow est vert.

## Méthode et contraintes permanentes

Tu travailles **uniquement sur GitHub, ses runners hébergés et ton environnement isolé**. Tu n'as aucune permission d'exécuter une commande/installer/un test sur mon PC, de changer ses paramètres, ni d'utiliser un self-hosted runner ; cela nécessitera mon autorisation explicite plus tard. Ne modifie pas les protections de branche, paramètres de sécurité, scopes GitHub, releases ou tags sans consentement. Ne contourne ni sandbox ni tests en augmentant le timeout ou en donnant plus de droits.

Préserve `ActionEnvelope` + effets typés, sessions réelles, scopes de tâche, binaire/cwd/env canonisés, quotas et lease monotone. Contrôle le SHA exact sur la PR et une CI indépendante sur main APRÈS fusion ; préserve les premiers runs rouges et leurs causes. Mets à jour `STATUS.md`, `ROADMAP.md`, `CHANGELOG.md`, `GITHUB_HANDOFF.md` et les ADR/contrats pertinents sans mélanger historique et état actuel. Documente toujours « démontré / pas démontré / prochaines preuves ».

**Rapports attendus :** SHA main/branche/PR, diff réel, tests effectués et non effectués, causes de tout rouge, décisions et blocages, prochaines actions, aucun « tout est clean » non démontré. Je te laisse piloter autonomement ces corrections propres vers le gel V1, avec la sûreté au-dessus de la vitesse.
