# Recu de stabilisation Windows : PR #505 a #509 (4 novembre 2026)

**Document historique et auditable, non autorisation V1**. Les SHA et CI ci-dessous etaient verifies le 4 novembre 2026. Apres la PR documentaire, un nouveau `main` sera cree ; reverifier ses runs independants. L'issue [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) reste **OPEN**, Windows V1 **NO-GO**.

## Suite cumulative de petites PR, du vrai OS au reporting

| PR | Merger main | Correctif prouve | Recu |
| --- | --- | --- | --- |
| [#505](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/505) | `011a928b37f2940496fb83f11152604dd89196e5` | Echec `OwnedJob::create` apres creation d'un original suspendu : terminer par HANDLE original, attendre physiquement ou signaler une incertitude | [historique #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) |
| [#506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/506) | `90f74c142038ab19795c56a7423ec829289d17be` | Echec *reel* `AssignProcessToJobObject` avec droits PROCESS insuffisants : meme stop original fail-closed | [preuve #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6632042715) |
| [#507](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/507) | `458ddc378cdca6949f79a240dabd697503a39d37` | `ProcessManager` refuse un start dont `stop_requested` a deja ete publie avant l'appel spawn | [preuve #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6632669189) |
| [#508](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/508) | `489432612687ccf897c26f69f081a9827fdf7213` | Wrapper opaque `start_kill==Ok` ne signifie PAS root signale ; annoncer la terminaison incertaine | [preuve #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6633281233) |
| [#509](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/509) | `767dda7cfbf56ce3cf94098a996eee5bbac68461` | Apres erreur Win32 native *typee* « termination uncertain », garder la reservation d'origine, les quotas et l'ID du job pour reconciliation MCP | [preuve #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6634058656) |

Tous ces SHA designent des jalons historiques connus ; **la seule reference actuelle au moment de la redaction est `main=767dda7...`** avec CI independante `push/main` [#41852887021](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41852887021), **4/4 SUCCESS first attempt**, tree `f85c0f6297c14e220ee0be17897e4c3004499d38`.

## Detail de #509 et capacites de preuve

- Commit exact teste `54dd57dc6f4bde55224c3bc96f954e62460f8f26`, tree `f85c0f6297c14e220ee0be17897e4c3004499d38`; meme tree sur squash.
- [CI PR #41851820058](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41851820058), event=pull_request, attempt=1, **4/4 SUCCESS** ; trois pipelines specifiques #41851820339, #41851820435, #41851820251 **SUCCESS** attempt=1.
- [CI independante #41852887021](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41852887021), event=push, attempt=1, head exact squash, **4/4 SUCCESS**, verifiee avant le recu #323.
- RED : vraie racine Windows CREATE_SUSPENDED/PROCESS, PID observe non nul et processus pas sorti ; `CommandWrap::spawn` echoue apres creation, `ProcessManager` supprime a tort le JobRecord, **active_session_job_count=0** plutot que 1, `cargo test` RED exit101. SHA256 log `2D2FE34DB5D4FF63E11703A8A8FCD5BC742664190B27740EC1C47256B446F426`.
- GREEN : `UnconfirmedWindowsChildTermination` type prive marqueur dans Windows ; `UnconfirmedWindowsSpawn` contient `job_id` cote runtime et conserve `TerminationUncertain`, stop flag, quotas, sans EOF invente. La couche MCP mappe `optic.process_spawn_termination_uncertain` plus `job_id`. GREEN SHA256 `873AD763B69AF4EF3EB05ADDF2D88F445965F81DFDFA316AA00F7BAF1769D5FC`.
- Controle negatif : veritable fichier `.exe` non PE, `CreateProcessW` refuse avant process, `Io` ordinaire et quota rendu ; GREEN log `BA8088BD23DE86E5F3195500271912BA93BE218D459C3D310A4116F082AE6351`.
- MCP test GREEN `05702D58A131A3161B340DCD7F7B263336E499122CBDDD2DD2D0591D6C46D931`.
- Windows local : repetitions 12/12 incertitude + 12/12 echec avant creation, crate Windows 82 pass/4 ignored, runtime 475 pass/11 ignored, workspaces standard et all-features 810 pass/15 ignored chacun, Clippy `-D warnings`, fmt et diff PASSED. Logs principaux : Windows crate `D391FB11E38F4FB2700A04CDCC326BAB8831A47CCE28596FE175B785F2BB2245`, runtime `3619DCEA0E08A7F686858C128E4E211D762E6390B3C280BF99BD4538D1220A30`, workspace standard `FD3C523091A1D567F599223197E8AEB2B05F2D48FD2CE40DEFD8D8A4240E98C6`, workspace all-features `5A5A2BF889C8338E4CB23CBF6B0BAC9CBC7249FDD7060BB68D1519BA720EA7E2`, Clippy `3B088692AF847FCD7B81A886FC5462B9D1FD91B9AE8B3579781AB97EC7387EC9`.

## Emplacements de preuves hors Git (Windows 10 autorise)

Sous `E:\Optic-IA-Bridge`, chaque dossier suffixe `-lab-2026-11-04` conserve `_evidence` et `_target` non commits. Pour #509 :

`E:\Optic-IA-Bridge\optic-p0-uncertain-start-retains-capacity-lab-2026-11-04\_evidence`

Fichiers de preuve principaux : `RED_NATIVE_UNCERTAIN_START_BOOKKEEPING.txt`, `GREEN_UNCERTAIN_NATIVE_START.txt`, `GREEN_REAL_CREATE_FAILURE_ROLLBACK.txt`, `GREEN_MCP_UNCERTAIN_CODE.txt`, `RUN_VALIDATIONS.ps1`, `VALIDATION_RESULTS.txt`, `WORKSPACE_DEFAULT.txt`, `WORKSPACE_ALL_FEATURES.txt`, `CLIPPY_STRICT.txt`, `ISSUE323_PR509_RECEIPT.md`. La machine conserve aussi les LAB isoles #505, #506, #507 et #508.

**Limite decisive** : un statut `TerminationUncertain` n'est **pas** un process kill confirme, ni un Job vide, ni deux EOF, ni un issuer/revoke atomique. Aucune preuve de reprise de GitRead reel apres revoke ; #355 toujours non admissible ; #323 OPEN, Windows V1 NO-GO. Conserver les vrais logs et ce recu, mais ne pas citer les anciens checks comme validation du HEAD d'une nouvelle PR.

## Procedure de publication documentaire

Cette passation est faite sous forme de PR **docs-only** distincte ; meme une PR documentaire requiert le parcours normal : PR HEAD exact, 4 jobs hosted first attempt, merge protege, Git tree identique et 4 jobs independent push/main first attempt. Une fois validee, re-verifier et publier le nouveau SHA/doc PR dans la passation ZIP plutot que modifier retroactivement les vieux recus de tests.
