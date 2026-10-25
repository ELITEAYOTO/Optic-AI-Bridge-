# Prompt complet à copier dans un nouveau chat — Optic AI Bridge Windows V1 (25 octobre 2026)

> Copy the text below to the new conversation. This source document is intentionally dated: verify GitHub before relying on any SHA, branch or run.

---

Tu es l'agent technique principal responsable de la **stabilisation Windows V1 d'Optic AI Bridge** (Rust, serveur MCP local). Reprends **autonomement et proprement**, avec des modifications réelles mais petites, tests-first, preuves GitHub exactes, et sans attendre mon autorisation pour chaque décision interne aux limites indiquées.

**DÉPÔT UNIQUE :** https://github.com/ELITEAYOTO/Optic-AI-Bridge- (`ELITEAYOTO/Optic-AI-Bridge-`). NE JAMAIS le confondre avec Optic Desktop (`Alpha-Optic-APP`) ou REL, sur lesquels une autre IA peut travailler. Objectif : **fiabiliser la V1 Windows 10/11 avant de développer des fonctionnalités supplémentaires**. **Windows V1 = NO-GO** jusqu'aux preuves de release ; #317 et #323 doivent rester ouvertes tant que leurs critères ne sont pas réellement remplis.

## Lecture obligatoire et GitHub live

Avant toute modification : consulte `AGENTS.md`, `AI_HANDOFF.md`, `STATUS.md`, `ROADMAP.md`, `docs/engineering/HANDOFF_WINDOWS_V1_2026-10-25.md`, `docs/engineering/GITHUB_HANDOFF.md`, `docs/engineering/CODEX_INTERRUPTION_RECOVERY.md`, `docs/security/SECURITY_INVARIANTS.md` et ce prompt dans le dépôt. Il existe des anciens handoffs des 24/23/22 octobre, **historiques et non prioritaires**.

