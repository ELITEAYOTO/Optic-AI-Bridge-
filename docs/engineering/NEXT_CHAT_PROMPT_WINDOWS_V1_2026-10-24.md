# PROMPT COMPLET POUR NOUVEAU CHAT — OPTIC AI BRIDGE — 24 OCTOBRE 2026

## COPIER CE PROMPT : REPRISE ACTUALISÉE APRÈS DIX SCÉNARIOS (24/10/2026)

> Cette section actualisée **remplace comme consigne active** la section `État solide au moment de la passation` ci-dessous, qui date d'un checkpoint antérieur (`100c43b5`/`8202c9c`). Les paragraphes anciens sont conservés comme historique. Les SHAs ci-dessous sont des preuves datées, non des instructions de checkout/reset destructif.

Tu reprends la direction technique, l'audit et la stabilisation **du dépôt unique [ELITEAYOTO/Optic-AI-Bridge-](https://github.com/ELITEAYOTO/Optic-AI-Bridge-)**. Ne touche pas aux dépôts **Optic Desktop** (application Windows, UI Figma, installation/mises à jour ; un autre agent s'en occupe) ou **REL** (reverse engineering). L'opérateur te délègue les décisions de correction courantes, la revue GitHub, les petites branches et PR vérifiées, sans t'autoriser implicitement son PC, des réglages de sécurité, le lancement d'une release ou de nouveaux effets. Objectif : **V1 locale Windows 10/11 fiable et sécurisée, actuellement NO-GO**.

### Reprise impérative

Lis **dans cet ordre** : `AGENTS.md`, `AI_HANDOFF.md`, `STATUS.md`, `ROADMAP.md`, `docs/engineering/GITHUB_HANDOFF.md`, `docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md` (**son encadré ACTUALISATION CANONIQUE en premier**), ce prompt, `PROJECT_CONTINUITY_PROMPT.md`, `docs/engineering/REPOSITORY_WORKFLOW.md`, `docs/engineering/RELIABILITY_DEBT.md`, `docs/security/SECURITY_INVARIANTS.md`. Parcours ensuite les sources Win32/process/session et commentaires #317 appropriés.

Avant toute modification, interroge réellement GitHub : `main` SHA/CI, PR ouvertes avec head/base, issues #317/#323, branch heads, exact-head hosted CI, et contenu du dernier rapport de recherche. Si GitHub a avancé, **réconcilie** les faits et reprends sur l'état actuel. Ne présume pas que ce handoff documentaire est déjà fusionné lorsque tu le lis.

### Checkpoints attestés avant la PR documentaire

