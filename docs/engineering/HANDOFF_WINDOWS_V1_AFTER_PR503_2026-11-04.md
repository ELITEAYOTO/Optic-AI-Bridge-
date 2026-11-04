# Windows V1 — passation canonique apres PR #503 (2026-11-04)

> Checkpoint date, non un SHA permanent. Toujours verifier GitHub en direct avant un nouveau travail. **Windows V1 reste NO-GO**, issue #323 OPEN.

## Main et CI exacts

- Repo : https://github.com/ELITEAYOTO/Optic-AI-Bridge-
- Main apres #503 : 150cd5e4f7f4220bcef7406b1dc4466bde2acee5.
- Arbre Git : 9a294586129ec1b2ac0ffa605e3e252c1430b8fe, identique au code local teste, au HEAD de PR et au commit merge.
- PR #503 MERGED : https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/503
- CI PR #503 : https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41820133578 : 4/4 SUCCESS, event=pull_request, attempt=1, head=86a934bdd1b0ef8ed77af59fbc7bb29cee08c881.
- Trois workflows natifs PR : #41820133529, #41820133685, #41820133676, tous SUCCESS au premier essai, SHA exact.
- CI independante push/main : https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41820929161 : **4/4 SUCCESS, attempt=1**, Windows, Ubuntu, Dependency policy, Release evidence guard, event=push, head exact 150cd5e4.
- Compte rendu final et preuves : https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6629513383
- Si une PR documentaire est fusionnee plus tard, main et sa CI changeront. Ne pas reemployer les anciens SHA comme etat courant.

## Correctifs recents

- PR #498 : verifier le proprietaire du vrai HANDLE thread avant native ResumeThread, non le seul TID potentiellement perime.
- PR #499 : OPEN/BLOQUEE, premiere CI Windows rouge a cause de l'intermittence Node AppContainer #185. Ne pas relancer pour obtenir du vert, ne pas fusionner en force.
- PR #500 : diagnostics CPU du vrai processus/thread AppContainer pendant timeout Node, sans attribuer abusivement une cause.
- PR #501 : job.rs, Win32 ResumeThread renvoie le compteur AVANT decrement ; l'ancien code certifiait une reprise pour old count=2 alors que le thread restait suspendu. RED reel, GREEN, 12 repetitions, CI push/main #41810657089 4/4 attempt1, main intermediaire d770f224b7123513a21e198442f0119c031c267a.
- PR #502 : appcontainer.rs, correction equivalente distincte de la reprise du thread principal original AppContainer ; RED reel exit101 pour suspend count 2, GREEN old count 0 et 2 avec 10 repetitions chacune, une seule tentative native, kill du PROCESS HANDLE exact en anomalie. Workspace default et all-features 801 pass /15 ignored, CI push/main #41815680086 4/4 attempt1. Main intermediaire 2293b12178336d2784c4f57680f307d4352991ef.
- PR #503 : job.rs / LimitedJobObject::wrap_child. Lorsqu'un wrapper opaque ne fournit aucun HANDLE du fils deja cree CREATE_SUSPENDED, ancien chemin Unsupported abandonnait le fils sans demande de terminaison. Test Windows fixe cmd.exe avec HANDLE PROCESS independant : RED 101, WAIT_TIMEOUT (258) apres erreur pendant 3000ms ; un cleanup test-only securise la fixture. Correction : appeler inner.start_kill() sur le proprietaire original avant de le perdre ; si cet appel echoue, retourner termination uncertain, jamais succes. GREEN wait WAIT_OBJECT_0, 12 repetitions PASS, crate Windows 79 pass/4 ignored, workspace standard 802 pass/15 ignored, all-features 802 pass/15 ignored, fmt + Clippy -D warnings + diff PASS.
- Pour #503 les deux git push ont retourne GitHub HTTP 500. Publication via Git Data API : blob EXACT b98e640ccfb17cc566248f39a72ed6fd32513183, arbre EXACT 9a294586129ec1b2ac0ffa605e3e252c1430b8fe, parent original 2293b121... ; commit local teste e9acdf715eaa133aaccc093ce004705507002b8e, commit GitHub 86a934bdd1b0ef8ed77af59fbc7bb29cee08c881 et squash main 150cd5e4f7f4220bcef7406b1dc4466bde2acee5 : tous trois ont le meme arbre source. Premiere CI et push-main independant ont reussi.