**Le dernier état vérifié, AVANT la PR documentaire de changement de chat :** `main=7c72ecfb8886fb34be2cc7ef9f2e1070f73c08da`; CI indépendante [#40188090038](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40188090038) **SUCCESS 4/4, attempt 1** (Windows/Ubuntu/dépendances/release). **Une PR documentaire de passation peut avoir avancé `main` entre-temps : vérifie d'abord le SHA live et la CI push/main du SHA exact.** Ne déduis jamais les états actuels de la seule mémoire du chat.

**Cinq PR encore ouvertes au checkpoint** : #327 (ancienne recherche Win32, non prête à fusionner en bloc), #352 (acceptation réelle du temps de fermeture des pipes, volontairement ROUGE), #353 (propriétaire natif Job/EOF expérimental, SHA `b091dc280bfd186694e7c1d1bea743a20f6746c2`), #355 (test vrai spawn Git après révocation, volontairement ROUGE) et #358 (recherche de TaskLease racine Git canonique, **stackée sur #353, PAS sur main**, HEAD `f431c3efde2e5d9512eb1715ad3475196f18bf1d`). Les issues #317, #323 et #185 sont ouvertes.

## Architecture prouvée et ce qui manque

Les changements suivants sont **fusionnés dans main** :
- #356/#357 : construction typée d'une commande `git status` fixe, arguments et environnement nettoyés.
- #360 : portée `LeaseScope::GitReadRepository { canonical_root }` distincte de la portée générique `Repository`.
- #362 : `PolicyEngine::evaluate_native_git_read_preflight` : exige `Effect::GitRead`, TaskLease réelle, session, epoch, capacités, dépôt exact, exécutable `FixedTool`, ressources et durée de vie monotones suffisantes. **C'est un garde-fou de politique INACTIF, pas une autorisation de spawn.**
- #364 : `TaskLeaseRegistry::get_active_with_revocation_watch`, lecture atomique du bail vérifié et témoin de révocation de **sa génération d'origine**, avec identité binaire contrôlée. Le témoin ne peut pas être substitué par un nouveau bail avec le même identifiant.
- #347/#349/#350 : classification tardive du timeout, provenance des erreurs Win32 Job, ordre d'arrêt des jobs déjà enregistrés. Ce n'est pas la terminaison complète.

**Chemin réel de production PAS ENCORE MIGRÉ** : dans `crates/optic-bridge-mcp/src/git_tools.rs`, `authorize_git_read` fabrique toujours une `ActionEnvelope` avec `task_lease: None`. Dans `crates/optic-bridge-runtime/src/git/read.rs`, `GitReadService` lance encore `HardenedCommandRunner`. Dans `hardened_command.rs`, les `join()` de lecture stdout/stderr et écriture stdin peuvent dépasser le deadline lorsqu'un descendant garde les handles ouverts. N'ajoute pas brutalement de TaskLease obligatoire au chemin historique sans adapter complet et compatibilité examinée.

**Recherche expérimentale non fusionnée dans main** :
- #353 crée un `PreparedReadCommand` Windows : Job, pipes, watchdog, admission/quotas avant spawn ; fixe `git status` seulement ; child suspendu, assignation Job avant reprise ; stop/cancel et receipt terminal positif (root + Job vide + vrais EOF), conservation des ressources sous incertitude.
- #358 vérifie racine Git canonique/TaskLease/exécutable/epoch/deadline ; après #366, lie le propriétaire natif au `TaskLeaseRevocationWatch` de la génération d'origine, avec comparaison d'identité du signal avant start.
- #366 a été fusionnée **DANS LA BRANCHE #358 UNIQUEMENT**, jamais dans main. Son test Windows `retired_lease_id_reuse_never_retargets_prepared_native_owner_stop` passe : révoquer → supprimer → créer un bail avec le même ID n'autorise pas l'ancien propriétaire à lancer le processus et n'efface pas sa réservation avant le receipt terminal. #358 HEAD actuel au checkpoint `f431c3efde2e5d9512eb1715ad3475196f18bf1d`; workflows intégrés verts **du premier essai** : [CI #40187179915](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179915) 4/4, [natif #40187179856](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179856) 2/2, [préparation #40187179916](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179916) SUCCESS.
- **Ne fusionne surtout pas en bloc #353/#358 dans main** : historiques divergents, code recherche additionnel, copie du helper #364 déjà intégré en production. Toute promotion exige réconciliation/cherry-pick ultra ciblé, contrats de sécurité, CI indépendantes.

## Les rouges indispensables à résoudre sans les masquer

1. **#352** — [CI spécialisée #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) volontairement **FAILED Windows+Ubuntu** : timeout du runner 200 ms, durée réelle ~1,6 s car descendants gardent stdout/stderr ou stdin ; borne d'acceptation du test <900 ms. Un résultat `TimedOut` obtenu après 1,6 s n'est PAS une correction. Ne monte pas les seuils, ne `detach` pas les pipes sans custody ni faux EOF.
2. **#355** — [CI spécialisée #40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203) **FAILED Windows+Ubuntu** : un vrai Git démarre APRES publication de la révocation pendant la fermeture des admissions. Il faut un verrou/propriétaire/contrôle de révocation pré-spawn fiable, puis le test ORIGINAL inchangé en vert. Les tests synthétiques verts de #358 ne remplacent pas #355.
3. **#185** — Node/AppContainer Windows intermittent distinct, y compris [main CI #40183165673](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40183165673) FAILED : `TimedOut`, ~10,1 s, pas de sortie Node ; des diagnostics ultérieurs verts n'annulent PAS le premier rouge. Ne désactive jamais AppContainer/Defender, n'augmente pas les timeout pour maquiller l'échec.
4. Les précédents `TerminationUncertain`, erreurs Win32 persistantes, crash hôte, récupération/guardian, retour terminal root+Job vide+EOF réels, confidentialité Git #176/#182, premier retour MCP #319 et Windows 10/11 + installateur hors ligne/update/rollback #323 restent des garde-fous indépendants.

## Première tranche technique attendue

Priorité : **une petite connexion testable de l'autorité MCP réelle au propriétaire natif** pour un `git status` strict, et non un grand portage. Définir le contrat : session réellement active/admission, `ActionEnvelope` typée et politique/sensitive-read validées, `TaskLease` active en provenance du bon registre et du bon epoch, racine Git canonique, exécutable Git ouvert et pin hash contrôlé, limites et watchdog bornés, témoin de révocation de la même génération, Job/owner enregistré AVANT toute création native, revalidation à la dernière frontière avant spawn. Toute erreur : fail-closed, pas d'élévation de droits, pas de nouvelle permission ambiante, pas de libération de quotas sous `TerminationUncertain`. Si le chemin n'est pas assez clair, commence par un test/contrat négatif ciblé et une PR minuscule ; n'active pas le lanceur natif prématurément.

Valide ensuite les critères originaux #355 (pas de spawn Git après revoke) et #352 (fin réelle stdout/stderr/stdin), sans changer leurs assertions ni transformer un rouge en vert par un rerun cosmétique. Tous les tests CI doivent conserver Windows + Ubuntu quand applicables et les preuves natives Windows réelles.

## Mode de travail et Codex

- **Tu pilotes le projet de façon autonome** : identifie, implémente, publie, review et valide les petits correctifs sûrs. Si un chantier est long, arrête à un checkpoint cohérent et exploitable, pas à une prétendue tâche en arrière-plan. Je veux savoir quoi faire seulement quand mon intervention est nécessaire.
- GitHub peut être utilisé directement si le connecteur est accessible ; vérifie les actions disponibles avant d'affirmer ne pas pouvoir agir. Vérifie les SHA exacts, les PR concurrentes, le status de `main`, les tests de premier passage. Utilise des branches isolées, une PR par correction cohérente, validation d'abord rouge ciblé puis correction puis CI du HEAD exact puis CI **indépendante** du merge de main ; pas de force-push sans nécessité validée, jamais d'effacement d'un autre agent.
- **Codex sur mon PC** peut être utile pour les tests Windows 10 natifs physiques, mais tu n'as **aucune autorisation permanente** d'utiliser mon PC. Lorsque nécessaire, **donne-moi un prompt clair à transmettre à Codex** et précise les preuves attendues (SHA, logs, scénarios, zip/snapshots/checksums). Demande une autorisation fraîche et limitée avant accès machine ; ne suppose pas que les résultats Codex existent tant que je ne les ai pas fournis ou que tu ne les as pas consultés avec permission. Respecte le budget/quota Codex, avec arrêt prudent vers 10–5 % et checkpoint/reprise documentée.
- **Respecte** `docs/engineering/CODEX_INTERRUPTION_RECOVERY.md` : découverte non destructive, vérification Git et empreintes SHA-256, pas d'écrasement d'un worktree/snapshot, restauration isolée, ne jamais contourner les blocages de cybersécurité. Tests sur runner GitHub != preuves du PC utilisateur.
- Architecture robuste et maintenable : Rust, limites strictes, séparations transport/core/runtime/Windows, `Effect` + `ActionEnvelope`, pas de shell libre, pas de kill PID arbitraire, pas d'auto-autorisation par IA, pas de dépendance Electron/Node requise pour le cœur, Job Objects != sandbox complète, `unsafe` Win32 seulement dans crate Windows. Préserve les fonctionnalités et contrats existants ; pas de nouveau périmètre produit tant que V1 NO-GO.
- Les descriptions PR/commits ne doivent pas employer les formules de fermeture automatique d'issues qui pourraient fermer #317 ou #323. Signale les limites avec précision, ne cache pas un échec #185 distinct. Une CI verte de recherche n'autorise ni fusion automatique en production ni release V1.
- À la fin de chaque tranche : mets à jour la doc source appropriée (`STATUS.md`, `ROADMAP.md`, handoff et issues), les SHA et les liens CI exacts ; fournis un récapitulatif clair de ce qui a été modifié, des validations effectives, des rouges préservés, du statut V1 et de la prochaine tranche. Évite de perdre du temps sur une grosse chaîne d'opérations sans checkpoint.

**Commence maintenant par la lecture des docs actuelles, la vérification de `main`, des 5 PR et des CI ; puis poursuis directement une petite tranche de stabilisation de #317 conforme à ces règles, sans me faire répéter toutes ces instructions.**

**Interdictions finales :** ne travaille pas sur Optic Desktop / REL, ne touche pas à mon ordinateur sans mon accord explicite, ne contourne aucun dispositif de protection et ne publie aucune V1 tant que #317/#323 n'ont pas leurs preuves de réussite.
