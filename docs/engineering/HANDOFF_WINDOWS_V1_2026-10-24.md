# Optic AI Bridge — Passation canonique Windows V1 (24 octobre 2026)

## ACTUALISATION ULTÉRIEURE — 24 octobre 2026, après récupération Codex et preuves Windows 10

> **État de continuation prioritaire.** Cette section prime la précédente « ACTUALISATION CANONIQUE — après dix scénarios » et les nombreuses entrées historiques. Les anciens SHA restent des preuves, pas un état courant ni une instruction de reset. Vérifier GitHub au nouveau chat.

### Références GitHub et séparation des dépôts

- Dépôt **unique** : [ELITEAYOTO/Optic-AI-Bridge-](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/), socle Rust/MCP. **Jamais Optic Desktop ni REL**.
- `main=dce94cc02f178d91b1eec4dc06134ab57df47ff6` à la fusion documentaire de [#343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/343). Son [push/main CI #40075855681](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40075855681) doit être contrôlé à l'état final sur ce SHA ; la [PR-head CI #40075523748](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40075523748) avait les 4 jobs verts, attempt 1 au SHA `4a3a0787`. Après cette passation, `main` peut encore avancer : toujours re-query.
- **[#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) et [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN, Windows V1 NO-GO.** Recherche propriétaire natif [PR #327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) ouverte, indépendante, ne pas fusionner en bloc.
- **Ancien red de production à préserver :** `main=cf5a6c3f5aff7d9846106effafd52027126e3ba3`, [CI #40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) Windows rouge : `timeout_terminates_owned_process` obtient `TerminationUncertain` au lieu de `TimedOut`. Une série de succès sur branche de recherche ne corrige pas une telle preuve ni ne démontre sa cause.

### Recherches prouvées mais non intégrées

| PR | Contenu et preuve | Conclusion |
| --- | --- | --- |
| [#339](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/339) | Descendant stdout/stderr après root ; [CI #40059685148](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059685148) et #40059681412, SHA `dc8cd3db` | Deadline root-only et readers joins après root : **bug caractérisé**, pas corrigé |
| [#340](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/340) | Win32 confirmation positive normale, `kill_request=Ok(())`, `confirmed=true`, `confirmation_ms=1`; [CI #40062376385](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40062376385) et #40062376329 | **Pas** de reproduction naturelle de `TerminationUncertain` |
| [#341](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/341) | Descendant stdin/EOF, writer `join()` tardif ; [CI #40063153249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40063153249) et #40063153233, SHA `7c1bc62f` | **Bug caractérisé**, pas corrigé |
| [#342](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/342) | **PC Windows 10 19045.7807** : 6 tests synthétiques supervisés, 4 confirmations à 1 ms, outer Job vide avant cleanup ; [CI native #40064790347](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40064790347) et [générale #40064793614](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40064793614) vertes au SHA final `505a4ef8`, Windows 608 pass/15 ignored et Ubuntu 477 pass | **Chemin normal fonctionnel dans cet échantillon**, cause du red inconnu |

Toutes ces PR #339–#342 ont été fermées **sans fusion** et restent consultables. Les tests #339/#341 réussissent parce qu'ils constatent intentionnellement une violation de délai. Ne jamais les promouvoir comme critère de correction ni réutiliser une branche entière.

### Récupération après interruption Codex : règle de projet

[PR documentaire #343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/343) intègre le guide [CODEX_INTERRUPTION_RECOVERY.md](CODEX_INTERRUPTION_RECOVERY.md) et sa référence dans `AGENTS.md`. Avant toute action, le nouvel agent lit les règles puis réalise, **si la machine est accessible et autorisée**, la découverte en lecture seule d'un checkpoint externe aux worktrees, sa validation SHA-256 et sa confrontation à Git. La procédure n'exécute aucun daemon et ne contourne aucune protection de cybersécurité. Une session purement GitHub doit noter « local disk inaccessible / not verified » et ne jamais inventer un état local.

L'opérateur a rapporté une récupération locale sous `E:\Optic-IA-Bridge\codex-recovery`, branche historique `codex/317-native-wait-phases` HEAD `c3c5e655…`, quatre commits, snapshot `0010-final-verified`, et plus tard checkpoint `0013-policy-published-ci` à **55 empreintes concordantes**. Il s'agit de **preuves déclarées par Codex, non d'une récupération vérifiée par ce chat**. Six diagnostics et deux tests runtime localement réussis ; Win32 24 / HRESULT 0x80070018 natif dans un scénario rejeté ; root pending / Job non vide / erreur root 6 simulés. Le travail n'est pas une réparation de production, et aucune publication de la branche locale #317 n'est supposée.

### Décisions et chantier suivant

- La production conserve `OwnedSessionAdmissionPermit` mais GitRead utilise encore `HardenedCommandRunner` sans guardian natif complet. Le Job comptable (`JobStore`) **n'est pas** un Job Object Windows.
- Construire la prochaine **petite preuve** sur les quatre phases (`root waiting`, `root exited / Job nonempty`, `QueryInformationJobObject` error Win32 préservée, `root + Job empty`), sans prétendre expliquer l'intermittence si elle n'apparaît pas. Un cas négatif déterministe, borné et supervisé, et vérification native du propriétaire/quotas/admissions même lorsque l'observation est durablement indisponible.
- Le signal d'arrêt doit être publiable avant le **drainage potentiellement bloquant** des admissions ; la confirmation Job-tree/EOF et la publication terminale sont distinctes. Pas de libération prématurée de permis de session ou quota, et ne jamais nettoyer une mauvaise génération. Readers stdout/stderr et writer stdin doivent être contrôlés et annulables avec preuves de terminaison, pas `join()` non bornés ni augmentation de délais.
- Séparer ultérieurement sécurité GitRead/provenance (#176/#182), intermittent AppContainer/Node (#185), premier message MCP (#319), reprise après disparition du service, tests réels Windows 10/11 et matrice release/rollback (#323).
- Respecter `#![forbid(unsafe_code)]` runtime, Win32 unsafe seulement dans `optic-bridge-windows`, typed Effect/ActionEnvelope, TransportGuard, leases et scopes, délais monotones, buffers bornés, fail-closed et CI Windows/Ubuntu sur SHA exact. Pas de tests user PC sans **autorisation explicite par mission**, ni contournement de blocage Codex, ni fusion de recherche en bloc. Conserver les premiers rouges et les logs de tests vraiment exécutés.

**Prompt opérationnel complet :** [NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md) (sa section « REPRISE PRIORITAIRE » précède désormais les dix scénarios).

---


## ACTUALISATION CANONIQUE — 24 octobre 2026, après dix scénarios de crash supervisés

> **Cet encadré est le point de départ actif.** Les tableaux et SHA des sections historiques ci-dessous, qui indiquent encore `100c43b5` et `8202c9c`, décrivent un état **antérieur**, non le dernier checkpoint. Ce document n'est pas un ordre de reset/merge. Vérifier GitHub à chaque nouveau chat et après la présente PR documentaire.

### Production GitHub et garde-fous, vérifiés avant la PR de passation

- **Dépôt unique :** [ELITEAYOTO/Optic-AI-Bridge-](https://github.com/ELITEAYOTO/Optic-AI-Bridge-) ; **ne pas confondre** avec les dépôts séparés `Optic Desktop` (UI/Figma/installation/update) ou `REL` (reverse engineering), suivis par d'autres agents.
- `main = 16ca67a75fd49c5681a0672773ffc268c3635f6b`, [CI push/main #40047950688](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40047950688) **success sur ce SHA**. Dernier changement intégré depuis cette passation initiale : documentation #337 ; la primitive de production #336 `OwnedSessionAdmissionPermit` est bien présente, mais **GitRead utilise encore `HardenedCommandRunner`** et ne possède pas le guardian natif expérimental.
- Une seule PR ouverte : [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327), recherche indépendante à préserver, **ne pas intégrer en bloc**. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) et [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) toujours ouvertes ; Windows V1 **NO-GO**.
- Conserver séparées les préoccupations #176 (lectures sensibles), #182 (provenance/autorisation Git), #185 (Windows AppContainer/Node) et #319 (première réponse MCP). Ne pas annoncer de fermeture ou de résolution sans vérifier l'état réel de chacune.
- **GitHub-only** : CI Windows/Ubuntu sur runners hébergés ; aucun test, binaire, processus, modification de configuration ou installation sur le PC de l'opérateur sans **nouvelle autorisation explicite**. Codex a rencontré des contrôles de cybersécurité sur la campagne d'essais locaux : **ne pas tenter de contourner ces contrôles par des prompts alternatifs** ; la recherche est continuée sur GitHub. Si un test autorisé est réellement bloqué, suivre la procédure officielle d'accès, ou rester dans les CI hébergées.

### Preuves expérimentales ultérieures du 24 octobre (toutes HORS production)

Les travaux sont conservés sur **branches de recherche distinctes** ; les comptages ne doivent pas être confondus avec des tests d'acceptation Windows 10/11 physiques. Chaque ligne ci-dessous donne un **SHA de code** et la CI qui a réellement exécuté ce code. Les commits Markdown ultérieurs n'établissent aucune correction supplémentaire.

| Recherche / source | Preuve GitHub sur SHA exact | Portée exacte |
| --- | --- | --- |
| `research/317-repeated-native-observation-2026-10-24` — `85e6f280` puis `47b6a631` | [#40049514408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40049514408), [#40050805563](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40050805563) verts | Six scénarios de Job/comptabilité incertaine, puis transfert réussi des handles d'un thread lanceur vers guardian ; les erreurs de requête sont **injectées** et les crashs atomiques non prouvés |
| `research/317-crash-before-native-ack-2026-10-24` — `3df5ebfd` | [#40053510714](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40053510714) vert ; document `9d6dedf` [#40053843692](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40053843692) vert | Six scénarios sous **superviseur extérieur** : trois `PendingNoNativeReceipt`, deux `GuardianJobEmptyProven`, un contrôle volontaire `FirstRedGuardianLost`. Pending **ne signifie pas récupération automatique** |
| `research/317-guardian-preowns-job-2026-10-24` — `949126a2` | [#40054595150](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40054595150) vert | Guardian en processus unique prépossède le vrai Job, garde admission/quota si le thread créateur sort sans ACK ; pas de crash entre processus |
| `research/317-service-preowned-native-job-2026-10-24` — `49c39d03` | [#40055108927](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055108927) vert | **Deux processus Windows distincts**. Le service prépossède et conserve Job + admission ; lanceur sort avant/après création d'enfant suspendu sans ACK. **Huit** scénarios supervisés |
| `research/317-limited-launcher-job-rights-2026-10-24` — `17ec42cc` | [#40055810146](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055810146) vert | Prêt du Job par `DuplicateHandle` avec seulement `ASSIGN_PROCESS (0x0001) | QUERY (0x0004)` ; vrai `TerminateJobObject` refusé `ERROR_ACCESS_DENIED` avant création ; enfant synthétique encore créable |
| `research/317-failed-job-delegation-2026-10-24` — `9122615d` | [#40057184478](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057184478) vert, document `3f426028` [#40057508929](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057508929) vert | **Neuf** scénarios ; réel `DuplicateHandle` vers destination invalide renvoie `ERROR_INVALID_PARAMETER (0x80070057)`, sans offre/ACK/enfant ; maintien du Job/quota jusqu'à preuve native positive |
| `research/317-failed-native-create-2026-10-24` — **`b5a91deea1b306c56a2c03de7bb5a86b699d1d6f`** | **[#40057931887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887)** : `completed/success`, **attempt 1**, Windows **355 passed/0 failed/13 ignored**, Ubuntu **270 passed/0 failed**, fmt et Clippy strict, **dix** scénarios supervisés et tests `--workspace` Windows réussis | Vrai `CreateProcessW` du **binaire libtest fixe** échoue pour cwd inexistant. Job du guardian reste vide ; quotas, permis et état `Running` retenus pendant erreur d'observation injectée ; `Failed` et restitution **après** vraie requête native Job vide. Le code d'erreur précis n'est pas enregistré dans le manifeste (seulement deux codes cwd manquant acceptés). |

Le rapport détaillé et son manifeste sont sur la branche **`research/317-failed-native-create-2026-10-24`**, doc SHA **`4bef362d1e9c45ec3b81a4d6807ae924f6122218`** :
[ISSUE317_CRASH_BEFORE_NATIVE_ACK_2026-10-24.md](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/research/317-failed-native-create-2026-10-24/docs/research/ISSUE317_CRASH_BEFORE_NATIVE_ACK_2026-10-24.md),
[artefact CI #12546595512](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887/artifacts/12546595512),
[rapport #317 #6379288501](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6379288501).
La **CI de ce commit purement documentaire** [#40058237376](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40058237376) était **en cours au contrôle préparant cette passation** : recontrôler son résultat plutôt que présumer qu'elle est verte.

Le premier échec [#40057892808](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057892808) (strict Rustfmt **avant** les tests) est conservé. Dans la campagne de duplication précédente, [#40056998380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40056998380) avait découvert le code d'erreur Win32 effectif ; l'assertion exacte a ensuite été corrigée. Ne jamais réécrire ces rouges comme des succès sur le même SHA.

### Ce qui n'est PAS prouvé ni livrable

Ces essais sont des **fixtures natives de recherche sur Windows Server 2025** et enfant libtest **suspendu, jamais repris**, sous superviseur extérieur. Ils ne sont **ni une correction de `HardenedCommandRunner` en production, ni un sandbox**, ni une preuve de récupération après crash du service/guardian. Aucune vraie erreur Win32 permanente, reprise durable des quotas après redémarrage, descendance active gardant stdout/stderr, stdin/EOF, shutdown gracieux borné, GitRead task lease/provenance ou acceptation Windows 10/11 physique n'est démontrée. `JobStore` reste un registre comptable, différent du Job Object natif. Une opération `finish` ne vérifie pas intrinsèquement le Job : le **propriétaire** doit fournir et ordonner la preuve réelle avant publication terminale. Ne pas promouvoir ces tests ni élargir l'autorisation.

### Ordre de travail conseillé au successeur

1. Vérifier **en direct** `main`, les PR, la CI #40058237376, les docs de cette passation, les issues #317/#323 et leur état (une nouvelle PR documentaire peut faire évoluer `main`).
2. Priorité démontrable #317 : **pipes stdout/stderr tenus par un descendant, arrêt/EOF de stdin, descendant hors sortie du root, stop avant drain, fermeture bornée et observabilité après erreur native persistante**, avec vrais handles, budgets bornés, fixtures purement synthétiques et supervisor externe. Ne pas multiplier à l'infini des variantes déjà validées de l'enfant suspendu ; rechercher un défaut d'architecture/fonctionnement démontrable.
3. Choisir un mécanisme durable de propriété des Jobs et permis, puis définir la réconciliation des `Pending` **sans libération sur timeout**. À défaut de preuve, garder les limites ouvertes et préparer une ADR/revue.
4. Extraire **un seul petit incrément de production** sur une branche neuve depuis `main` courant **uniquement si** le code et les tests couvrent une propriété indépendante. Fenêtres stop/revoke/JobStore, session génération, lease/repo/exécutable et liveness restent à auditer. Aucun merge global de recherche. PR courte, CI strictes Windows/Ubuntu exact-HEAD, revue, squash puis CI push/main du SHA résultant.
5. L'acceptation V1 #323 réclame encore #176/#182, #185/#319, Windows 10/11 autorisé, installer/offline/update/rollback/release. Les préoccupations UI/installateur d'**Optic Desktop** sont dans **un autre dépôt** : ne pas intervenir ici.

**Consignes non négociables :** `#![forbid(unsafe_code)]` runtime ; Win32 `unsafe` exclusivement dans `optic-bridge-windows` ; `ActionEnvelope+Effect` typé, `TransportGuard`, permissions explicites, sessions/leases et deadlines monotones ; aucun PID arbitraire, aucun Job traité comme sandbox, aucun privilège ou timeout élargi pour rendre un test vert, aucun abandon de handle ou restitution sans preuve. Préserver PR #327, protéger l'historique rouge, laisser #317/#323 ouvertes et **V1 NO-GO**.


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
