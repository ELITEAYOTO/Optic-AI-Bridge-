# OPTIC AI BRIDGE — PASSATION WINDOWS V1 POUR CHAT CLASSIQUE

## 27 octobre — instantané de contrôle TaskLease original, PR #390 draft

Après [PR #389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/389), le `main=2702e8f9de9982a7654bb09559e18865b8ebe763` est validé par [CI indépendante #40516482577](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40516482577), 4/4 tentative 1. La nouvelle [PR draft #390](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/390) ajoute une observation non exécutante : relecture de la politique TaskLease/empreinte exécutable et comparaison du témoin de génération de l'admission initiale, refus d'un autre registre avec le même ID et d'un registre session étranger. Toutes les ressources originelles restent détenues avec le Job Windows vide.

Test-first [CI #40517792327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40517792327) ROUGE pour API manquante (E0433/E0599), plus rustfmt indépendant ; pas une reproduction physique #352/#355. Contrôler le HEAD final et ses 4 jobs avant toute fusion, puis indépendamment le nouveau push/main. Ce snapshot N'EST PAS un verrou atomique CreateProcess. Aucun processus Git ni MCP switch. #317/#323 OPEN, V1 NO-GO ; ne touche ni aux autres dépôts ni au PC.

**27 octobre 2026 — GitHub uniquement — Windows V1 NO-GO**

> Document actif pour la reprise demandée en **Chat classique**. Copier le
> [prompt complet](NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md).
> Les passations des 22–26 octobre restent des historiques techniques ; leurs
> SHA/CI ne décrivent pas automatiquement le main actuel. Les invariants et
> méthodes restent applicables. Cette mise à jour documentaire fera elle-même
> avancer main : toujours vérifier les références en direct.

## 27 octobre — #388 fusionnée et validée sur main

**Confirmed integration after PR #388 (27 October 2026):** `main=720a38833705e661049841cfd39852e770687643`. PR HEAD `0b29e833a53195f199b5d3b53f808091d020bdc5` passed [CI #40513121389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40513121389), first attempt, four jobs individually SUCCESS and all three native-specialized workflows successful. The reviewed HEAD and squash tree are identical (`1d79045f8ac60db9dd2dc2ffe6e23b72af0811f9`). Independent [push/main #40514100157](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40514100157) on that exact integrated SHA completed SUCCESS, attempt 1, all four Windows/Ubuntu/dependencies/release jobs successful.

**Scope remains pre-spawn only:** original Git executable and repository root opened and pinned by verified Windows handles, with the already counted admissions, TaskLease witness, unstarted Job and stdio. No child, final synchronized `CreateProcess` authority, native terminal/EOF proof, MCP GitRead switch, physical Windows 10/11 acceptance or release. Keep #317/#323 OPEN, #352/#355 original acceptance RED, and Windows V1 NO-GO. This documentary commit itself changes the HEAD; revalidate live `main`/`push/main` separately after any merge.

## 27 octobre — ciblage GitRead par handles épinglés (PR #388 candidate)

La [PR #388](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/388) complète **sans lancement** le Job Windows précréé de #387 : ouverture du binaire Git original et du répertoire canonique avec des handles conservés, comparaison des chemins finaux retournés par Windows, aucun argv arbitraire. La garde GitRead demande une vraie lecture de répertoire pour empêcher son renommage (les autres usages du pin générique restent inchangés). Les permis session/transport et quotas restent détenus.

Rouges préservés : [#40510524394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40510524394), Windows 371 réussis/2 échecs de garde, Ubuntu rustfmt ; [#40511257834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40511257834), Windows 372 réussis/1 échec de renommage de racine. Correctif code `382857c0269b7173c24bef8eb948d08ad0e558ae`, [CI #40512075888](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075888) success 4/4 tentative 1, trois CI natives vertes. Vérifier la CI du HEAD final documentaire puis, après fusion éventuelle, CI push/main indépendante.

NON prouvé : hash Git réévalué à la frontière CreateProcess, décision synchronisée session/policy/lease/révocation, création/assignation root, terminaison Job et EOF après un vrai enfant, crash-recovery, route MCP. #352/#355 RED, #317/#323 OPEN, Windows V1 NO-GO, aucune release.

## 27 octobre — reprise technique après précréation du Job natif (#387)

La [PR #387](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/387)
contient une tranche **non exécutante** : Job Win32 véritable précréé avec
limites CPU/mémoire/processus, pipes stdout/stderr ouverts et stdin `NUL`,
le tout attaché aux réservations d'origine session/transport/TaskLease et
au registre des processus. Aucun Git, aucune création de processus ou
raccord MCP. Tests Windows : Job réellement vide, pipes non EOF tant que les
writers parent sont ouverts, révocation et maintien des admissions.
Vérifier le HEAD, ses quatre jobs CI et les workflows natifs avant fusion ;
après fusion revérifier séparément `push/main`.

P0 suivant : pin de l'identité exécutable/racine, synchronisation au
dernier contrôle avant CreateProcess, détention Job/root/pipes/quotas et
vraies preuves root-terminé + Job-vide + EOF. Les originaux #352/#355
restent RED. #317/#323 OPEN ; Windows V1 NO-GO ; aucun release.


## ADDENDUM — après l'intégration #385 (27 octobre 2026)

Le `main` effectivement vérifié après [PR #385 fusionnée](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/385)
est `e5017c48de5d4d6433f92e3117d63f1677aecb59`. Sa CI indépendante
[push/main #40503510236](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40503510236)
est **SUCCESS 4/4**, tentative 1, sur ce SHA exact. Le code retenu effectue
une **réservation comptable non exécutante** de processus, CPU, mémoire et
sorties pour `PreparedNativeGitStatus`, sous les permis originaux de la
session/du transport et le témoin de la TaskLease. Il n'ouvre PAS de véritable
Job Win32, ne crée ni processus ni pipes, ne fournit ni EOF ni preuve de
terminaison, ne raccorde PAS le MCP `git_status`. Les recherches #353/#358
restent isolées. Les acceptances originales #352 et #355 restent rouges.

**Incident :** le squash #385 a automatiquement clos #317, qui a été
[explicitement rouverte](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6441832785)
et revérifiée ; éviter à l'avenir les intitulés `fix(#317)` tant que les
critères ne sont pas prouvés. #317/#323 OPEN et Windows V1 NO-GO.
Priorité : propriétaire natif Windows réel avec Job/root/pipes enregistrés
et autorisation synchronisée de lancement, puis preuve Job vide/root/EOF et
rebranchement MCP avec filtres sensibles inchangés. Aucun accès PC/Work/Codex,
aucune release. Le présent addendum est aussi une PR documentaire et son
propre nouveau `main`/CI indépendant doit être revérifié après fusion.

## 1. Périmètre et mode de collaboration

Dépôt unique autorisé : **https://github.com/ELITEAYOTO/Optic-AI-Bridge-**.
Bridge MCP local en Rust, priorité Windows 10/11 V1. Ne toucher ni à
`Alpha-Optic-APP` (Desktop, autre IA), ni à REL, ni à leurs branches,
fichiers, PR ou worktrees. Stabilisation, pas de nouvelles fonctionnalités.

La reprise se fait dans un **échange Chat classique**, avec les outils GitHub
effectivement disponibles. Ne créer ni tâche Codex/Work, ni agent local, ni
automation ou travail en arrière-plan sans demande explicite. Le rôle technique
et les décisions courantes GitHub restent délégués ; ce mode de conversation
n'accorde aucune autorité de lancement local.

**Aucun accès à l'ordinateur de l'opérateur**, même en lecture seule, aucune
commande locale, installation, exécution de test, serveur ou runner auto-hébergé
sans accord explicite, récent et limité. Les vérifications se font via GitHub et
CI hébergée. Une future intervention Codex locale ne peut être qu'un recours
séparément autorisé, avec prompt exact branche/SHA/scénarios/outils/preuves.

Si les outils GitHub requis ne sont pas disponibles dans le nouveau chat,
l'indiquer précisément et demander l'accès ou les contenus manquants. Ne pas
inventer de lecture, commit, PR, test, fusion ou activité invisible.

## 2. Baseline vérifiée avant cette PR documentaire

- `main=75947c9b4fb72fab2f3d45f34f5b750f09bc31b2`, après squash de
  [#383](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/383).
- [CI indépendante push/main #40477277322](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40477277322) :
  **completed / success, run_attempt=1, 4/4** : Rust Windows, Rust Ubuntu,
  Dependency policy, Release evidence guard, tous sur ce SHA exact.
- Arbre Git du squash : `932688cad29ddf90c816f1e61a45b29631ed2d6c`,
  identique à la tête validée #383
  `81fc85dbd36e2ae1081b241ef887b4f2440188ea`.
- Issues **#317, #323, #185 et #319 ouvertes** ; cinq PR de recherche ci-dessous.
- Le main vérifié ne contient encore **aucun raccord GitRead natif actif**.
  **Windows V1 NO-GO**, aucun tag/release ou clôture anticipée.

Après fusion de la présente documentation, récupérer le nouveau SHA de
`branches/main` et sa propre CI `event=push / branch=main`. Ne pas recopier
la baseline ci-dessus comme si elle était éternellement actuelle. Les reçus de
la PR documentaire et de sa CI indépendante sont conservés dans sa conversation.

## 3. Ce qui a été intégré le 27 octobre

| PR | Changement réel et limites | SHA intégré | CI indépendante push/main |
| --- | --- | --- | --- |
| [#380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/380) | Worker MCP à admission possédée, permis originaux transférables sans seconde admission ; contrôles sensibles Git conservés. Ne lance pas Git natif. | `ab45498c32b2cb9cdfddfdeaa32f0230447d9de0` | [#40467892414](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40467892414), 4/4, tentative 1 |
| [#381](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/381) | Capture de l'échéance absolue propre à ActionEnvelope dans le préflight ; un transport plus long ne prolonge plus l'action. Stop ne libère pas les réservations. | `588f50f0a3fee4e2f6b1c08c21db4950888d6d77` | [#40471474314](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40471474314), 4/4, tentative 1 |
| [#382](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/382) | GitReadService prépare son status fixe avec cibles configurées, budget exact, enveloppe immuable, préflight et permis originaux. Méthode non raccordée au handler actif, aucun processus. | `d9d004b2bbd65c4f7cc520abfde3beb7963f0095` | [#40474172602](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40474172602), 4/4, tentative 1 |
| [#383](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/383) | NativeProcessReservation ne synthétise plus la fermeture stdout/stderr sous TerminationUncertain ; observations et capacités conservées. Correction d'un producteur privé existant, pas de preuve OS. | `75947c9b4fb72fab2f3d45f34f5b750f09bc31b2` | [#40477277322](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40477277322), 4/4, tentative 1 |

Têtes finales et preuves candidate, distinctes des fusions :

- #380 : `ad22863edf616b069684baedef0bc74afc7471cb`,
  [#40467381485](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40467381485) 4/4, tentative 1.
- #381 : `28fe4e9f6ae4a1add6421368348f723ec1f0f2f1`,
  [#40470907663](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40470907663) 4/4 et
  [préparation native #40470907712](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40470907712), tentative 1.
- #382 : `26672d702cadb018c35d4d01bdb5bc1058b4280a`,
  [#40473467587](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40473467587) 4/4 et
  [préparation native #40473467600](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40473467600), tentative 1.
- #383 : `81fc85dbd36e2ae1081b241ef887b4f2440188ea`,
  [#40476637258](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40476637258) 4/4 et
  [préparation native #40476637275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40476637275), tentative 1.

Les prérequis #369/#371/#373/#375/#377, ainsi que scopes, politique, identité
épinglée et témoin de génération, sont décrits dans la
[passation du 26 octobre](HANDOFF_WINDOWS_V1_2026-10-26.md) et le
[contrat GitRead natif](NATIVE_GIT_READ_WITNESS_CONTRACT.md). Ce sont des
préconditions, jamais des permissions de spawn indépendantes.

## 4. Preuves rouges à conserver et qualifier exactement

- #380 : [format #40466491881](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466491881),
  puis [compiler #40466543183](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466543183),
  E0599 x3 pour interface absente. **Pas un échec runtime.**
- #381 : [test-first #40470425246](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40470425246)
  sur `ff2ad7d472663a182e2b241d2afc8345adfc6d70` : format/Clippy passés,
  **runtime rouge** à l'échéance de 100 ms de l'action (admission 10 ms,
  stop attendu 110 ms), Ubuntu 289 pass / 1 fail, Windows 361 pass /
  1 fail / 11 ignored. Contrat inerte, aucun processus natif exécuté.
- #382 : [format #40472675779](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40472675779),
  [compiler #40472797939](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40472797939)
  E0432 + E0599 x4, puis
  [format implémentation #40473274272](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40473274272)
  (Windows annulé lors du commit suivant). Les quatre nouveaux tests utilisent
  des répertoires synthétiques et des octets épinglés **inertes**, sans exécution
  de Git ni validation d'un vrai dépôt par le constructeur.
- #383 : [test-first #40476042507](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40476042507)
  sur `7da5c15299ca012ffcb216771da2121298c3e833` :
  **runtime Windows rouge**, 366 pass / 1 fail / 11 ignored, assertion EOF
  non observé après incertitude. Format/Clippy passés. Transition **simulée**
  dans le véritable registre, sans enfant ni injection de faute Win32.
  Après correction, 367 tests runtime Windows passés / 11 ignored.
  Ubuntu passe mais n'exécute pas cette régression cfg(windows).

Ne pas transformer une erreur de format ou de compilation en reproduction
comportementale. Ne pas transformer un état simulé, une préparation ou un
résultat CI standard en preuve de Job vide, d'EOF physique ou de récupération.
Aucun rerun destiné à masquer ces rouges n'a été demandé dans ces tranches.

## 5. Cinq recherches ouvertes, à conserver séparées

| PR | Base | HEAD vérifié avant cette documentation | État |
| --- | --- | --- | --- |
| [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) | `bootstrap/issue-317-windows-rust-owner` | `ef3208d787baa0c6433abfa0b4aba687d8bc6bb7` | ouverte, non-draft, recherche historique |
| [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) | `main` | `2fa45d6b2d1f3dd39309ca7d18a2209216b7f959` | draft, acceptance rouge |
| [#353](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/353) | `main` | `b091dc280bfd186694e7c1d1bea743a20f6746c2` | draft, propriétaire natif expérimental |
| [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) | `main` | `5c104ee67f26b8d7b68132cf05a5f86eb53b580f` | draft, acceptance rouge |
| [#358](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/358) | `research/317-prepared-read-native-staging-2026-10-24` (#353) | `f431c3efde2e5d9512eb1715ad3475196f18bf1d` | draft, empilée sur #353 |

Ne jamais fusionner #353/#358 en bloc dans main. #366 n'a été intégré que dans
la recherche #358. Historiques divergents et protections déjà portées : chaque
portage doit être ciblé, débarrassé des doublons et revu sur le main réel.

## 6. Chemin de production exact et limites toujours présentes

- `crates/optic-bridge-mcp/src/server.rs` :
  `run_owned_admitted_read` possède une vraie admission comptée et le
  `RequestPermit` original dans le worker. L'interface empruntée délègue à
  ce même worker sans seconde admission.
- `crates/optic-bridge-mcp/src/git_tools.rs` :
  les lectures Git passent désormais par le worker possédé et gardent les
  contrôles sensibles, **mais authorize_git_read construit toujours
  task_lease: None** pour le chemin legacy.
- `crates/optic-bridge-runtime/src/git/read.rs` :
  `GitReadService` exécute toujours `HardenedCommandRunner`.
- `crates/optic-bridge-runtime/src/git/native_status.rs` :
  `prepare_native_status` crée un `PreparedNativeGitStatus` non clonable,
  avec commande fixe, enveloppe immuable et préflight admis. Cette méthode
  **n'est pas appelée par le handler MCP actif**. Elle ne réserve ni quota
  natif CPU/mémoire/processus, ni Job/root/pipes ; elle ne lance rien.
- `native_git_read_preflight.rs` :
  conserve les générations originales et l'échéance propre à l'action ;
  `deadline()` donne la première échéance absolue capturée. Un stop watch
  faux n'est jamais une autorisation de création.
- `process.rs` / `process/tests.rs` :
  #383 corrige la publication d'EOF par `NativeProcessReservation`
  sous incertitude ; les capacités sont retenues. Ce mécanisme n'est pas
  devenu pour autant le propriétaire Git natif complet.

Les joins stdin/stdout/stderr du runner legacy restent le problème #317.
Ne pas imposer brutalement une TaskLease au legacy sans migration complète et
conservation des autorisations existantes et de la confidentialité.

## 7. Prochaine étape technique attendue

**Un véritable propriétaire Windows enregistré dans le service**, prenant en
charge le `PreparedNativeGitStatus` exact de #382 avec les mêmes permis
worker/session/transport et la TaskLease canonique, sans réadmission.

Inspecter `process.rs` (`reserve_native_private`, capacités, Drop, finish),
`private_node_task.rs`, les registres session/lease, et les primitives
Windows `job.rs` / `output_pipe.rs`, en les comparant aux propriétaires
expérimentaux #353/#358. Ne pas réutiliser aveuglément une réservation privée
ou importer tout un historique de recherche.

L'owner doit enregistrer ses ressources et réserver les budgets globaux et
par session CPU/mémoire/processus/sortie avant toute création native. Fixer et
contrôler l'identité ouverte/épinglée/hashée de Git et la racine canonique.
Réévaluer politique/epoch/session/lease/révocation/échéances à la dernière
frontière de lancement **synchronisée**. Un check puis spawn séparés ne
résolvent pas la course de révocation.

Sous `TerminationUncertain`, garder owner, handles, réservations, permis,
lease et éléments de récupération. Ne libérer qu'avec les preuves adaptées :
racine terminée, Job vide, véritables EOF stdin/stdout/stderr selon le modèle
de flux. Couvrir descendants, abandon du demandeur, watchdog, crash hôte et
récupération. Pas de lecteurs détachés, faux EOF ou nettoyage supposé.

Une étape non exécutante reste justifiée si un verrou de sécurité manque,
mais chaque tranche doit éliminer un obstacle concret au raccord ; ne pas
accumuler indéfiniment des helpers sans plan de consommation par l'owner.

## 8. Acceptances toujours rouges et gates indépendants

- **#352** : [#40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209)
  failure, tentative 1, sur sa tête exacte : runner 200 ms, durée totale
  environ 1,6 s pour borne <900 ms. `TimedOut` tardif n'est pas un succès.
  Garder le test et ses seuils inchangés.
- **#355** : [#40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203)
  failure, tentative 1, sur sa tête exacte : vrai Git lancé après publication
  de révocation pendant drainage. Exiger l'owner enregistré avant création
  et une autorité de création synchronisée, pas un témoin isolé.
- **#185** : intermittence Node/AppContainer Windows, indépendante.
- **#319** : première réponse MCP autour de 30 secondes, cause non démontrée.
- **#176/#182** : confidentialité Git/contenus sensibles/provenance et
  approbations restent des exigences distinctes.
- **#323** : preuves Windows 10/11 réellement autorisées, versions/builds,
  installation hors ligne, mise à jour, rollback, incidents, fiabilité et
  provenance release. CI Windows Server hébergée seule insuffisante.

#317 et #323 restent ouvertes tant que leurs exigences ne sont pas entièrement
satisfaites. Aucun tag, release, GO V1, fermeture cosmétique ou protection
affaiblie pour rendre une CI verte.

## 9. Méthode GitHub obligatoire dans le nouveau Chat

1. Lire main courant : AGENTS, AI_HANDOFF, STATUS, ROADMAP, CHANGELOG,
   PROJECT_CONTINUITY_PROMPT, ce handoff et son prompt, GITHUB_HANDOFF,
   NATIVE_GIT_READ_WITNESS_CONTRACT, SECURITY_INVARIANTS,
   REPOSITORY_WORKFLOW et CODEX_INTERRUPTION_RECOVERY.
2. Requêter branches/main, dernière CI push/main sur ce SHA, tentative et
   chacun des quatre jobs ; inventorier PR ouvertes avec bases/HEAD/draft,
   issues et nouveaux commits/travaux concurrents. Ne pas réutiliser un
   ancien vert de PR/recherche comme preuve intégrée.
3. Branche isolée depuis le SHA vérifié, une correction cohérente à la fois ;
   aucun force-push/réécriture des travaux d'autrui, aucun nettoyage large.
4. Test-first pertinent, rouge comportemental lorsque possible. Qualifier
   format/compilation/runtime/simulation/natif séparément. Pour une simple
   passation documentaire, ne pas fabriquer un test rouge artificiel.
5. Format, Clippy warnings refusés, Windows/Ubuntu, dépendances, garde release
   et validations natives applicables. Conserver les premiers rouges.
   Corriger sur nouveau commit, pas de rerun cosmétique.
6. Draft jusqu'aux validations de la tête exacte ; revoir diff, fichiers,
   base, HEAD, invariants et retours de revue. Fusion squash avec
   `expected_head_sha` ; si ce contrôle est indisponible, ne pas prétendre
   l'avoir appliqué et ne pas fusionner sans alternative sûre vérifiée.
7. Après fusion : récupérer le nouveau SHA de main et attendre sa CI
   indépendante push/main, tentative et résultat réel des jobs.
8. Tracer dans les issues et documents propriétaires les changements,
   commits, CI exactes, premiers échecs et limites. Pas d'auto-close #317/#323.
9. Au checkpoint, bilan clair : fait/non fait, PR/commits/CI, main, rouges,
   NO-GO et prochaine priorité. Aucun travail invisible promis hors du chat.

## 10. Récupération et sécurité permanentes

Respecter [CODEX_INTERRUPTION_RECOVERY.md](CODEX_INTERRUPTION_RECOVERY.md)
sans en déduire une permission d'accès local. Dans ce travail GitHub-only :
**local recovery location inaccessible — not verified**.
Ne pas prétendre avoir lu STATE/ACTIONS/TESTS/RECOVERY, CURRENT.json,
VERIFIED.json, checksums, remote/HEAD ou snapshots d'un disque inaccessible.
Une intervention locale ultérieurement autorisée impose découverte et
vérification réelle, snapshots hors worktrees, aucun reset/écrasement de travail
non enregistré, et checkpoint avant épuisement de quota.

L'IA propose, l'autorité applicative et la politique déterministe autorisent,
l'isolation contient. Identités serveur, scopes structurés, capacités
révocables, délais monotones, chemins canoniques/reparse/symlinks, quotas
bornés, environnement allowlist et couches unsafe Win32 dédiées restent
obligatoires. Pas d'auto-autorisation IA, shell libre, arrêt de PID arbitraire,
secret ambiant, désactivation AppContainer/Defender ou contournement d'un refus.

**Windows V1 NO-GO. #317 et #323 ouvertes.**
