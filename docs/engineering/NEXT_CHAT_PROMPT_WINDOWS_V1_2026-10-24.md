# PROMPT COMPLET POUR NOUVEAU CHAT — OPTIC AI BRIDGE — 24 OCTOBRE 2026

Je te confie la **direction technique courante**, l'audit, la stabilisation et les prochaines étapes de mon projet **Optic AI Bridge**, dans la continuité de longues conversations précédentes. Je veux un travail autonome, rigoureux, concret, avec une progression réelle. Je peux t'aider, fournir des informations et **faire travailler Codex sur mon PC** si les runners hébergés ne suffisent pas ; n'hésite pas à me fournir un prompt Codex prêt à copier **lorsque c'est réellement utile**.

**Dépôt officiel : https://github.com/ELITEAYOTO/Optic-AI-Bridge-**

## Mission et limites

Objectif : rendre fiable, cohérent, sûr et maintenable le **socle Rust/MCP existant** pour une future **V1 locale Windows 10/11**. Stabilisation d'abord, pas de nouvelles fonctionnalités. La V1 reste **NO-GO** jusqu'à validation explicite des contrats/process/flux/sécurité, vrais appareils Windows et matrice release ; ne jamais annoncer une V1 sur la seule base de CI vertes.

Je te délègue les décisions techniques ordinaires, les tests sur GitHub, les branches de recherche isolées, la préparation de PR limitées et la documentation. Ne change pas sans mon accord les permissions/politiques de sécurité, réglages de protection de branche, la portée produit, l'accès distant, mon PC, ou les releases/tags. Ne fais pas de refonte Electron/UI, d'intégration distante, de nouveau transport, d'accès aux vrais projets, de chantier Linux post-V1 ni de fusion massive des alphas/recherches.

**Règle centrale : l'IA demande, la politique autorise, l'isolation contient.** Les effets passent par le contrat ActionEnvelope + Effect typés, avec session + task-lease vérifiables, quotas, deadlines monotones, scopes et provenance. Les Job Objects Windows ne sont pas une sandbox.

## Reprise obligatoire : vérifie GitHub AVANT toute édition

1. Inspecte réellement la branche main, SHA, CI push sur ce SHA, PR ouvertes, leurs heads/bases, reviews, issues #317/#323/#176/#182/#185/#319, workflow de sécurité et état des branches de recherche.
2. Lis dans l'ordre : AGENTS.md, AI_HANDOFF.md, STATUS.md, ROADMAP.md, docs/engineering/GITHUB_HANDOFF.md, **docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md** (PASSATION CANONIQUE), ce prompt, PROJECT_CONTINUITY_PROMPT.md, docs/engineering/REPOSITORY_WORKFLOW.md, docs/engineering/RELIABILITY_DEBT.md et docs/security/SECURITY_INVARIANTS.md. Lis ensuite les fichiers/ADR/branches de recherche concernés.
3. Ne traite jamais un SHA de passation comme un ordre de reset, de checkout destructif ou de merge. Ce document est daté ; GitHub en direct prévaut. Vérifie si la PR documentaire de passation a bien fusionné et si la CI **post-fusion** du nouveau main a réussi.
4. Préserve le travail non commité, les branches, la PR #327 et les résultats de recherche précédents ; n'écrase pas les commits non examinés.

## État solide au moment de la passation (revalider)

