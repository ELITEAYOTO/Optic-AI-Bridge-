# Optic AI Bridge — prompt intégral de reprise après PR #458 (31 octobre 2026)

## Archive notice — 1 November 2026

**Prompt 31 octobre dépassé :** utiliser le [prompt du prochain chat](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-11-01.md) et la [passation canonique 2026-11-01](HANDOFF_WINDOWS_V1_2026-11-01.md). La PR #464 est intégrée ; `main=a22d32e94b742eaece9321d7991208ce89edd9cf` est le jalon **avant** la PR documentaire actuelle, non une autorité permanente. La V1 reste NO-GO.

---


## Historical / superseded checkpoint — Reprise active après PR #458, avant finalisation de cette PR documentaire

**Point vérifié avant cette PR documentaire :** `main=a9b6de0467fe2cdb0d9a74276e0ea0ec1fc40b8a`, arbre Git `6d4988c300740f3cc0aefec285b9284a83169366`. [PR #458](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/458) a mis à jour huit documents et été squash-fusionnée. [CI du HEAD #41228285220](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41228285220) **4/4** ; [CI indépendante post-fusion #41228767639](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41228767639) **4/4, tentative 1, source=checkout=squash**. Windows : **428 tests OK / 0 échec / 11 ignorés** ; Ubuntu **318 OK**. Rapports physiques schema8 `complete=true`, **10 cas Windows et 4 Unix**, même SHA/run/attempt, artefacts conservés. Reçus : [PR #458](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/458#issuecomment-6547380732) et [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6547381263). **Ce SHA est historique** : rechercher le nouveau `main` après fusion de la présente PR documentaire.

**État du code / NO-GO :** #455 a corrigé le rejet d'un délai original expiré **avant** `spawn` dans le vrai runner legacy. #456 évite de relire stdout/stderr **après** dépassement du budget partagé, sans pouvoir annuler un `read()` déjà bloqué. #457 force l'arrêt sur mutex empoisonné dans le registre **inerte** des appels natifs. Aucun de ces patchs ne borne universellement `spawn/wait/join`, ne connecte le GitRead natif au MCP, ni ne synchronise l'admission/révocation réelle de `CreateProcessW` / `ResumeThread`. Il faut un guardian d'autorité original et non détachable qui coordonne session/TaskLease génération/policy epoch/hôte/exécutable et dépôt épinglés, opérations Win32 futures et en vol, puis conserve l'admission/Job/root/stdin/stdout/stderr/quota jusqu'à preuve indépendante de root-exit + Job-empty + EOF des deux pipes + aucune création future + réconciliation après crash. [Recherche négative #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) : un vrai Git a été lancé **après** révocation ; ne jamais la considérer comme un test vert. Les PR #327/#353/#355/#358 restent non fusionnées. Issues #176/#182/#185/#319 et [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) toujours ouvertes. **Windows 10/11 V1 NO-GO ; pas de release, activation native, assouplissement sécurité, runner auto-hébergé ni action sur le PC.**

**Preuves historiques RED/CI :** #455 test-only run **annulé avant le test** : ne pas déclarer de RED observé ; [HEAD CI #41202575165](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41202575165) et [#317 #41202575174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41202575174) verts. #456 test-only [#41223798735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41223798735) : **vrai RED Windows + Ubuntu** ; [intermédiaire #41224075957](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41224075957) RED d'un ancien scénario de late-EOF ; [final #41224435690](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41224435690) 4/4 et [spécial #317 #41224435710](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41224435710) 12/12 par OS. #457 premier run test-only échoué **rustfmt**, pas preuve du bug ; [test-only formaté #41225740874](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41225740874) : vrai RED Windows (427 OK, 1 échec, 11 ignorés), [HEAD corrigé #41226054364](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41226054364) 4/4 (428/0/11). Les PR #455/#456/#457 ont toutes été fusionnées après validation du HEAD et suivies d'une CI `push/main` verte sur leur SHA squash exact, avec arbres Git identiques. Commentaires de revue personnelle non approbation externe.

**Méthode intégrale de reprise :** une seule PR à la fois **terminée**. Interroger GitHub pour `main`, SHA, arbre Git, PR/issues, preuves et CI. Choisir un bug P0 réel et limité ; ajouter un test comportemental d'abord quand possible ; conserver les RED (et qualifier les erreurs rustfmt, compilations annulées comme **non-comportementales**). Modifier seulement les fichiers nécessaires, pousser, ouvrir PR, valider sur HEAD exact via Windows/Ubuntu hébergés + workflows spécialisés pertinents, lire logs et rapports physiques avec SHA source/checkout/run/attempt. Revue complète et auto-revue **COMMENT** seulement ; jamais assimilée à un avis indépendant. Fusion **squash** uniquement après CI verte et base inchangée, vérifier arbre HEAD=arbre merge temporaire=arbre squash, puis **nouvelle CI `push/main` indépendante 4/4 tentative originale** sur squash exact, preuves physiques archivées, reçus PR et #323. Ne commencer une nouvelle PR qu'après ces preuves. Ne jamais augmenter des timeouts, affaiblir des tests, permis, sandbox ou contourner la cybersécurité.

**Orientation du prochain chat :** reprendre la stabilisation du **cœur Rust/MCP** sur le nouveau `main` vérifié. Priorité : réelle chaîne d'autorisation synchronisée create/resume/revoke (pas de nouveau compteur inerte en remplacement), échec #355 préservé, preuves physiques distinctes, sans activation du GitRead natif non autorisée. Si une décision sensible réclame un choix du propriétaire, poser la question ; sinon accomplir une PR complète autonome. Cette passation date du 31 octobre, pas un ordre de reset sur l'ancien SHA.

---

## Point historique après PR #457 (remplacé)

`main=291c152cdd132753668563031cf34bcfe0810940` **before the current documentation PR**, with independent [push/main #41226531396](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41226531396) **SUCCESS 4/4, first attempt** on exact integrated SHA and checkout. Windows runtime **428 passed/0 failed/11 ignored**. Schema8 physical reports `complete=true`, **10 Windows/4 Unix** scenarios/characterizations, SHA/run/attempt concordant and artifacts saved. This is a historical verified checkpoint, not an immutable current branch: always requery GitHub after this docs PR.

| PR | Final verified scope | Squash main | Independent push/main |
| --- | --- | --- | --- |
| [#454](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/454) | Docs after #453; not runtime acceptance | `50607a1b72373c6058a46c74968f80fc6165335d` | [#41197559450](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41197559450) 4/4 |
| [#455](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/455) | Live `HardenedCommandRunner`: reject known expired original deadline **before spawn** | `0bdf3621afdccb812d2165021a4788128b4f989f` | [#41202989935](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41202989935) 4/4 |
| [#456](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/456) | Live stdout/stderr reader: stop **new** reads after shared output ceiling | `ef403d14890dc15127431560ebda1fdfb7cdc8d0` | [#41224958029](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41224958029) 4/4 |
| [#457](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/457) | Inert native call tracker: poisoned mutex implies stop-required, counter remains unknown | `291c152cdd132753668563031cf34bcfe0810940` | [#41226531396](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41226531396) 4/4 |

**Tests and distinctions:** #455 final [PR #41202575165](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41202575165) 4/4, specialized [#41202575174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41202575174) 11/11 each OS; its test-only CI was **cancelled before regression ran** (no observed behavioral RED). #456 **genuine test-only Windows/Ubuntu RED** [#41223798735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41223798735), two assertions on each OS; intermediate #317 [#41224075957](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41224075957) RED due to an obsolete fixture requiring a long wait after overflow. Final [PR #41224435690](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41224435690) 4/4 and [#317 #41224435710](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41224435710) **12/12 each OS**; the final fixture retains a quiet descendant's stdout but isolates stderr, expects early output-limit classification. #457 first unformatted test-only CI stopped at **rustfmt**, not behavioral failure; formatted test-only [#41225740874](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41225740874) observed **Windows RED 427 passed/1 failed/11 ignored** on poisoned-ledger stop; final [PR #41226054364](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41226054364) 4/4, Windows 428/0/11; [Private Preparation #41226054503](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41226054503) success. All final PR-head/temporary merge/squash **Git trees identical**, independent `push/main` attempt1 4/4 and physical reports preserved; self-reviews were COMMENT, never independent approval. Reuse [PR #455 receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/455#issuecomment-6543659921), [#456 receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/456#issuecomment-6546840125), [#457 receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/457#issuecomment-6547082325), and their #323 audit comments.

**Not GO:** production `GitReadService::status()` continues using `HardenedCommandRunner`. Known expired deadlines now fail before spawn, but no atomic create/revoke fence or interruption of blocked `spawn`, `wait` or pipe `.join()`. Post-overflow read-stop does not cancel another reader already blocked on an inherited handle. The native GitRead owner/Job/watchdog/stop-tracker remains **non-executing**, does not issue native launch authority, and poison-as-stop is only unknown-accounting fail-closed. A durable service guardian needs joint original session/TaskLease generation/policy epoch/host/pinned-target *future and in-flight* CreateProcessW/ResumeThread synchronization and independent proof of root exit, kernel Job-empty, stdout and stderr EOF, no future spawn, crash/revocation uncertainty and retained quotas. Research [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) demonstrates negative **real Git spawn after revoke**; #327/#353/#355/#358 remain unmerged. #323 OPEN / Windows V1 **NO-GO**; #319/#185/#176/#182 distinct. No tag, release, native switch, security changes, self-hosted runner or user-PC commands without new authorization.

**Instruction prioritaire pour le prochain chat :** vérifier d'abord le HEAD GitHub et la CI `push/main` actuels, puis avancer une seule PR complète à la fois sur un véritable défaut P0, de préférence une garantie concrète de coordination create/resume/revoke plutôt qu'une nouvelle jauge inerte. Ne jamais basculer GitRead natif sans chaîne originale session/TaskLease/policy/hôte/targets et guardian durable, ni publier de V1 sans un GO explicite sur #323. Conserver le test RED de la recherche #355. Revue du diff, exact-head Windows/Ubuntu, preuves physiques, squash, arbres identiques, CI push/main indépendante et reçus sont obligatoires. Aucun accès au PC, runner auto-hébergé ou assouplissement de sécurité.

---

> **À utiliser dans le nouveau chat** : continue réellement l'ingénierie du dépôt
> GitHub ci-dessous, de manière autonome, une PR complète à la fois. Ne reste pas
> à un audit ou un plan si l'implémentation, les tests et le squash peuvent être
> réalisés proprement. Pose une question seulement si un choix de sécurité ou de
> produit exige ma décision. Ce texte est un **checkpoint historique à
> revérifier en direct**, jamais un SHA imposé pour faire un reset.

## Mission et limites non négociables

Dépôt unique : https://github.com/ELITEAYOTO/Optic-AI-Bridge-

Objectif : stabilisation du **cœur Rust/MCP Windows 10/11 V1** existant,
sécurité, liveness, propriétaires de processus et tests. Ubuntu CI est requis
mais n'est pas la cible release V1. Ne développe pas Optic Desktop, REL,
de nouvelles capacités généralisées de travail distant ou de fonctionnalités
hors stabilisation. **Windows V1 NO-GO** tant que [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323)
ne dispose pas de toutes les preuves et d'un GO explicite du propriétaire.
Aucun tag, release, installateur V1 approuvé ou activation du GitRead natif.

**Jamais d'accès/test/commande/installation sur mon PC sans nouvelle
autorisation ciblée**, aucun runner auto-hébergé. Ne change pas les permissions,
secrets, protections GitHub, AppContainer, réseau, sandbox, timeouts,
contrôles de contenu, ni les mécanismes de stop pour obtenir des tests verts.
Ne contourne pas les contraintes de cybersécurité/Codex. Privilégie
le **connecteur GitHub** et des runners Windows/Ubuntu GitHub-hosted.

## Point historique de reprise après #453 (remplacé ci-dessus)

Au checkpoint du 31/10/2026, **avant la PR documentaire** :
`main=bfcd9dcba0f73bf6d8aa4d148fc5c654c819edf6`.
La [CI indépendante push/main #41195919444](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195919444)
est terminée SUCCESS **4/4 tentative 1**, SHA source et checkout
identiques au squash. Windows runtime **424 tests réussis / 0 échec /
11 ignorés**, `OPTIC_PHYSICAL_HOST_STOP` schema 8 `complete=true`,
10 cas Windows et 4 Unix, rapports et artefacts conservés.

PR techniques **déjà intégrées**, avec pour chacune tests sur HEAD exact,
revue (une auto-revue n'est pas une approbation indépendante), squash,
arbres Git testés et fusionnés identiques, CI indépendante postmerge 4/4 :

| PR | Correctif avéré | Squash main | CI indépendante |
| --- | --- | --- | --- |
| [#450](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/450) | compteur d'appels Create/Resume natifs **inert**, arrêt publiable | `f907ab462704bc7c67c2780f6f9ef2b9bf351f1b` | [41191950148](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41191950148) |
| [#451](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/451) | compteur lié aux arrêts originaux session/TaskLease/Job/hôte et à l'Instant absolu du watchdog, **sans enfant** | `9a3b9b070320d8dd13f8440c0af44be4a90f7d9e` | [41193687286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41193687286) |
| [#452](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/452) | ticket Rust emprunté au propriétaire, plus de compteur Clone/Default détachable | `1d7934de941041afdfa8a347110001221268f94a` | [41194832125](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41194832125) |
| [#453](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/453) | **production** `HardenedCommandRunner` : chronomètre original AVANT spawn, copie input et pipes, pas de reset après | `bfcd9dcba0f73bf6d8aa4d148fc5c654c819edf6` | [41195919444](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195919444) |

PR #453 final HEAD `7354f5b0d847861bd6d5b3add611bb8ca603a91e`,
Git tree identique HEAD/squash `b89c232b7038771a73596d3bbc8a7ea8447b1974`.
[CI de PR #41195558970](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195558970)
4/4 SUCCESS ; [#317 late-pipe #41195558883](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195558883)
SUCCESS 11/11 sur Windows **et** Ubuntu ;
[Private Project Preparation #41195558899](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195558899)
SUCCESS. Reçus :
[PR #453](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/453#issuecomment-6542696895)
et [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6542697249).

**ATTENTION : cette PR documentaire peut avancer `main`. Recommence par
une recherche LIVE** des branches, PR, issues et Actions. Ne base aucune
modification sur ce SHA sans confirmer qu'il est toujours HEAD.

## Architecture et blocages à ne jamais minimiser

1. Le MCP `git_status` de production utilise **toujours**
   `crates/optic-bridge-runtime/src/hardened_command.rs`
   (`HardenedCommandRunner`). La PR #453 fait compter `Command::spawn()`,
   validation/entrée, copie input et préparation des pipes dans l'échéance
   originale, et **rejette une fausse réussite** après un startup lent.
   **Elle NE borne PAS le temps réel** : `Command::spawn`, `wait` et
   stdout/stderr `.join()` / écriture stdin peuvent rester bloqués si des
   descendants conservent les handles. La fermeture administrative de
   [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317)
   ne signifie pas une acceptation liveness ; preuve exigée via #323.
2. Le candidat GitRead natif
   `crates/optic-bridge-runtime/src/git/native_status.rs` et le tracker
   `git/native_inflight_calls.rs` sont **inertes**, non exécutants, non
   reliés au MCP réel. Même les quatre signaux d'arrêt, les timers et les
   emprunts Rust ne synchronisent pas l'émission de vrais
   `CreateProcessW`/`ResumeThread` avec une révocation simultanée.
   Un appel déjà admis peut entrer dans le noyau **après** stop.
3. [PR #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355)
   est une recherche **négative** montrant un vrai Git après revoke ;
   ne masque pas ou ne fusionne pas cette preuve. Les quatre PR recherche
   #327/#353/#355/#358 restent hors main au checkpoint. Les watchdogs
   sur un Job vide/terminé n'empêchent **pas** des créations futures (#439).
4. Il faut une vraie autorité/guardian de service : même génération
   session + TaskLease, policy typée et epoch, hôte original, exécutable
   et racine épinglés, arrêt et délivrance synchronisés pour appels
   Create/Resume futurs **et en vol**, sans garder les verrous d'autorité
   pendant des appels Win32/I/O potentiellement bloquants. Le guardian
   retient quota, admission, Job, root, thread, pipes et HRESULT/état incertain
   jusqu'à preuve **séparée** root exit + Job vide physiquement +
   EOF stdout ET stderr + absence de futur launch ; recovery après crash/
   abandon du demandeur obligatoire.
5. Issues toujours ouvertes au checkpoint : [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323)
   NO-GO, [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319)
   première réponse MCP ~30s, [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185)
   AppContainer/Node, [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)
   et [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182)
   contenu sensible et provenance Git. Aucun contournement de sécurité,
   augmentation de timeouts ou retry-to-green.

## Références à lire sur le `main` actuel

`README.md`, `AGENTS.md`, `AI_HANDOFF.md`, `STATUS.md`,
`ROADMAP.md`, `CHANGELOG.md`, le
[handoff canonique](HANDOFF_WINDOWS_V1_2026-10-31.md),
`docs/engineering/RELIABILITY_DEBT.md`,
`docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md`,
`docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md`,
`docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md`,
`docs/engineering/REPOSITORY_WORKFLOW.md` et les sources et ADR concernés.
Prends en priorité les **dernières sections du haut** des documents très longs :
les anciennes sections `CURRENT` plus bas sont historiques.

## Méthode d'exécution OBLIGATOIRE — PR par PR, jusqu'au squash contrôlé

1. **Vérification vivante :** interroger le connecteur GitHub pour HEAD
   `main`, arbre Git, PR ouvertes et dernières fusionnées, issues, dernier
   `push/main`, branch protection, CI natives et historique rouge. Ne
   fabrique aucun SHA, run, test ou état ; ne considère pas un titre de doc
   comme preuve de l'état courant.
2. **Choisir UN défaut P0 réel** ou une condition de sécurité mesurable,
   de préférence une barrière comportementale plutôt qu'une accumulation
   de composants inertes. Prouve un échec déterministe quand c'est
   possible, spécifie ce que le correctif ne démontre pas. Crée une
   branche unique à partir de `main` vérifié.
3. **Implémenter avec tests**, créer des commits, **push** puis ouvrir
   **une seule PR technique** cohérente. Ne pas toucher à d'autres dépôts,
   ne pas activer GitRead natif ou de privilèges hors décision.
4. **Gates sur le HEAD exact :** `cargo fmt --all -- --check`,
   Clippy `-D warnings`, tests/workspace Windows/Ubuntu, dependency
   policy, release evidence guard, workflows spécialisés natifs
   et #317 si pertinents. Inspecter les **logs, SHA source/checkout,
   run/attempt, artefacts et rapports physiques**, pas simplement les
   badges. Une preuve schema8 `complete=true` Windows10/Unix4 n'est
   à citer **que si elle a été vue pour le run exact**. Runners GitHub
   hébergés ≠ essais physiques sur le PC utilisateur.
5. **Préserver les rouges :** si fmt/compilation/test/physique échoue,
   conserver le HEAD et les logs, corriger avec un **nouveau commit**.
   Jamais de rerun pour gommer la première erreur, ni baisse d'assertion,
   ni réécriture de preuve. Si impossible : PR ouverte et blocage explicite.
6. **Revue critique du diff**, de la sécurité, des permis, de la base
   `main` et du HEAD final. Une auto-revue/commentaire ne vaut
   **pas** approbation indépendante. Squash-merge uniquement quand
   tous les checks exigés sont verts et que main n'a pas dérivé.
7. **Après squash**, comparer les **Git tree SHA** du HEAD testé et
   du squash, vérifier `main`, puis suivre la **CI `push/main`
   indépendante** sur le SHA fusionné exact (4/4 et preuves physiques
   pertinentes). Une PR n'est jamais « terminée » après la seule CI de PR.
8. Publier les **reçus dans la PR et #323** avec SHAs exacts,
   URLs, cases, réussites/échecs et non-claims. Ensuite **et uniquement
   ensuite** passer à la PR suivante. Ne promets pas de travail asynchrone.

**Priorité recommandée :** établir le véritable contrôle des appels
Win32 Create/Resume en vol ou du liveness de production avec une preuve
de comportement et un owner durable, ou réaliser un prérequis étroit
si une barrière complète n'est pas encore sûre. **Windows V1 NO-GO**,
zéro release, zéro PC utilisateur.

**Je te laisse reprendre proprement, développer et fusionner une PR à la
fois jusqu'aux preuves indépendantes post-fusion. Une question est possible
si une décision de sécurité ou de produit est réellement bloquante.**