## Verrou P0 Windows V1 non resolu

La voie MCP GitRead de production reste la voie historique HardenedCommandRunner ; le nouveau proprietaire natif reste NON EXECUTANT. Original SessionRegistry, TaskLeaseRegistry, policy/epoch, host-stop, Job, source Git/repo pinnee et deadline service sont observes dans des etapes distinctes, PAS un issuer de lancement atomique. NativeCallStopTracker ne suit que des appels in-flight ; un ticket n'est PAS une autorisation de CreateProcessW/ResumeThread.

Exigences de securite :
1. Issuer service-owned non clonable ; verrouiller l'admission future en face des revoke originaux, sans garder mutex de registre a travers un appel Windows susceptible de bloquer.
2. Processus suspendu sous Job original avant reprise, gardien durable ; un appel Create/Resume admis avant revoke peut terminer apres revoke et DOIT etre physiquement reconcilie sans reprise ni publication abusive.
3. Tests reels RED/GREEN Windows et courses deterministes revoke avant create, create in-flight, revoke avant resume, resume in-flight, revoke multi-sources, expiration, ID reuse, ledger poison, crash/unknown.
4. Preuves independantes originales : processus root signale, Job vide, stdout EOF physique, stderr EOF physique, absence de future creation/reprise et ledger scelle/drain. Le retour de TerminateJobObject ou d'un compteur zero seul ne prouve PAS la terminaison.
5. Le vrai test negatif Git lance apres revoke de recherche PR #355 doit devenir vert SANS contourner ce test avant tout remplacement de GitReadService::status.

Lire docs/engineering/ISSUE317_NATIVE_LAUNCH_REVOKE_PROTOCOL_2026-11-02.md, ISSUE317_NATIVE_REAL_ROOT_EOF_2026-11-01.md, NATIVE_GIT_READ_WITNESS_CONTRACT.md, ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md, et les passations historiques apres #496.

## Reperes code

- crates/optic-bridge-runtime/src/git/native_status.rs : preparation, custody et receipts originaux inertes, jamais issuer.
- crates/optic-bridge-runtime/src/git/native_inflight_calls.rs : tickets de comptage create/resume, pas autorisation.
- crates/optic-bridge-runtime/src/session_registry.rs et task_lease_registry.rs : generations et revocation.
- crates/optic-bridge-windows/src/job.rs : Job kernel, root pin, resume native #501, cleanup opaque #503.
- crates/optic-bridge-windows/src/appcontainer.rs : AppContainer original thread resume #502.
- crates/optic-bridge-windows/src/job_list_launch_tests.rs : fixture CreateProcess suspendu avec Job-at-create, pas de GitRead product.

## Etat ouvert / discipline

Issues #323 (freeze V1), #185 (Node AppContainer intermittence), #319 (initialize MCP debut), #176 et #182 (read sensible, a verifier) ; PR #499 bloquee, #491 doc ancienne et recherches #327/#353/#355/#358 non fusionnees (a recontroler). Issue #317 historiquement CLOSED ne ferme PAS P0.

Machine de travail Windows 10 DESKTOP-IOSTF61 via RDC, device 28a27731-11a0-4f26-bce2-0d238709b315 ; E:\Optic-IA-Bridge. LAB preuves : optic-native-resume-suspend-count-lab-2026-11-04 (#501), optic-appcontainer-resume-count-lab-2026-11-04 (#502), optic-missing-process-handle-cleanup-lab-2026-11-04 (#503, 29 fichiers _evidence ; _target nettoye et ~16,1GiB libres).

Une PR complete a la fois : main exact + issue/CI, nouveau LAB, test comportement reel RED, GREEN, repetitions, full workspace standard + all-features offline Rust 1.99, fmt, Clippy all-targets/all-features -D warnings, diff, CI PR attempts=1 Windows/Ubuntu/native, commentaire de revue COMMENT uniquement, squash --match-head-commit + meme arbre, CI push/main independante 4/4 first attempt, recu issue #323. Jamais rerun-to-green, jamais affaiblir les tests, jamais release/GO. Ne pas toucher au Bridge quotidien, Desktop, REL, plugin ChatGPT, ni aux runners/protections GitHub.

Prompt de reprise : NEXT_CHAT_PROMPT_WINDOWS_V1_AFTER_PR503_2026-11-04.md.
