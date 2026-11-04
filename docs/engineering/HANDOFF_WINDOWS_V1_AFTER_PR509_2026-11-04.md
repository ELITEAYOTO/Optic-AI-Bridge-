# Optic AI Bridge — passation canonique Windows V1 apres PR #509 (4 novembre 2026)

> **Checkpoint date : ne pas reutiliser les SHA sans verification.** Ce document remplace les anciennes passations comme point de depart. Il ne donne AUCUN GO V1. **Windows V1 NO-GO**, issue [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN.

## 1. GitHub verifie : etat source et CI

- Depot unique autorise : https://github.com/ELITEAYOTO/Optic-AI-Bridge-
- Dernier `main` **verifie apres fusion #509** : `767dda7cfbf56ce3cf94098a996eee5bbac68461`.
- Git tree du commit merge : `f85c0f6297c14e220ee0be17897e4c3004499d38`. Identique au tree du commit HEAD teste de #509 `54dd57dc6f4bde55224c3bc96f954e62460f8f26`.
- [PR #509 MERGED](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/509), squash protege `--match-head-commit`.
- [CI PR #509 #41851820058](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41851820058) **4/4 SUCCESS, attempt=1, event=pull_request**, SHA exact; trois workflows natifs [Owned Pipes #41851820339](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41851820339), [Private Rust #41851820435](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41851820435), [Private Preparation #41851820251](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41851820251), **SUCCESS attempt=1**.
- [CI independante push/main #41852887021](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41852887021) : **4/4 SUCCESS, attempt=1, event=push, head exact `767dda...`** (Rust Windows, Rust Ubuntu, Dependency policy, Release evidence guard).
- [Recu d'audit #323 apres #509](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6634058656), avec logs SHA256 et limites des tests.
- **Attention :** la PR documentaire issue de cette passation fera avancer `main` et declenchera une *nouvelle* CI. Au prochain chat, controler cette nouvelle CI et le SHA reel avant tout code.

## 2. Cinq corrections Windows successives (#505–#509)

| PR | Effet exact | Limite toujours presente |
| --- | --- | --- |
| [#505](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/505) | Fermeture de racine Win32 deja creee/suspendue quand la creation du Job echoue : tenter terminate via original PROCESS HANDLE, attendre et signaler l'incertitude si l'exit n'est pas confirme | Pas de verrou atomique de revoke |
| [#506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/506) | Meme garde lors de l'echec reel `AssignProcessToJobObject` (HANDLE sans `PROCESS_SET_QUOTA`), RED/GREEN OS et 12 repetitions | Pas de preuve service-owned sur toutes autorites |
| [#507](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/507) | `ProcessManager` refuse un `start` deja stoppe au dernier point pre-spawn et libere alors sa reservation | STOP intervenant *apres* cette lecture n'est toujours pas synchronise avec CreateProcessW |
| [#508](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/508) | Un wrapper sans HANDLE qui repond `start_kill=Ok` n'est plus assimile a une preuve de mort ; `termination uncertain` explicite | Wrapper opaque peut laisser une racine vivante hors Job |
| [#509](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/509) | Une erreur native *typee* de terminaison incertaine APRES creation conserve le `JobRecord`, tous quotas, `TerminationUncertain`, et l'ID en retour MCP | Conserver le quota N'EST PAS tuer la racine ni prouver les EOF |

**Ne pas ecrire « #317 corrige » :** il existe plusieurs protections partielles et le ticket administratif historique ne vaut pas preuve d'issuer complet. Le blocage V1 reste porte par #323.

## 3. Derniere correction #509 : preuve RED/GREEN, sans faux succes

Avant correction : le `ProcessManager::start` reserve un `JobRecord`, puis toute erreur `CommandWrap::spawn` provoquait `remove_job()`, meme si un vrai processus Windows suspendu avait deja ete cree. Un wrapper test opaque a retourne un arret `Ok` non effectif. Le test a observe le processus reel avec un PID non nul et `try_wait()==None`, puis la suppression injustifiee du quota : **RED exit101 ; 0 reservation contre 1 attendue**.

Apres : `UnconfirmedWindowsChildTermination` est un *type* d'erreur introduit dans `optic-bridge-windows`, pas une chaine analysee. Pour ce cas seulement, `optic-bridge-runtime` conserve le record et les ressources ; marque `stop_requested=true`, `TerminationUncertain`, sortie incomplete et **aucun EOF physique invente** ; retourne `UnconfirmedWindowsSpawn { job_id, source }`. Cote MCP : `optic.process_spawn_termination_uncertain`, avec `job_id` pour interrogation du proprietaire.

Le cas contraste lance la vraie creation Windows sur un fichier `.exe` volontairement invalide : `CreateProcessW` echoue **avant** creation et la reservation est liberee (controle GREEN reel). Les fixtures ont un cleanup d'urgence **test-only**, ce qui ne constitue pas une garantie native de production.

**Preuves locales Windows 10, Rust `+1.99.0 --locked --offline` :**
- 12/12 repetitions d'echec native incertain ; 12/12 repetitions d'echec normal de creation ;
- crate `optic-bridge-windows` 82 PASS / 4 ignored ;
- crate `optic-bridge-runtime` 475 PASS / 11 ignored ;
- workspace entier **810 PASS / 15 ignored**, tant normal que `--all-features` ;
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `git diff --check` et staged check verts ;
- SHA du RED : `2D2FE34DB5D4FF63E11703A8A8FCD5BC742664190B27740EC1C47256B446F426` ; details supplementaires dans le [recu #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6634058656).

## 4. Architecture actuelle et verrou P0 reel

Pieces existantes mais **non connectees a une autorisation native complete de GitRead** :
- `crates/optic-bridge-runtime/src/git/native_inflight_calls.rs` : suivi d'appels Win32 en vol ; un ticket n'est **pas** un droit de Create/Resume.
- `crates/optic-bridge-runtime/src/git/native_status.rs` : enregistrement et preparation, custody native encore **inerte**.
- `crates/optic-bridge-windows/src/job.rs` : Job kernel borne, racine originale epinglee, creation suspendue, confirmation borne et collecte des deux EOF dans les parties deja testees ; **ne pas confondre ces essais avec une voie GitRead activable**.
- `crates/optic-bridge-runtime/src/process.rs` : gestion ordinaire des quotas/job, correction #507 et #509, mais **pas** de section atomique entre toutes les generations de revoke et les appels Win32.
- `crates/optic-bridge-runtime/src/session_registry.rs` et `task_lease_registry.rs` : droits et generations originaux ; politique et host stop forment d'autres autorites distinctes.
- `GitReadService::status` de production utilise toujours **`HardenedCommandRunner` legacy**.
- [Recherche PR #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) : contre-exemple d'un vrai Git demarre apres revoke, **non corrige/non admissible**.

**Cible P0 :** issuer/guardian **service-owned, non clonable, generation-bound**, avec admission future / invalidation linearisable en face des **vrais** appels `CreateProcessW` et `ResumeThread`, pour **session + TaskLease + policy epoch + host stop + source Git/repo/executable epinglee + deadline absolue**. Il ne doit jamais laisser un appel Win32 qui revient tardivement reprendre ou publier un enfant deja revoque.

Ne pas bloquer un mutex de SessionRegistry/TaskLeaseRegistry pendant un appel Windows potentiellement bloquant. Compter et drainer ou reconcilier les appels *en vol*, armer un watchdog du Job original et conserver le vrai proprietaire (HANDLE original) jusqu'aux preuves suivantes : **racine signalee, Job vide, stdout EOF physique, stderr EOF physique, aucun lancement/reprise possible apres fermeture de l'autorite**. Le retour `TerminateJobObject=Ok`, un `start_kill=Ok`, un `Drop`, un timeout ou des compteurs seuls ne suffisent jamais.

Echecs de mutex, IDs recycles, deadline, cancel multi-sources, crash et inconnus restent **fail-closed et sans faux recu**. Ne pas raccorder MCP GitRead natif avant la preuve complete et la correction du vrai RED #355.

Lire [protocole d'issuer/revoke](ISSUE317_NATIVE_LAUNCH_REVOKE_PROTOCOL_2026-11-02.md), [preuve root/EOF](ISSUE317_NATIVE_REAL_ROOT_EOF_2026-11-01.md), [witness contract](NATIVE_GIT_READ_WITNESS_CONTRACT.md), [audit #323](ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md).

## 5. Pistes concretes de prochaine PR, sans anticiper un succes

1. **Audit executable de la frontiere entre ledger et vrais Win32 Create/Resume** : ecrire une matrice de proprietes/ordres avec un vrai test OS RED qui doit echouer sur `main` actuel, avant toute integration ; capturer le compteur **running 1 test** et un OS handle reel. Une revue documentaire seule ne suffit pas.
2. Definir la section d'admission non bloquante sur les registres avec synchronisation des generations originales. Refuser la creation ou la reprise lorsque revoke precede l'engagement ; reconcilier physiquement les create/resume en vol. Tester *chaque* interleaving avant extension.
3. Emission/reprise native seulement sous custody Job + root HANDLE + watchdog/horizon, sans creer de voix d'execution parallele non surveillee. Conserver le chemin legacy jusqu'a acceptance reellement complete.
4. Une PR/CI a la fois ; interrompre si l'experimentation n'obtient pas de vrai RED, si une CI requise est rouge, ou si une preuve native reste inconnue.

## 6. Machine et LAB — ne pas toucher au quotidien

Machine autorisee : **DESKTOP-IOSTF61** (RDC ID `28a27731-11a0-4f26-bce2-0d238709b315`), Windows 10. Contacter/verifier la liste des devices avant usage, car sa disponibilite change.

Dossier de travail historique `E:\Optic-IA-Bridge` ; **ne jamais modifier le Bridge installe quotidiennement**, ni `Optic Desktop`, `REL` ou le plugin ChatGPT. LABs isoles et preuves :
- #505 : `optic-p0-native-failed-stop-uncertain-lab-2026-11-04`
- #506 : `optic-p0-unassigned-root-stop-lab-2026-11-04`
- #507 : `optic-p0-process-cancel-prespawn-lab-2026-11-04`
- #508 : `optic-p0-opaque-stop-confirmation-lab-2026-11-04`
- #509 : `optic-p0-uncertain-start-retains-capacity-lab-2026-11-04`
- passation documentaire : `optic-docs-handoff-after-pr509-2026-11-04`

Dans chaque LAB technique `_evidence` stocke scripts, logs RED/GREEN et hash ; `_target` est cache Cargo **non commite**. Ne pas les detruire pour une reprise. Les builds peuvent occuper plusieurs Go : verifier `Get-PSDrive E`, ne pas nettoyer sans inventorier et preserver les preuves.

## 7. Methode imperatives pour les futurs chats

1. Re-verifier en direct `main`, commit/tree, CI event et attempt, issues et PR en cours, scope autorise et etat des LAB, avant toute supposition.
2. Nouveau LAB Windows isole, clone depuis SHA exact. Une anomalie **observee** a la fois ; test RED de comportement d'abord (vrai kernel, vrai processus, vrai timeout si requis). Distinguer erreur de compilation/formatage, fixture et vrai RED comportemental.
3. Fix minimal, GREEN et repetitions. Tests `+1.99.0 --locked --offline` crate cible + workspace standard/all-features, fmt, Clippy strict, diff/staged. Pas de faux OS mock, pas de relachement des caps, deadlines ou assertions.
4. PR revue dans la branche, HEAD fixe, CI Github-hosted Windows/Ubuntu, dependency/release et workflows specialises **attempt 1** ; pas de runner self-hosted. Auto-revue **COMMENT** uniquement, pas d'APPROVE independant fictif.
5. Si une CI requise echoue **au premier essai**, ne pas rerun-to-green : analyser, conserver preuves, et STOP ; la correction est une nouvelle iteration avec preuves explicites. Pas de force push, bypass, release ou merge dans le doute.
6. Squash protege `gh pr merge N --squash --match-head-commit SHA`, verifier SHA et **Git tree egal au tree teste**, puis **nouvelle CI independante `push/main` 4/4 attempt1** sur exact commit. Documenter les recus dans la PR et #323.
7. **Windows V1 NO-GO** jusqu'au GO explicite appuye par tests physiques, revocation globale et acceptance #355. Ne pas fermer les issues sur la base de protections partielles.

### 8. Open items au checkpoint

[#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN** (V1 bloquee) ; [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) **OPEN** (intermittence Node/AppContainer hosted) ; [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) **OPEN** ; [PR #499](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/499) **OPEN / BLOQUEE** apres premiere CI Windows rouge, ne pas rerun/merge de force. Les etats sont dates et doivent etre reconfirmes.

Vision long terme (PAS une tache de V1 imminente) : profils/permissions fines, read/edit/create/restore/undo, Git complet, compilations et projets A->Z, tests isoles et securises, Library MCP (Figma/Blender) puis IA locale isolee. Ne pas elargir le perimetre de cette stabilisation pour la vision future.