- main production au dernier contrôle : **100c43b53c795be5678c8b4f2c0af4353ca5b196**. Son push/main [CI #40027279676](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40027279676) était intégralement réussi sur ce SHA, attempt 1.
- Dernière modification #317 dans main : PR **#336** squash intégrée, nouveau permis **OwnedSessionAdmissionPermit** non clonable, propriété Arc du registre, même compteur d'admissions et témoin stop de génération/expiry initiale. **Cela ne prouve pas l'arrêt natif** et ne migre pas le runner Git.
- PR toujours ouverte : **#327**, tête ef3208d787baa0c6433abfa0b4aba687d8bc6bb7, recherche. Ne pas la fusionner avec main simplement parce que des prototypes ultérieurs ont des CI vertes.
- **Issue #317** : arrêt des pipes et propriété de processus natifs, toujours ouverte ; **#323** : V1 freeze gate **NO-GO** ; #176/#182 : lecture sensible et Git provenance ; #185 : flake Windows AppContainer/Node ; #319 : première réponse MCP ~30s. Toutes ouvertes au dernier contrôle.
- Dernière recherche complète : **research/317-native-observation-uncertain-2026-10-24**, SHA **8202c9c633bf797be9ea69f9ce994b5ec5da9b86**, [CI #40046563179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40046563179) **Windows 348 pass, 0 fail, 11 ignored ; Ubuntu 270 pass, 0 fail ; fmt/Clippy strict ; explicit test 1/1, first attempt on exact SHA**. Son [rapport #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6377736151) et sa doc de recherche contiennent les détails.
- Cette **recherche n'a pas été fusionnée** ; main utilise encore le runner HardenedCommandRunner. La PR documentaire actuelle peut avancer main après ce snapshot : vérifier.

## Progression importante de #317 : tests séparés, puis combinés

L'historique est complet dans la nouvelle passation canonique et dans les commentaires de #317. Ne réexécute pas mécaniquement ce qui est déjà démontré.

1. Codex, branche **codex/317-native-registration-authority** SHA **bcad195208bc5ecb0222fbfe5cd0595d07cb5938** : prototype d'autorité native, 22 scénarios Windows supervisés (CI #39954689295), mais non branché en production et Drop persistent incertain.
2. Recherche commit natif **b6653f7** : engagement d'un vrai JobStore sous le même verrou SessionRegistry que la révocation, une seule fois, sans Win32 sous le verrou.
3. Recherche **555d32f** : un worker indépendant retient une admission possédée et une vraie réservation JobStore. **JobStore ≠ kernel Job**.
4. Recherche **669b3ad** : vrai Job Object précréé avant CreateProcessW, association via **PROC_THREAD_ATTRIBUTE_JOB_LIST** ; supervision extérieure du crash du créateur, le Job extérieur restant ouvert pendant la preuve 1 → 2 → 0.
5. Recherche **75024b0** : composition admission possédée + JobStore + Job Object + enfant suspendu ; vrai stop publié pendant que le propriétaire retient ses ressources ; libération après preuve root arrêté + Job vide. [CI #40042298153](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40042298153), Windows 347 / Ubuntu 270.
6. Recherche **8202c9c** : fail Win32 **simulé** sur QueryInformationJobObject APRÈS la sortie du root ; en phase incertaine le worker garde permis, Job, réservation et statut **non terminal** ; le registre refuse l'effacement prématuré et le retour de revoke ; seulement après une nouvelle **lecture réelle Job vide** publication de Stopped et release. Windows 348 / Ubuntu 270. **Ce test n'est PAS une panne Win32 persistante réelle, ni la preuve d'un shutdown borné ou d'un guardian durable**.

Points de vigilance :
- LimitedJobObject::new est une configuration de limites, pas un handle kernel ; OwnedJob::create fait l'appel natif.
- Un TerminateJobObject réussi, un root terminé ou stdout/stderr EOF ne démontrent pas forcément la quiescence de tous les descendants.
- Le modèle de révocation production attend le drain des admissions avant certaines étapes de cancel_session : une attente naïve sur un worker persistant pourrait bloquer indéfiniment.
- Une observation incertaine impose le maintien d'un propriétaire et des ressources jusqu'à preuve positive ; **ne pas appeler finish(Stopped), Drop, forget ou détacher par timeout**.
- **#![forbid(unsafe_code)] dans optic-bridge-runtime est absolu**. Les wrappers Win32 unsafe restent exclusivement dans optic-bridge-windows avec API sûre et features de recherche non activées par défaut. La précédente recherche a volontairement gardé le premier échec de compilation qui a détecté cette violation.
- Distinguer test de laboratoire, code compilé sous cfg(test), candidat production, code fusionné sur main, puis acceptation native physique Windows. Ne pas transférer ces statuts implicitement.

## Chantier à reprendre en priorité

**A — Séparer arrêt et drainage sans perdre la propriété native.** Étudie les contrats actuels SessionRegistry::revoke, SessionLifecycleManager::revoke, OwnedSessionAdmissionPermit, ProcessManager::cancel_session, NativeProcessReservation, JobStore et HardenedCommandRunner. Concevoir un **arrêt/close explicite et non bloquant** distinct du **reap/quiescence** ; le stop doit parvenir hors verrou et avant tout drainage. Sous observation OS durablement erronée, conserver un propriétaire/guardian réel, handles et quotas ; autoriser une réponse de fermeture bornée **sans prétendre que le processus est terminé**. Évaluer panics, destruction du service, erreurs de Job, races commit/stop, reaper ABA et lock-order. Ne propose pas un « timeout puis abandon » ; si une garantie réclame une décision d'architecture/ADR ou un service persistant, documente-la précisément.

**B — Preuves Windows indépendantes.** Commencer par une petite branche expérimentale de test persistant, pas un changement spéculatif de production. Ajouter, lorsque justifié, des scénarios supervisés : observation erronée répétée, descendant qui garde les sorties stdout/stderr ouvertes, stdin fermé sous stop, parent/descendant indépendants, host-kill, stop au moment create/resume, sortie EOF, quotas bloqués quand incertain, aucune libération fausse. Préparer un protocole d'échec et nettoyage extérieur vérifié. Toujours utiliser des fixtures synthétiques sur GitHub-hosted Windows, pas des répertoires/projets réels.

**C — Intégration progressive.** Si un mécanisme tient réellement, extraire uniquement le plus petit sous-ensemble depuis le **main courant** vers une PR cohérente avec tests, docs et absence de nouveau droit. Valider Windows/Ubuntu, dépendances et CI requises sur tête PR exacte, puis squash review et nouvelle CI **push/main du SHA intégré**. Ne pas merger un arbre research wholesale. Maintenir issues #317 et #323 ouvertes.

**D — GitRead en dernier.** Un seul effet Git read-only scellé avec task lease, repo/exe pin, policy/provenance, révisions, scopes, et contrôle des réponses. Résoudre ou refuser de manière déterministe les cas #176/#182 avant toute exposition de projet réel ; #185/#319 restent des sujets séparés.

## Bonne méthode de développement et de validation

- Une petite unité de travail démontrée à la fois ; audit, hypothèse, tests de régression, correction, preuve. Pas de « vert » inventé.
- Sur GitHub : créer des branches spécifiques depuis le bon ancêtre, éditer proprement, conserver les SHA. Rustfmt strict, Clippy avec -D warnings, tests Windows/Ubuntu, scénario natif exact, journaux, exclusion des tests ignorés explicitement expliquée.
- Inspecter les jobs et les **logs**, confirmer que les cas natifs ciblés ont réellement exécuté ; une CI uniquement Ubuntu ne valide aucune assertion Win32.
- Si le premier run est rouge, **conserver l'échec et son SHA** ; corriger puis relancer sur un nouveau SHA. Ne jamais masquer un flake (#185/#319) par timeouts élargis, skip, permissions nouvelles ou retry-to-green.
- Dans les descriptions de PR éviter les mots-clés de fermeture automatique accompagnés de numéros d'issues, même dans des négations : GitHub avait accidentellement fermé #317/#323 pendant une PR docs.
- Mettre à jour les documents canonique STATUS, ROADMAP si l'ordre change, CHANGELOG si le runtime change, doc de contrat et commentaire de l'issue ; ne pas réécrire les vieux handoffs. Chaque affirmation de résultat doit avoir une preuve exploitable.
- Ne jamais promettre « zéro bug », ni que les CI Server garantissent Windows 10/11 physiques ; la matrice V1, release/rollback et permission owner sont encore ouvertes.

## Comment je veux que tu collabores avec Codex

**Je peux faire exécuter Codex sur mon propre PC lorsque tu estimes que ça débloque une vraie limite des CI hébergées.** Tu peux me donner un **prompt Codex complet et prêt à copier**. Je transmettrai moi-même le prompt et je te renverrai ses logs, résultats, commits et, si nécessaire, un ZIP de preuves. Ne suppose pas que tu disposes directement de Codex ou de la machine sans autorisation et outil explicites. Ne gaspille pas son quota sur une simple lecture GitHub ou des tests déjà possibles sur GitHub-hosted Windows.

Si tu veux utiliser Codex, rédige son prompt avec :
- dépôt, branche et SHA exacts à réinspecter ; problématique et invariants ;
- worktree **isolé**, jamais d'écrasement du répertoire principal, ni de suppression d'archives/quarantaines ;
- périmètre des modifications et fixtures synthétiques, Windows natif, nettoyage **uniquement** des PIDs/Jobs/handles explicitement créés par la campagne ; pas de modification de paramètres Windows, antivirus, pare-feu, politique GitHub ni secrets ;
- test à reproduire **avant** correction, commandes/observations et délais bornés, supervision extérieure, aucun PID arbitraire, traces filtrées de secrets ;
- exigence de rapports (VERSION, SHA, environnement Windows, commands, résultats réussites/échecs/ignorés, exit codes, preuve de nettoyage, diff, limites), CI GitHub distincte, PR non fusionnée sans review, ZIP optionnel avec manifeste/empreintes ;
- retour utilisateur attendu pour que tu puisses faire une revue indépendante.

Historique : Codex avait déjà effectué des campagnes importantes et pu utiliser le PC Windows 10 sur des worktrees séparés lorsqu'il avait reçu une instruction spécifique. **Cette autorisation historique n'est pas une délégation automatique nouvelle.** Tu peux demander la permission au moment opportun avec ton prompt.

## Réponse attendue de toi à cette reprise

Ne recommence pas l'historique des anciennes recherches à l'aveugle. Vérifie GitHub, lis le nouveau handoff et les fichiers du dépôt, résume brièvement **le dernier état vérifié**, sélectionne un risque concret du chantier A, puis **commence le travail réel** sur GitHub selon la méthode ci-dessus. Tu peux demander une décision de sécurité importante, mais ne bloque pas par des questions mineures déjà répondues. Je suis disponible si besoin. Donne des bilans clairs avec liens GitHub, SHA, tests exacts, résultats et limitations ; après une tranche, continue proprement sans élargir artificiellement le périmètre.
