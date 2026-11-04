# PROMPT INTEGRAL DE REPRISE — Optic AI Bridge apres #509, 4 novembre 2026

**Mission :** reprendre le developpement technique, prudent et autonome d'Optic AI Bridge **dans le depot** https://github.com/ELITEAYOTO/Optic-AI-Bridge- et, si connectee et autorisee, sa machine Windows 10 de tests **DESKTOP-IOSTF61** (RDC ID `28a27731-11a0-4f26-bce2-0d238709b315`). **Ne toucher ni au Bridge quotidien, ni a Optic Desktop, ni a REL, ni au plugin ChatGPT. Ne pas faire de release.** Windows V1 demeure **NO-GO**.

**LECTURE OBLIGATOIRE AVANT LE CODE :** `docs/engineering/HANDOFF_WINDOWS_V1_AFTER_PR509_2026-11-04.md`, `docs/engineering/PR505_509_VALIDATION_RECEIPTS_2026-11-04.md`, `docs/engineering/ISSUE317_NATIVE_LAUNCH_REVOKE_PROTOCOL_2026-11-02.md`, `docs/engineering/ISSUE317_NATIVE_REAL_ROOT_EOF_2026-11-01.md`, `docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md`, le ZIP de passation du dernier chat (particulierement `00_LIRE_EN_PREMIER.md`, `01_PROMPT_INTEGRAL_NOUVEAU_CHAT.md`, `02_ETAT_GITHUB_ET_PREUVES.md`, `03_ARCHITECTURE_ET_VERROUS_P0.md`, `04_METHODE_RED_GREEN_ET_CI.md`).

## Etat verifie avant PR documentaire (a re-verifier, pas a supposer)

- `main=767dda7cfbf56ce3cf94098a996eee5bbac68461` (squash PR #509), tree `f85c0f6297c14e220ee0be17897e4c3004499d38`.
- `PR509 HEAD=54dd57dc6f4bde55224c3bc96f954e62460f8f26`, meme tree ; PR CI #41851820058 4/4 SUCCESS attempt1 et trois native runs #41851820339/#41851820435/#41851820251 SUCCESS attempt1. Nouvelle CI independante `push/main` #41852887021 4/4 SUCCESS attempt1, event=push, sur SHA exact.
- Issue #323 **OPEN** ; #185 et #319 **OPEN** ; PR #499 OPEN/BLOQUEE apres premiere CI Windows failed. #355 contient encore le test negatif reel « Git after revoke ». Une nouvelle PR documentaire peut avoir avance `main` : controler son HEAD et sa CI avant le prochain correctif.
- #505 : creation Job echouee + stop de la racine ; #506 : AssignProcessToJobObject echoue + stop racine ; #507 : stop complet avant spawn refuse ; #508 : acknowledgment `start_kill=Ok` sans HANDLE -> terminaison INCERTAINE ; #509 : sur cette erreur typee post-create, conserver `JobRecord` et quotas et retourner job_id au client.
- Dernier test Windows #509 : RED comportemental exit101 sur vraie creation native, 12/12 repetitions uncertain et 12/12 create-failed, Windows crate 82 pass/4 ignored, runtime crate 475 pass/11 ignored, workspace standard et all-features 810 pass/15 ignored chacun, Clippy strict/fmt/diff PASS.

## Priorite technique, pas de saut a l'activation

P0 toujours ouvert : **issuer/guardian unique de lancement Win32 service-owned, non clonable, generation-bound et revocation-linearizable** sur original SessionRegistry, TaskLeaseRegistry, policy epoch, host stop, executable/repo Git pinnes, Job original et deadline absolue. Il doit empecher les **futurs** CreateProcessW/ResumeThread apres revoke et reconcilier les appels **en vol** qui se terminent apres revoke, sans reprise interdite. Ne jamais bloquer les mutex de registres sur un syscall Win32 potentiellement suspendu. Preuves exigees : root exit original, kernel Job empty, vrai EOF stdout, vrai EOF stderr, aucun futur create/resume, ledger scelle et etat fail-closed en cas de crash/unknown. Le controle atomique n'existe pas encore, malgre les pieces inertes ; production GitRead est toujours `HardenedCommandRunner`. Le vrai contre-exemple #355 doit devenir vert sous acceptance complete avant migration.

## Methode pratique imperative

1. **Verifier le monde reel** : repo `main`, arbre, CI exact-head/event/attempt, issues/PR et machines/labs. Ne reutiliser aucun ancien SHA sans verifier. Resoudre et annoncer les contradictions.
2. Travailler dans un **nouveau LAB isole sous `E:\Optic-IA-Bridge`** depuis `main` exact ; preuves dans `_evidence` non committees, `CARGO_TARGET_DIR=_target` local. Respecter quotas disque et tous les guardrails. Aucun outil self-hosted sur GitHub Actions ; Windows validations sur **runner hosted Windows**, plus tests reels sur PC en LAB pour les races noyau.
3. Isoler **un seul defaut prouvable** ; faire un vrai RED reproduit par un test de comportement (identifier `running 1 test`, erreur de l'assertion, OS original, PID/HANDLE original si applicable). Une erreur de formatage/compilation/commande n'est PAS un RED comportemental.
4. Correction minimale ; GREEN physique, repetitions, tests crates et workspaces `+1.99.0 --locked --offline`, fmt, strict Clippy `--workspace --all-targets --all-features -- -D warnings`, diff/staged. Preserve all SHA256 and raw logs. Ne pas modifier les caps, profils, networking, timeout, protections, release ni criteres d'acceptation.
5. Une PR a la fois ; HEAD exact pousse, workflows normaux et speciaux first attempt success, self-review COMMENT only (jamais faux independent APPROVE). Si premier passage requis echoue : analyser le RED, **STOP**, pas de rerun-to-green ni de force/bypass.
6. Merger uniquement si checks applicables verts et main toujours identique : `gh pr merge <N> --squash --match-head-commit <HEAD>`; verifier Git **tree** de squash identique au tested-head, puis **CI independante `push/main` 4/4 attempt1** sur commit squash exact. Conserver preuves dans PR et issue #323. Si echec, laisser en suspens et dire precisement ce qui manque.
7. Mettre a jour docs de temps en temps pour les coupures, et produire une nouvelle passation ZIP sur demande, **pas** de travail asynchrone implicite.

**Decisions et limites absolues :** Windows V1 NO-GO, #323 reste ouverte, pas d'activation GitRead natif prematuree, pas de tag/release, pas de CI self-hosted, pas de changement au Bridge quotidien, Desktop, REL ou plugin. La vision future (profils, FS write/undo, Git complet, build, projets A->Z, Library MCP Figma/Blender et IA locale) reste **vision**, pas scope du prochain correctif.

**Premier message utile au user :** verifier le SHA `main` et le resultat de la **PR documentaire de passation** (s'il y en a une), puis expliquer la prochaine hypothese testable et les limites. Ensuite travailler reellement selon les conditions ci-dessus.