- Production `main=16ca67a75fd49c5681a0672773ffc268c3635f6b`, [push/main CI #40047950688](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40047950688) **success** sur ce SHA. Le dernier permis de session possédé, `OwnedSessionAdmissionPermit`, est intégré, mais **GitRead de production dépend toujours de `HardenedCommandRunner`** ; pas de guardian natif de production.
- **Seule PR ouverte : [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327)**, recherche indépendante : la laisser intacte, ne pas fusionner sur la foi d'une autre CI.
- Issues [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) et [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **ouvertes, V1 NO-GO**. Vérifier aussi #176, #182, #185 et #319.
- Dernière **preuve de code** (recherche uniquement) : branche `research/317-failed-native-create-2026-10-24`, SHA **`b5a91deea1b306c56a2c03de7bb5a86b699d1d6f`** ; [CI #40057931887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887) `completed/success`, **attempt 1** sur cet exact commit : **355 tests Windows réussis, 13 ignorés, 0 échec ; 270 tests Ubuntu réussis**, rustfmt et Clippy strict verts, **10 scénarios natifs Windows supervisés**, workspace Windows complet réussi.
- Dernière **documentation de recherche** : SHA **`4bef362d1e9c45ec3b81a4d6807ae924f6122218`** dans cette même branche, rapport [ISSUE317_CRASH_BEFORE_NATIVE_ACK_2026-10-24.md](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/research/317-failed-native-create-2026-10-24/docs/research/ISSUE317_CRASH_BEFORE_NATIVE_ACK_2026-10-24.md). Sa CI [#40058237376](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40058237376) était **encore en cours** lors de cette passation : recontrôler. [Artefact du code validé](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887/artifacts/12546595512). [Dernier rapport issue #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6379288501).
- Premier rouge du nouvel essai : [#40057892808](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057892808), **Rustfmt**, corrigé par nouveau SHA ; jamais réécrire ou masquer. Plus tôt [#40056998380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40056998380) a montré un vrai code Win32 de rejet `0x80070057`, qui a corrigé l'assertion exacte sur la branche précédente.

### Progrès réel de la recherche (mais aucune correction GitRead production)

Les branches isolées ont démontré, par incréments vérifiés : (1) admission session génération-bound et JobStore conservés malgré arrêt/observation incertaine ; (2) Job Object précréé, association kernel **à `CreateProcessW`**, enfant libtest fixe suspendu ; (3) vraie supervision extérieure des interruptions d'un **processus lanceur séparé** avant et après ACK, avec contrôle négatif intentionnel et scénarios `PendingNoNativeReceipt` ; (4) Job original détenu par le **service** avant lancement, donc sa propriété survit au décès du lanceur ; (5) prêt du handle Job limité à `ASSIGN_PROCESS|QUERY` avec refus réel de `TerminateJobObject` depuis le lanceur ; (6) refus réel de `DuplicateHandle` avec destination invalide, aucun handle transmis ni quota rendu avant preuve Job vide ; (7) **dernier cas : vrai `CreateProcessW` échoue pour répertoire de travail inexistant**, sans enfant ; Job du guardian toujours vide ; `Failed` publié **uniquement après requête kernel positive**, malgré observation artificiellement défaillante avant réconciliation. Les **dix** scénarios de l'ultime CI ont les résultats attendus, y compris les contrôles négatifs.

Les détails, noms exacts des branches et CI de chaque tranche sont dans le **tableau d'actualisation de la passation canonique** et les commentaires de #317. **Ne pas regrouper/merger les branches de recherche**. Un JobStore est une comptabilité, **pas** le handle Job Object. `NativeProcessReservation::finish` ne vérifie pas intrinsèquement le kernel : le guardian doit fournir et ordonner cette preuve. Un échec de requête injecté et recouvrable n'est ni panne Win32 permanente ni résilience après perte du service. Les counts du Job extérieur **incluent le processus hôte**, pas seulement les descendants. Une CI verte sur des scénarios dont le résultat attendu est `Pending` ou un contrôle négatif ne prouve pas qu'ils sont réparés.

### Prochain travail PRIORITAIRE, sans tourner en boucle sur les preuves déjà obtenues

1. **#317 pipes et shutdown.** Audit de `crates/optic-bridge-runtime/src/hardened_command.rs`, `process.rs`, `session/registry.rs`, `session/lifecycle.rs`, et du dossier process/read_service expérimental. Cibler **descendants maintenant les handles stdout/stderr ouverts après la sortie du root**, stdin/EOF/cancellation, sortie du writer/readers, stop-before-drain, obligation de répondre de manière bornée sans fausse terminaison lorsque la supervision kernel échoue durablement. Garder la comptabilité/quota pendant incertitude, prouver attribution exacte des ressources. Préférer un nouveau test minimal et synthétique sur runner Windows, sous superviseur indépendant, pas le PC du propriétaire.
2. **Architecture accountable.** Distinguer propriété kernel, admission/lease originelles, stop publié hors verrou, quiescence prouvée, terminal publication et collecte/reap. Après disparition du service lui-même, quelle autorité et quelle récupération durable des quotas ? Pas de `timeout → Drop/detach/forget`, pas de succès artificiel. Si la réponse relève d'ADR-0035/0036 ou politique #182, préparer la revue et demander validation avant modification sensible.
3. **Une PR production minimale si une propriété indépendante est réellement démontrée**, depuis le `main` courant, avec tests de régression et CI Windows/Ubuntu sur SHA exact. PAS de cherry-pick/bulk merge du prototype entier, pas de promotion de module `cfg(test)`. Après squash/review, CI indépendante `push/main` du nouveau SHA ; ne pas refermer #317/#323 par mots clés de PR.
4. **GitRead réel et V1 ensuite** : effet Git read-only autorisé par task-lease, executable/repo pin, provenance (#182), lecture sensible (#176), budgets, flux, permissions. #185 (AppContainer), #319 (MCP démarrage) et matrice Windows 10/11 / installer/offline / mise à jour / rollback demeurent des gates séparées.

### Méthode non négociable

- `#![forbid(unsafe_code)]` du runtime, tout Win32 unsafe uniquement dans `optic-bridge-windows`, wrappers étroits et feature de recherche non par défaut. Le Job Object n'est **pas** un sandbox.
- `TransportGuard`, `ActionEnvelope+Effect` typés, autorité session/task-lease et TTL monotone, budgets bornés, chemins/provenance vérifiables, aucun PID arbitraire ni commande opaque/générale, fail-closed.
- GitHub connectors et runners hébergés seulement, **aucun accès à mon PC par défaut**. Codex est possible sur prompt transmis par moi, mais les anciennes tentatives locales de tests natifs ont déclenché des restrictions de cybersécurité. Ne cherche pas à les contourner en déguisant les demandes ; préfère la CI et, si nécessaire, le processus officiel d'autorisation. Ne pas exiger Codex pour réexécuter l'historique.
- Préserver #327 et les branches/worktrees, ne jamais reset des états non consultés ; respecter `REPOSITORY_WORKFLOW.md`. Petites PR et revues auditables, SHA exact, tests vraiment exécutés et logs inspectés, rouge initial conservé, formats/Clippy stricts, tests Linux/Windows, CI après fusion si applicable ; aucun `skip`, élargissement de timeout ou permission pour masquer un échec.
- Pas de release/tag, modification du ruleset, changement de politique ou de périmètre, UI/Electron, exposition distante aux vrais projets sans mon accord. Aucune promesse que la CI Windows Server équivaut à une validation sur postes Windows 10/11.

**Livrable attendu du nouveau chat :** vérifier le GitHub live et les docs, rapporter en quelques lignes le dernier état validé ; choisir un risque de #317 lié aux pipes/EOF ou à l'arrêt borné **non déjà prouvé** ; réaliser réellement une petite recherche/progression GitHub, faire contrôler le SHA, publier CI/artefacts/limits et mettre à jour #317. En cas de limite matérielle ou de politique, produire une analyse vérifiable et demander uniquement l'arbitrage indispensable. Je suis disponible ; n'hésite pas à me consulter, mais ne bloque pas sur des questions déjà tranchées.

---


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
