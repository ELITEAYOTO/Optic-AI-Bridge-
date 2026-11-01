# Windows V1 — état de reprise vérifié le 1er novembre 2026 (après #470)

> **Point de reprise, pas autorité permanente.** À la création de ce document,
> le `main` officiel est `bd121996ce38757362e35c85d3113dda089aefd1`.
> Rechercher systématiquement le **nouveau SHA** et les Actions GitHub à chaque
> changement de PR, et après toute fusion de cette documentation.

## Sources officielles et statut

- Dépôt : [Optic AI Bridge](https://github.com/ELITEAYOTO/Optic-AI-Bridge-).
- **Windows 10/11 V1 : NO-GO**. [Issue de lancement #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) toujours **ouverte**. Ne pas publier de release ni annoncer un installateur prêt.
- [PR #469](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/469) fusionnée : les noms d'artefacts API-key rejoignent le classificateur de lecture sensible.
- [PR #470](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/470) fusionnée : le Git de mutation sous Windows utilise `-c core.longpaths=true` **par processus seulement** ; `main` squash = `bd121996ce38757362e35c85d3113dda089aefd1`.
- [CI post-fusion #470](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41332810883) : Windows, Ubuntu, dépendances et contrôle des preuves **SUCCESS 4/4** sur ce `main`.
- [PR #471](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/471) : **ouverte, DRAFT, NON fusionnée**, commit `c9638574db95ce47f09372e8d885f6033ea65b07`. [CI #41335658243](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41335658243) **SUCCESS 4/4**. Cette PR propose un **changement volontairement incompatible** : `git_diff` refuse tout contenu par défaut ; le propriétaire local doit explicitement passer `--allow-git-diff-content` au démarrage. Les droits FileRead/ReadSensitive restent séparés. **CI verte ne signifie PAS validation de cette politique produit.** ADR-0036 est uniquement proposé dans cette PR ; ne pas fusionner sans décision du propriétaire.
- [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) toujours ouvert : provenance des objets Git stockés après staging, alias NTFS et métadonnées sensibles. La PR #471 propose une *limitation d'exposition des octets*, pas une preuve de provenance.
- Autres verrous : [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) première réponse MCP, [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) AppContainer/Node, [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) autorité de lecture sensible.

## P0 : la révocation de session ne clôt pas le lancement Git (#317)

**Constat technique prouvé sur `main=bd121996…` :** après admission d'un worker GitRead, publier une révocation de session pendant que le worker est bloqué à une barrière pré-spawn n'empêche pas le démarrage effectif du `git status` après relâchement. Régression ignorée par la suite normale :
`git_tools::tests::issue317_revocation_between_authorization_and_git_spawn_must_prevent_execution` (issue [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317), **fermée administrativement, pas corrigée comportementalement** ; [PR de recherche #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) encore draft et RED).

Preuve Windows 10 réelle, dépôt Git **synthétique**, sans secrets réels :
- Sur un `CARGO_TARGET_DIR` **réservé uniquement à cette copie source**, **3/3 exécutions** du test sélectionné ont été **RED (exit 101)** avec l'assertion `ISSUE317_POST_REVOKE_GIT_SPAWN`; chacune indiquait **`running 1 test`**.
- `33/33` tests de préparation native `native_status_tests` et `5/5` tests du kernel Job Windows `precreated_native_read_tests` **PASS**, sur ce même LAB isolé. **Ces 38 tests ne lancent pas le vrai Git sous une autorité native de bout en bout** et n'infirment pas le P0.
- Détails datés, empreintes et contexte : [réception de #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6562060813).
- Preuves locales privées, non versionnées : `NATIVE317_ISOLATED_ACCEPTANCE_EVIDENCE_20261101.zip`, SHA-256 `A76B5BF9120BEDE82038721E6EC7D8A041577C6DFB682FC1C8D3864C20DC375D`. Elles résident dans un LAB contrôlé par l'opérateur. Ne pas supposer qu'elles existent sur un autre poste.

### Piège de test détecté : compilation partagée

Un `cargo test` a **retourné exit 0 en exécutant zéro test** alors que le fichier source local contenait bien le test #317. Plusieurs copies de travail avaient réutilisé la même destination `CARGO_TARGET_DIR`. Cette exécution est **INVALIDE**, pas « GREEN ». Conserver le journal original et **exiger un répertoire Cargo distinct par copie Git**, en plus d'une vérification du nombre de tests vraiment exécutés.

Checklist minimale du banc :
1. Vérifier le SHA exact de la copie et uniquement le changement de test autorisé. Utiliser un dépôt de test jetable, sans secrets, et un `CARGO_TARGET_DIR` indépendant.
2. Avant exécution, `cargo test --locked --offline -p optic-bridge-mcp --lib issue317_revocation_between_authorization_and_git_spawn_must_prevent_execution -- --ignored --list` doit lister **exactement un test**.
3. Lancer explicitement le test ignoré avec `--ignored --nocapture --test-threads=1`. Lire l'exécutable réellement lancé et imposer **`running 1 test`**.
4. Distinguer **assertion comportementale RED** (`ISSUE317_POST_REVOKE_GIT_SPAWN`) de rustfmt, compilation, fixturing, timeout du harnais, annulation ou `0 test`. Ne jamais « rerun-to-green » pour effacer le premier échec.
5. Archiver logs et empreintes avant toute édition. Le script local `Test-Native317-SafeReplay.ps1` (non versionné, parce que son chemin de LAB est spécifique au PC) illustre un garde qui refuse un résultat à zéro test.

## Pourquoi les fondations natives ne constituent pas encore une correction

- La voie de production `crates/optic-bridge-runtime/src/git/read.rs` lance toujours `GitReadService::status_with_max_bytes` via `HardenedCommandRunner`, pas le propriétaire natif Windows.
- `git/native_status.rs` prépare et réserve le contexte, les identités et le Job **sans lancer Git** ; `git/native_inflight_calls.rs` compte des appels OS potentiels **sans être une permission CreateProcessW/ResumeThread**.
- `crates/optic-bridge-windows/src/job.rs` dispose de Job, watchdog, handles et précréation de pipes, mais **aucun propriétaire de GitRead réel n'est connecté à ces preuves**.
- Deux reçus ou contrôles consécutifs de session/TaskLease ne sont pas une **transaction atomique** avec politique, arrêt hôte, CreateProcessW et ResumeThread. Un simple `get_active()`, un `AtomicBool`, une revérification « juste avant spawn » ou un mutex tenu pendant un appel Win32 bloquant ne suffisent pas.
- Le succès de `TerminateJobObject` sur un Job **vide** n'est pas la preuve d'**exit réel de la racine, Job vide post-exécution, EOF physiques stdout/stderr, absence de futur spawn et appels Win32 en vol résolus**.

## Plan de correction natif (sans publier un faux GO)

1. En LAB indépendant, construire un propriétaire natif qui conserve les identités Git/repo épinglées, la session et TaskLease originales, politique/host-stop, quotas et horizon absolu.
2. Définir/mesurer le point de linéarisation **revocation vs CreateProcessW/ResumeThread**, y compris quand un appel est déjà entré dans le noyau. Toute ambiguïté doit être gérée par le propriétaire/Job, pas « oubliée ».
3. Prouver sur de **vrais processus synthétiques** Windows : création suspendue, affectation au Job avant resume, watchdog indépendant, absence de futurs lancements après stop, root exit, Job empty, stdout/stderr EOF, tickets in-flight retirés, capacité/lease libérés uniquement après preuve physique.
4. Rejouer #355 RED→GREEN pour une **raison démontrée**, puis tests adversariaux : descendants avec pipes hérités, dépassement stdout/stderr, révocation pendant Create/Resume, deadline, échec Win32, crash, abandon du propriétaire.
5. CI exacte Windows et Ubuntu, tests de politique et de preuves ; une PR ciblée, revue, squash protégé seulement après validation, puis CI indépendante `push/main`. **#323 reste NO-GO** tant que les garanties V1 complètes ne sont pas prouvées.

## Règles de passation

- Relire d'abord GitHub en direct (`main`, PR #471, PR de recherche, issues #182/#323, Actions). Les SHA de cette page sont un **instantané daté**.
- **Une correction/PR à la fois**, tests rouges conservés, aucune fusion automatique de PR de recherche, pas de relaxation des politiques ni de suppressions d'assertions. Documentation au fil de l'eau, avec reçus exacts.
- Pas de tag/release ni annonce V1 prête. Ne pas toucher au Bridge quotidien, à Optic Desktop, REL, projets privés ou secrets. **Aucun test sur PC opérateur sans autorisation spécifique à la campagne** ; les précédents tests ont été effectués uniquement sur LAB synthétique explicitement autorisé.
- Lire en complément la [passation précédente](HANDOFF_WINDOWS_V1_2026-11-01.md), le [prompt de reprise](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-11-01.md) et [l'audit natif](ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md).
