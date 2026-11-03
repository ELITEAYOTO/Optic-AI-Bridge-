# Windows V1 — reprise technique canonique apres PR #496 (2026-11-03)

> **Checkpoint date, non substitut aux API GitHub en temps reel.** Ce document prend le relais du dernier handoff apres #490 et n'efface pas ses preuves ou limitations. V1 Windows 10/11 : **NO-GO** ; [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN**.

## Etat reel fusionne et premier essai post-main

- **Dernier main verifie :** `77922d5dcb83eb54b7677dc4be1856c70f40620c`. Git tree `68545693c4ae534b3c9280c7eb3a38658b7376e7` identique a l'original teste de [PR #496](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/496).
- **CI independante :** [push/main #41681091122](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41681091122), event=push, attempt=1, exact SHA, **4/4 COMPLETED SUCCESS** : Win Rust, Ubuntu Rust, dependency policy, release evidence guard. Hosted Windows job `126074610342` execute explicitement nouveau test de processus sorti, ancienne revocation originale et Job + 2 EOF `ok`, native 72 PASS / 4 ignored, runtime 454 PASS / 11 ignored. AppContainer Node demarre et la tentative reseau est refusee (exit 42, accepted=None).
- **CI PR-head :** [#41680030589](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41680030589) 4/4 first attempt; [native lifecycle #41680030498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41680030498), [native preparation #41680030708](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41680030708), [private Rust IR #41680030875](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41680030875) first-attempt success.
- La [PR documentaire #491](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/491) est **OPEN** ; ne pas fusionner automatiquement : sa premiere CI [#41521859700](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41521859700) a echoue sous Windows AppContainer/Node et [issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) demeure ouverte. Les sections datees dans README/AGENTS/STATUS restent historiques.
- Au dernier controle GitHub, [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est **CLOSED**, mais cela ne ferme aucun des blocages de l'issuer natif suivi par **#323**. Issues [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319), [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176), [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) OPEN.

## Dernieres PRs techniques et preuves, une a la fois

| PR MERGED | Changement effectif (portee delimitee) | CI independante push/main |
| --- | --- | --- |
| [#492](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/492) | Stop legacy child Git lie a la session originale revoquee | [#41598078452](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41598078452) 4/4 |
| [#493](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/493) | `PrecreatedNativeReadCustody::pin_original_job_root` refuse le stop original gagne apres `DuplicateHandle` | [#41601494769](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41601494769) 4/4 |
| [#494](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/494) | `NativeCallStopTracker` reobserve les sources de revoke originales et deadline au commit d'un ticket natif de comptage | [#41605739458](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41605739458) 4/4 |
| [#495](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/495) | Mutex de `in_flight` et `TrackedNativeCall::drop` borne a 25 ms, stop fail-closed sans faux drain. **Premiere correction immediate-try_lock a cause un vrai echec full suite**, preserve avant affinement. | [#41609640179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41609640179) 4/4 |
| [#496](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/496) | Zero-time `WaitForSingleObject` sur le **HANDLE duplique** : refuse une racine physiquement sortie apres duplication mais avant retention | [#41681091122](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41681091122) 4/4 |

### RED/GREEN kernel #496

Un vrai `cmd.exe` fixe sous Windows est cree `CREATE_SUSPENDED`, assigne a un vrai Job, un watcher original arme. Un callback test-only **apres vrai `DuplicateHandle`** termine le processus avec `TerminateProcess` sur le HANDLE original et constate `WAIT_OBJECT_0`, sans publier revoke. L'ancien code acceptait `Ok(())` pour un root deja sorti : **RED comportemental execute, exit 101**, SHA-256 `6DA22207798647B2A0975410FBEFF6F64F5394C07A44D32FBCAA0DAB5BDC5B53`. Le fix controle le HANDLE **duplique** avant retention, puis reobserve stop/deadline originaux; **GREEN**, SHA `EB46F719C5775C200D822C7789C723F32A7B2DA6B04DA61440CE3BCF24E87C0F`. Les deux tests Win32 (sortie, revoke) passent **12 campagnes sur 12**. Aucun process Git ni `ResumeThread` dans ce test.

### Regression intermediaire importante #495

Les deux nouveaux tests de mutex avaient execute leur RED (exit 101). Un premier `try_lock` immediat passait les tests cibles mais faisait echouer **un vrai test de drainage de la suite complete** (453 runtime pass / 1 fail). L'affinement final attend jusqu'a 25 ms via `try_lock`/yield, puis stoppe de maniere conservative; il a passe 19 tests du ledger x12, full standard et all features. **Conserver le log du premier full RED, pas de rerun-to-green.**

## Architecture source et prochaine priorite P0

- `crates/optic-bridge-runtime/src/git/native_status.rs` : preparation, capacity, custody originaux session+TaskLease, original watch source, watchdog Job **sans vrai lancement GitRead natif**.
- `crates/optic-bridge-runtime/src/git/native_inflight_calls.rs` : tickets non clonables de comptabilite CreateSuspended/Resume, bornes fail-closed, source original; **NE SONT PAS DES PERMITS Win32**.
- `crates/optic-bridge-windows/src/job.rs` : Job kernel, watchdog, handles duplicats, root pin, preuve root+Job+EOF; #493 et #496.
- `crates/optic-bridge-windows/src/job_list_launch_tests.rs` : tests directs de vrais `CreateProcessW` suspendus et JOB_LIST, **tests fixture**, pas un issuer MCP.
- Registres session/TaskLease + policy/host : observations sous locks partiels; aucune frontiere unifiee avec `CreateProcessW/ResumeThread`.

**Prochain jalon :** un issuer service-owned non clonable et synchronise avec les revokers **session generation originale, TaskLease originale, policy epoch, host stop, Job**, avant les vrais appels `CreateProcessW` et `ResumeThread`, sans tenir les mutex de registre pendant un syscall bloquant. Un appel precedemment admis peut revenir *apres* revoke : comptabilisation et reconciliation obligatoires. Puis validation complete des etats root signale, Job vide, stdout EOF, stderr EOF, ledger scelle/draine, aucun futur spawn et crash/unknown (jamais faux succes). Ne pas migrer le `GitReadService` de production du legacy `HardenedCommandRunner` avant ces preuves et le test reel negatif de [research #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355).

## Discipline de reprise

Une PR complete : preflight live SHA + CI attempt=1; nouveau LAB Windows 10 isole; test RED qui **execute**; GREEN, repetitions; Rust 1.99 fmt, Clippy strict workspace all-targets/all-features, full workspace serial standard **et all-features**, diff; PR et CI exact-head Windows/Ubuntu/policy/release + workflows natifs, lecture des logs; self-review COMMENT seulement; squash expected-head; arbres Git identiques; CI independante `push/main` 4/4 **premier essai**; recu PR + #323. Aucun self-hosted runner, aucune relaxation des protections, aucun changement au Bridge quotidien/Desktop/REL ou release V1. Le nouvel agent doit revalider live GitHub avant toute nouvelle ecriture.

**Recu final :** [PR #496 comment](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/496#issuecomment-6610593200), [#323 comment](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6610595707).