# Current project status

## Contrôle réel Windows #317 — 28 octobre 2026, PR #393 intégrée (recherche test uniquement)

**Baseline vérifiée avant travail :** `main=204f8619ff14c86fb3991f752eb3efed77ab2706`, [CI indépendante #40556021484](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40556021484) SUCCESS 4/4. [PR #393 fusionnée](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/393) depuis HEAD exact `14e73802b853c57b67c0b00bf169af6fd5a47793` vers `main=c81c1f5caf937695b3bbd9439f508ff9cb28f6f4`. Arbre HEAD et squash identique : `12c27901f8ec3f6dd4b8f6abfbfae44aaa16f987`. [CI PR #40649262906](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262906) SUCCESS 4/4 tentative 1, plus trois workflows natifs #40649262958/#40649262919/#40649262961 SUCCESS. **CI indépendante après fusion : [#40649847690](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649847690), en cours lors de la rédaction de cette branche documentaire ; revalider ses quatre jobs avant fusion de documentation.**

**Portée limitée :** nouveau test Windows `job::job_list_launch_tests::suspended_root_is_in_configured_job_at_createprocess_return` dans `crates/optic-bridge-windows/src/job_list_launch_tests.rs`, enregistré via `job.rs`. Avec `PROC_THREAD_ATTRIBUTE_JOB_LIST`, il crée un `cmd.exe` système suspendu, sans le reprendre, vérifie `IsProcessInJob` et le compte noyau `ActiveProcesses=1`, puis observe root terminé et Job vide après terminaison. Le test réel est passé dans le job Windows de la CI PR. **Aucun code de lancement produit, aucun GitRead MCP, aucune TaskLease/politique ou interface client modifiés.** Premiers RED : rustfmt [#40648989661](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40648989661), puis import windows-rs E0432 [#40649081510](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649081510), **pas** reproduction de #317.

**Toujours non démontré :** interruption pendant `CreateProcessW`, sérialisation des révocations session/TaskLease/policy epoch à l'engagement, réveil du processus, propriétaire et watchdog réels, descendants conservant stdout/stderr, EOF des flux, conservation sous `TerminationUncertain`, reprise, voie MCP réelle, tests d'acceptation #352/#355 et Windows 10/11 de destination. **#317/#323/#185/#319 OUVERTES ; Windows V1 NO-GO.** Les sections datées ci-dessous conservent leur valeur historique et ne représentent pas le dernier SHA.

## Dernière passation vérifiée — 27 octobre 2026, après PR #391 fusionnée

**Périmètre : stabilisation locale Windows 10/11 V1, GitHub-only, NO-GO.** `main=c208348f0b63d4810db97f9a5284b325c5b64271` après [PR #391 fusionnée](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/391) (HEAD `e7f684ae9c32b3d1026ef103d9832f8a7767a535`, arbres Git identiques `da6ff9bcbeb63ffb3a64ffc590187ae4d2be4765`). [CI PR #40526579598](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40526579598) SUCCESS **4/4 tentative 1** et trois workflows natifs #40526579328, #40526579323, #40526579729 SUCCESS. [CI indépendante push/main #40553879389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40553879389) **completed/SUCCESS, tentative 1, quatre jobs individuellement SUCCESS** sur le SHA intégré exact. La **présente PR documentaire** fera avancer ce SHA : revalider son HEAD, sa CI puis l’indépendant push/main.

**#391 ne modifie AUCUNE production** : un seul test Windows ajouté à `crates/optic-bridge-windows/src/directory.rs`. Il constate que la garde actuelle `pin_native_git_read_repository_root` interdit déjà le renommage d’un répertoire **parent** du dépôt pendant la détention du pin, puis autorise le renommage après relâchement. L’hypothèse d’une lacune n’a **pas** été reproduite. Ne pas inventer de correctif ni prétendre qu’un lancement de Git a été testé.

**#390 est intégrée** sur le checkpoint précédent `main=7f663454039b294d84cd3fb2333cbbd73372f91a`, [push/main #40520286289](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40520286289) SUCCESS 4/4. Son observation `observe_original_prelaunch_snapshot` vérifie politique GitRead, identité exécutable épinglée, registre de session initial et **même génération réelle TaskLease** (Arc du témoin de révocation, non pas son ID réutilisable). Elle reste **non exécutante** et **non synchronisée** à la création Win32.

**État effectif** : GitRead MCP actif emploie encore `HardenedCommandRunner` et `task_lease: None` sur l’ancienne voie. Les tranches intégrées #382/#385/#387/#388/#390 préparent l’action Git fixée, retiennent admission MCP session/transport, scope/TaskLease/epoch/deadline, enregistrent quotas et Job Win32 **vide**, pipes stdout/stderr avec writers parents + stdin `NUL`, pins binaire Git et dépôt canonique et un instantané du registre TaskLease d’origine. Aucun enfant créé, aucun `CreateProcessW` sécurisé intégré, aucune preuve native root-exit/Job-empty-après-exécution/EOF, aucun véritable watchdog/crash recovery ni raccord MCP natif.

**P0 :** architecturer et prouver le contrôle **synchronisé** révocation/politique/TaskLease/session/deadline/identité Git à la création native Windows, puis assurer attachement Job avant exécution, conservation des propriétaires/quotas/admissions sous `TerminationUncertain`, preuves positives root terminé + Job vide + **EOF réel** et traitement des descendants, avant toute activation du `git_status` MCP avec chemins sensibles inchangés. Les acceptations de recherche [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) (finitude temporelle/stdio) et [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) (révocation/création) restent **RED pour la V1**. #353/#358/#327 recherche isolée, jamais fusion en bloc. #317, #323, #185, #319 **OPEN** ; V1 **NO-GO**, aucune release ni accès au PC.

Le nouveau **[prompt complet à copier](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md)** est canonique ; la [passation technique](docs/engineering/HANDOFF_WINDOWS_V1_CHAT_2026-10-27.md) et [AGENTS.md](AGENTS.md) démarrent par ce checkpoint. Les sections datées suivantes sont historiques, leurs mentions « draft » ou anciennes bases ne reflètent plus le `main` courant.



## 27 October 2026 — unstarted native GitRead original-lease generation snapshot (#390 draft)

[PR #390](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/390) adds a **read-only, non-launching observation** to already-precreated Windows GitRead custody. It rechecks current TaskLease policy and pinned executable content identity through the registry, compares the **same underlying revocation record** against the original admitted TaskLease witness (`Arc::ptr_eq`, not merely the reusable opaque ID), confirms the original session registry, and rechecks the captured monotonic stop predicate before and after. The true empty Job, pinned executable/repository, stdio and original session/transport/process permits stay owned throughout. A foreign registry with the same lease ID, wrong session registry, and revoked original generation must fail closed, leaving permits held until the known-no-child custody is explicitly dropped.

**RED evidence:** test-first SHA `589388e1caae7213eb96ddb70f7e7ebd5f4def56`, [CI #40517792327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40517792327), showed Windows compiler E0433/E0599 (missing `NativeGitStatusSnapshotError` and method) plus independent Ubuntu rustfmt differences. This is *not* original runtime behavior #352/#355. Implementation `968d279326e33cc27626c4352fd7b17ef0ef7a00`; follow-up rustfmt `f4a7eb5b4d72926fea612c07d2143a734ac1f2ed`, [CI #40518650462](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40518650462), must be verified before the new documentation HEAD can be accepted.

**Still absent:** the snapshot is NOT synchronized with `CreateProcessW`; it is NOT approval to spawn. No root creation/Job assignment/resume, real EOF/Job-empty-after-run, watchdog/restart recovery, live MCP `git_status` switch, original #352/#355 acceptance or Windows 10/11 freeze proof. Keep #317/#323 OPEN and Windows V1 **NO-GO**; no release.


## Integration receipt — 27 October 2026, post #388

**Confirmed integration after PR #388 (27 October 2026):** `main=720a38833705e661049841cfd39852e770687643`. PR HEAD `0b29e833a53195f199b5d3b53f808091d020bdc5` passed [CI #40513121389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40513121389), first attempt, four jobs individually SUCCESS and all three native-specialized workflows successful. The reviewed HEAD and squash tree are identical (`1d79045f8ac60db9dd2dc2ffe6e23b72af0811f9`). Independent [push/main #40514100157](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40514100157) on that exact integrated SHA completed SUCCESS, attempt 1, all four Windows/Ubuntu/dependencies/release jobs successful.

**Scope remains pre-spawn only:** original Git executable and repository root opened and pinned by verified Windows handles, with the already counted admissions, TaskLease witness, unstarted Job and stdio. No child, final synchronized `CreateProcess` authority, native terminal/EOF proof, MCP GitRead switch, physical Windows 10/11 acceptance or release. Keep #317/#323 OPEN, #352/#355 original acceptance RED, and Windows V1 NO-GO. This documentary commit itself changes the HEAD; revalidate live `main`/`push/main` separately after any merge.


## 27 October 2026 — integrated original-target pins for Windows native Git status (#388)

[Merged PR #388](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/388) extends the already **unstarted** and counted #387 native status custody with a pinned canonical executable handle and a special READ/LIST-access root-directory handle that denies Windows DELETE-sharing rename of the repository root itself. Both opened-handle final paths must match the original service command's canonical executable/root; no caller-supplied path, argv or renewed TaskLease is accepted. The original session/transport permits and process quotas remain retained with the empty Job and its stdio handles. Existing zero-access directory-chain callers are unchanged.

**Behavioral red, preserved:** [CI #40510524394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40510524394) test-first SHA `968adc2f58a5b8c012a6455fdef2b8f053a6873c`: Windows **371 passed / 2 failed / 11 ignored** (repository renamed during unstarted custody; already-moved repository admitted); Ubuntu rustfmt separately RED. [CI #40511257834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40511257834) first pin implementation SHA `132f988131b202ad1ed5cf9122ba0c3f26945648`: Windows **372 passed / 1 failed / 11 ignored**, showing that the general zero-access directory pin did not block root rename. That finding led to the GitRead-specific READ/LIST pin on `382857c0269b7173c24bef8eb948d08ad0e558ae`: [CI #40512075888](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075888) completed SUCCESS, attempt 1; [native lifecycle #40512075880](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075880), [native Rust IR #40512075887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075887), [native preparation #40512075891](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075891) all SUCCESS on this SHA. A later documentation HEAD still requires its own CI.

**Explicit limits:** no new verified content hash at the CreateProcess boundary, no synchronized final session/TaskLease/policy/epoch/deadline check, no child/root assignment/resume, no watchdog/real EOF/Job-empty-after-child, and no live MCP Git status switch. Original #352/#355 acceptance stays RED; #317/#323 OPEN, Windows V1 **NO-GO**. No release.


## 27 October 2026 — real, unstarted native Git status Job custody (#387)

[Draft PR #387](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/387) adds a **no-spawn**
step after the integrated #385 registered native status. On Windows, the
original `RegisteredNativeGitStatus` consumes its already counted MCP session
admission, transport permit, registered TaskLease witness, action horizon and
process-store reservation into `PrecreatedNativeGitStatusCustody`. The
platform `PrecreatedNativeReadCustody` constructs a **real kernel Job Object**
with the existing process/memory/CPU ceilings and kill-on-final-handle-close,
two owned stdout/stderr anonymous pipes with parent writers still open, and an
opened `NUL` stdin. A second original-generation stop check occurs after
precreation. An expired/revoked admission fails closed without a child; only
the **known-unstarted** owner may use ordinary RAII cleanup.

This is **not** the native process owner or the live MCP Git status path:
there is no CreateProcess method, no root, assignment, resume, pinned Git
identity, synchronous final policy/revocation/epoch gate, positive post-run
Job-empty/root-exit/EOF evidence, or host-loss recovery. A zero process count
on a never-started Job and idle pipe with no EOF prove only precreation.
Live MCP `git_status` still uses `HardenedCommandRunner` and `task_lease: None`.
The original real behavioral acceptance #352/#355 stays RED. #317/#323
OPEN, Windows V1 NO-GO; #353/#358 remain separate research.

**Test-first** `35223758a95a9d6874b8bae42e8a0e789cd6aa2d`,
[CI #40506640216](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40506640216):
Windows compiler E0433/E0599 absent interface and Ubuntu rustfmt, **not**
native runtime failures. The first implementation `cce80c41998a75d7b2172a33875c2cde63d71daf`
has [format RED #40506898619](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40506898619)
while its native lifecycle, private Rust compilation and native preparation
checks passed. Formatter correction at
`9f2c9600ad9ad7e85b045a94261fa9fa566bc042` triggers
[CI #40507126521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40507126521)
and separate native workflows. [CI #40507126521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40507126521)
passed Ubuntu, dependency and release gates, but was **Windows Test RED**:
a platform unit-test incorrectly called filesystem `File::metadata()` on
the `NUL` device (38 passed, 1 failed, 4 ignored in that test binary).
The real Job/pipes and admitted-session revoke tests passed. This was
a **test observation failure**, not #352/#355 native-run acceptance.
Commit `88dc28a8c9c13dbdff74359f02372d6665e3603e` removes only
the unsupported metadata check. [CI #40507901498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40507901498)
and the later **final PR HEAD** must be checked independently, including
each native workflow, before merging. The independent post-merge
`push/main` is also required and not implied by a PR check.



## 27 October 2026 — integrated registered native Git status accounting (#385)

[Merged PR #385](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/385) adds a Windows-only, **non-executing** handoff from the exact
`PreparedNativeGitStatus` of #382 to the existing native private process
registry. The registry reserves the process count, global/session output,
memory and CPU capacity under the **original TaskLease workload class**, and
keeps the actual original counted MCP session admission, transport permit,
revocation watch and monotonic deadline with the registered action. The process
manager root must equal the canonical service GitRead root. A revoked/expired
preparation or foreign root is denied before reservation.

The new registered value holds an existing `NativeProcessReservation`, whose
`LimitedJobObject` only **configures future limits**; this stage does **not**
create a kernel Job, root process, pipes or stdin/EOF observation, and does not
grant a synchronized launch permit. Only synthetic no-child cleanup is tested.
No `git_status` MCP handler or legacy `HardenedCommandRunner` path changes.
Do not treat this stage as production native Git execution or termination
proof. Next P0 is a service-owned Windows Job/root/pipe guardian and final
revocation-synchronized creation gate, before enabling an actual MCP status
route. #352/#355 remain RED; #317/#323 OPEN; Windows V1 NO-GO.

Test-first commit `ab474c0b228e7f42cba2848db12360d0895108c3`:
[CI #40501489580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501489580)
failed on Windows E0433/E0599 (absent interface) and Ubuntu rustfmt; this is
**compilation/formatting**, not a runtime reproduction. Intermediate
`fbb610e89e855fed6322134910c57491451dfcb9` failed formatting; its
other long jobs were cancelled after the next commit. Implementation and
rustfmt head `9a66557d29cfe66da7f06a68ca80e68be0576a02` passed
[CI #40501987327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501987327)
4/4 and [native preparation #40501987334](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501987334).
These receipts prove only their exact candidate SHA, not any later HEAD or
independent integrated-main result.

**Post-merge verification:** `main=e5017c48de5d4d6433f92e3117d63f1677aecb59`;
independent [push/main CI #40503510236](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40503510236)
`completed/success`, attempt 1, all four Windows/Ubuntu/dependency/release jobs
on that SHA. The integrated tree `cb7889cfc4f9f2de995d27c0d1765cbc2fd30a5f`
matches the reviewed PR head tree exactly. A squash auto-close inadvertently
closed #317 at 12:24:27 UTC; it was explicitly reopened at 12:25:27 UTC and
the [reason recorded](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6441832785).
#317/#323 OPEN, V1 Windows NO-GO. Avoid `fix(#317)`-style merge titles
before all acceptance gates are satisfied.


## PASSATION ACTIVE — CHAT CLASSIQUE — 27 octobre 2026

[Passation technique actuelle](docs/engineering/HANDOFF_WINDOWS_V1_CHAT_2026-10-27.md)
et [prompt complet à copier](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md).
Avant cette documentation : main `75947c9b4fb72fab2f3d45f34f5b750f09bc31b2`,
[push/main #40477277322](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40477277322)
SUCCESS 4/4, tentative 1. Vérifier le nouveau SHA et sa CI après cette PR.
#380 admission worker possédée, #381 échéance d'action, #382 préparation
service-bound et #383 correction EOF incertain intégrées. Le handler GitRead
reste legacy : aucun propriétaire Git natif enregistré/actif, pas de preuve
root/Job/EOF/crash. Reprise en Chat via GitHub, sans lancement Codex/Work ni
accès machine implicite. #352/#355 ROUGES ; #317/#323 OPEN ; Windows V1 NO-GO.


## 27 October 2026 — uncertain native reservation EOF correction (#383)

Native private-worker reservations preserve original stream observations under
`TerminationUncertain` instead of synthesizing stdout/stderr EOF. The Windows
test-first [#40476042507](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40476042507)
failed the exact reservation-state assertion (366 pass / 1 fail / 11 ignored)
after format/Clippy passed. This is a simulated state test using the real store,
not native process/pipe fault injection.
[Contract](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md);
final-head and independent-main receipts: [#383](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/383).
Baseline `main=d9d004b2bbd65c4f7cc520abfde3beb7963f0095`,
[independent #40474172602](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40474172602)
SUCCESS 4/4 attempt 1. Native Git guardian/routing remains unimplemented.
#352/#355 RED; #317/#323 OPEN; Windows V1 NO-GO.


## 27 October 2026 — service-owned native status preparation (#382)

The status preparation boundary now derives repository, executable, fixed
arguments/environment and exact output budget from GitReadService and the
original ActionEnvelope. It consumes #380's existing owned permits via the
registered preflight and retains an immutable envelope plus original deadlines.
Four non-executing tests cover identity/budget denials and resource custody;
[contract and test-first compiler evidence](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
Final-head/post-merge receipts: [#382](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/382).
Baseline main `588f50f0a3fee4e2f6b1c08c21db4950888d6d77` integrates #381,
[independent CI #40471474314](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40471474314)
SUCCESS 4/4 attempt 1. This change does not route live MCP to native execution.
Next: registered Windows guardian custody before creation and synchronized final
launch authority. #352/#355 RED; #317/#323 OPEN; Windows V1 NO-GO.


## 27 October 2026 — preserve the native GitRead action horizon (#381)

The admitted native GitRead preflight captures the original absolute monotonic
`ActionEnvelope.resources.timeout_ms` deadline with checked addition. Its stop
predicate now also stops at that boundary when transport/session/lease outlive
the action. Expiry requests stop; it does not drop either counted reservation
or prove native termination. Both fresh and transferred worker admissions use
the same corrected finish path. Legacy GitRead policy/runner is unchanged.

**Behavioral test-first:** `ff2ad7d472663a182e2b241d2afc8345adfc6d70`,
[CI #40470425246](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40470425246),
attempt 1, passed formatting and strict Clippy, then failed the exact 100ms
action-boundary assertion on Ubuntu (289 passed / 1 failed) and Windows
(361 passed / 1 failed / 11 ignored). This is runtime evidence for the inert
contract, not real Git/Job/EOF acceptance. The regression also checks that an
expired action retains transport/session capacity until explicit no-process
fixture cleanup. Final-head and independent-main receipts are maintained in
[PR #381](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/381).

Baseline `main=ab45498c32b2cb9cdfddfdeaa32f0230447d9de0` already integrates
[owned MCP worker #380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/380),
with independent [push/main #40467892414](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40467892414)
SUCCESS 4/4 at attempt 1. No native launch activation. Next remains registered
service-owned Windows Job/root/pipes custody, canonical TaskLease and final
synchronized launch authorization. Original #352/#355 remain RED;
#317/#323 OPEN, Windows V1 NO-GO.

## 27 October 2026 — MCP owned read worker candidate #380

Candidate #380 adds a live owned worker admission seam and routes Git reads
through it without second admission or changes to sensitive-output checks.
Two non-executing regressions cover transferred capacity beyond worker completion
and denial of native authority without a TaskLease. Test-first
[CI #40466543183](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466543183)
failed compilation (E0599, absent interface); initial formatting failure
[#40466491881](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466491881)
is preserved. Implementation candidate `ba05382e353f401198dcb979f48701b8d442e05b` passed
[CI #40466899740](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40466899740)
SUCCESS 4/4 at attempt 1. Final-head and post-merge CI receipts are tracked in
[PR #380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/380).
Baseline main `ebe753ee0ee3f8bead396170e272873dcaa35538` has independent
[push CI #40365242899](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40365242899)
SUCCESS 4/4, attempt 1. No native launcher or termination proof; legacy runner
and policy remain. #352/#355 RED; #317/#323 OPEN, Windows V1 NO-GO.
[Owning contract](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).


## PASSATION INTER-CHAT CANONIQUE — 26 octobre 2026

**Documents actifs :** [Passation canonique Windows V1 du 26 octobre](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-26.md) et [prompt complet à copier](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-26.md). Avant cette PR documentaire : `main=c058ccf3f9db18c7d67194d117af99cc3e0cea52`, [CI indépendante push/main #40358111878](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40358111878) **SUCCESS 4/4 au premier essai**. Cette PR fera avancer `main` : vérifier le nouveau SHA et la CI `push/main` après fusion. #369/#371/#373/#375/#377 sont des contrats GitRead **inertes** ; production MCP GitRead toujours legacy ; #327/#353/#358 recherches isolées, originaux #352/#355 **ROUGES** ; issues #317/#323/#185/#319 ouvertes ; Windows V1 NO-GO, aucun accès PC/Optic Desktop/REL. Prochaine priorité réelle : transférer la vraie admission du worker MCP + RequestPermit + TaskLease dans un propriétaire natif Job/stdio préenregistré ; ensuite réussir les tests d'acceptation #355 et #352 sans modifier leurs seuils, et les critères V1 #323.

---

## 26 October 2026 — transferred native GitRead admission is registry-bound (#377)

- `main=a18faaea8bcf8b97f019873b7b6a8d48b6587d57` integrates [PR #377](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/377), a **non-executing** future native GitRead ownership seam. `NativeGitReadOwnedPermits` consumes a genuine **already-counted** `OwnedSessionAdmissionPermit` and real `RequestPermit` from the worker; `finish_owned_native_git_read_preflight` refuses an admission belonging to a foreign `Arc<SessionRegistry>` despite an identical opaque session handle and retains all original-generation stop/deadline/TaskLease policy checks. Existing `begin_admitted_native_git_read_preflight` delegates through the same checks. No production Git launch, native Job, pipes or new permissions.
- Exact final PR head `5692a271a33affa9db73e4d3fcf747419d07cc61`: [CI #40355535498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40355535498) **SUCCESS 4/4 first attempt** Windows/Ubuntu/dependency/release; [native preparation #40355535687](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40355535687) SUCCESS. Preserve test-only format RED [#40354691682](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40354691682), formatted compiler RED [#40354870468](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40354870468) and first Clippy warning RED [#40355329042](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40355329042).
- Independent [post-merge push/main CI #40356405717](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40356405717) on `a18faaea8bcf8b97f019873b7b6a8d48b6587d57` was completed **SUCCESS 4/4 at attempt 1** on the exact integrated commit, independently verified after squash; previous independent main #40354148146 on `5fd8e8d7a07b696a447895d81b99a1d9959df697` was SUCCESS 4/4 first attempt.
- **Original native acceptance RED:** [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) actual wall-clock pipe completion and [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) real Git spawned after published revoke. #327/#353/#358 are isolated research; #185 AppContainer separate. **#317/#323 OPEN; Windows V1 NO-GO.** No user PC or unrelated repo access.

---

## 26 October 2026 — native GitRead admission also owns MCP transport permit (#375)

- Production `main=99c639a0169de72c6bd692ac8493cadef6a1e638` integrates [PR #375](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/375): inert, noncloneable `NativeGitReadAdmittedPreflight` now consumes the real `RequestPermit`, retaining the transport slot and original monotonic request deadline together with genuine counted session admission and registry-verified TaskLease/policy/stop witnesses. An already-expired permit is refused; failed admission releases the request slot. A future registered native guardian must retain the entire result until *real native terminal proof*. No live MCP `git_status` rewrite, native child, Job/pipes activation or permission change.
- **Test-first evidence:** initial test-only [format fail #40350852162](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40350852162); formatted test-only `a244941fda821bc2053269fac715b6d6827857e1` [compiler RED #40351046826](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40351046826) E0061 x2 and E0599 before implementation. Fixed exact-head `62431d8d34ec56df6d01d93f8c1d9adbbfde8933` [CI #40351438430](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40351438430) **SUCCESS 4/4 first attempt** Windows/Ubuntu/dependency/release; [native preparation #40351438537](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40351438537) SUCCESS.
- **Independent integrated** [push/main #40352227922](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40352227922) for the exact `99c639a0169de72c6bd692ac8493cadef6a1e638` completed **SUCCESS 4/4 at attempt 1** on exact integrated commit, independently verified after squash; previous [main CI #40350315919](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40350315919) succeeded 4/4 at `55bbf35d44a9fc330e009dac23f796a6486683e8`.
- **Still RED independently:** [#352 real stdout/stderr/stdin wall-clock](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352), [#355 post-published-revoke actual Git spawn](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355). #327/#353/#358 research remains isolated and #185 AppContainer is separately tracked. **#317/#323 OPEN; Windows V1 NO-GO.** No user computer access.

---

## 26 October 2026 — TaskLease cleanup generation stop, PR #373

- Production `main=621bf4ed977e99a1dd8825f8a2cb28a0e04aab91` includes [PR #373](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/373). `TaskLeaseRegistry::remove_session` now publishes the **original lease generation's** revoke signal *under its registry mutex before physically deleting the record*, even when called without a prior `revoke_session`. Other sessions are untouched and reuse of an opaque lease ID cannot re-enable the old watch. This is fail-closed watch behavior, **not native process termination proof**, not a change to the ordinary lifecycle's ordering and not a new MCP permission.
- **Test-first evidence:** original test SHA `b5e3631a9399106f79893a5d246f49e35727aa69` [CI #40340672365](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40340672365) reproduced the exact runtime assertion `physical cleanup must publish original-generation stop` (Ubuntu: 284 pass, 1 targeted failure). The initial formatting failure [CI #40340364259](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40340364259) is also preserved. Fixed SHA `74fa640b1bbb4576a39d392cbd9ee724a301d23e`: [CI #40341168843](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40341168843) **SUCCESS 4/4 at attempt 1**, [private native prep #40341168733](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40341168733) SUCCESS.
- Independent integrated [push/main #40342061604](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40342061604) on the exact `621bf4ed977e99a1dd8825f8a2cb28a0e04aab91` completed **SUCCESS 4/4 at attempt 1** on exact integrated SHA, independently verified after the squash merge; previous [push/main #40340011075](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40340011075) passed on `300b68cf89ec165ce9b6ef184c32c954bf6cc007`.
- Unchanged V1 blockers: original #352 real pipe wall-clock and #355 real Git post-revocation-spawn checks **RED**; #327/#353/#358 experimental owners not merged, #185 AppContainer separate. Still need native admission-to-owner wiring, proper Job registration/termination proof, true stdout/stderr/stdin EOF and bounded cancellation. **#317/#323 OPEN, Windows V1 NO-GO.** No operator PC access and no other repositories modified.

---

## Latest integrated preflight — 25 October 2026, after PR #371

- **`main=2148281b2960e777a759d39682b4a831e61d47b6`** integrates [PR #371](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/371), a **non-executing** native GitRead prerequisite: trusted application session handle -> counted `OwnedSessionAdmissionPermit` -> registered TaskLease/policy witness from #369. The non-cloneable result observes **both original-generation session and lease revocation/monotonic expiry**, and guards against cross-session ActionEnvelope spoofing. Failed preflights drop admission; a future native Job guardian must retain successful admission until independently proven termination. No MCP handler changed and no native process starts through this helper.
- **Exact candidate CI** on `04350c2e1190335398c7e8df5a7286e1a09b083e`: [general #40196130286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40196130286) **4/4 SUCCESS first attempt** (Windows/Ubuntu/dependencies/release), and [private preparation #40196130318](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40196130318) SUCCESS. Original rustfmt and Clippy failures [#40195715393](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40195715393), [#40195817977](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40195817977) and [#40195985392](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40195985392) remain preserved.
- **Independent post-merge push/main [CI #40196515745](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40196515745)** on exact `2148281b2960e777a759d39682b4a831e61d47b6` completed **SUCCESS 4/4 first attempt** (Windows/Ubuntu/dependency/release) on the exact integrated commit, independently verified after the PR squash.
- **Blockers unchanged:** [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) real stdio wall-clock [RED #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209), [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) real post-revocation Git spawn [RED #40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203), isolated #327/#353/#358 research (with #358 stacked on #353), separate #185 AppContainer and #319 first MCP response, #176/#182 sensitive provenance, Windows 10/11 physical/installer/update/rollback #323. **#317/#323 OPEN, Windows V1 NO-GO.** No operator computer access.

---

## Current integration — 25 October 2026, after focused PR #369

- **Production `main=60da40a1c8ad5109eb5abd30c5daae045a404f9a`** includes [PR #369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/369), a non-executing registered-TaskLease + typed-policy + original-revocation-witness preflight for *future native* GitRead. The original MCP `authorize_git_read` remains lease-less and `GitReadService` still calls `HardenedCommandRunner`; no native owner is activated. Detailed [scope, non-goals and obligations](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
- **Exact PR evidence:** head `49d375972583cad769403b0f7f8d5d4aa96e9525`, [CI #40194006038](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40194006038) **SUCCESS 4/4 first attempt** Windows, Ubuntu, dependency and release, plus [private preparation #40194006046](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40194006046) SUCCESS. The earlier first-run rustfmt failure [#40193634170](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40193634170) and Windows pinned-file fixture cleanup failure [#40193787323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40193787323) are retained, not rerun away.
- **Independent post-squash `push/main`:** [CI #40194463604](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40194463604) on exact `60da40a1c8ad5109eb5abd30c5daae045a404f9a` completed **SUCCESS 4/4 at attempt 1** (Windows, Ubuntu, dependency policy and release guard), verified independently after the squash merge. Previous independently verified [main #40192736394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40192736394) was 4/4 green at `5cc3cee13b1dfc3b5d929d4e27a24ceff2204a0c`.
- **Unresolved:** [#352 real pipe-wall-clock CI #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) and [#355 real post-published-revoke Git spawn CI #40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203) remain **RED**. Experimental #327/#352/#353/#355/#358 are not merged by #369; #358 remains stacked on #353. Separate [#185 AppContainer](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) remains open. **#317/#323 OPEN, WINDOWS V1 NO-GO.** No user-PC access or modifications to other repositories.

---

## Handoff verification — 25 October 2026, before documentation PR

**Active continuation is [HANDOFF_WINDOWS_V1_2026-10-25.md](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-25.md), with [full successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-25.md).** Immediately before this documentation update, production `main=7c72ecfb8886fb34be2cc7ef9f2e1070f73c08da` passed independent [CI #40188090038](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40188090038) Windows/Ubuntu/dependency/release **4/4 first attempt**. The documentation merge will change main; check the new SHA and its own independent push CI.

Native research draft [#358](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/358) is at `f431c3efde2e5d9512eb1715ad3475196f18bf1d` (stacked on #353), after [#366](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/366) merged **only into research**. Exact research-head CI: [general #40187179915](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179915) 4/4, [native #40187179856](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179856) 2/2, [private #40187179916](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179916) SUCCESS, first attempt. The verified original-generation TaskLease stop witness does not make native execution live. Five research PRs #327/#352/#353/#355/#358 remain unmerged into main. **#352 actual stdio wall-clock and #355 Git spawned after published revocation are RED** in their dedicated original acceptance. #185 independent AppContainer intermittence remains open. **#317/#323 OPEN; Windows V1 NO-GO.**

---


## Latest research reconciliation — 25 October 2026 after #366 (NOT production)

- **Production remains `main=288fa176251ac9038a8c6d1b8df74c6eac724d3c`**, independently [CI #40185881982](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40185881982) 4/4 first attempt. No native GitRead launcher or release-readiness status changed in main.
- [Research-only PR #366](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/366) was merged **into draft research PR #358, not into main**. #358 exact HEAD `f431c3efde2e5d9512eb1715ad3475196f18bf1d`, still stacked on #353 `b091dc280bfd186694e7c1d1bea743a20f6746c2`. Research-only changes port the production #364 atomic verified TaskLease + original revocation witness to the divergent native owner. The owner now stores and checks original signal identity before Git status start, with a real Windows synthetic `revoke → remove → same lease ID reuse` no-spawn and retained-Job-capacity regression.
- **Integrated research SHA CI first attempt:** [general #40187179915](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179915) 4/4 green Windows/Ubuntu/dependencies/release; [native #40187179856](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179856) 2/2 green; [private preparation #40187179916](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179916) green. PR #366 head `6d793094…` also green on its original SHA after preserving first Rustfmt and Ubuntu unused-Windows-method errors. First-failure evidence and exact run links are recorded on #366, #358 and [issue #317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317).
- **Still RED/NO-GO:** #355 original post-revocation real Git-spawn acceptance, #352 true stdin/stdout/stderr wall-clock and EOF acceptance, separate #185 Node/AppContainer intermittence, actual MCP→native owner launch and host-crash recovery, plus Windows 10/11 packaging/update/rollback #323. This research branch contains a *duplicate* of the live #364 helper because ancestry diverges; reconcile surgically before any production promotion. Five experimental PRs #327/#352/#353/#355/#358 remain unmerged into main. **#317/#323 OPEN; Windows V1 NO-GO.** No operator-PC access, no other repositories.

---

## Latest integrated Windows V1 checkpoint — 25 October 2026, after PR #364

> Re-query GitHub before any action; all older SHA/CI snapshots below are historical, not reset or merge instructions.

- **Production `main=88593315d84571585df7e5d4639f34fc5491f1b9`**: independent [push/main CI #40185041267](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40185041267) **SUCCESS 4/4 at attempt 1** (Windows, Ubuntu, dependencies, release).
- **Integrated [PR #364](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/364)**: `TaskLeaseRegistry::get_active_with_revocation_watch` atomically captures a verified, active TaskLease (including configured executable identity check) and read-only original-generation revocation signal under one registry lock. `get_active` retains its old return contract, delegating to this shared verification. Synthetic tests prove the witness cannot retarget after physical removal and even same-ID reuse; wrong session/expiry/revocation and Unix executable-content drift deny. No new Win32 unsafe code or process behavior.
- **First failures preserved:** test-only `8582d48…` failed Rustfmt; format-only `26ad44d6050a58cd0449a9b1baf025678677292c` demonstrated compiler **E0599** on Windows and Ubuntu ([CI #40184509735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40184509735)). Fix head `16e3605da3d9f59c0b648e234b2c473335ade9ae` passed [full CI #40184694171](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40184694171) 4/4 attempt 1 and [native preparation #40184694195](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40184694195) SUCCESS.
- **P0 remains unresolved:** Native owner research #353/#358 does not yet consume the new verified watch; GitRead MCP still uses `HardenedCommandRunner` rather than a service-owned native Job. A witness with `is_revoked() == false` is *not* launch approval. #355 post-revoke real Git spawn and #352 actual stdin/stdout/stderr wall-clock acceptance remain RED, separate #185 Node/AppContainer Windows intermittence remains open (including original [main RED #40183165673](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40183165673)). Native root/Job/EOF and host-crash recovery and V1 Win10/Win11 installer/update/rollback are unproved. Five research PRs #327/#352/#353/#355/#358 remain separate, no bulk merge. **#317/#323 OPEN; Windows V1 NO-GO.** No access to operator PC, Optic Desktop or REL.

---

## Canonical Windows V1 checkpoint — 25 October 2026, after native GitRead policy preflight #362

> Dated verification of production and release evidence. Re-query live GitHub; older checkpoint headings below remain historical, not the present source of truth.

- **Integrated `main=739bdd5de3a86d72eaa251cb4f9f311e69b33e0e`**, code [PR #362](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/362) squash-merged. This adds a **policy-only, fail-closed** `PolicyEngine::evaluate_native_git_read_preflight` with typed `Effect::GitRead` + exact TaskLease ID/session/policy epoch, explicit GitRead capability, generic Repository and canonical-root `GitReadRepository`, exact FixedTool Git executable, nonzero finite resource ceilings and watchdog horizon within both session and lease deadlines. It does **not** launch Git, verify OS handles/pins, prove registry revocation, or replace existing `authorize_git_read` / `HardenedCommandRunner`.
- **Test-first evidence:** initial test-only formatting red preserved separately; formatted research head `a2b5fc23…` produced E0599 missing preflight method in Windows and Ubuntu [CI #40182668628](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40182668628). Fixed exact PR head `39abd8276cd6cd1254163a2d1b56ea6a87148cae` passed [CI #40182783889](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40182783889) **4/4, attempt 1**, including three new policy regression tests.
- **Independent post-merge [push/main CI #40183165673](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40183165673) FAILED** on the exact `739bdd5…` SHA: Windows job `Test` failed on **separate #185 Node/AppContainer** `capability_free_appcontainer_denies_node_loopback_connection` with `status=TimedOut`, `terminal_observed_ms=10111`, empty stdout/stderr, expected `Exited`. Three subsequent serialized diagnostic-only checks succeeded and do **not** clear the original red. Ubuntu, dependency policy and release guard were green. No rerun-to-green, containment weakening, test skip or timeout inflation. Evidence recorded on [issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185).
- **Remaining P0:** research PRs [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327), [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352), [#353](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/353), [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) and [#358](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/358) remain unmerged; #358 remains stacked/reconciled research only. #352 real pipe wall-clock and #355 revoke-before-Git-spawn acceptance remain RED. Next: bind actual MCP action/lease/registry to the native owner under a pre-spawn stop gate, prove stdout/stderr **and stdin**, root+Job-empty+both EOF, retention on uncertainty, guardian crash recovery and real Windows 10/11 release. **#317/#323 OPEN, Windows V1 NO-GO.** No standing permission to access the operator PC, and no Optic Desktop/REL changes.

---

## Current integrated Windows V1 checkpoint — 25 October 2026, after PR #360

**Production `main=0b2fcc6ee5329d3112500e79fc2a724eb9736c31`**: independently confirmed [push/main CI #40181293600](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40181293600) **SUCCESS (attempt 1, Windows/Ubuntu/dependencies/release 4/4)**. This checkpoint supersedes older dated heads; re-query live GitHub before action.

- [PR #360](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/360) merged only the inert `LeaseScope::GitReadRepository { canonical_root }` type and its non-interchangeability regression with generic `Repository`. First test-only head `987cfdf7…` produced intended compile E0599 failures on Windows and Ubuntu ([CI #40180872178](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40180872178); overall cancelled after new push but first failing job logs preserved). Corrected head `5335519401932629eb93897a5179b744c4d70e09` passed [CI #40181004649](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40181004649) **4/4 first attempt**. No new policy permission, active native launcher or process guarantee; real `GitReadService` continues to use `HardenedCommandRunner`.
- [Draft #358](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/358) has reconciled research HEAD `735619a2c39898e612c98a367718b614fc21f804`, retaining native #353 `b091dc28…` ancestry without force-push. Dedicated [native CI #40179682571](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40179682571) and [preparation #40179682501](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40179682501) passed; full [CI #40179682499](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40179682499) **FAILED first attempt, separate #185 Windows Node/AppContainer timeout**, preserved in PR #358 and issue #185. The enum variant is now also on production main; reconcile overlapping changes before any new research integration.
- **Open #317/#323, Windows V1 NO-GO.** Five research PRs #327/#352/#353/#355/#358 remain unmerged. Dedicated #352 actual pipe wall-clock and #355 post-revocation Git spawn acceptance are RED; #185 intermittent AppContainer is independent. An exact repo marker is necessary but not sufficient: next P0 must bind real MCP `ActionEnvelope`/policy, a nonexpired scoped TaskLease, native Job ownership before spawn, stop-before-drain, valid root+Job+EOF receipts, stdin and crash recovery. No speculative release or operator-PC access.

---

## CANONICAL LIVE CHECKPOINT — 25 OCTOBER 2026, AFTER #356/#357 AND RESEARCH #353/#358

> This is the newest dated checkpoint, not permission to overwrite or reset old work. Earlier "latest" sections remain historical. Always verify live GitHub again on takeover.

- **Production**: `main=d7d4d50adf95b6a3cf4a5df0ef06a6ba526d6810`, independently confirmed [push/main CI #40173338835](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40173338835) SUCCESS, attempt 1, 4/4 Windows/Ubuntu/dependency/release. Merged [#356](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/356) introduced the common sanitized GitRead command builder; merged [#357](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/357) introduced its fixed typed `git status` command spec. **Production `GitReadService` still calls `HardenedCommandRunner`**, not any research Windows Job/guardian. Neither merge fixes #317 process lifetimes.
- **Issues**: [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) OPEN, [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN; Windows V1 **NO-GO**. During #357 integration GitHub auto-closed #317 due to accidental closure-keyword wording; corrected and reopened. Never write auto-closing issue keywords plus issue numbers in PR descriptions/commit messages.
- **Exactly five open experimental PRs at checkpoint**: [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) old native ownership experiment, [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) real pipe wall-clock **RED**, [#353](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/353) pre-spawn native owner/ledger and restricted real `git status` tests GREEN, [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) post-revocation Git spawn **RED**, [#358](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/358) **stacked on #353, NOT main**, canonical repository-pinned TaskLease tests GREEN. No bulk merges.
- **#352 head `2fa45d6b2d1f3dd39309ca7d18a2209216b7f959`**: specialized [CI #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) failed **both Windows and Ubuntu** because stdout/stderr and stdin inherited by synthetic descendants caused ~1.602–1.614 s real latency with 200 ms configured deadline and <900ms research acceptance. Routine [CI #40087176244](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176244) GREEN does not override this behavioral RED. Merged #347 changes eventual `TimedOut` classification but does not bound `join()`.
- **#355 head `5c104ee67f26b8d7b68132cf05a5f86eb53b580f`**: specialized [CI #40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203) explicitly FAILED on both OS at `ISSUE317_POST_REVOKE_GIT_SPAWN`: production GitRead can spawn after revocation is published, before admissions finish draining. Routine [CI #40158288199](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288199) also FAILED Windows because of **separate #185 Node/AppContainer intermittence**, not the same finding. Preserve first failures.
- **#353 head `b091dc280bfd186694e7c1d1bea743a20f6746c2`**: research Windows `PreparedReadCommand` allocates Job, watchdog, stdout/stderr pipes **before spawn**, typed fixed Git status only, counted session admission and native owner/quota ledger. Requires `Capability::GitRead` plus scoped active TaskLease, exact Git executable/session/policy generation; revalidates immediately before create and checks recorded *actual watchdog timeout* fits within both original session and current lease expiry. Keeps native owner on uncertain spawn; only trusted, never-created/unexposed preparation failures can reclaim capacity; terminal requires root + empty native Job + both genuine EOFs. [full CI #40177026825](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177026825) 4/4 SUCCESS and [native #40177026834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177026834) 2/2 SUCCESS on exact head. **Unmerged, not wired to live GitReadService.**
- **#358 head `59d1a331513d10ea9a459f76b634e7282d53353d`, base #353 head `b091dc28…`**: research-only `LeaseScope::GitReadRepository { canonical_root }` + existing generic Repository scope, rejecting wrong root, missing root and generic-only lease *before capacity reservation*; two distinct Git fixtures share one fixed executable. [full CI #40177299174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299174) 4/4, [native #40177299159](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299159) 2/2 and [preparation #40177299120](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299120) SUCCESS on its exact isolated head. A future merge into moving #353 needs an independently reviewed integration SHA and fresh CI. A lease pinned to repo remains necessary, **not sufficient** for real MCP policy/action authority.
- **Other integrated safeguards**: #349 preserves Win32 raw error provenance; #350 signals stop of registered jobs before admission drain plus late sweep. Neither yields a proven native tree receipt, crash recovery, actual bounded stdin/out joins or canonical live GitRead owner. Preserve main red [#40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) for unexplained `TerminationUncertain` and [#40084443275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40084443275) for #185 Node/AppContainer.
- **Next P0**: prove complete live MCP GitRead pre-spawn authority (session, lease, policy epoch, canonical repo, pinned Git executable, fixed arguments/environment, sensitive read, actual budget), service-owned Job registered *before* spawn, revoke visibility before child create, separately bounded stdout/stderr **and stdin**, root/Job/EOF positive quiescence, retained admission/quotas under uncertainty, recovery after host loss. Turn #352/#355 behavioral acceptance green for **real reasons** before V1. Keep #176/#182, #185, #319 and #323 release acceptance separate.
- **Operational safety**: permanent [Codex interruption recovery](docs/engineering/CODEX_INTERRUPTION_RECOVERY.md) mandates read-only checkpoint discovery, immutable verified SHA-256 snapshots and isolated restoration when the operator machine is actually accessible. GitHub-only agents must not claim to see `E:\Optic-IA-Bridge\codex-recovery`. A cybersecurity block is not to be bypassed; there is no standing permission to run commands on the operator PC. Never touch Optic Desktop or REL.

---


## Canonical continuation — 25 October 2026, after #347/#349/#350 and research #352/#353

**This is the most recent verified GitHub checkpoint as this documentation PR was prepared.** Older “latest” sections below are retained as dated historical evidence only. Always re-query main, independent push/main CI, open PR heads/bases, issue status and CI before editing. Never interpret a historical SHA as a reset or merge command.

- **Production `main=6a6457155758df3050e2aad81ee9446952d7eedd`**, after docs-only merged [#351](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/351). Independent [push/main CI #40086602634](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40086602634) **completed SUCCESS**, attempt 1, all four jobs. Only repository `ELITEAYOTO/Optic-AI-Bridge-`; never Optic Desktop or REL.
- **Integrated code changes:** [#347](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/347) marks an eventual post-root late-pipe completion as `TimedOut` rather than false success, preserving `OutputLimitExceeded` priority. It **does not bound the blocking joins**. [#349](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/349) preserves native Win32 error codes from Job accounting and non-Win32 HRESULTs. [#350](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/350) publishes revocation/stop for registered owned jobs **before** admission drain, then sweeps late registrations after the barrier, without inventing terminal proof. [#351](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/351) updates prior documentation. Independent earlier [main CI #40084443275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40084443275) remains **RED** for separate #185 Node/AppContainer `NETWORK_ATTEMPT` stall; prior [main #40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) `TerminationUncertain` red remains unexplained.
- **Open research [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352), head `2fa45d6b2d1f3dd39309ca7d18a2209216b7f959`:** dedicated Windows/Ubuntu **behavioral RED** [#40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) legitimately measures ~1.602–1.614 s elapsed for descendant-held stdout/stderr and stdin with a 200 ms configured runner deadline and <900 ms wall-clock acceptance. The ordinary [CI #40087176244](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176244) is GREEN; these dedicated failure assertions are ignored by ordinary cargo test **but explicitly run in the specialized always-visible RED gate**. Green classification CI does not supersede failing liveness acceptance. **Do not merge #352 while still red or loosen acceptance to turn it green.**
- **Open experimental [#353](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/353), head `261e9df9eb98417a74b2fe7d2a794b12f9ba0d23`:** stages an owned Windows `PreparedReadCommand` primitive from #327 with native pre-created Job/watchdog/pipes, root+Job-empty+both EOF terminal receipts and bounded nonblocking polling. Its experimental session-native ledger owns counted admission/capacity; tests cover live-root revoke, pre-adoption revoke, never-spawned preparation failure reclamation and **required `Capability::GitRead` for reservation** (necessary, not sufficient). [Specialized Windows #40153942636](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40153942636) and [full CI #40153942601](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40153942601) **completed SUCCESS on that exact SHA**, attempt 1. **Not wired to production `GitReadService`**; no production accepted wall-clock guarantee, task-lease, repo/executable pin or complete stdin/crash recovery. Draft/unmerged.
- **Independent open [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327)** is older native owner research, not an authorized bulk merge. **Issues [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) and [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN, Windows V1 NO-GO.** Existing production `GitReadService` still uses `HardenedCommandRunner`; progress in #353 does not imply otherwise.
- Permanent [Codex interruption recovery](docs/engineering/CODEX_INTERRUPTION_RECOVERY.md) was integrated by #343: read in `AGENTS.md`, verify external immutable snapshots/sha256/Git when actually accessible. A GitHub-only agent cannot claim PC disk checks; never bypass security filters or treat local evidence as CI. No blanket authorization for operator PC execution.
- **Next small slice:** review #353 native pre-spawn owner/ledger semantics and the explicit #352 RED wall-clock criteria, then establish one minimal production-safe service authorization/admission/owner integration from **live main** with independently supervised Windows/Ubuntu native proof. Must bind actual authorized `GitRead` session **plus** scoped nonexpired task lease, exact repo+Git executable, policy generation, proper stop-before-drain and fail-closed native uncertainty. Keep stdout/stderr and stdin handling independently proven; do not make blocked readers/writers detached or free quotas on request alone. No V1 tag or release.



## Production checkpoint — 24 October 2026 after #349 and #350

**This section supersedes the historical SHA inventories below.** Re-query live GitHub before all work; do not treat dated commits as reset targets.

- **Integrated code checkpoint `main=e0de79147e04daaea75cf59694d0e791b824fedc`.** [Merged PR #349](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/349): recover recognized raw Win32 error codes from `QueryInformationJobObject`, preserving complete non-Win32 HRESULTs rather than silently losing native information. Original [draft #346](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/346) is closed *without* merge as superseded. [Exact-head CI #40084066508](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40084066508) completed 4/4 successful first attempt, specialized native Windows checks successful, and all three native error tests explicitly observed in Windows logs.
- **Merged [PR #350](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/350): real stop-before-drain ordering.** In `SessionLifecycleManager::revoke`, session revoke is published first, known owned process stop flags requested **before** waiting for counted admissions; approval/lease cleanup still occurs **after** the unchanged admission-drain barrier. A second owner-scoped cancellation sweep captures processes registered by a previously admitted effect. Stop is a request, **not** terminal proof or resource release.
- **Proven before/after:** [pre-fix CI #40085019486](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40085019486) expected behavioral red on both Windows and Ubuntu in one deterministic test (Windows 343 pass/1 fail/11 ignored, Ubuntu 270 pass/1 fail). [Fixed-head CI #40085443930](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40085443930) completed SUCCESS attempt 1 on all 4 jobs. Both native test logs confirm the new regression, late-admitted-job, multi-session isolation and approval-drain invariants passed. Intermediate corrected-head format-only red #40085298973 stays separate.
- **Independent post-merge `main` CI:** [#40085754973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40085754973), exact integrated `e0de7914`, **completed SUCCESS, first attempt, all four jobs** (Windows/Ubuntu/dependency/release).
- **Important separate red:** [push/main CI #40084443275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40084443275), at `37da993b`, failed the existing [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) Node AppContainer probe *after* `NODE_PROBE_STARTED` and `NETWORK_ATTEMPT`; this is distinct from the prior no-output #185 signature. The three succeeding diagnostic-only serial comparisons do **not** clear the original red.
- **Release blockers remain:** `HardenedCommandRunner` can still block indefinitely in stdout/stderr/stdin `join()`; newly launched but not-yet-registered effects still need native stop ownership; root + Job-empty + complete EOF, durable uncertainty/recovery, Git provenance/sensitive reads #176/#182, startup #319, Node #185, true Windows 10/11 release/rollback all remain. #327 experimental owner research is not a production fix. **#317/#323 OPEN; Windows V1 NO-GO.** No operator-PC access.

---

## Verified integration — 24 October 2026 after PR #347

**Newer than the dated recovery checkpoints below.** Always re-query live GitHub; the SHAs below are evidence, not checkout/reset directives.

- `main=d33ccaad22167d13923f664286267a410bc5974a`: [PR #347](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/347) squashed and merged. `HardenedCommandRunner` no longer reports success if stdout/stderr/stdin joins finish *after* the configured deadline, and already-established output-limit failures retain `OutputLimitExceeded` rather than becoming a post-join timeout. This is **result accuracy, not a wall-clock or process-tree ownership guarantee**.
- First deterministic pre-fix Windows/Ubuntu red: `6538cc68`, [#40081162463](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40081162463); existing code reported `Ok(...)` after ~1.6s for a 200ms deadline. Final source `126a47b75c178a5778124a13666e0163dc0ef814`, [targeted #40082678386](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40082678386) **11/11 Windows and 11/11 Ubuntu**; [full PR CI #40082678379](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40082678379) **all four jobs succeeded, attempt 1**. Separate first Windows full-CI red [#40081265695](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40081265695) hit existing AppContainer Node #185. Its three green diagnostic comparisons do not erase the failed gate or explain its cause.
- Independent post-merge `main` [push CI #40083036492](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40083036492): **completed/success, attempt 1, all four Windows/Ubuntu/dependency/release jobs**, on exact integrated `d33ccaad`.
- **#317 and #323 OPEN, V1 NO-GO.** Reader and writer `join()` are still potentially unbounded; no integrated service-owned Windows GitRead guardian, root+Job-empty+pipe EOF proof, stop-before-admission-drain fix, or crash recovery. [Research PR #327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) remains open but independent. [Research error preservation PR #346](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/346) remains draft/unmerged. Prior original Windows `TerminationUncertain` red [#40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) remains unexplained; #176/#182, #185 and #319 are independent gates.
- Next correct **production** slice: replace unsafe lifecycle assumptions around GitRead with pre-spawn service-owned reservation/native Windows Job, cancellable bounded stdio and authenticated out-of-band stop. Require positive native root, Job and EOF evidence before terminal success or releasing resources; preserve uncertainty otherwise. Keep stdin and session-drain semantics explicitly gated. No user-PC activity or security-policy weakening.

---

## Verified continuation — 24 Oct 2026, after Codex native evidence and recovery protocol

> **Latest active status at documentation preparation.** Historical checkpoints below (`16ca67a7`, `100c43b5` etc.) are not live HEAD instructions. Verify GitHub anew at every takeover; do not reset to these SHAs.

- **Production `main=dce94cc02f178d91b1eec4dc06134ab57df47ff6`** after documentation-only [PR #343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/343) squash. Original source `cf5a6c3` has a **preserved failed** [push/main CI #40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) on Windows (`timeout_terminates_owned_process` expected `TimedOut`, received `TerminationUncertain`; Ubuntu and other gates green). The new [push/main CI #40075855681](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40075855681) was started after #343; check its **final** result independently rather than assuming success. PR-head [CI #40075523748](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40075523748) passed Windows, Ubuntu, dependency and release guard on the policy fix SHA `4a3a0787`.
- **GitHub issues [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) and [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN, V1 NO-GO.** [PR #327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) remains the independent open native-owner research PR; no bulk merge. All later diagnostic PRs #339, #340, #341, #342 were deliberately closed **without merge**, with tests/commits/reports kept.
- **#339 descendants stdout/stderr:** hosted Windows/Ubuntu regression shows `HardenedCommandRunner`'s post-root reader joins can outlive a 200ms deadline; [CI #40059685148](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059685148) green on research SHA `dc8cd3db` *because it detects the existing defect*. **No production fix**.
- **#340 Windows termination observation:** synthetic normal confirmation `kill_request=Ok(())`, root + Job-empty receipt, `confirmation_ms=1` in one run; [CI #40062376385](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40062376385) + [CI #40062376329](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40062376329) succeeded. The intermittent `TerminationUncertain` in main remains unexplained.
- **#341 stdin/EOF:** synthetic descendant inherits stdin, delays reading, writer join exceeds root-only 200ms timeout; [CI targeted #40063153249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40063153249) + [CI general #40063153233](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40063153233) passed on research SHA `7c1bc62f`. **Not a bounded stdin production fix**.
- **#342 Windows 10 physical evidence:** operator-provided Codex report of six native synthetic supervised cases on Windows 10 Famille build 19045.7807; four positive timeout confirmations (1ms observed, **not** a guaranteed bound), externally supervised Job empty before cleanup. Physical code checkpoint `dc36a4b`, final docs-only SHA `505a4ef8`. [CI native #40064790347](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40064790347) and [CI general #40064793614](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40064793614) succeeded on final SHA (Windows workspace 608 passed / 15 ignored; Ubuntu 477 passed); no natural `TerminationUncertain` reproduced, no cause established.
- **Permanent Codex interruption recovery** documented by merged #343 in [CODEX_INTERRUPTION_RECOVERY.md](docs/engineering/CODEX_INTERRUPTION_RECOVERY.md), linked from [AGENTS.md](AGENTS.md). It is a read-only-at-start agent instruction, **not** an automatic daemon or a bypass of platform security. A separate operator report described local snapshots under `E:\Optic-IA-Bridge\codex-recovery`, branch `codex/317-native-wait-phases` and simulated/real Win32 cases. The operator later reported `0013-policy-published-ci` (55 local checksums); remote docs do **not** prove local disk access or whole-new-session restore. Do not publish local snapshots or infer tests that were not run.
- **Production still uses `HardenedCommandRunner` for GitRead**, despite having `OwnedSessionAdmissionPermit`. Native Job ownership, stdout/stderr/stdin cancellation, persistent Job-accounting errors, stop-before-admission-drain, generation-bound quota/handles, guardian crash/restart reconciliation and bounded shutdown are still gates. Keep #176/#182, #185, #319 and Windows 10/11 release/rollback validation independent.

**Next:** discriminate `LimitedJobChild::wait` root-vs-Job observation phases and preserve original Win32 error codes under a bounded synthetic negative case; prove admission/quota/Job owner retention and stop-before-drain, then create an inverted stdio/stdin regression for a reviewed minimal production patch from *live* main. No timeout inflation, unchecked handle release, status-forcing or research-branch bulk merge.

---


## Latest verified 24 Oct handoff — ten externally supervised native scenarios (documentation only)

At the pre-passation GitHub inspection, **`main=16ca67a75fd49c5681a0672773ffc268c3635f6b`**, with independent successful [push/main CI #40047950688](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40047950688). Production includes the counted `OwnedSessionAdmissionPermit` from #336 but **still runs native GitRead through `HardenedCommandRunner`**: NO production guardian/pipes fix. **Only PR #327** open, **#317 and #323 OPEN, Windows V1 NO-GO**.

Latest isolated **code** checkpoint: `research/317-failed-native-create-2026-10-24@b5a91deea1b306c56a2c03de7bb5a86b699d1d6f`, exact [CI #40057931887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887) **completed/success (attempt 1)**: Rustfmt/strict Clippy Windows + Ubuntu, Windows library **355 pass / 0 fail / 13 ignored**, Ubuntu **270 pass / 0 fail**, ten Windows-native externally supervised scenarios, full Windows workspace suite. Fixed synthetic libtest `CreateProcessW` is required to fail on missing cwd, leaving service-preowned kernel Job empty. Ledger quota and real owned session admission stay retained during **injected** observation failure; terminal `Failed` and release only after true kernel Job-empty observation. Research-only, **not** production safety or actual Windows 10/11 acceptance. Initial red [#40057892808](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057892808) stopped at Rustfmt before tests. Report-only SHA `4bef362d1e9c45ec3b81a4d6807ae924f6122218`, [CI #40058237376](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40058237376) **in progress at passation precheck**, re-query.

**Still missing:** native descendant-held stdout/stderr and stdin EOF, durable or bounded shutdown under real persistent Win32 errors, guardian/service crash and restart reconciliation, Git read lease/provenance #176/#182, flaky native #185 and startup #319, operator-authorized Windows 10/11 testing, installer/offline/rollback gate. Next priorities and historical research sequence: [updated technical handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md), [copy-ready next prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md), [latest #317 issue report](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6379288501). This documentation PR does **not** resolve #317/#323; verify the PR merge SHA and subsequent push/main CI before updating production status.


## Current verified checkpoint — 24 October 2026, after merged PR #336

**GitHub observed before the 24 Oct documentation-only PR:** production main = **100c43b53c795be5678c8b4f2c0af4353ca5b196**; independent first-attempt [push/main CI #40027279676](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40027279676) succeeded on the exact SHA (Windows/Ubuntu, dependency/release guards). **PR #336** introduced non-cloneable counted OwnedSessionAdmissionPermit and a captured-generation stop witness, but **did not** migrate native GitRead ownership. Only PR **#327** was open; #317/#323 remain open, V1 **NO-GO**, as do security #176/#182 and CI #185/#319. After this documentation PR is integrated, re-query main and prove its new exact push/main CI before claiming an updated production checkpoint.

**Latest synthetic native research, strictly unmerged:** branch research/317-native-observation-uncertain-2026-10-24 at **8202c9c633bf797be9ea69f9ce994b5ec5da9b86**, [CI #40046563179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40046563179) completed **success at attempt 1** on this exact SHA (Windows **348** library passes / 0 failures / 11 ignored, Ubuntu **270** passes / 0 failures, strict fmt + Clippy, named Windows failure-injection test 1/1). The owner keeps the original session admission, JobStore record and real Job after an **injected, temporary** accounting error following root exit, and releases only when a native Job query recovers and proves zero members. This is **not** proof of genuinely persistent Win32 errors, bounded shutdown, full pipe/descendant EOF, service guardian after crash or a production fix. [Full handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) / [successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md) / [issue verification](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6377736151). The earlier 23 Oct status below is historical.

## Verified source checkpoint — 23 October 2026, after PR 334

**Production source:** `main=cdeb891f27f22385c908438113e74dc57cc9912a` following [PR 334](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/334). Accepted PR source `3ca8a74fb02c50a4a21f21a8353af8d07a648536` passed first-attempt [full Windows/Ubuntu/dependency/release CI 40021859463](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40021859463) and [specialized native-preparation 40021859457](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40021859457). Production integrates **only** a read-only `SessionAdmissionStopWatch` combining the original admitted generation's explicit revocation with its immutable monotonic admission deadline. Genuine worker-held permit, renewals and deliberate handle-reuse regressions passed on Windows (340 library tests, 11 ignored) and Ubuntu (267 library tests). It does **not** issue launch authority, prove process termination or migrate any Git runner. [Contract and limitations](docs/engineering/ISSUE317_ADMISSION_STOP_WATCH_2026-10-23.md).

**Post-integration independently accepted:** push/main [CI 40022465939](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40022465939) completed **success at attempt 1** on the exact integrated `cdeb891f27f22385c908438113e74dc57cc9912a` SHA, with Windows Rust/MCP/installer and AppContainer network proof, Ubuntu Rust, dependency policy and release evidence guard all green. This is first-run integration evidence for the small admitted stop witness, not for native ownership or V1 readiness.

**Preceding steps:** research owner/authority/shutdown [contract PR 333](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/333) was squash-integrated at `dedca5f4`, followed by independent first-attempt success [main CI 40019752828](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40019752828). Test-only `research/317-revoke-generation-aba-2026-10-23=5edc4792` reproduced handle-reuse cleanup misdirection under a deliberately reused opaque ID and competing real `try_reap`; [Windows/Ubuntu proof 40020639068](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40020639068) passed, but the shipping provisioning path creates random handles and this is **not** a demonstrated remote exploit or a production patch. Full evidence recorded in the [issue 317 thread](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317).

**Open gates:** [issue 317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) still lacks a production-owned native Git process tree, stop-before-drain integration, stdin/EOF/crash cleanup and proven bounded service shutdown. [issue 323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) Windows V1 acceptance remains **NO-GO**. Both were **accidentally auto-closed** at the docs-only PR 333 squash through a negated GitHub closing-reference phrase and were explicitly reopened on 23 Oct; see their event timelines, not the transient completed state. Security [issues 176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)/[182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182), intermittent Windows [185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185)/[319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319), and research [PR 327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) remain distinct. No release, Windows 10/11 physical-device acceptance, security relaxation, operator-PC testing, Electron work or real-project remote enablement was authorized.

**Next engineering slice:** private generation-bound **owner registration + short authority commitment** with genuine pre-spawn JobStore accounting, independent stop delivery, and explicitly retained uncertain native resources. Prove actual direct/lifecycle close paths and suspended-at-creation Job membership with hosted Windows native supervision; later migrate ONE sealed read-only Git command only with #182 provenance and task-lease/pin/policy gates. Avoid whole-research-branch merges or a fake timeout-based cleanup. Historical checkpoints below are dated observations, not current state.

## Active verified handoff checkpoint — 23 October 2026

At the verification **before this documentation-only PR**, production `main=3fe3de0c258b2f8fc4328d6c405c8bf05cb22e3d`; one existing research [PR #327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) open (head `ef3208d7`, old [CI #39845778580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39845778580) **red #185**), `main.protected=false`. Experimental `codex/317-service-retention=1c7dac973c8f41f27b303811041b54031f7272ae` passed [CI #39888522519](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522519) and [native #39888522200](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522200) (Windows **608 passed/25 ignored**, Ubuntu **463 passed**), but **no GitRead production wiring**. Independent native tests proved the suspended-child **host crash before Job assign leaves a surviving child** (then externally cleaned); revoked session admission currently covers submit, not the whole worker; Drop may hang under persistent native observation uncertainty. No V1 gate closed or security policy changed. #182 [research proof #39865183807](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39865183807) confirms indexed fake secret persists after hardlink replacement, **not** a fix.

**New canonical [23 Oct Windows V1 handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-23.md)** and [copyable prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-23.md) supersede the 22 Oct handoff for sequencing; older text below remains historical. Re-check current GitHub after this PR and obtain independent main CI if merged. #317/#176/#182/#185/#319 and V1 #323 remain open/NO-GO.

## Latest verified research / handoff checkpoint — 22 October 2026

At the handoff audit, **main was `82ad1234f184591e0f2c80c86296a65a883ecc20`**, with **zero open PRs**, `main.protected=false`, and **no approved V1 release**. This is a dated checkpoint; re-query live GitHub, especially after the documentation PR. Production `HardenedCommandRunner` remains uncorrected for [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317). The research branch `bootstrap/issue-317-windows-rust-owner` reached `33c95e00aa384774ccac6ccbab24b8f54fa5ab11`, with exact-head [CI #39748924515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748924515) green (native Windows Rust/MCP/Node, Ubuntu Rust, dependency policy). It includes 13 independently supervised native fixture cases (earlier [#39735459098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39735459098)), true SessionRegistry/Job regression and a private **unwired** owner-registry prototype using inert owners in unit tests. **Neither branch research nor successful CI fixes #317 on main.**

[Canonical next-chat technical handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-22.md) and [copyable full successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-22.md) record accepted evidence, all limitations and next development steps. [V1 freeze gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **NO-GO**. Other open local-gate reviews: #176/#182 sensitive read and Git provenance, #319 MCP 30s intermittency, #185 Windows Node/AppContainer intermittency; #171–175 and #177 are out-of-scope remote milestones for the proposed local V1, not authorized exposure. Windows10/11 physical-device acceptance is absent; no PC execution, release, feature/Electron resumption or branch-protection changes.

Older status paragraphs below are historical dated snapshots and do not override this checkpoint.

## Current verified checkpoint — Windows V1 after PR #321 (2026-10-22)

At this audit: `main=65aa5db0dbaad24894f2cf4b050d5ac0bd61eac9`; PR [#320](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/320) and [#321](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/321) squash-merged, zero PRs open. Exact-merge independent [CI #39722724572](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724572), [pwsh native #39722724496](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724496), [Windows PowerShell 5.1 #39722724477](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724477) completed/success, attempt 1. **Recheck live heads/checks before each operation.**

- **#320:** tagged prerelease source now needs exact integrated `main` ancestry plus completed successful push/main CI and required jobs; absent/red/ambiguous evidence fails closed. The 26 offline fixtures remain in regular CI. No tag or release produced. The publication build job still has `contents:write` over a broader scope than ideal ([contract](docs/engineering/RELEASE_SOURCE_GATE.md)).
- **#321:** Windows PowerShell 5.1 emitted a UTF-8 BOM (`ef bb bf`) before MCP JSON on stdin. The real doctor now sends BOM-free UTF-8, with scoped .NET Framework console-encoding restoration and failure-safe cleanup. Real byte-level/Unicode and Bridge/Node native proofs passed. This **does not** explain natural 30-second MCP startup issue [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319), nor distinct AppContainer [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185). [Acceptance limits](docs/engineering/WINDOWS_POWERSHELL_ACCEPTANCE.md).
- **#317:** research-only native Rust owner slice `d295d92ec813612a3a919b56143ed23dcbb89485` passed eight supervised Windows cases ([native #39718651623](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39718651623), [CI #39718651583](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39718651583)). **Not integrated** into production `HardenedCommandRunner`: stdin/native error lifecycle, pre-spawn capacity, service-owned Session/HostStartup, stop-before-drain, seal/reap and Git CAS uncertainty remain. [Research audit](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/d295d92ec813612a3a919b56143ed23dcbb89485/docs/research/WINDOWS_V1_OWNER_AUDIT_2026-10-22.md) and [proposed ADR-0035](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/d295d92ec813612a3a919b56143ed23dcbb89485/docs/decisions/ADR-0035-WINDOWS-GIT-OWNER-STAGING.md) exist **only on the research branch**.
- **Security gates:** [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)/[#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) are open before any real-project remote exposure. Lexical sensitive-path and verified Git-read checks are **not** content DLP or atomic Git provenance; H-02–H-05 auth/session/approval guarantees and recovery under crashes remain distinct. `main.protected=false` and no rulesets were found; changes require operator approval and appropriate admin privileges.
- **Scope:** Windows 10/11 is the V1 *target*, not certified by hosted Windows Server CI. Existing Ubuntu shared-code CI stays; Linux functional confinement is post-V1. No user-PC execution, unapproved release, new feature, Electron work, remote real-project access or security/timeout relaxation. Preserve first red attempts, including the #321 Desktop BOM failure. No security program can honestly guarantee zero vulnerabilities.

Immutable [Codex checkpoint](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319#issuecomment-6332848895); older paragraphs below are historical and must not override this header.

## Windows PowerShell acceptance coverage

A real Desktop5.1 doctor smoke exposed UTF-8 BOM bytes before initialize.
The doctor now starts redirected MCP streams with BOM-free UTF-8; the Framework
fallback restores console encoding after Start, including failure. Native byte,
Unicode, pipe and real Bridge/isolated-Node checks accompany current pwsh coverage.
The test worker adapts Framework argument quoting and records shell version/edition. These checks are not Windows10/11 installation certification.
[Acceptance scope and remaining risks](docs/engineering/WINDOWS_POWERSHELL_ACCEPTANCE.md).


## Release source provenance guard — 22 October 2026

The tag workflow now requires the exact source to be integrated into main and
its latest matching independent main CI to be complete/successful, including
native Windows, Ubuntu and dependency jobs. Missing, pending or ambiguous evidence
fails closed before release build/publication. This does not authorize a release
or certify V1 readiness. [Contract and limitations](docs/engineering/RELEASE_SOURCE_GATE.md).

#317 has a new **research-only** native Rust owner slice (8 cases), not a production
runner correction: [verified checkpoint](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6332249104).
Stdin ownership, resource/session registry, revoke/seal/reap and CAS remain gates.


## Windows V1 stabilization — operator direction, 22 October 2026

Windows10/11 is the V1 target; supported OS/PowerShell combinations still require
their own acceptance. Hosted Windows Server checks are native Windows evidence,
not proof that every Windows10/11 installation works. Functional Linux delivery
is post-V1; existing Ubuntu Rust/dependency checks remain required regression guards.

The #318 change rejects non-octal raw Git modes and bounds Windows doctor stderr
capture after EOF/timeout using a sole-consumer native pipe snapshot. Tests preserve
primary failures, cap bytes/messages, reject unknown labels and supervise all fixture
processes. No startup deadline, full executable identity/pin, resource/permission,
AppContainer or network boundary is relaxed. Exact integration status and source
checks are in [PR318](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/318);
the [native proof](docs/research/WINDOWS_DOCTOR_PIPE_LIFETIME_2026-10-22.md)
distinguishes demonstrated error-path failures from startup hypotheses.

**Still open:**319 natural first MCP30s timeout (cause not established),185 separate
Node/AppContainer10s failure,317 demonstrated generic command pipe-join overrun
(no production Rust ownership correction),176/182 sensitive/provenance boundaries.
These remain V1/release gates where applicable. An unrelated Git/parser correction
and a green current CI do not certify startup reliability or real-workspace safety.
No feature unfreeze, Electron, generalized Cargo, alpha merge or user-PC execution.

Older checkpoints below describe their dates, not current branch heads.

## Historical verified checkpoint — #314 (2026-10-21)

**Verified source of truth:** `main=7bc17c16a52383bf6fb7ba68dc4b9348b259be9a`, 0 open PRs. [PR #314](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314) is squash-merged: a hard-linked file can bear both a classified sensitive name (`.env`) and an innocent-looking alias (`README.txt`); `fs_read` now requires existing matching `ReadSensitiveScope` in addition to ordinary FileRead for multi-linked opened native identities. This is intentionally conservative for harmless hard links. Native Windows/Linux regression tests passed on accepted source SHA `fbf7c9f40bbae18d4017f237b8783a19964620c8` in [CI #39576724127](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39576724127); three specialized native workflows also passed. Independent [push CI #39577461324](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39577461324) passed **first attempt on the exact squash SHA**, Windows Rust, Linux Rust and dependencies. Earlier rustfmt failure on a superseded PR head remains preserved. [Complete proof](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314#issuecomment-6312779381).

**Residual trust boundaries:** #176 (arbitrary secrets in ordinary files, native ancestor identity/races), #182 (Git index/commit content provenance and branch/ref metadata), and #185 (intermittent 10-second AppContainer Node timeout of unknown cause) remain **OPEN**. A separate ~30-second first MCP response flake remains unproven. [Targeted GitRead review](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182#issuecomment-6312922336) confirmed existing status/diff protections and recorded that status `# branch.head` / `# branch.upstream` values are returned as Git metadata without separate sensitivity classification; this does **not** prove credential leakage. Do not change the public metadata policy without an explicit contract and synthetic regression.

**Execution:** pre-alpha, feature work frozen, no real-project remote access, no alpha/generated-project-editing merge, no user-PC or self-hosted execution, no privilege/sandbox/timeout relaxation. Source head must always be rechecked live. All later candidate checkpoints below are dated history, not unmerged current work.

## Verified Policy B path-floor integration — after #310 (2026-10-21)

**Source of truth verified on GitHub:** `main=bd2915c3a9833c1da5a8fc5005445d1884260d4f`; [#310](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/310) is squash-merged; zero PRs are open at this checkpoint. Independent [main CI #39552523257](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39552523257) succeeded **first attempt** at the exact merge SHA on native Windows, Ubuntu and dependency policy. #310's accepted head `154d7ea6ab6b10aa967ab0a2f2034f2f06400fe9` passed [source CI #39551993433](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993433) and [native preparation #39551993151](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993151) first attempt. Earlier red PR CI attempts remain evidence of corrected rustfmt and verified Git raw-protocol test failures; do not overwrite them.

**Integrated security path floor:** #308 adds shared Rust-core sensitive-name classification and the FileRead / fs_list additional scoped authority checks. #309 parses Git porcelain-v2 `-z` metadata and denies unknown or unauthorized protected names. #310 produces `git diff --raw -z --patch` in a single hardened process, validates NUL-delimited raw changed paths **and the extra NUL before the patch**, denies unverifiable streams, and requires active typed FileRead (ReadContent + ReadSensitive) authority for classified protected patch contents while read admission remains held. No new default grants, remote transport, feature expansion or weaker containment. See [ADR-0034](docs/decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md).

**Still blocked:** lexical filename rules cannot prove ordinary-named contents are secret-free (#176); Git metadata and diff coverage remain subject to concurrent repository changes, unproven alternate path identity and other provenance/authority guarantees (#182). Windows Node/AppContainer timing #185, separate MCP first-response intermittency, crash recovery atomicity/persistence, ancestor races, stress/fault tests and incomplete code audit remain. #176 and #182 are OPEN; #176 was accidentally auto-closed during #310 merge by a GitHub closing keyword in prose and explicitly reopened with evidence. No real-workspace remote exposure or feature unfreeze. Older sections marked *candidate* below are **historical pre-merge records** and are superseded by this verified checkpoint; do not reapply earlier base SHAs.

## Git diff path-verified output boundary — candidate (2026-10-21)

Live GitHub before branching: `main=ccaa7c05af27cfaab02d5dcb93369f60a0c01f53`; [#309](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/309) squash-merged; independent [post-merge CI #39549267903](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39549267903) green first attempt (native Windows/Linux/dependencies); zero open PRs. New **candidate** `security/git-diff-raw-output-authority`: not integrated until exact-PR-SHA CI and independent main CI succeed.

Proven #182 gap: unlike the now-protected `git_status`, `git_diff` returned raw staged/worktree patch bytes without classifying any changed path. Candidate correction: one hardened Git invocation `--raw -z --patch` with raw changed-path verification and strict rejection of malformed/unknown/native-unrepresentable outputs. No preflight/patch race from two separate Git calls. For protected patch paths, check an additional active typed FileRead effect (both content and sensitive scopes) **inside session admission**, then release only the patch suffix. Normal source patches stay available. Unit/MCP regressions target unscoped/path-scoped/dir-scoped/staged protected patches, ordinary code, malformed raw and resource budgets.

**Limit:** Git diff source metadata and patch come from the same Git diff command, but worktree snapshots are not atomic against arbitrary concurrent edits. Lexical path rules do not detect secrets in normal-looking file contents. #176/#182 remain OPEN; no remote real-project deployment or restart of new features.

## Git status sensitive-path classification — candidate (2026-10-21)

Live baseline for this increment: `main=bfba4b97a531440de08012ca50106797d995646b`, [#308](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/308) squash-merged, independent [post-merge CI #39546153987](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39546153987) green first attempt, zero open PRs. Operator-approved Policy B remains binding, with no remote exposure to real workspaces.

**Candidate `security/git-status-sensitive-paths` (unmerged until exact-head CI):** validate every path in the returned Git porcelain-v2 `-z` status output, including rename original names, before returning status bytes. Reuse the Rust-core classifier and require an active matching sensitive read lease in the admitted MCP worker; refuse malformed/unknown/unrepresentable records. Native Windows/Linux tests cover sensitive staged and untracked files, mixed normal content, explicit grants, prefix mismatches and invalid output. No new CLI, transport, grant, override or runtime privilege.

**Open:** #182 remains blocked by the *independently unclassified raw `git_diff`*; harmless-looking files can carry secrets and #176 remains incomplete. Do not close either issue or enable real-workspace remote access. Inspect GitHub CI and final SHA before considering this candidate merged.

## Sensitive-read authority — policy B approved, first scoped fix under CI review (2026-10-21)

After merged #307, live `main=3d9a0f78304578af53fd23b9fed7d08603348e0b`, zero open PRs, and post-merge hosted Windows/Linux/dependency CI #39542505029 passed first attempt. The operator has now approved **Policy B**: shared deterministic classification of sensitive path/content across FileRead, FileSearch and GitRead, with fail-closed handling of unverifiable outputs. **This paragraph describes a candidate increment on `security/sensitive-file-authority-scope`, not a verified merge.**

First narrow corrective implementation: pure conservative sensitive-path classifier in core, enforced by `PolicyEngine` for file reads (ordinary content and matching sensitive scopes both required), and by `fs_list` before releasing sensitive directory names (whole affected page denied without additional sensitive grant). Unit and MCP regressions cover path ancestry, case, absent/mismatched sensitive leases, ordinary content and explicit scoped access. Exact-head hosted Windows/Linux acceptance and independent main CI are **not yet claimed**.

**Important limitation:** lexical name detection is not secret-content inspection. `GitRead` status/diff remain outside this increment and can independently disclose names or secret-bearing patches. Both #176 and #182 remain **open blockers** to real-project remote/autonomous exposure. Do not infer safe remote access, close these tickets, or restart features. Next correction must treat Git output records and unverifiable outputs fail-closed under the **same** classifier (ADR-0034) with exact SHA CI. No broad grants, deadline changes, PC execution or alpha merge.

## Successor handoff snapshot — 2026-10-21, after merged #306

GitHub live before this documentation handoff: `main=dadaf48539788235b689dd4e09adcd2fb94904f3`, **zero PRs open**, [PR #306](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/306) squash-merged (governance and stabilization exit-gates documentation). [PR #306 source CI #39540522392](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540522392) passed first attempt, and independent [main CI #39540940136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540940136) passed first attempt on exact merge SHA (hosted Rust Windows, Rust Linux, dependencies). This is **dated evidence, not an instruction to restore a commit**. Verify live GitHub for changes after this handoff's own PR.

**Next step:** security source audit of the `fs_read`/`fs_list` vs `GitRead`/status/diff data-exposure contract, [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) and [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182), with **no sensitive classifier yet proven** and no permission broadening. Other open reliability boundaries remain [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) Windows 10s Node/AppContainer, distinct MCP ~30s initial response, recovery atomicity/ancestor races/power-loss durability, soak/fault tests and incomplete exhaustive code audit. Administrative issues #180/#181/#183 have code-level regressions/protections recorded and need acceptance reconciliation, not automatic recreation or closure.

**Operating rule:** one PR at a time only after proving a defect, tests+docs together, exact SHA CI and independent post-fusion main verification; no operator PC, self-hosted runner, new features, release or changing GitHub branch protections without approval. Full future-chat instructions live in [PROJECT_CONTINUITY_PROMPT.md](PROJECT_CONTINUITY_PROMPT.md) and [GITHUB_HANDOFF.md](docs/engineering/GITHUB_HANDOFF.md); old candidate headings below are archive records.


## Verified integration checkpoint — 2026-10-21 (before this documentation update)

At the **live GitHub check**, `main` was `43d3a4aeedee04dd03e895e27e81d3bc5af6652c`, with **zero open PRs**. Independent push [CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed **first attempt** on this exact commit (hosted Windows Rust, Linux Rust, dependency policy). This is a dated observation, **not** an instruction to reset or a claim about later commits.

The recent stabilization work is **already merged**:

| PR | Main merge SHA | Narrow effect and verified integration |
| --- | --- | --- |
| [#301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) | `05b5a928` | Preflight all unpublished recovery artifacts before deleting; [main CI #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) passed first attempt. Source CI #39529419994 needed an unchanged-SHA Windows job retry after the known 10s Node/AppContainer failure. |
| [#302](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/302) | `c8c1110f` | Documentary continuity; [main CI #39530925055](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530925055) passed first attempt. |
| [#303](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/303) | `9c6c9109` | Canonical directory scope before `fs_list` authorization; [main CI #39534614408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39534614408) passed first attempt. |
| [#304](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/304) | `9d90485f` | Reserve canonical `.git` control paths from generic mutation; [main CI #39536541153](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39536541153) passed first attempt. |
| [#305](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/305) | `43d3a4ae` | Fit the final `git_log` response **including** newly added continuation cursor; [main CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed first attempt. |

**Not closed by green CI:** Windows Node/AppContainer 10-second no-output intermittency ([#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185)); separate ~30-second MCP first-response intermittency; recovery multi-delete non-atomicity, ancestor namespace races and Windows power-loss durability; sensitive-file read classification ([#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)) and GitRead sensitivity composition ([#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182)). #180/#181/#183 still appear **open administratively** despite already-integrated scoped corrections/tests; reconcile their *remaining acceptance criteria*, do not repeat the implementation or silently close them.

The core is **pre-alpha**. The feature freeze and GitHub-hosted-only restrictions remain. See [stabilization exit gates](ROADMAP.md), [current handoff](docs/engineering/GITHUB_HANDOFF.md) and [reliability debt](docs/engineering/RELIABILITY_DEBT.md). The older sections below are **dated historical reports**, including wording such as “candidate” written before those PRs merged.


## Git-log response size & pagination — bounded regression

A deterministic `git_log` response-fitting flaw existed when the final history page had `next_cursor: None`: trimming entries to meet the structured response-byte limit then **added** a cursor containing the snapshot HEAD and new offset without checking the resulting serialized size. A too-large MCP response could be emitted despite the configured output budget. The narrow fix repeatedly refits after cursor insertion, reducing the page while preserving at least one advancing entry; if no entry plus cursor fits, it fails closed with `optic.response_too_large`. Cursor arithmetic now rejects overflow rather than saturating into a non-advancing token.

Focused tests construct an end-of-history page with a carefully selected byte budget to prove rejection when the cursor cannot fit, acceptance when it can and preservation of a complete cursor-free last page. GitRead capabilities, HEAD snapshot pinning, Git subprocess sandbox and resource limits are unchanged. **Verify the actual PR, exact-head CI, and independent `main` CI before claiming this change is integrated.** No new feature, user-PC activity or CI bypass.


## Git-control mutation authority boundary — stabilization candidate (2026-10-21)

The [security audit #180](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/180) identified a real authority-composition gap: generic FileWrite/FileDelete with whole-workspace or prefix grants could address `.git` control metadata, despite dedicated GitIntegrate authority. The scoped candidate reserves every exact `.git` path component (case-insensitively, including linked-worktree `.git` files) from generic transactional write/delete/patch preparation, **after canonical target observation**; generic directory creation rejects the same namespace. It does not expand session/lease rights or change the Git integration runtime. Ordinary dotfiles such as `.gitignore` remain regular project files.

Cross-platform preparation tests, a Unix internal alias test, native Windows end-to-end authorized mutation and directory tests, and explicit MCP denial-code tests are included. **Do not describe as delivered until exact-head GitHub-hosted Windows/Linux/dependency/native-preparation CI and post-merge main checks succeed.** Parent path swaps, hardlink alias identity and power-loss durability remain independent issues. Never infer that sensitive-content reads or remote exposure are solved by this change. No new features, permission expansion, operator-PC activity or self-hosted CI.


## Scoped fs_list canonical-target authorization — security stabilization (2026-10-21)

Source review of the existing local MCP `fs_list` found a concrete lexical/canonical scope mismatch: policy evaluated a requested directory alias before `BoundedFileSystem::list` followed the alias to a different workspace subtree. Under prefix-scoped `SearchMetadataScope`, this permits metadata enumeration outside the granted prefix. The bounded correction resolves the **canonical directory target before policy evaluation** and lists that resolved canonical root; returned paths identify the actual directory. A Unix symlink regression distinguishes restricted-prefix denial from whole-workspace allowed canonical listing; a portable root/normal-directory regression runs on hosted Windows and Linux.

The change does not grant new permissions, create a transport, add a sensitive-content classifier, pin ancestor directory handles, or eliminate concurrent namespace replacement races. Review the exact source PR and native CI before claiming delivery. [Security issue #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) remains open for its separate sensitive-read classifier and remaining path/race boundaries; #301 recovery and Node #185 remain independent. Feature freeze remains in force.


## Stabilization: unpublished recovery evidence — PR #301 (2026-10-21)

[PR #301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) was squash-merged into `main` at `05b5a928c862601926e8b0cc11540ce020caf271`. Its scoped fix preflights **all** unpublished `*.prepared.tmp` entries with existing native opened-file, single-link and no-reparse checks before the first deletion, and rechecks each immediately before removal. The new ordered-fixture regression proves a later hard-linked unsafe entry cannot erase earlier unrelated evidence; repair permits normal cleanup. Windows and Linux logs explicitly record `later_aliased_unpublished_entry_preserves_all_earlier_evidence ... ok` on accepted source `089e64a2b6c95dc0705eeaaa22f90194dfb7be66`.

[Source CI #39529419994](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529419994) passed after **attempt 2**, not on first attempt. Initial Windows attempt failed in the already-known native Node/AppContainer 10s timeout (no captured launcher/Node marker); three serial same-head diagnostics passed and remain evidence in [issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185#issuecomment-6306039365). Linux and dependency checks passed on the original attempt; [native preparation #39529420240](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529420240) passed first attempt. A failed-Windows-job-only rerun passed the full native suite and smokes on unchanged SHA. Independent [main CI #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) **passed first attempt on exact squash SHA** `05b5a928c862601926e8b0cc11540ce020caf271` (hosted Windows Rust, Linux Rust and dependency policy).

**Limit:** This closes one demonstrable early-cleanup-on-later-unsafe-entry defect. It does **not** guarantee atomicity of multiple deletions if a later actual deletion fails, concurrent ancestor/path identity, Unix unlink race safety or Windows power-loss directory persistence. Node/AppContainer (~10s) and separate MCP first response (~30s) intermittencies remain open. Feature freeze and GitHub-only execution restrictions unchanged.



## Stabilisation Windows — PR #299 fusionnée le 21 octobre 2026

À l'observation GitHub après la fusion : `main=6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea`, aucune PR ouverte. La [PR #299](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/299) a été fusionnée par squash depuis la tête exacte `aca382535f24b6f6b1c26c2313302d315099ed4e`. Sa [CI source #39526324369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324369) a réussi **du premier coup** (Windows Rust natif, Linux Rust, dépendances), de même que la [préparation native #39526324493](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324493). La [CI indépendante post-fusion #39526677098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526677098) a **réussi dès la première tentative**, sur le SHA exact `6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea` (Windows Rust, Linux Rust et dépendances). Les docs sont un relevé daté, jamais la tête GitHub live.

**Incrément réel :** jalons monotones opt-in, à libellés fixes, écrits sur `stderr` par le lanceur d'isolation Windows : entrée, requête, entrées validées, profil, grants, enfant créé suspendu, token vérifié, reprise, fin. `OPTIC_LAUNCHER_PHASES=1` est transmis uniquement en tant que constante opt-in à ce lanceur, dans la preuve Windows existante et les répétitions diagnostiques déclenchées lors d'un échec. Le test natif vérifie la présence des phases et conserve le refus réseau effectif, la sortie attendue, les limites de 10 secondes et les règles d'isolation.

**Exemple validé :** `target_created_suspended=14 ms`, `target_resumed=14 ms`, `target_exited=361 ms` à partir de l'entrée du lanceur. Le parent a séparément observé `launch_returned=13 ms`, `terminal_observed=407 ms` ; ces horloges ont des origines différentes. Ces données prouvent le fonctionnement de l'instrumentation sur un essai **réussi**, pas la cause des expirations précédentes. Les intermittences Windows AppContainer/Node de ~10s et MCP première réponse de ~30s restent **ouvertes**. Le nettoyage multi-fichiers, les courses d'ancêtres et la persistance après coupure restent non validés. Aucun correctif fonctionnel de ces défauts n'est revendiqué. Le gel des nouvelles fonctions, l'interdiction de toute intervention sur le PC et la distinction `main`/alpha restent en vigueur.

Voir [la dette](docs/engineering/RELIABILITY_DEBT.md), [l'issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) et [la reprise](docs/engineering/GITHUB_HANDOFF.md). Les sections antérieures restent des archives datées.


## Point de reprise canonique — stabilisation vérifiée le 21 octobre 2026

**Observation avant cette mise à jour documentaire** — `main` = `83329d48789c20b1fc235d3096c3199dc9dc5176`; **aucune PR ouverte**. La [PR #297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) est fusionnée. Sa [CI indépendante post-fusion #39514832814](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514832814) a réussi **dès la première tentative**, sur le commit exact : Rust Windows natif, Rust Linux et politique des dépendances. La préparation native sur la tête source exacte a également réussi. Vérifier de nouveau l'état GitHub : ce point de contrôle n'est pas une tête à réinitialiser.

**Livré sur main (preuves précises dans les PR, ne pas répéter ces incréments)** :

| PR | Fusion (SHA abrégé) | Résultat réellement intégré |
| --- | --- | --- |
| [#293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/293) | `de973989` | Prévalidation coordonnée des journaux et fichiers de staging avant nettoyage des preuves temporaires |
| [#294](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/294) | `da3ab6f1` | Marqueurs de temps MCP du processus enfant, strictement filtrés dans le diagnostic Windows |
| [#295](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/295) | `615e6219` | Mesures distinctes des étapes Git, session, lecture et leases de processus |
| [#296](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/296) | `72966f92` | Mesures opt-in résolution de chemin / ouverture épinglée / lecture et hachage d'exécutable |
| [#297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) | `83329d48` | Refus doublon/capacité **avant** la capture d'identité ; seconde vérification sous verrou avant insertion, tests négatifs et concurrents Windows/Linux |

La CI indépendante sur `main` a réussi dès la première tentative pour #293, #295, #296 et #297. **Pour #294, la CI post-fusion n'a réussi qu'à la deuxième tentative**, après une répétition du timeout natif Node/AppContainer de 10 secondes sur Windows ; l'échec de première tentative reste une preuve, pas un résultat à masquer.

**Mesures et portée exacte** : sur une exécution Windows de #296, `path_ms=0`, `pin_ms=0`, `hash_ms=4869` (temps de **lecture bornée et BLAKE3**, pas le calcul BLAKE3 isolé). Le correctif #297 évite ce travail coûteux **uniquement si l'inscription du lease est déjà irrecevable** ; il ne supprime pas et ne met pas en cache la vérification d'intégrité d'une inscription valide.

**Risques ouverts et indépendants** : intermittence Windows Node/AppContainer (~10 s sans premier marqueur du script malgré retour rapide du lancement), intermittence MCP de première réponse (~30 s historiquement), nettoyage multi-fichiers non atomique et risques d'identité des répertoires ancêtres / persistance sous coupure électrique. Les preuves Cargo préparées n'autorisent pas un Cargo général ; Electron avec sa sandbox normale reste non validé. L'alpha repose encore sur des protections/authentifications non considérées comme prêtes pour la production. Le projet reste **pré-alpha, sans release de production**.

**Directive de l'opérateur en vigueur** : gel des nouvelles fonctionnalités, Electron, Cargo produit, interface graphique, fusion globale de l'alpha et refactorings non justifiés. Corriger seulement un défaut démontré, avec tests ciblés ; ne pas fabriquer une PR lorsqu'aucun défaut n'est trouvé. Aucun build/test/installateur/permission sur le PC de l'opérateur, aucun runner auto-hébergé ni release, aucune suppression massive ; GitHub et ses runners hébergés uniquement. Préserver la sécurité des identités épinglées, leases/permissions, budgets, préconditions et preuves de récupération.

Voir [le relais des agents](docs/engineering/GITHUB_HANDOFF.md), [la dette de fiabilité](docs/engineering/RELIABILITY_DEBT.md) et [le prompt de reprise](PROJECT_CONTINUITY_PROMPT.md).

## Historical checkpoints — dated evidence superseded by the canonical section above

## Canonical verified stabilization checkpoint — 2026-10-20

At live inspection, `main` is `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`, **zero open PRs**, and the independent post-merge [CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) completed successfully **on the exact main SHA, first attempt**, with hosted Windows Rust, Linux Rust and dependency-policy jobs all green.

- [PR #290](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/290) is **merged**, at `0c2a125537d020e3a3aff4124d12745d7a8c5c4c`. Its Windows Node/AppContainer test now records monotonic launch/terminal timings and child script-entry markers. Exact-source [CI #39392262806](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392262806) and independent [post-merge #39392829136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392829136) passed. This **does not resolve** the intermittent 10-second Node timeout or distinct 30-second MCP first-response timeout.
- [PR #291](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/291) is **merged**, at `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`. A Windows recovery scan now validates **all** existing staging artifacts before deleting any, then revalidates each through the existing exclusive native delete handle. Exact-source [CI #39393849343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849343), [native preparation #39393849287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849287) and post-merge #39394389914 passed. The negative Windows regression `corrupt_later_staging_does_not_erase_earlier_verified_staging` passed. The second deletion pass is **not multi-file atomic**; ancestor namespace races and sudden-power-loss persistence are not proved.

**Operator delivery policy remains stabilization-only:** no new functionality, broad refactor, experimental alpha merge, weakened CI, permissions or timeouts. Continue only with evidenced defects, targeted tests, and accurate documentation. No tests, builds or permissions changes on the operator's PC, no self-hosted runners, release or mass deletion. Every historical checkpoint below is dated evidence, **not** a live head or pending assignment; verify GitHub before further changes.

## Stabilization-only baseline — 2026-10-20

**Temporary delivery policy (operator decision, 2026-10-20): stabilization only.** Pause new features, product expansion, experimental merges, architectural reshuffles and broad refactors. Work exclusively on verified defects, security and recovery boundaries, flaky-test root causes, focused regressions, and accurate status/debt documents. Do not create code changes merely to keep PRs moving; when no defect can be demonstrated, record the evidence and stop. Preserve all existing permissions, isolation, deadlines, budgets and CI gates. Resuming feature development requires fresh operator direction.

Verified GitHub `main` at this checkpoint: `b1edc83438571867bd164ea0b2b9653304eed752`. [PR #288](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/288) was squash-merged at `b1edc83438571867bd164ea0b2b9653304eed752` from source `60206dcadbf2efb57800ca93f0bd226acf16f6fb`. Its hosted [CI #39388280621](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280621) passed Windows Rust, Linux Rust and dependency policy; [native preparation #39388280521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280521) passed too. Independent [post-merge CI #39388749406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388749406) succeeded on the same merge SHA, first attempt (Windows, Linux, dependencies). GitHub showed zero open PRs at verification.

**Delivered behavior:** `MutationRecoveryJournal::inspect_pending` validates all stable journal records before deleting any well-formed unpublished `*.prepared.tmp` files. A Windows/Linux regression preserves both the temporary and a later corrupt stable record on a failed scan, then checks normal cleanup after resolving corruption. Remaining boundaries include non-atomic multi-temp deletion, parent namespace/identity races, Windows directory persistence and unexplained Windows Node/AppContainer and MCP-first-response intermittent timeouts. These are risks for targeted investigation, not permission to claim fixes without proof.

## Archived proposal checkpoint — PR #288 (now integrated)

The preceding candidate wording was written before PR #288 passed native CI and merged. Its delivery is confirmed in the stabilization baseline above; do not implement it again.

## Linux journal directory sync integrated — PR #287 (2026-10-20)

[PR #287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/287) was squash-merged as `0ca2305e7f01d5263330b67c4a0bb0520d452c8a` from final head `66161c9dee9e1a565b074d1cf105cf8961a712c2`. Hosted [PR CI #39386566516](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566516) and [native preparation #39386566570](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566570) passed; independent [main CI #39387133506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39387133506) passed Windows Rust, Linux Rust and dependency policy on its first attempt. Linux GNU syncs the open recovery directory after publishing the stable prepared journal; failed sync retains a published record for recovery. Windows durability and sudden-power-loss guarantees remain distinct and unproven.

## Atomic no-replace journal publication — integrated by #286 (2026-10-20)

[PR #286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/286) was squash-merged as `67c65a5d78a2921cee826184a2ad8bc573e4fb88` (exact head `7007ad3d202e42f642cece5f8df131035806ff35`). Hosted Windows/Linux Rust, dependency policy and the three native checks passed on that head; independent [post-merge CI #39385303625](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39385303625) passed Windows/Linux/dependencies on first attempt. The stable journal publication uses native Windows `MoveFileExW` without replace/copy flags and Linux GNU `renameat2(RENAME_NOREPLACE)`; competing publishers and existing destination evidence tests passed. Other OS targets fail closed. Directory pinning and complete power-loss durability remain separate.

## State-root creation preflight — pending focused PR (2026-10-20)

GitHub main `adeed077215c2103991254e4b5e2c64fa2d632de` and its [post-merge CI #39381445867](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39381445867) were checked: hosted Windows Rust, Linux Rust, and dependency policy succeeded on first attempt, with no open PRs before this increment. A separate targeted branch `fix/recovery-state-root-preflight` addresses a concrete setup side effect: `MutationRecoveryJournal::open` previously called `create_dir_all` before rejecting a requested state path inside the workspace. This could create nested directories in the workspace despite a rejected state-root request. The candidate first canonicalizes the closest existing ancestor and fails closed on an in-workspace ancestor *before* creating a directory, while retaining the post-create canonical check. Tests cover both rejected absent in-workspace state roots without side effects and accepted absent external state roots. This is **not integrated until exact-head hosted CI and merge**. Concurrent ancestor substitutions and symlink races remain outside this limited check; no general directory pinning or power-loss guarantee is claimed.


## Windows recovery staging cleanup — scoped source correction, 2026-10-20

At resumed inspection, `main` was `d11d27c8efb1048f504d96024e4015ec60fd1535`, with no open PRs and post-merge hosted [CI #39376986248](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376986248) passing all Windows Rust, Linux Rust and dependency checks on attempt 1.

[PR #284](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) isolates a Windows-only recovery staging-cleanup defect: the original code read/validated bytes, closed that file and unlinked a separately resolved name. Valid matching bytes in a staging hard link to an outside file were accepted without detecting the second link. The change reuses the existing exclusive no-reparse, single-link native delete handle, verifies the opened length and hash, and deletes via that **same handle**. A negative native Windows test requires both hard links and the journal evidence to survive failed recovery. This paragraph describes PR scope, not its merge/CI outcome; inspect the [PR's latest head/checks](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) before claiming delivery. Parent-directory pinning, concurrent ancestor swaps and power-loss guarantees remain unproven. The experimental alpha and Windows Node intermittency are unchanged.


## Recovery journal append alias hardening integrated — 2026-10-20

[PR #282](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/282) was squash-merged into `main` as `bf79b320b4e7aa04a02c87fe448d589296bc8bb2` from final source head `c837cead816fb59c5d3a94b8d9b5fe1926987644`. [CI #39375905332](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905332) passed hosted Windows Rust (including native AppContainer and MCP smokes), Linux Rust and dependency policy; hosted [native preparation #39375905331](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905331) also passed. The original head failed Clippy Windows due to an unnecessary `mut`, corrected without waiving checks. No PRs remained open after merging. The distinct [post-merge main CI #39376336498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376336498) was in progress at documentation time; check its actual final result.

Journal `append` no longer writes to hard-linked external evidence: Windows uses the existing exclusive no-reparse single-link open handle, seeks to EOF and syncs; Unix validates opened device/inode/link count against pre-open metadata. The size ceiling is rechecked on the opened file, and a cross-platform regression preserves evidence bytes when the file has an external hard link. No journal schema, authority or state transition changes. **This is not a solution** to Unix concurrent namespace/size races, Windows parent-directory pinning, staged artifact cleanup or general power-loss durability.

Windows Node/AppContainer 10s loopback and MCP first-response 30s intermittencies remain unresolved despite successful latest CI checks. Earlier failed attempts and exact-head retries remain evidence, not accepted causal fixes.

## Windows MCP startup timing evidence integrated — 2026-10-20

Fresh GitHub verification: `main` was `0f857b81a0715b2fcb7cb8f1e2a4250bf9221fa6`, with no open PRs; post-merge [CI 39370890039](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39370890039) passed on its second attempt. Its first attempt still contains a failed 30-second TEMP-only Node MCP initialization smoke; the separate AppContainer Node loopback 10-second intermittency also remains unresolved. These are not proven journal regressions.

[PR #280](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/280) was squash-merged as `8b6ccdc2270f8fa130df5d182d728d07466df908` from exact head `ce15d036332d314e7f951042285d4f442d0219a7`, which passed [hosted CI 39373512931](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373512931) (Rust Windows, Rust Linux and dependency policy). The separate post-merge [main CI 39373992973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373992973) was still in progress at this checkpoint. The scoped change provides opt-in monotonic startup phase timestamps on the bridge's stderr and host-side process-start/initialize timings for the existing GitHub-hosted Windows TEMP-only doctor. Static phase labels contain no path, credential, permissions or MCP payload. Ordinary bridge launches are unchanged unless `OPTIC_STARTUP_TIMING=1` is explicitly selected by the host. The diagnostic does **not** change 10s/30s timeouts, grant/isolation rules, CI acceptance conditions or the Rust core's optional language dependencies. The Windows TEMP-only doctor and Rust opt-in predicate test passed on the exact PR head. Its host-side timings on one passing Windows run were process-started=7ms, initialize-sent=30ms, initialize-received=4823ms (each from the doctor stopwatch). A successful diagnostic run does **not** prove or repair the cause of earlier timeouts.


## Recovery journal read and retirement hardening integrated — 2026-10-20

- [PR #277](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/277)
  was squash-merged into `main` as
  `c2282ff5eba85d2a7480dd353db638a0623bd06d`.
  Its final source head `825bcdb963d80ac8419df887e8b8c307aa497caf`
  passed hosted Windows Rust, Linux Rust, dependency policy
  ([CI 39368379416](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379416))
  and native preparation
  ([run 39368379422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379422)).
  An earlier head failed rustfmt and was corrected; canceled intermediate runs
  are not counted as passing. The independent post-merge `main` CI
  [39368741554](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368741554)
  completed successfully on exact post-merge `main` (Windows Rust, Linux Rust,
  dependency policy).
- Read hardening rejects hard-link journal aliases on both hosted platforms:
  Windows uses the existing exclusive no-reparse, single-link read primitive;
  Unix validates the opened descriptor's identity and link count against
  pathname metadata. This is not a complete race/power-loss guarantee.
- [PR #278](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/278)
  squash-merged as `b15fe7636dcb31033d71ecdfea9ed51bdc786a2b`
  from exact final head `9778b497320d82db493bfbe4c02ff13d113c8ad5`.
  Hosted [CI 39369331802](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331802)
  passed Windows Rust, Linux Rust and dependency policy; hosted
  [native preparation 39369331816](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331816)
  passed. An earlier PR head failed rustfmt and was corrected, not waived.
  Windows journal retirement now uses an exclusive validated handle for
  deletion. Unix rejects aliased evidence before the existing pathname unlink;
  a concurrent pathname replacement remains possible. The separate post-merge
  `main` [CI 39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540)
  initially failed its Windows Rust test on the **known intermittent**
  `capability_free_appcontainer_denies_node_loopback_connection`: owned Node
  returned `TimedOut` after 10 seconds with no output. Three immediate
  serial diagnostics passed. The original first attempt is preserved and
  the failed-Windows-job-only second attempt on the identical `main` SHA
  **passed** all applicable Windows tests and smokes. Linux/dependency jobs
  passed on the original attempt. CI #39369706540 is therefore completed
  successfully on attempt 2, with the original failure retained as evidence.
  There were zero open PRs immediately after the merge.
- Next priority after this recurrent failure: isolate native AppContainer/Node
  startup and scheduling with bounded diagnostics **without** increasing the
  timeout or disabling the network-isolation proof. A separate future domain
  review is journal appends and their opened-file identity, followed by staged
  artifact cleanup or directory pinning, with demonstrable defects and tests.
  Append, staging cleanup, power-loss durability and Unix pathname deletion
  races are not covered by #277/#278. Experimental alpha remains separate.
  No operator-PC or self-hosted execution occurred.

## Final GitHub-verified continuation checkpoint — 2026-10-20

- At this checkpoint, `main` = `e222ff912ac784eb7ae29d09e74ae1780f8a051b` (squash merge
  [PR #275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/275)).
  GitHub reported **zero open pull requests**.
- The exact post-merge `main` CI
  [run 39364949397](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39364949397)
  completed successfully: GitHub-hosted Windows Rust, Linux Rust and dependency
  policy all succeeded. This confirms the state of that commit, **not** an
  unconditional guarantee for future branch heads.
- Post-merge `main` CI for #274
  [run 39364345041](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39364345041)
  also completed successfully. #273 post-merge run 39363288153 succeeded.
- Source/test outcome remains narrowly scoped: six unchanged Rust source files
  reorganized under `src/mutation/`; the recovery parser now rejects complete
  blank journal records, retaining evidence while tolerating torn final lines.
  The first-attempt AppContainer/Node loopback timeout on #274 remains a separate
  unresolved intermittent failure despite a successful exact-head rerun.
- `alpha/generated-project-editing` remains a separate experimental branch;
  no full alpha-to-main merge, release, branch bulk-deletion, local operator-PC
  test, installer, tunnel or self-hosted run occurred.

**Next bounded review:** mutation recovery journal open/read/retire boundaries
and evidence retention, or a dedicated investigation of Windows Node/AppContainer
cold-start timing. Read [GITHUB_HANDOFF](docs/engineering/GITHUB_HANDOFF.md)
and [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md) before changing code.

## Mutation and recovery stabilization — 2026-10-20

- [PR #273](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/273)
  merged into `main` as `58ebc5c1a4f0fd015b26c04f8cd83cf4255d5782`.
  Six unchanged Rust source blobs were relocated into `src/mutation/`;
  their original crate-root module names, public exports and inline tests
  remain unchanged. Exact source head `9758492` passed all four applicable
  hosted Windows/Linux Rust, dependency and native-preparation checks;
  post-merge `main` CI 39363288153 passed.
- [PR #274](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/274)
  merged as `58a72a202bbdb57da425fba461a7b1a97b2f9d92`.
  The recovery journal parser now fails closed on a complete empty journal
  record while still allowing a nonterminated torn final record. Regression
  tests cover interior/trailing blank records, retained journal evidence and
  torn-tail classification. The first source head `8d51d19` failed rustfmt
  and was corrected. Exact final head `283c9e0` passed Linux, dependency
  policy and native preparation; Windows test failure on the first CI
  attempt was an AppContainer/Node loopback probe timeout, **not** a parser
  failure. Three serialized diagnostics passed on the same source, and a
  failed-Windows-job-only rerun passed the entire applicable Windows job.
  Both attempts are retained in CI run 39363540900. The cause of that
  intermittent native test remains unknown and is tracked in
  [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md).
  Post-merge `main` CI run 39364345041 subsequently completed successfully
  on hosted Windows/Linux and dependency policy.

No operator-PC execution, install, permission change, tunnel or self-hosted
runner. These changes do not establish general Cargo/Electron support,
durable disk quotas, power-loss ACID recovery or a complete source audit.

## Latest reviewed stabilization — 2026-10-20

- [PR #270](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/270)
  merged as `636a8b5e917fd731dbd8566e118958f141f67542`.
  Four session-owned modules were moved byte-identically into
  `crates/optic-bridge-runtime/src/session/`. Crate-root module names,
  test namespaces and public exports did not change. Final source head
  `4d5ff0a` passed GitHub-hosted Windows/Linux Rust, dependency policy
  and native preparation checks; post-merge main CI run 39360825623 passed.
- [PR #271](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/271)
  merged as `8610c8dc2558695842412c5a0fce993d2420d6fb`.
  Unexpected stdout/stderr Tokio drain task join failures are now treated
  as incomplete output, not silently discarded. Two synthetic regressions
  cover task panic and intentionally aborted drains after
  `TerminationUncertain`. Final source head `fb9ffdb` passed four hosted
  Windows/Linux Rust, dependency and native preparation checks. The initial
  `bed3ca9` head failed rustfmt and was corrected. Separate post-merge
  main CI run 39361585398 passed.
  
No tests/builds/installers/permissions/tunnels were executed on the operator's
PC; no self-hosted runners. These increments do not prove normal Cargo,
sandboxed Electron, general disk quotas, universal process logging durability
or completion of an exhaustive source audit.

## Offline Cargo research integration — 2026-10-20

PR255 is merged as `f1e72f8`. It completes the fixed research fixture and optional
preparation design, not general Cargo or administrator setup. At final head
`6bbcb64`, research [run 39339860948](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39339860948)
passed prepared job118358440446, including unrelated-profile denial, known-output
cleanup and explicit revoke. Independent normal job118358440142 retained NUL
error5 / exit101. Standard direct-Rust CI is unchanged. See the
[research owner](docs/research/PRIVATE_OFFLINE_CARGO_NUL_2026-10-19.md).


Reviewed: 2026-10-20. This page describes source and measured acceptance, not a
production release. Detailed historical notes are retained in
[the exact previous status snapshot](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/e1b82888dcf9c079740777be27341ca943758859/STATUS.md).

Current continuation is [GitHub-only](docs/engineering/GITHUB_HANDOFF.md): no
operator-PC testing or runtime changes. The [new-chat prompt](PROJECT_CONTINUITY_PROMPT.md)
and [agent entry point](AGENTS.md) do not require old chat memory or local files.

## Current structural delivery and Windows smoke evidence

[PR #268](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/268)
merged to `main` as `d79d60b88b60b2e95c85039c2d31427d0954a865`.
The ten prior `src/git_*.rs` modules are now under `src/git/`, with
identical content hashes and unchanged crate-root module names and
public exports. Exact PR head `b2b5207` passed four hosted checks:
Windows/Linux Rust, native preparation and dependency policy. This is a
mechanical organization improvement, not new behavior or an audit of all files.
Post-merge `main` CI run 39358651176 passed; the subsequent #269
post-merge main CI 39359157209 also passed.

Earlier [run 39356540735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39356540735)
on `main` initially failed the Windows TEMP-only Node MCP doctor smoke
after a 30-second response timeout with the bridge still alive. Re-running
only the failed Windows job at the exact same SHA succeeded, without a
source or workflow change. Root cause is not proven; the failure remains
recorded in [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md).

## Merged and validated

PR266 merged into main as `5a8157e`: a targeted process stdout/stderr
reader-error correction no longer treats a read failure as clean EOF.
A distinct private read-failure bit marks output incomplete, stops a running
owned job through the existing bounded confirmation path, and converts an
otherwise successful exit to `Failed` after pipe drains complete. Timed-out,
stopped and termination-uncertain jobs retain their specific status. Two
synthetic async-reader regressions were added. Final PR head `4e4c712`
passed hosted Windows/Linux Rust, dependency policy and native preparation
checks; the prior revision failed rustfmt and was corrected. This is not a
general guarantee that every process log is durable or fully collected.
[PR #266](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/266).
The separate main push CI run 39356012838 was still executing at the
documentation checkpoint; check its live result.

PR265 merged into main as `a9d9a31`: the `process.rs` test body
was moved unchanged into `process/tests.rs`; production logic remained
byte-identical, with 27 tests preserved. Exact head `f65f69b` passed four
hosted Windows/Linux Rust, dependency-policy and native-preparation checks.
This is structural code quality, not a runtime security fix. The separate
process-output diagnostic issue and its regression work are tracked by
[CODE_QUALITY_REVIEW](docs/engineering/CODE_QUALITY_REVIEW.md); an unmerged
fix is not counted as shipped behavior.

PR263 merged into main as `bcb5e8a` after exact head `2d8a410` passed four applicable hosted checks (Windows/Linux Rust, dependency policy, native preparation). It extracts unchanged inline project-input test logic into a child test module and introduces the scoped [code quality register](docs/engineering/CODE_QUALITY_REVIEW.md); it is **not** an exhaustive file audit or a behavioral improvement. [PR #263](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/263).

PR262 merged into main as `bca2eae` after its exact head `14b978b` passed all six applicable hosted checks (Windows/Linux Rust, dependency policy, native preparation, lifecycle and direct Rust). It adds a host-only, version-pinned retirement API for strictly closed private task evidence, with no model-visible deletion, automatic replay or operator workflow. This does **not** solve general retention UX or eliminate unresolved recovery capacity. [PR #262](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/262). No operator-PC execution. A separate source-organization review is tracked in [the quality register](docs/engineering/CODE_QUALITY_REVIEW.md); extracted tests require their own exact-head CI proof.


PR259/`97f4098` adds lossless Windows path boundary checks and a native
regression for a reproduced alias read across distinct non-Unicode roots.
Git path comparisons/arguments and operator-policy placement receive the same
representation audit. See [ADR-0033](docs/decisions/ADR-0033-LOSSLESS-NATIVE-PATH-BOUNDARIES.md)
and the [reviewed debt register](docs/engineering/RELIABILITY_DEBT.md); this is not
a claim that every filesystem or compatibility gate is closed. Exact
CI39338661674 and preparation39338661374 passed before merge.

| Track | Latest delivered increment | Evidence |
| --- | --- | --- |
| Main core | Native path hardening #259; fixed offline Cargo research #255 | #259 merged `97f4098`; #255 head `6bbcb64` passed CI 39339859057, preparation 39339859114, pipes 39339859071 and direct Rust 39339859055; merged `f1e72f8`. Research normal failure remains separate. |
| Generated FS alpha | Completed Electron diagnosis #250; hardened-core ancestry alignment #260 | #260 head `a4091bc` passed CI 39340735293, adapter/package 39340735266, preparation 39340735187, pipes 39340735200 and direct Rust 39340735265; merged `5245e3d`. Cargo/Electron research baselines remain red. |
| Previously verified alpha package | Source CI merge `ac76efd6ea833e46dfced7bee3148af968754566`, artifact 12229445645 | 85 archive entries and 80 source files verified; native packaged 32 KiB HTTP editing and Node fail/fix/check/test passed before the no-PC continuation. |

The alpha ZIP SHA256 is
`3563bb8e9666455e6559cf6e30362b99849eb437fe91769fc1b6691c2113bc00`.
Its source identity remains fixed; a documentation change does not relabel it.
It uses generated projects, shared connection authority and public dummy auth.
The older `b2b34a47…` package predates #259 and is historical. Actions artifacts
can expire; inspect availability in [run 39340735266](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39340735266)
instead of assuming a local ZIP is accessible to a new agent. No production
release or new PC test is implied by this provenance update.

The core includes scoped/versioned filesystem mutation, separately authorized
Git operations, bounded process lifecycle and private-service foundations.
Not every internal service is exposed as a general MCP project/build tool.

## Compatibility evidence and remaining limits

- **Electron #250:** bounded diagnostic scope merged into alpha as `e62ea95`.
  At head `62f8177`, ordinary CI/adapter/preparation/pipes/Rust passed. The
  staging rerun job118357480984 passed both diagnostic controls and observed
  normal exit `0x80000003` / primary result11 before app startup. The separately selected outer-only diagnostic succeeds; no automatic
  sandbox downgrade or public GUI permission follows. The baseline research
  staging workflow remains red, including run39340735239 on #260; its controls
  and separately selected outer-only comparison do not change normal support.
- **Cargo #255:** ordinary capability-free execution fails opening NUL, exit 101.
  At source `21ac09b`, disposable CI 39291321987/job 118192403768 passed the prepared
  exact-profile comparison, actual checks/tests, output cleanup and explicit
  revoke with restored denial. This is not ordinary Windows Cargo acceptance.
- **Optional Windows preparation:** development direction approved; the proposed
  contract is [in main](docs/research/OPTIONAL_WINDOWS_NUL_PREPARATION.md). No local privileged helper or device permission change
  has been applied. Authentication, lease provenance and interrupted recovery
  remain implementation gates.
- **Real projects:** writable output containment, dependency restore, secrets,
  enrollment and artifact publication require their own proven contracts.
- **FS failures:** retained local evidence cannot expose FS backend logs or
  restore hidden model state; unknown writes must be observed before retry.

## Repository consolidation

41 historical PRs were closed with explicit disposition. Their work remains in
alpha ancestry, exact GitHub pull refs or published archive tags. The later #250
and #255 integration completes their bounded measurement scopes; unresolved
compatibility work remains in this status and the debt register, rather than
indefinitely open diagnostic carriers.

244 historical remote branches were removed after explicit operator approval,
exact-head/preservation revalidation and backup hash verification. The four
reference branches and all preservation refs were verified unchanged at that
retirement checkpoint. After later integrations #250/#255/#260, the pre-handoff
inventory was zero open PRs and two branches: `main` and
`alpha/generated-project-editing`; completed short-lived branches were retired.
This dated inventory must be refreshed live by the next agent. Exact historical
heads, approval and observed results are in the
[consolidation record](docs/engineering/REPOSITORY_CONSOLIDATION_2026-10-20.md).

Next delivery steps are owned by [ROADMAP.md](ROADMAP.md). Core and alpha remain
separate until their applicable release gates are satisfied.
