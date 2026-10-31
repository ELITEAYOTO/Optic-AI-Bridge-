# Optic AI Bridge — prompt de reprise intégrale, 31 octobre 2026

**À copier dans un nouveau chat.** Utilise le connecteur GitHub déjà connecté pour reprendre **directement** le développement. Ne te limite pas à un audit ou à un plan si tu peux produire une PR complète. Pose-moi une question uniquement si une décision produit/sécurité est véritablement indispensable ; pour les questions simples, je suis disponible.

## Mission et dépôt

Dépôt unique : https://github.com/ELITEAYOTO/Optic-AI-Bridge-
Travail demandé : **solidifier le cœur Rust/MCP Windows 10/11 V1 existant**, sans ajouter de nouvelles fonctionnalités, sans intervenir sur Optic Desktop ni REL, sans travaux distants sur de vrais projets ni nouveaux privilèges. Linux/Ubuntu reste vérifié en CI, mais **n'est pas la cible V1**. Statut **Windows V1 NO-GO** tant que les gates de [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) ne sont pas prouvées, sans tag/release.

## Point de reprise (checkpoint, à revérifier en direct)

Au 31/10/2026 **avant la PR documentaire de passation**, `main=7f1de707c130d61f8a6878728ec2ba8bd6ac23a8`, [CI indépendante push/main #41188920407](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188920407) **SUCCESS 4/4 tentative 1**. Les PR techniques #445, #446, #447 et #448 ont été **poussées, vérifiées, squash-merged** et validées sur la CI indépendante de leur commit `main` exact :
- [#445](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/445) diagnostic borné du démarrage/pinned executable et curseur stderr robuste → `main=6cc47415717f4bfa3eeb6674a3445a3a0f373f3c`, [CI post-merge #41185358982](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41185358982).
- [#446](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/446) revérification de la deadline initiale GitRead après vérification de contenu sensible → `main=198c33b8b2a1a9efc265eb38c6b34aead0677852`, [CI #41186369040](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41186369040).
- [#447](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/447) vérification de l'horloge d'origine après armement du watchdog Windows non exécutant → `main=b0b68dd1d5e35fa6f0c3774659187e150cb361ae`, [CI #41188103927](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188103927).
- [#448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/448) revérification horloge d'origine après précréation du Job/pipes Windows non exécutant, libération des quotas si expiration → `main=7f1de707c130d61f8a6878728ec2ba8bd6ac23a8`, [CI #41188920407](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188920407).
Pour #448 : CI exacte de PR [#41188629382](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188629382) 4/4, préparation native [#41188629411](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188629411) verte. Le test `clocked_native_custody_rejects_expiry_after_job_precreation_and_releases_capacity` passe sur Windows. Rapports physiques schema8 complets : Windows 10 cas, Unix 4, 21 tests hors ligne par OS. Les quatre merges ont les arbres Git identiques à leurs têtes testées.

PR de recherche restantes au checkpoint : #327 (base bootstrap), #353 (draft), #355 (draft ; test de spawn réel **après révocation**, rouge en recherche), #358 (draft empilée sur #353). **Ne jamais les fusionner en bloc ni supposer qu'elles sont déjà en production.**
[#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est **fermée administrativement**, mais les preuves de correction métier restent insuffisantes dans [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). #323, [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319), [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176), [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) sont ouvertes au checkpoint.
Échecs de démarrage historiques : [main #41123153308](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41123153308) et [ancienne tête #445 #41127771145](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41127771145). Le diagnostic ajouté ne prouve **aucune** cause CPU/disque/Defender ; ne masque pas ces échecs.

**Le SHA indiqué est une photographie AVANT la PR de passation documentaire.** En commençant, recontrôle `main`, les PR, les issues, les runs, la protection de branche et la dernière CI push; la PR documentaire peut avoir changé `main`. Ne checkout/rebase jamais aveuglément sur un ancien SHA.

## Fichiers sources à lire

Commence par `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`, `ROADMAP.md`, `CHANGELOG.md`, `PROJECT_CONTINUITY_PROMPT.md`, `docs/engineering/GITHUB_HANDOFF.md`, `docs/engineering/HANDOFF_WINDOWS_V1_2026-10-31.md`, `docs/engineering/RELIABILITY_DEBT.md`, `docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md`, `docs/engineering/REPOSITORY_WORKFLOW.md`, les ADR et tests concernés.

## Vrai P0 à poursuivre

Le service-owned `git_status` natif Windows est encore **non exécutant**, non relié au GitRead MCP de production. Les observations session/TaskLease/policy/host/horloge et les reçus de watchdog sont indépendants ; ils ne forment **pas** une autorité atomique de création/reprise. Étudier une **barrière synchronisée** unique contrôlant les demandes et appels `CreateProcessW` et `ResumeThread`, le stop/revoke/expiration, les appels déjà engagés, la génération originale, le contenu autorisé, l'exécutable et la racine canonique, tout en évitant de tenir des verrous d'autorité pendant des I/O/Win32 bloquantes. Le watchdog n'est pas un sceau de création future; un Job vide/terminé n'est pas une preuve de non-recréation (#439). Si la prochaine étape ne peut pas être sûre, établir d'abord un contrat + test négatif déterministe sur un petit périmètre. **Ne pas activer la voie native en production** sur une simple suite de fixtures vertes.

Conserver admission, quota, original Job, handle root et stdin/stdout/stderr jusqu'à **root exit + kernel Job-empty + EOF des deux pipes + exclusion des créations futures** ; conserver l'incertitude native, les HRESULT réels et les preuves de crash/recovery. Le runner legacy peut encore attendre sur ses pipes descendants (#317 historique). Priorités séparées : #319 première réponse MCP ~30s; #185 AppContainer/Node; #176/#182 protection de contenu et provenance Git; #323 Windows10/11 installation/rollback, soak et mise en production.

## Méthodologie OBLIGATOIRE — PR PAR PR, COMPLET JUSQU'AU SQUASH

1. Connecteur GitHub : inspecter **l'état vivant** avant chaque étape ; ne pas inventer de HEAD, d'issue, de CI ou de test passé.
2. Choisir **une seule** correction fondée sur un défaut réel / une invariant manquant, limitée et vérifiable, sur la branche `main` effectivement actuelle. Présenter problème, portée, non-claims et test attendu.
3. Implémenter, ajouter un test de non-régression qui échouerait sans correctif, commit, **push** et créer **une PR complète**. Pas d'empilement de multiples PR techniques.
4. Suivre la CI sur le **HEAD exact** : `cargo fmt --all -- --check`, Clippy strict, Rust workspace Windows/Ubuntu, dependency policy, release evidence guard, workflows natifs concernés, rapports physiques. Vérifier logs et SHA source, pas uniquement badges. Conserver les tentatives rouges et corriger sur **un nouveau commit**, sans retry-to-green / sans assouplir assertions.
5. Faire une revue du diff et des droits, vérifier base `main` non modifiée, tests et preuves suffisantes ; respecter protections et revues requises. Une revue écrite par toi-même n'est **pas** une approbation de pair indépendante.
6. Effectuer le **squash merge** de la seule PR validée, comparer les **Git tree SHA** du HEAD testé et du squash, puis attendre et contrôler la **CI push/main indépendante** sur le commit intégré exact (4/4 sur la première tentative si c'est le cas, sans embellir).
7. Publier un résumé vérifiable avec liens, SHAs et restrictions dans la PR et #323 ; **une PR est terminée uniquement après sa validation post-fusion**. Ensuite seulement commencer la suivante. En cas d'échec/blocage, laisser la PR ouverte et expliquer, ne jamais la fusionner de force.
8. Poursuivre spontanément les PR suivantes lorsque le travail est possible, sans reposer des confirmations inutiles, et me fournir à chaque fois le résultat exact.

**Contraintes absolues** : aucune commande, installation, test ou modification sur mon PC sans nouvelle autorisation précise ; aucun runner auto-hébergé ; aucune désactivation de sécurité, consentement/permission, réseau, AppContainer, limite de temps, stop, isolation ou contrôle de contenu ; aucun contournement des blocages Codex ; aucune release/tag, promotion V1 ou branche-protection modifiée sans décision de ma part ; aucun changement Optic Desktop/REL. Travailler via **plugin GitHub** et CI GitHub-hosted ; utiliser uniquement un environnement de travail isolé disponible et autorisé. Préserver le statut Windows V1 **NO-GO**.

**Je te laisse avancer réellement avec cette méthode, PR par PR, push et squash compris. Pose-moi les questions importantes, même simples, seulement si elles bloquent une décision.**
