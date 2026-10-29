# Changelog

## 29 October — bounded host-stop first-failure evidence

The physical supervisor now preserves optional failed-report diagnostics before
its temporary files are deleted, including early ownership-handshake failures.
Each worker/Bridge stderr tail reads at most 4096 bytes; missing/unreadable or
oversized logs cannot mask the case error, and later recovery errors cannot
replace it. Eleven offline observer tests cover these paths. Successful case
criteria, eight Windows/four Unix cases, deadlines, no-retry rule and product
code stay unchanged. #431 main CI #40940330506 remains first-failure evidence
with unknown worker cause; this change does not claim a startup fix or V1 GO.

## 29 October — private isolation launcher without a console (#431)

The Windows LimitedJobObject wrapper gains a typed opt-in no-console mode,
selected only for the private isolation launcher. Other managed launches keep
the default; CREATE_SUSPENDED, Job limits, assignment and resume ordering remain.
No arbitrary creation flags, grants or protocol options are exposed. Schema 6
adds physical Ctrl+Break with a running isolated Node and open stdin. The console
inventory still requires exactly worker plus Bridge before broadcast; its first
hosted candidate refused a third client rather than allowing a direct signal to
the helper. Exact-head/main evidence is required. V1 remains NO-GO; no release,
held-admission, product resource-release or native Git acceptance is inferred.

## 29 October — physical active managed Node EOF case

The hosted schema-5 supervisor adds one real MCP-started isolated Node job to the
existing stdio shutdown cases. It verifies nonce/PID output, operator image and
original outer Job membership, retains a read-only native process handle, then
requires normal Bridge exit and Node kernel-signaled exit within one unchanged
five-second window after transport EOF. Natural timeout and fixture recovery
cannot supply a successful result. Atomic ownership records and seven offline
observer regressions guard the new fixture path. No production code, permission,
deadline, sandbox or release behavior is changed. This is not held-admission,
physical target-stream EOF, product Job/quota/profile cleanup or native Git proof;
final exact-head/main evidence remains mandatory and Windows V1 stays NO-GO.

## 29 October 2026 — Windows stdout ownership (#429)

Windows stdio now owns a dedicated bounded output worker for synchronous pipes
and redirected disk files. Flush observes actual write completion; host shutdown
retains original session/admission cleanup, cancels the exact writer thread and
actually joins both IO owners. Native IO errors propagate and no pooled writer,
forced exit or drain timeout is used. Character output is rejected before grants.
Both original IO factories additionally attest the retained handle's synchronous
mode and reject overlapped/unknown mode before spawning, using a binding feature
of the existing pinned Windows crate. No data IO is attempted on an unsupported
handle and no driver/privilege or dependency version is added.

Physical schema 4 requires root exit with parent input and unread stdout kept
open; closing/draining output cannot make it pass. Initial CI #40919730293 instead
exposed a partial-stderr parser crash before reaching that gate; complete-record
observation and three offline regressions fix that harness defect without
weakening its assertions. Native byte/flush/EOF, cancellation/Drop, disk and
broken-reader tests supplement hosted proof. [Contract and limits](docs/decisions/ADR-0038-WINDOWS-STDOUT-OWNERSHIP.md).
Final exact-head/main evidence is required. Windows V1 NO-GO, native Git inactive.

## Unreleased — Windows unread-stdout characterization (#428)

- Add a Windows-only physical Ctrl-Break case using 32 bounded metadata-only
  `session_info` requests, then retain the unread stdout pipe across original
  cleanup and stdin-reader join. Verify process retention for 500 ms and exit 1
  after closing only the output reader, with input open through exit.
- Publish schema 3: six Windows cases and four unchanged Unix cases. Mark the new
  case explicitly as characterization; retain the normal Windows exit/join gates.
- Hosted first candidate `9e0750b4cdb742bcb8a834180f594471bd9c5951`, CI
  [40877904438](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40877904438),
  attempt 1, all four jobs successful. No production runtime or GitRead activation
  change; Windows V1 remains NO-GO.

## 29 October 2026 — owned Windows synchronous-pipe input

PR #427 replaces Tokio stdin on Windows with a non-inheritable owned pipe handle,
dedicated bounded reader and real cancel/join after original session cleanup.
One process claim prevents rearming; console/disk stdin is rejected before grants.
A stop request never substitutes for completed join. Unix keeps its existing input.
Physical Windows cases now require exit while parent input remains open and an
ordered join marker; schema v2 separates that acceptance from Unix retention.
Four native regressions cover bytes/EOF, early/idle stop, backpressure Drop and
non-pipe rejection. The regular Rust test step gains an external 10-minute CI
budget; product deadlines and the physical 5s gate are unchanged.
[Contract and limits](docs/decisions/ADR-0037-WINDOWS-PIPE-STDIN-OWNERSHIP.md). V1 NO-GO; no release/native GitRead.

## 29 October 2026 — physical process-stop coverage

PR #426 adds hosted real-binary Ctrl+C/Ctrl+Break and SIGINT/SIGTERM fixtures,
private-console inventory and external supervision, plus Windows headless MCP/EOF.
Three existing opt-in diagnostic phase markers expose listener registration,
stop receipt and original cleanup return; permissions/deadlines/drain stay unchanged.
Eight corrected-code signal samples reached cleanup with stdin open but retained
the process at 500ms; input closure preceded reaping. This is retention evidence,
not bounded open-stdin exit. First fixture CLI error is preserved separately.
[Proof, exact evidence and limits](docs/engineering/PHYSICAL_HOST_SHUTDOWN_PROOF.md).
Windows V1 NO-GO; no release or native GitRead activation.

## 29 October 2026 — cooperative process requests close original stdio authority

PR #425 observes private process-owned Ctrl+C/Ctrl+Break (Windows) and
SIGINT/SIGTERM (Unix) during MCP initialize/serving, then uses original-session
cleanup. A ready stop wins over a ready service result; source errors stay errors,
and counted admissions remain until drainage. Four notification-injected RMCP
regressions first failed behaviorally on both OS in CI #40857686017.

Physical OS delivery, runtime/process exit with open stdin and bounded legacy
cleanup are not established. No forced exit, new native GitRead permission or
release; Windows V1 NO-GO. Exact final receipts are maintained in #425/#323.

## 29 October 2026 — close original stdio session independently of renewal

Transport EOF and returned MCP initialization/wait errors now invoke cleanup on
the original lifecycle and session even without `--supervise-session`. Cleanup
publishes stop and waits for counted admissions; successful cleanup preserves
the original service result. Renewal timing and MCP permissions stay unchanged.

Local Windows test-first reproduction obtained three behavioral failures with
conditional cleanup, then three passes after correction. Real RMCP transport
and original registries cover EOF, failed initialize, session/lease stop, retained
capacity during drain and another session's noninterference. The actual stdio
binary suite also covers active default-mode EOF.
Hosted exact-head and independent-main CI must be checked before integration is
claimed. This is ordinary service cleanup, not a native HostStartup/policy launch
fence or physical Git termination; legacy reader joins remain unbounded.
**Windows V1 NO-GO.**

## 28 October 2026 — original lease stop before admission drainage

Baseline `main=eea5438873293243cd747b0f646ed903fb9f1282` integrates [#420](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/420); independent [push/main #40752232971](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40752232971) verified SUCCESS 4/4, identical tested/integrated trees.

The next targeted regression holds a genuine counted session admission during lifecycle revoke. Original session stop is already published before drainage, but the original TaskLease stop is currently published only afterward. A lease-only observer can therefore wait for a stop that depends on its own admission being released. The test-first candidate preserves that order to obtain behavioral RED.

Required correction: publish irreversible lease revoke flags before waiting for session admission drainage, retain the lease records/pins and counted resources, then repeat the lease sweep after drainage to catch registrations by already-admitted producers. Sum newly revoked counts from both sweeps without double counting; attempt all cleanup and propagate errors. This is stop publication, **not** destruction or physical process termination.

The real-registry test checks both original stop signals while the permit is held, failed early removal due to in-flight admission, revoke still waiting, a late registration caught after drain, exact revoke counts and eventual no-child reaping. No process is created; no policy/network/timeout relaxation. Final corrected-head and independent-main evidence must be checked before claiming integration. **Windows V1 NO-GO.**

See the new [current continuation](docs/engineering/WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md) (from engineering pages: `WINDOWS_V1_CONTINUATION_CLOCK_LEASE_2026-10-28.md`) for the next host/policy authority boundary and exact remote-check procedure.



Behavioral RED on first HEAD `cad47990342c8aa84a9ce6bcdb62943f7e90bdfa`: [Windows specialized #40753355684](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40753355684), job 123045630581, failed `lease stop waited for admission drainage`; ordinary [CI #40753355716](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40753355716) Ubuntu job 123045629922 failed the same runtime assertion (309 passed / 1 failed), after fmt/Clippy. No native child.

The correction publishes only irreversible TaskLease stop flags before drainage, retains original records/pins/capacity, and keeps the post-drain sweep. Newly revoked counts from both sweeps are combined; all existing cleanup attempts occur before either lease result is propagated. Final corrected-head and independent-main receipts are required before claiming integration.

## 28 October 2026 — session renewal admission/validation interleaving

Baseline `main=d66014405c7d4694ed3d7e31c0bf354a140f308d` integrates [#419](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/419); independent [push/main #40750356069](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40750356069) verified SUCCESS 4/4, identical tested/integrated tree.

The next targeted regression concerns a lifecycle debug assertion comparing an admission's captured expiry to a later registry observation. Admission and validation use separately released guards; a second valid renewal can extend that **same** session between them. Their numeric expiries need not remain equal. A private phase observer permits a deterministic real-registry interleaving without native execution. The first test-first candidate deliberately preserves the old assertion to obtain behavioral RED before correction.

Required final contract: ordinary renewal validation (active/nonrevoked, strict extension, maximum horizon and subordinate authority validation) remains mandatory. A successful report's previous expiry refers to this attempt's original admission, not a promise that no concurrent renewal happened. Existing operation permits keep their original deadline; session close between admission and validation must fail before extending any lease. Removing an invalid debug-only equality must not remove the real validation or change release-build authorization.

Two exact Windows-specialized tests cover the interleaved extension and interleaved close, alongside ordinary Windows/Ubuntu tests. Final corrected-HEAD and independent-main evidence must be checked before claiming this increment fixed/integrated. No Git process, launch authority, new permission/dependency/unsafe, timeout change or #185/#319 resolution. **Windows V1 NO-GO.**



Behavioral RED obtained on test-first HEAD `7553f7498178fdd8c704c6d8f931c9c41fe8d7e8`: [Windows specialized #40751137896](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40751137896), job 123037798106, failed the original equality with `MonotonicTime(200)` vs `MonotonicTime(100)`, and the regression then reported the outer renewal panic. This is an actual lifecycle/registry interleaving, no process simulation. Ubuntu in ordinary [#40751137862](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40751137862) failed formatting separately; no claim of Ubuntu behavioral RED.

The corrected implementation removes that debug-only equality and its unused observation variable, while preserving the **actual** registry validation and all later checks. It keeps the admitted original expiry in the report/old permit; it does not substitute the current renewed expiry as an operation deadline. Final-head and independent-main validation remain required.

## 28 October 2026 — TaskLease identity IO outside revocation mutex

Baseline `main=5e8d26e4f2b4bef2844cd4d4efb9c5d3876b3cfb` integrates [#418](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/418); independent [push/main #40748015791](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40748015791) verified SUCCESS 4/4, tested/integrated trees identical. #418's first natural Node/AppContainer red remains preserved in #185, not explained or erased by later green source.

Lookup and session-wide lease renewal previously canonicalized/opened/hashed executables while holding the lease revocation mutex. A stalled identity read could therefore prevent original revoke/removal from publishing. The next slice captures immutable lease/generation/clock metadata plus shared **original executable pins** under that mutex, performs real identity verification outside it, then re-enters the mutex to reject revoke/removal/replacement before returning. A concurrent renewal cannot extend the lookup's captured horizon or replace its provenance.

Renewal verifies outside the mutex, then validates the entire original active set, generation and captured expiry before any update. Concurrent additions/removals/reuse/revoke or overlapping renewal fail closed with the existing public lifecycle denial; no partial expiry publication. Retained Arc authorities keep Windows pins alive after physical registry removal until verification finishes. No reader/IO thread is detached or abandoned.

Three deterministic registry tests inject a bounded pause at the actual verification boundary while retaining real file pins and hashing afterward. They prove revocation/removal/replacement can complete before verification is released, stale lookup refusal, original horizon after renewal, unchanged-set all-or-nothing renewal, and real Windows write denial after registry removal. This is a concurrency/ownership proof with an injected phase stall, **not** an observed blocked kernel filesystem call or native Git termination. Specialized hosted Windows CI executes these exact tests.

No native launch, new unsafe/dependency, timeout increase, permission relaxation or public MCP error change. Cross-registry session/lease/policy/host effect linearization, actual service owner, physical root/Job/stdio and #185/#319 remain unproved. **Windows V1 NO-GO.** Verify final exact-head and independent post-merge CI before using this slice as integrated evidence.


## 28 October 2026 — original TaskLease clock provenance

Baseline `main=6bcdd77d3dcf259dfe03145fe583f9ff832e2974` integrates [#417](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/417); independent [push/main #40744570584](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40744570584) verified SUCCESS 4/4, tree identical to tested source.

The next inert slice issues a **new** TaskLease expiry on the retained StdClock from a positive checked lifetime. Lease identity/scope/capability metadata is retained; an existing registration or transferred witness cannot be retroactively tagged. Original registration provenance is captured together with the executable-verified lease and generation stop signal under the existing mutex. The service-clock native constructor checks **that retained witness**, not a second public-ID lookup, before binding the watchdog clock. Missing or foreign lease provenance fails closed and releases the no-child worker permits.

Successful untagged expiry extension removes provenance for future preflights; already captured witnesses retain their original horizon, origin and generation stop signal. Legacy registration/lookup/renewal and production MCP remain compatible. Clock-coherent fixtures now issue transport, session and TaskLease on the same service clock; the action horizon is sampled by preparation on that retained clock. Existing independent watchdog/revoke/rearm tests remain non-executing.

Tests cover raw/foreign/cloned origins, duplicate-registration refusal, invalid/overflow lifetime, real expiry/revoke, renewal and same-ID replacement, and native preparation rejection with permit cleanup. No unsafe/dependency, new native launch or security-policy change. Ordinary exact-head Windows/Ubuntu CI, specialized preparation and independent push/main must be checked before declaring integration complete.

**Remaining limits:** this proves an internal inert construction, not live MCP worker wiring, cross-registry atomic launch, HostStartup/policy cancellation, crash recovery, physical Git/descendant exit or pipe EOF. Raw legacy permits cannot arm this native watchdog. Native GitRead stays disabled; Windows V1 stays **NO-GO**.


## 28 October 2026 — original session clock issuance and admission

Previous increment [#416](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/416) is squash-integrated at `36fc443cfa70bd36873aa25ba2ca17161ec4276f`, tree identical to its tested HEAD; independent [push/main #40742225141](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40742225141) verified SUCCESS 4/4.

The next inert slice binds original session registration and owned admission to that retained service clock identity. The internal registration issuer computes a **new** expiry from its own clock sample and a positive checked lifetime; its SessionGrant parameter supplies identity/capability metadata, not an expiry to attest. It cannot tag an already registered grant or transferred permit. The clocked owned-admission constructor captures the exact registration provenance under the existing admission mutex, checks it against its clock and rechecks original expiry/stop after admission. Logical-only admission remains untagged even for a clock-issued session. Native service-clock preparation rejects missing/foreign session provenance before creating any native resource.

Legacy register/admit/renew remain compatible. A successful untagged renewal clears provenance for **future** clocked admissions; existing permits retain their original immutable expiry and identity. No renewal can extend their native horizon. Tests cover foreign/missing origin, raw admission, Clone, checked deadline errors, real expiry, revoke, session handle reuse, renewal and RAII rejection cleanup; existing empty-Job/watchdog tests are retained with clock-issued session fixtures.

**Not end-to-end authority:** TaskLease expiry still lacks provenance; production MCP still uses legacy GitRead. No child, new launch API, global session/lease/host/policy fence or physical terminal proof is added. No unsafe/dependency or GitHub-policy change. Windows V1 remains **NO-GO**. Exact final PR CI and independent post-merge CI must be verified before claiming this slice integrated.


## 28 October 2026 — transport clock provenance (first P0 slice)

Baseline `main=d7025f7a62edcbdd25545ebb794791072eca7d98`; independent [push/main CI #40711907388](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40711907388) verified SUCCESS 4/4; #415 merged.

The inert native preparation now rejects transport permits admitted without clock provenance or on another StdClock. A crate-private clocked admission captures the time itself and retains an allocation identity minted by StdClock; clones retain identity, new clocks cannot substitute it. Identity is attached only during original transport admission, never retrofitted to a transferred permit. Rejection consumes and RAII-releases the original worker permits before native registration; the legacy logical admission and production MCP path remain compatible.

This deliberately small slice establishes **transport provenance only**. SessionGrant and TaskLease expirations and session admission still carry untagged MonotonicTime. Their clock provenance is NOT demonstrated; successful inert preparation is NOT end-to-end temporal authority or launch permission. Action time is sampled by the retained preparation clock. Before native activation, bind original session/lease issuance and admission to that same authority, preserve captured deadlines through renewal, then prove unified policy/host/create/resume ordering and physical root/Job/EOF ownership. **Windows V1 NO-GO; #323 remains open; no native GitRead activation or release.**

Tests add equal logical deadlines with distinct origins, clock clone/foreign/missing transport identity, original deadline retention, native rejection cleanup, and valid clone preparation. Existing no-child watchdog expiry/rearm/revocation tests remain applicable. CI receipts must be checked on the final PR HEAD and independently after squash; no local project code was run. Historical local recovery notes were read but not restored or treated as current GitHub evidence.


## 28 October 2026 — service-clock watchdog binding

Continuation from verified `main=aa50c09570d5894118d33590657465df644299f3`, [independent CI #40693716141](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40693716141) SUCCESS 4/4. The non-executing native owner now requires preparation through the internal `prepare_native_status_on_clock` constructor. It samples the retained service `StdClock` itself and projects the earliest captured absolute deadline as `clock.origin + deadline`, using checked Instant arithmetic. Arm accepts neither a replacement clock nor a caller-supplied logical `now`. Logical-only legacy preparation has no watchdog-clock binding and fails closed with `ClockRequired`.

Tests cover projection retaining the original origin across clock clones and an already elapsed horizon, missing-clock rejection, actual independent watchdog expiry, and expiry-before-arm after renewal. Existing original session/TaskLease-ID-reuse/Job stop tests now admit their worker permits on the same retained clock. No child is created; quota/transport cleanup remains checked.

Validation receipts and final integrated SHA belong to the targeted PR and [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). This entry describes the change, not an unverified CI success. **Windows V1 NO-GO**, no release, no native MCP GitRead switch. Future real service integration must supply its original worker clock; permits still use untagged logical times, so foreign-clock provenance across arbitrary permits is not claimed solved. Host/policy cancellation, unified create/resume authority and physical root/Job/EOF receipts remain required. Older checkpoints below are historical.


## 28 October 2026 — PR #414: original-authority watchdog wiring

Technical source `8438534eabd4d4678d97b430180c45811af2a9db`: [CI #40691925094](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691925094) **SUCCESS 4/4**, including hosted Windows/Ubuntu, strict Clippy, fmt, dependency policy and release evidence guard. Windows job `122828738433` explicitly passed:
- `git_read::native_status_tests::original_watchdog_observes_session_lease_generation_and_job_stops_without_child`
- `git_read::native_status_tests::original_watchdog_rejects_expiry_before_arm_without_resetting_original_horizon`

Same-source specialized Windows runs succeeded: [native lifecycle #40691925103](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691925103), [private preparation #40691925095](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691925095), [Rust compilation #40691925262](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691925262). Initial candidate `0476ac28` failed fmt in [#40691730888](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40691730888); review also corrected an erroneous textual timer edit before final validation. No retry-to-green.

The non-executing owner arms #412 from its retained original session, TaskLease-generation and Job cancellation signals. No MCP-controlled flags or public-ID lookup selects them. A conservative absolute Instant horizon is captured **before** preparation from the earliest original action/transport/session/lease horizon, and passed unchanged through handle duplication and worker startup. Session renewal, delayed arm and repeated arm cannot restart it. Tests separately observe the worker reacting to session close, original lease revoke despite ID reuse, and Job stop; expiry before arm after session renewal fails closed. Original transport/process capacity remains counted until no-child disposal.

**Limits / next priority:** source-to-service clock provenance and unified host/policy/session/lease create/resume ordering still require an integrated native guardian; the trusted logical `now` must come from the same service clock as admission. Independent registry receipts are not an atomic launch grant. Worker completion is not a successful TerminateJobObject receipt, root/descendant termination or stdout/stderr EOF. No process is created, and live MCP GitRead still uses HardenedCommandRunner. **#323 OPEN, Windows V1 NO-GO; no release.** Validate the final PR HEAD (including this documentation) and independent push/main CI before calling integration complete; the exact squash SHA and postmerge receipts are recorded in [PR #414](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/414) and [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). Earlier dated entries below are historical.

GitHub-hosted checks only. Local recovery disk is outside this task scope and unverified; no project build/test or installation ran on the operator PC.


## 28 octobre 2026 — jalon #412 : watchdog natif précréé, V1 toujours NO-GO

**État vérifié de `main` avant la présente PR documentaire :** `f773ab0860f12bdaa7c48baa1f358d64ad219d1c`. La PR technique [#412](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/412), HEAD exact `4a1a3f1aa7bf50ab0388c1f3e17935e894930400`, est fusionnée. Sa [CI de PR #40682603293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40682603293) et la [CI indépendante de `main` #40683212714](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40683212714) ont chacune fini **SUCCESS, 4/4 jobs** au SHA correspondant. Le runner Windows de PR (job `122796825413`) affiche `test job::precreated_native_read_tests::unstarted_native_read_watchdog_is_nonrenewable_and_deadline_bounded ... ok`. Les trois workflows spécialisés sur ce HEAD sont verts : [Owned Native Process Pipes #40682603185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40682603185), [Private Rust Compilation #40682603354](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40682603354), [Private Project Preparation #40682603219](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40682603219).

**Portée exacte #412 :** un seul fichier Rust, `crates/optic-bridge-windows/src/job.rs`. `PrecreatedNativeReadCustody` peut armer **une fois** son `NativeJobDeadline` indépendant sur le **Job original** : délai positif, au plus quatre `Arc<AtomicBool>` d'arrêt, refus du réarmement et de paramètres invalides, duplication non héritable du handle de Job et destruction du watchdog avant le Job. Test sur un **Job réellement ouvert mais sans aucun enfant** : échéance du watchdog observée, Job vide. Ceci **ne prouve pas** la terminaison d'un GitRead réel, des descendants ou l'EOF physique des pipes; ni l'autorisation atomique de lancer. Les véritables signaux d'arrêt d'origine et le temps restant depuis la deadline monotone ne sont **pas encore câblés** à cette primitive dans le runtime.

**Preuves précédentes conservées :** [#410](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/410) prouve en test Windows synthétique qu'après sortie du processus racine un descendant peut garder stdout/stderr ouverts, et que la terminaison du Job permet d'observer **séparément** Job vide + EOF des deux pipes. [#411](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/411) a consigné cette preuve en documentation. Aucun de ces tests ne migre le `git_status` MCP vers un nouveau launcher.

**P0 suivant (une PR à la fois) :** définir une liaison sûre entre la custody précréée et les **témoins d'arrêt originaux et non substituables** (session, TaskLease, stop du Job, host), conserver la **deadline absolue initiale** et empêcher qu'un renouvellement/redémarrage du compteur ne donne plus de temps. Ne pas exposer des signaux contrôlables par un appelant non fiable. Exiger des tests de révocation/expiration entre étapes sans créer de processus. Ce câblage n'est **pas** une barrière globale : session et TaskLease utilisent deux mutex distincts. Une véritable borne de linéarisation avec politique, identité de cible, HostStartup et CreateProcessW/ResumeThread reste à concevoir, sans verrou détenu pendant les appels Win32/I/O ni abandon incertain de ressources.

**Garde V1 :** [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN / Windows 10-11 V1 NO-GO**; [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) et [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) **OPEN**. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est fermée **administrativement**, sans preuve de correction du `HardenedCommandRunner` live dont les `.join()` des lecteurs de pipes peuvent rester bloquants. Ne pas activer GitRead natif ni publier d'installateur/release avant validation réelle. Interdiction d'intervenir sur le PC utilisateur, les dépôts Desktop/REL ou des runners auto-hébergés. CI GitHub Windows et Ubuntu uniquement. Méthode : **une seule PR ciblée à la fois, exact-head CI, revue, squash, puis CI indépendante de `main`**.

## 28 October 2026 — #410 physical descendant-held pipe/Job proof (NO-GO)

**Merged research-only test:** [PR #410](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/410) at `main=20ed7887715601e260cc6f0fe9e73c18a1b3170e`. Exact final PR HEAD `99ba2e5d773bb82ad0ac6b3f39142055b1a3cb70` passed [CI #40679470136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40679470136) **completed/success, 4/4 jobs**. Windows job `122786478971` logged `job::job_list_launch_tests::descendant_keeps_both_pipes_open_after_root_exit_until_job_termination ... ok`. Three exact-head hosted Windows workflows [Owned Native Process Pipes #40679470081](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40679470081), [Private Rust Compilation #40679470074](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40679470074) and [Private Project Preparation #40679470063](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40679470063) passed. Independent post-merge [push/main CI #40680226256](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40680226256) was **in progress when this checkpoint was written**; recheck its final exact-SHA, four-job outcome.

**Physical scope:** a fixed `cmd.exe` root starts suspended already inside a private kernel Job, with both `PROC_THREAD_ATTRIBUTE_JOB_LIST` and a three-handle stdio inheritance list. It starts one fixed sleeping Windows PowerShell descendant using `START /B` and exits with code 0 while Job accounting still sees a live descendant and **neither stdout nor stderr has physical EOF**. Explicit `TerminateJobObject` then results in independently observed bounded Job-empty, stdout EOF and stderr EOF. No user project, actual GitRead, network, production launch owner or MCP change; one Rust Windows test only. The first candidate CI failed rustfmt; it was corrected before the successful final HEAD.

**Still blocked (P0):** the MCP `git_status` implementation continues using `HardenedCommandRunner` with potentially unbounded blocking reader joins. No single synchronized session + original TaskLease + policy epoch + host shutdown native launch/revoke issuer exists; no production Win32 guardian, fail-closed owner-loss recovery, real native Git status/descendant budget/timeout acceptance, or validated Windows 10/11 device campaign exists. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) stays **OPEN — Windows V1 NO-GO**; #185 and #319 remain separate blockers. The next engineering PR must target genuine native ownership/authority, not activate the service from a synthetic witness. No releases, other repositories, self-hosted runners or user-PC operations.

## 28 October 2026 — #408 real stdout/stderr EOF on fixed Windows root

**Integrated technical test:** [PR #408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/408) tested exact HEAD `e346a0b6f219ca55a360b292879b4f16468e8c01` with [CI #40675433145](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433145) **completed/SUCCESS, 4/4 jobs, attempt 1**. Hosted Windows job `122772868039` logged `job::job_list_launch_tests::resumed_job_root_produces_two_physical_pipe_eofs ... ok`. The three specialized hosted Windows workflows [#40675433176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433176), [#40675433141](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433141), [#40675433144](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40675433144) also succeeded on the exact HEAD. Squash-integrated `main=fb10d7e98664a6b4deda2c8b509bdf1048672618`, tree unchanged `b65ff73a7351bac620256fb923457062cc4bfed0`. Independent post-merge [push/main CI #40676169755](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40676169755) was triggered for this precise SHA; **check all four final jobs live**, do not equate the PR checks to that independent CI.

**Exactly one Rust Windows test source**, no production GitRead or MCP changes. A fixed system `cmd.exe /D /Q /C` echo fixture is created **suspended** with kernel `PROC_THREAD_ATTRIBUTE_JOB_LIST` AND explicit inherited-only `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, `NUL` stdin and separate bounded pipes. The test verifies Job membership before `ResumeThread`, real root exit code `0`, separate kernel Job-empty, **physical EOF of BOTH stdout and stderr**, and distinct child output markers. The original candidate's first CI failed formatting only; corrected on final HEAD. No user-controlled project, real Git, external network, or production owner is involved.

**Critical nonclaims:** no actual service-owned GitRead process, descendant-retained writers or bounded timeout with descendants, native owner/Job uncertainty recovery, shared session/TaskLease/policy/host commit/stop ordering or safe Win32-call stall handling. `HardenedCommandRunner` remains live for MCP GitRead with known potentially unbounded reader joins. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is administratively closed, not behavioral acceptance. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN: Windows V1 NO-GO**; #185 and #319 remain separate blockers. [Native authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md) is still the required next technical design/proof. No release or PC changes.

## Unreleased — 28 OCTOBER 2026, Windows Job resumed-root proof #406

**Verified integration:** [PR #406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/406) is **squash-merged**. Tested final HEAD `f4c7b8163d36434f5a26486625d4963c42a0dee8`, integrated `main=be59544537d5dc027b54e4bb5cd7f553f979d59d`, identical trees `f4d702915e429821bd658d6317014e0cd316241a`. [PR CI #40672893075](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893075): **4/4 jobs SUCCESS, first attempt**, exact final HEAD. Same-HEAD hosted Windows workflows [Owned Native Process Pipes #40672892946](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672892946), [Private Rust Compilation #40672893056](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893056), and [Private Project Preparation #40672893094](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40672893094): all SUCCESS. Independently triggered [push/main #40673602054](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40673602054) on the exact integrated SHA; **verify its completed/final four job conclusions live** before using it as post-merge proof.

**One test-only Windows file**, `crates/optic-bridge-windows/src/job_list_launch_tests.rs`: `job::job_list_launch_tests::resumed_root_exits_and_kernel_job_drains_after_job_list_creation ... ok` in hosted Windows Rust log. A fixed synthetic Windows `cmd.exe /D /Q /C exit /b 0` is created suspended, already Job-associated via `PROC_THREAD_ATTRIBUTE_JOB_LIST` at creation, then actually resumed; root exit code 0 and **separate physical Job-empty** observation are bounded. Original PR CI failed `rustfmt` on an earlier HEAD, corrected and preserved as a formatting failure, **not** a launch/liveness failure.

**NOT demonstrated:** no service-owned native GitRead launch/MCP switch; no descendants, stdout/stderr EOF, independent host/session/TaskLease/policy cross-authority final commit, Win32 blocking stop, uncertainty guardian, or legacy runner elapsed-time correction. Sessions and leases still issue independent receipts under different mutexes. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN**; [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) closed administratively, original pipe liveness not shown resolved; #185/#319 open. **Windows V1 remains NO-GO; no release.** Follow the concrete mutator/lock/host audit in [#323 native authority audit](docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md); do not confuse static policy epoch equality with a live cross-authority issuer. Previous dated sections are historical snapshots.

## Unreleased — 28 October 2026, session/TaskLease stop-order regressions (#404)

- [PR #404](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/404) : HEAD testé `cc7a77b3b5415c6b8d601c477c2373e39f84f7b6`, squash intégré `main=0d14502da39d90ccecd8f113981cd3bdb6520159`, arbre source/intégré identique `55bd4edc12e645ddc3e27a1cb799db932208cec0`. CI [PR #40666361354](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361354) **4/4 SUCCESS sur HEAD corrigé**, [native #40666361300](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666361300) SUCCESS, et **CI indépendante push/main [#40667115232](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40667115232) `completed/success` 4/4 jobs, tentative 1, sur SHA intégré exact**.
- **#404 ne modifie que les tests Windows** (`crates/optic-bridge-runtime/src/git/native_status_tests.rs`, +189 lignes) : `session_close_between_original_session_and_lease_receipts_denies_native_custody` et `dual_receipt_session_close_after_lease_commit_drains_and_stops` ressortent **`... ok`** dans les logs Windows. Le premier scénario prouve qu'une fermeture lifecycle de session *entre* les reçus session et TaskLease interdit de poursuivre malgré une TaskLease encore active. Le second prouve qu'une révocation directe *après* les deux reçus publie le stop de génération d'origine, attend la libération du permis compté, et conserve le Job Windows **vide** et les quotas jusqu'au drop. Aucun lancement Git.
- [Première CI de #404 #40666050161](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40666050161) : échec `rustfmt` seul sur premier SHA, corrigé ; Windows première tentative annulé par supersession. **Pas** un échec comportemental natif.
- **Limites :** les reçus session (#400) et TaskLease (#402) sont émis sous **deux mutex indépendants**, et ne constituent PAS un fence atomique pour policy epoch, host shutdown, `CreateProcessW`, `ResumeThread` ou publication. Aucune preuve après exécution Job vide/descendants, stdout/stderr EOF, watchdog borné ou récupération de terminaison incertaine. MCP GitRead reste dans la voie legacy `HardenedCommandRunner` (pipe joins non prouvés bornés). [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) est **CLOSED administrativement**, sans preuve physique suffisante du pipe-join ; [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323), [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) **OPEN**. #352 fermée non fusionnée ; #355 recherche draft. **Windows V1 NO-GO, aucune release.**
- New complete [28 October 2026 chat-handoff prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-28.md). No release or production native launch path introduced.


## Unreleased — 28 October 2026, original TaskLease native read receipt (#402)

- [PR #402](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/402): exact tested HEAD `04a0c025202dbf2dfbd8cf43d3314c64260e6bee`, squash `main=fc53da251ff1c6b396fcdddc8092b9d18712cfd9`; identical tree `d287df495e1543e8d04e083651a34fd9ed641aa6`. Exact-head [CI #40663011191](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011191) **SUCCESS 4/4** and [specialized native #40663011174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663011174) **SUCCESS**; independent [push/main #40663744586](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40663744586) **SUCCESS 4/4 on exact merged SHA**.
- The Windows-only, non-executing native GitRead candidate now retains **two separately issued** original-generation stop receipts: session (#400) and TaskLease (#402). The TaskLease receipt is minted under the same `TaskLeaseRegistry.leases` mutex as direct/session revoke, physical removal and registration, requires exact original registration-signal identity and preserved admission-time expiry. Four hosted Windows tests prove stop after revoke, `revoke_session` and `remove_session`, same-ID reuse and foreign registry rejection, post-renewal deadline, preserved real **empty** Job and counted permits. Foreign/reused registration fails closed with existing `Revoked` (no new public MCP error variant).
- The first hosted [CI #40662644378](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40662644378) detected rustfmt and a Windows MCP exhaustive-match failure from a proposed additional public error variant. The change was revised to reuse the existing fail-closed `Revoked` error; no MCP/public error changes were merged. Intermediate superseded runs were cancelled. No first failure reproduced #317/#355 liveness symptoms.
- **No-go:** sequential session and TaskLease receipts are **NOT** an atomic session/lease/policy/host effect fence or CreateProcess permission. There is NO launched native Git root, no descendant process/Job/stdio EOF observation, no watchdog or uncertain-termination cleanup proof, and no MCP GitRead migration. Legacy runner is unchanged. GitHub [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is currently **closed**, but its described pipe-join bounded-wall-time acceptance has NOT been established by these no-child tests; closed/unmerged #352 is not evidence of resolution. [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains **OPEN — Windows V1 NO-GO**, no release, no operator-PC or other project changes.


## Unreleased — 28 October 2026, session-only owner commitment (#400)

- [PR #400](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/400) tested HEAD `a949fbf1745de0f97e03284628967c30b2fa887e`, squash integrated `main=67e3604c198f83c16465863facdf2fa855f983d7`, identical tree `444a3175b7241d85d4e959de456f8fa4f1341ae0`. [PR CI #40658720479](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658720479) **SUCCESS 4/4 first attempt**, [native preparation #40658719448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658719448) SUCCESS, and independent [push/main #40659396121](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40659396121) **SUCCESS 4/4, attempt 1 on the exact integrated SHA**.
- On Windows, the already-registered, **non-executing** native status owner may consume its precreated Job/pipes/pins into a **session-only** original-generation commit receipt issued under the same session mutex as direct and lifecycle revocation. The receipt captures the original stop watch and real private Job ID. Hosted Windows tests prove commitment preceding direct revoke retains original admission/transport/Job until drop and observes stop; direct or lifecycle close winning first prevents commitment. Both tests report `ok`.
- The first test SHA encountered hosted rustfmt and Windows `-D warnings` dead-code diagnostics ([initial CI #40658301013](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40658301013)); corrected with formatting and narrowly scoped, documented allowances on intentionally inert research methods, not global lint suppression. Superseded runs were cancelled; these were not reproductions of #317/#352.
- This is **NOT** a native launch grant or the final GitRead commit. TaskLeaseRegistry and policy epoch revocations remain independent; there is no CreateProcess/ResumeThread, actual Git process/root+descendant/stdio EOF, bounded guardian, termination uncertainty proof or production MCP GitRead switch. #352 was closed **without merge**, not accepted as proof; #355 behavioral acceptance remains unmet. #317/#323/#185/#319 remain OPEN. **Windows V1 NO-GO, no release**.


## Unreleased — 28 October 2026, original session close observation (#398)

- [PR #398](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/398) (exact tested HEAD `7456f3d96dc289eb5e278e3d9acbfce8970a6dcf`) merged into `main=8aa6fd205e7a264e7873e6af6b627ddb81207d9d`. Trees identical `8883d4c0ee6a3647b543799c29f9b573be93a028`. Exact-head [CI #40655468052](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468052) **SUCCESS 4/4**, [native preparation #40655468078](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655468078) **SUCCESS**. Independent [push/main #40656160001](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40656160001) **SUCCESS 4/4 on exact merged SHA, first attempt**. First candidate's [CI #40655374591](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40655374591) failed rustfmt, not #317 behavioral acceptance.
- The Windows-only, internal `SessionRegistry::observe_original_owned_admission_under_close_lock` rechecks an owned admission's original registry, registration-generation signal, revoked bit and captured expiry under the **same mutex** as direct `revoke` and lifecycle `publish_revocation_before_drain`. The precreated, **non-executing** native GitRead custody now invokes it both before and after its independent lease/policy snapshot. Windows tests `original_owner_observation_orders_with_direct_and_lifecycle_close` and `original_owner_observation_rejects_foreign_registry_and_renewed_deadline` passed on exact HEAD. No OS IO or policy work is performed under that session lock.
- **Nonclaims:** the mutex is released after this observation: it is not an owner **commit** or CreateProcess permit, nor a cross-registry TaskLease/policy epoch fence. Session or lease revoke may still race native creation. No real GitRead root, native child/descendant/stdio EOF, out-of-band stop, crash/uncertainty recovery or MCP switch is implemented; real #352/#355 acceptance remains RED. #317/#323/#185/#319 remain OPEN; **Windows V1 NO-GO**, no release or operator-PC access.


## Unreleased — 28 October 2026, independently verified native research checkpoints

- [#394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/394): documentation checkpoint, exact-head [CI #40650458632](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40650458632) and independent main #40651002381, 4/4 SUCCESS each.
- [#395](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/395): Windows-only test that revoking/removing/re-registering a TaskLease under the **same ID** cannot revive the original non-executing native owner. [PR CI #40651259497](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40651259497) and independent main #40651850790 SUCCESS 4/4, native preparation SUCCESS. Original format-only RED #40651188973 preserved.
- [#396](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/396): Windows-only kernel negative test of Job-list `CreateProcessW` missing-image failure, exact `ERROR_FILE_NOT_FOUND`, empty Job, no resumed child. [PR CI #40653261422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653261422) SUCCESS 4/4 and three specialized workflows SUCCESS, independently verified `main=8ee1ad0b2c135af3d639ab753703c3f911dc4941` ([push/main #40653860272](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40653860272), 4/4 SUCCESS, attempt 1).
- Neither test changes production, opens a native GitRead launch or proves commit/revoke, terminal Job/stdio or #352/#355 acceptance. #317/#323 OPEN, **Windows V1 NO-GO**. Complete receipts in [handoff](docs/engineering/GITHUB_HANDOFF.md).

## Unreleased — 28 octobre 2026 : recherche native Windows #393 (test uniquement)

- [PR #393 fusionnée](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/393) : nouveau test Windows de l'association d'un processus système suspendu au Job Object **au moment de `CreateProcessW`** via `PROC_THREAD_ATTRIBUTE_JOB_LIST`. Il valide l'appartenance noyau immédiatement après création puis root terminé et Job vide après arrêt ; aucun lancement GitRead réel, aucune activation MCP ni modification des politiques.
- HEAD testé `14e73802b853c57b67c0b00bf169af6fd5a47793` ; squash `c81c1f5caf937695b3bbd9439f508ff9cb28f6f4`, arbres égaux `12c27901f8ec3f6dd4b8f6abfbfae44aaa16f987`. [CI PR #40649262906](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262906) 4/4 SUCCESS tentative 1 ; [native lifecycle #40649262958](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262958), [compilation Rust #40649262919](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262919), [préparation #40649262961](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649262961) SUCCESS. CI indépendante [#40649847690](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649847690) **completed/SUCCESS 4/4, tentative 1 sur le SHA fusionné exact**.
- Échecs antérieurs distincts : `cargo fmt` [#40648989661](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40648989661) puis import `windows-rs` E0432 [#40649081510](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40649081510), corrigés avant validation ; pas des reproductions comportementales de #317.
- #317/#323 restent ouvertes, #352/#355 exigences d'acceptation non remplies, Windows V1 NO-GO.

## Unreleased — vérification GitRead de la garde des répertoires parents (#391) et passation (27 octobre 2026)

- [PR #391 fusionnée](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/391) ajoute un **test Windows uniquement** : lorsqu’un pin GitRead tient la racine, le renommage de son répertoire parent est déjà refusé ; après `drop`, il redevient possible. Aucun changement de code de production, aucun Git lancé, aucune nouvelle autorité.
- CI exacte de PR [#40526579598](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40526579598) **SUCCESS 4/4 tentative 1**, workflows natifs [#40526579328](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40526579328), [#40526579323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40526579323), [#40526579729](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40526579729) SUCCESS ; squash `c208348f0b63d4810db97f9a5284b325c5b64271`, arbre identique au HEAD testé, [indépendante push/main #40553879389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40553879389) **SUCCESS 4/4 tentative 1**.
- [PR #390 fusionnée](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/390) : observation originale de génération TaskLease (pas un droit de lancement), `main=7f663454039b294d84cd3fb2333cbbd73372f91a`, [CI indépendante #40520286289](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40520286289) SUCCESS 4/4 tentative 1. Les anciens termes « #390 draft » ci-dessous sont des preuves datées et ne doivent pas être lus comme la situation présente.
- Nouvelle passation canonique et nouveau prompt de reprise consolidé. Cette modification documentaire requiert **sa propre CI** et la CI push/main post-fusion. #317/#323 OPEN, tests natifs d’acceptation #352/#355 RED, Windows V1 NO-GO, aucune release.


## Unreleased — original-generation native GitRead snapshot (#390 draft)

- Add a non-executing post-custody observation of current registered TaskLease GitRead policy and pinned executable identity; compare original revocation record identity rather than an opaque lease ID that could be reused. Explicitly reject foreign session-registry and foreign same-ID lease while retaining original permits.
- Preserve test-first [CI #40517792327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40517792327): expected E0433/E0599 absent API and separate rustfmt failure. Do not claim it reproduces #352/#355 actual process behavior. Subsequent code and documentary HEADs require their own green CI.
- Snapshot is not a CreateProcess gate. No child/MCP change or native terminal proof; Windows V1 NO-GO.


## Unreleased — GitRead original-target handle custody (#388, 27 October 2026)

- Extend non-spawning Windows `PrecreatedNativeGitStatusCustody` with an opened/verified executable handle and READ/LIST-access repository-root pin held alongside Job/pipes and original counted admissions. The GitRead-specific root pin denies root rename; existing zero-access general directory-chain semantics remain untouched. Observed handle paths must match the trusted canonical service command.
- Preserve [test-first Windows runtime RED #40510524394](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40510524394) (371 pass / 2 fail) and [first pin Windows runtime RED #40511257834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40511257834) (372 pass / 1 fail: root rename). The earlier test-first Ubuntu rustfmt failure is classified separately.
- [Pin correction CI #40512075888](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40512075888) success, attempt 1, and native preparation/lifecycle/Rust IR workflows successful on code SHA `382857c0269b7173c24bef8eb948d08ad0e558ae`. Exact PR head also passed [CI #40513121389](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40513121389) 4/4 first attempt and all 3 native-specialized workflows; the merged `main=720a38833705e661049841cfd39852e770687643` passed independent [push/main #40514100157](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40514100157) 4/4 first attempt. Any later documentation merge still needs its own independent CI.
- No CreateProcess, MCP route switch, real process termination evidence or closure of #317/#323. Windows V1 NO-GO.


## Unreleased — precreate unstarted native Git status Job and pipes (#387, 27 October 2026)

- Under existing, already-counted Git status registration, precreate a real
  Windows Job Object with the original resource limits and kill-on-close,
  owned stdout/stderr pipes and `NUL` stdin, retaining original MCP session,
  transport, TaskLease generation and process capacity. The new type has no
  launch/resume API and checks for stop again after resource creation.
- Windows tests observe the **actual empty kernel Job**, idle pipes without
  false EOF, permit retention through revocation and rejection after revoke.
  No child/Git is started; no terminal native proof is claimed.
- Test-first [CI #40506640216](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40506640216)
  reports E0433/E0599 missing-interface compiler errors and format failure;
  not original #352/#355 physical runtime behavior. The first implementation
  also had format-only red [#40506898619](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40506898619).
- [CI #40507126521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40507126521)
  failed one new native Windows unit test because it asked `File::metadata()`
  of the `NUL` device; this is a test observation issue, not physical
  #352/#355 reproduction. Only that assertion was removed in
  `88dc28a8c9c13dbdff74359f02372d6665e3603e` and the original
  Windows log remains preserved.
  Exact-head independent Windows/Ubuntu/dependency/release validations still
  govern the final PR and `push/main`. Windows V1 NO-GO.



## Unreleased — integrated native Git status process reservation (#385, 27 October 2026)

- Add Windows-only `ProcessManager::reserve_prepared_native_status` for an
  already authorized `PreparedNativeGitStatus`, using the original lease
  workload class and the existing per-session/global process, CPU, memory and
  output accounting. Foreign manager root or expired preparation fails closed;
  original MCP session/transport permits and revocation witness remain owned.
- Add negative and no-child accounting tests. This does **not** create a real
  Win32 Job, root, pipes or native process, nor activate native MCP GitRead or
  solve #352/#355. Windows V1 remains NO-GO; #317/#323 stay open.
- Preserve the initial test-first compiler red
  [#40501489580](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501489580)
  (E0433/E0599) separately from formatting errors; candidate
  `9a66557d29cfe66da7f06a68ca80e68be0576a02` passes
  [CI #40501987327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501987327)
  4/4 and [native preparation #40501987334](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40501987334)
  on that exact SHA. No claim for any later commit or integrated-main run.


## 27 octobre 2026 — passation pour Chat classique

- Ajoute la passation technique et le prompt complet de reprise après #380–#383,
  avec SHA/CI, premiers rouges qualifiés, limites et prochaine priorité #317.
- Actualise les points d'entrée vers la reprise en Chat classique via GitHub,
  sans tâche Codex/Work ou accès PC implicite. Aucun changement de runtime.
  [Passation et preuves](docs/engineering/HANDOFF_WINDOWS_V1_CHAT_2026-10-27.md).
  #317/#323 restent ouvertes ; Windows V1 NO-GO.


## 27 October 2026 — uncertain native reservation streams (#383)

- Preserve stream observations under termination uncertainty; do not publish
  unobserved EOF from native private-worker reservation completion.
- Retain uncertain capacity/reaping behavior. [Test-first evidence and limits](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
  No native Git activation; #317/#323 OPEN, Windows V1 NO-GO.


## 27 October 2026 — native status service boundary (#382)

- Prepare the service's fixed status command with original worker-owned permits,
  registered canonical TaskLease, immutable ActionEnvelope and original horizons.
- Deny service identity/output mismatches; retain session/transport reservations.
  [Contract and evidence](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
  No live native routing or launch. #317/#323 OPEN; Windows V1 NO-GO.


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


## Unreleased — passation inter-chat complète (26 octobre 2026)

- Ajouter les références actives [Passation canonique Windows V1 du 26 octobre](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-26.md) et [prompt complet à copier](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-26.md) à AGENTS/AI_HANDOFF/PROJECT_CONTINUITY/GITHUB_HANDOFF/STATUS/ROADMAP. **Docs seulement** : aucun Rust, aucune politique, aucun workflow ni release.
- Avant cette PR documentaire : `main=c058ccf3f9db18c7d67194d117af99cc3e0cea52`, [CI indépendante push/main #40358111878](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40358111878) **SUCCESS 4/4 au premier essai**. Cette PR fera avancer `main` : vérifier le nouveau SHA et la CI `push/main` après fusion. #369/#371/#373/#375/#377 sont des contrats GitRead **inertes** ; production MCP GitRead toujours legacy ; #327/#353/#358 recherches isolées, originaux #352/#355 **ROUGES** ; issues #317/#323/#185/#319 ouvertes ; Windows V1 NO-GO, aucun accès PC/Optic Desktop/REL.

## Unreleased — transfer already-counted native GitRead permits (2026-10-26)

- [PR #377](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/377) adds `NativeGitReadOwnedPermits` and `finish_owned_native_git_read_preflight` to accept existing worker-owned `OwnedSessionAdmissionPermit` + `RequestPermit` instead of a second admission. Enforces **exact Arc session-registry provenance** and trusted handle before any typed TaskLease policy/stop check, preserving RAII on failure and original generation stop/deadlines. The existing begin helper delegates. No native Git start.
- Test-first [format #40354691682](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40354691682), formatted [compile RED #40354870468](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40354870468) and [Clippy RED #40355329042](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40355329042) preserved. Corrected `5692a271a33affa9db73e4d3fcf747419d07cc61`: [CI #40355535498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40355535498) 4/4 SUCCESS first attempt and [native prep #40355535687](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40355535687) SUCCESS. Integrated `main=a18faaea8bcf8b97f019873b7b6a8d48b6587d57`; independent [push/main CI #40356405717](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40356405717) completed **SUCCESS 4/4 at attempt 1** on the exact integrated SHA. #352/#355 remain RED; #317/#323 OPEN, Windows V1 NO-GO.

## Unreleased — native GitRead preflight retains MCP transport capacity (2026-10-26)

- [PR #375](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/375) extends inert `NativeGitReadAdmittedPreflight` to own a genuine noncloneable `RequestPermit` alongside counted session admission and typed TaskLease witness; the transport slot stays reserved, the original request deadline adds a stop condition, and expired permits fail closed with RAII capacity release. Synthetic child-free tests; no live MCP change or real native process.
- Test-only compile-time RED [#40351046826](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40351046826) on `a244941fda821bc2053269fac715b6d6827857e1` proves missing permit parameter/error variant; initial format RED [#40350852162](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40350852162) kept. Final SHA `62431d8d34ec56df6d01d93f8c1d9adbbfde8933`: [CI #40351438430](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40351438430) SUCCESS 4/4 first attempt, [native preparation #40351438537](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40351438537) SUCCESS. Main squash `99c639a0169de72c6bd692ac8493cadef6a1e638`; independent [push/main #40352227922](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40352227922) completed **SUCCESS 4/4 at attempt 1** on exact main SHA. Original #352/#355 RED, #317/#323 OPEN and Windows V1 NO-GO.

## Unreleased — TaskLease cleanup publishes generation-bound stop (2026-10-26)

- [PR #373](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/373): when `TaskLeaseRegistry::remove_session` physically drops session-owned leases, it first publishes each original-generation revocation signal under the registry mutex. Retained watchers become stopped even without an earlier explicit revoke, foreign session watches remain valid, and an opaque ID's replacement gets a new signal. **Only one Rust file** changed. This is no termination receipt, native launcher or new permission.
- Test-first regression [CI #40340672365](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40340672365) reproduced exactly 284 pass/1 failed, previously passed format/Clippy; original [formatting CI #40340364259](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40340364259) preserved. Corrected exact head `74fa640b1bbb4576a39d392cbd9ee724a301d23e`, [CI #40341168843](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40341168843) SUCCESS 4/4 first attempt, [native preparation #40341168733](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40341168733) SUCCESS. Integrated `main=621bf4ed977e99a1dd8825f8a2cb28a0e04aab91`; independent [push/main #40342061604](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40342061604) completed **SUCCESS 4/4 at attempt 1** on exact integrated SHA. Original #352/#355 behavioral gates remain RED; Windows V1 NO-GO, #317/#323 OPEN.

## Unreleased — counted native GitRead admission preflight (2026-10-25)

- [PR #371](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/371) adds a non-cloneable, inert trusted GitRead preflight combining a **real counted session admission** and the verified TaskLease/policy/revocation witness from #369, observing both captured monotonic expirations. Denies mismatched trusted session, missing registry session/TaskLease, expiry and revoked generations; deterministic tests prove admission capacity remains held until released and a reused lease ID does not redirect the old stop signal. No native job, child launch, MCP permission change or process cleanup proof.
- Candidate `04350c2e1190335398c7e8df5a7286e1a09b083e`: [CI #40196130286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40196130286) 4/4 SUCCESS attempt 1, [private prep #40196130318](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40196130318) SUCCESS; preserve first failing format/Clippy runs [#40195715393](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40195715393), [#40195817977](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40195817977), [#40195985392](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40195985392). Squash main `2148281b2960e777a759d39682b4a831e61d47b6`; independent [push/main CI #40196515745](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40196515745) completed **SUCCESS 4/4 at attempt 1** on the exact integrated SHA. Original #352/#355 RED, #185 separate, #317/#323 OPEN and Windows V1 NO-GO.

## Unreleased — registered native GitRead policy witness prerequisite (2026-10-25)

- [PR #369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/369) integrated an *inert* runtime helper joining a verified active TaskLease from the actual registry, exact typed GitRead policy and original-generation revocation watch. It denies unknown/legacy native leases, wrong repo/session/epoch/budget and keeps the old watcher revoked after record replacement. It does **not** launch Git, change live MCP GitRead permissions, activate the experimental native Job owner or satisfy real #352/#355 acceptance. [Contract](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md).
- Exact-head `49d375972583cad769403b0f7f8d5d4aa96e9525` [CI #40194006038](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40194006038) Windows/Ubuntu/dependency/release 4/4 SUCCESS first attempt and [preparation #40194006046](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40194006046) SUCCESS. Original formatting [CI #40193634170](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40193634170) and Windows pinned-file cleanup [CI #40193787323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40193787323) failures preserved. Squash main `60da40a1c8ad5109eb5abd30c5daae045a404f9a`, independent [push/main #40194463604](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40194463604) completed **SUCCESS 4/4 at attempt 1** on the exact integrated SHA. #317/#323 OPEN, Windows V1 NO-GO.

## Unreleased — verified TaskLease + original revocation witness (2026-10-25)

- [PR #364](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/364) captures a registry-verified TaskLease and read-only generation-bound revoke witness under **one lock**, including executable identity verification; existing `get_active` behavior stays compatible. Tests cover same-ID replacement, wrong session/expiry/revoke and non-Windows identity drift. Test-first Windows/Ubuntu compiler red [#40184509735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40184509735) was preserved; corrected exact-head [CI #40184694171](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40184694171) succeeded 4/4, [native preparation #40184694195](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40184694195) succeeded and merged `main=88593315d84571585df7e5d4639f34fc5491f1b9` passed independent [CI #40185041267](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40185041267) 4/4, all first attempt. **No native GitRead launch, permission increase, runtime revocation-spawn acceptance, real pipe lifetime or Windows V1 release.** #317/#323 OPEN, V1 NO-GO; preserve independent #185 Windows AppContainer intermittent red.

## Unreleased — fail-closed native GitRead policy preflight (2026-10-25)

- [PR #362](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/362) introduces a **policy-only** preflight for later GitRead native ownership, checking the real GitRead action, exact TaskLease identity/session/epoch, canonical repository scope, FixedTool executable and full bounded session/lease duration. Test-first missing-method E0599 reproduced on Windows/Ubuntu; corrected-head [CI #40182783889](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40182783889) passed **4/4** at `39abd8276cd6cd1254163a2d1b56ea6a87148cae`, first attempt. Squash main commit `739bdd5de3a86d72eaa251cb4f9f311e69b33e0e`. Existing MCP GitRead execution, policy and Win32 process ownership **unchanged**.
- Independent [push/main CI #40183165673](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40183165673) failed Windows from the separately tracked **#185 Node/AppContainer no-output timeout**; other three jobs green. Original failure retained despite three succeeding diagnostic-only comparisons. This red does not establish a GitRead regression; no suppression or automatic retry. #352/#355 still behavioral RED; #317/#323 OPEN; Windows V1 NO-GO.

## Unreleased — exact native GitRead repository lease scope (2026-10-25)

- [PR #360](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/360) adds `LeaseScope::GitReadRepository { canonical_root }` as a *typed, inert* exact repository marker and proves it does not alias generic `Repository`. Test-first Windows/Ubuntu compile failures were preserved; corrected-head [CI #40181004649](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40181004649) succeeded 4/4, and independent merged-main [CI #40181293600](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40181293600) succeeded 4/4, both first attempt. Integrated `main=0b2fcc6ee5329d3112500e79fc2a724eb9736c31`. **No production GitRead native launcher, policy elevation, bounded stdio completion or release readiness**; #317/#323 remain open, Windows V1 NO-GO.

## Unreleased — counted, owned session admission (2026-10-23)

- Add a non-cloneable `OwnedSessionAdmissionPermit` retained via `Arc<SessionRegistry>` for future service-owned workers, sharing the identical counted admission and original-generation stop/deadline semantics with the existing borrowed permit. Concurrency tests cover independent OS-thread lifetime, synchronous revoke stop-before-drain, mixed permits, expiry/renewal and deliberate handle reuse. **No native runner/Job owner migration, process termination proof, task-lease change, security policy or V1 release.** [Scope and proof obligations](docs/engineering/ISSUE317_OWNED_SESSION_ADMISSION_2026-10-23.md).

## Unreleased — immutable admission stop condition (2026-10-23)

- Add the read-only `SessionAdmissionStopWatch` captured from genuine session admission: original-generation explicit revoke **or** original absolute monotonic expiry requires stop, including across session renewal and deliberate opaque-handle reuse. A genuine worker-held permit test verifies that real revoke publishes the stop before its synchronous admission drain completes. The witness grants no execution right and proves no native process termination. No Git runner/JobStore, lifecycle revoke, lease, policy, resource limit, timeout, V1 readiness or release behavior changes. [Detailed scope](docs/engineering/ISSUE317_ADMISSION_STOP_WATCH_2026-10-23.md).

## Unreleased — generation-bound admitted revocation watch (#317, 2026-10-23)

- Capture a read-only, session-record-bound `SessionRevocationWatch` with each genuine admission permit, enabling downstream owners to observe explicit revocation without trusting reusable opaque session handles. Verify direct close, independent sessions, historic handle reuse and deadline independence. This **does not** change synchronous revoke, launch authorization, native process lifecycle, Pending shutdown or V1 #323 status. See [contract](docs/engineering/ISSUE317_ADMISSION_REVOCATION_WATCH_2026-10-23.md).


## Unreleased — Tokio dependency safety patch (2026-10-23)

- Resolve [RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386) by updating the single locked `tokio` package from 1.53.2 to the advisory-fixed 1.53.3 with Cargo's exact checksum and dependency graph. No dependency policy is ignored or weakened and no public API or runtime code is changed; Windows/Ubuntu CI and `cargo deny` must pass on the exact updated lockfile.


## Unreleased — owner-scoped uncertain Job cancellation (#317, 2026-10-23)

- Ask retained `TerminationUncertain` Job owners to stop when `cancel_session` has not already signalled them; keep quotas/ownership until independent native termination proof. Add scoped, idempotent synthetic regression. This does **not** fix synchronous revoke-before-cancel liveness, durable shutdown, GitRead authorization or the V1 NO-GO gate. See [scope/limits](docs/engineering/ISSUE317_UNCERTAIN_JOB_CANCEL_2026-10-23.md).


## Unreleased — internal revoke phase extraction (2026-10-23)

- Prepare a private extraction of session close/signal publication and synchronous
  admission drainage, preserving the public revoke result, lock handoff and error
  behavior. Added compatibility regressions; no runner/permission/contract change
  and no #317 or V1 gate closure. See [compatibility report](docs/engineering/ISSUE317_REVOKE_INTERNAL_COMPATIBILITY_2026-10-23.md).


## Unreleased — 23 October 2026 Windows V1 successor documentation

- Add [updated canonical engineering handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-23.md) and [full successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-23.md). Refresh entry pointers while preserving dated historic snapshots.
- Record experimental [`1c7dac9`](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/commit/1c7dac973c8f41f27b303811041b54031f7272ae) evidence ([CI #39888522519](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522519), [native #39888522200](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39888522200)) and important **unresolved** native gaps: submit-only admission, potentially blocking root Drop, suspended child surviving synthetic host exit **before Job assignment**. Production `main` unchanged at audit, no GitRead migration, ADR-0035 proposed, PR #327 old red #185 retained. Research #182 patch exposure remains uncorrected.
- **Documentation only:** no Rust/runtime/permissions/CI/timeout/installer/process behavior, V1 tag, release or GitHub protection change.


## Unreleased — documentation of Windows V1 continuation (2026-10-22)

- Updated the canonical handoff/roadmap/status pointers for the forthcoming chat and created a comprehensive [Windows V1 engineering takeover](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-22.md) with a [standalone successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-22.md). This is **documentation only**; no runtime behavior, feature, permission, packaging, timeout, protection, tag or release change.
- At the handoff check: production main `82ad1234` was separate from test-only research `33c95e00` (Windows/Ubuntu/dependency [CI #39748924515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39748924515) green). #317 remains open: its native fixtures and inert owner registry are not the production fix. V1 freeze [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) remains NO-GO.


## Unreleased — Windows stabilization

- [PR #320](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/320) merged at `751bc72a3090b934ee8e20c860bc7293ea357c29` (prerelease exact integrated-main CI source gate, 26 deterministic fixtures). [PR #321](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/321) merged at `65aa5db0dbaad24894f2cf4b050d5ac0bd61eac9` (Windows PowerShell 5.1 BOM-free MCP stdin, Unicode, failed-start and real Bridge/Node tests). Exact-main [CI #39722724572](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724572) and [native pwsh #39722724496](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724496)/[Desktop5.1 #39722724477](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724477) passed attempt 1. Separate #317 Rust native owner research is not a production fix; #319/#185 and #176/#182 remain open. No V1 release.
- Fix demonstrated Windows PowerShell5.1 MCP stdin BOM framing: explicit no-BOM UTF-8, scoped Framework console-encoding restoration and safe failed-start cleanup. Add actual-byte/Unicode/start-failure, real Bridge and owned-pipe regressions.
- Require an exact-source successful main CI and main ancestry before tag builds/publication; fail closed on incomplete evidence. Add offline and read-only API regressions. No release or V1 readiness is implied.

## Windows doctor failure-path lifetime and Windows V1 priority (2026-10-22)

- Native supervised fixtures reproduce unbounded stderr waits after stdout EOF
  and direct-child timeout with a retaining descendant. The Windows doctor uses
  at most32768 available pipe bytes/400ms polling,4096-char preview and24 approved
  phase messages without abandoned stderr read tasks. Preserve primary errors
  even if helper initialization fails; response waits use a monotonic clock.
- Keep the30-second MCP startup gate and full executable identity/pinning.
  Natural319 startup cause and distinct185/317 issues remain open.
- Windows V1 is the delivery priority; retain Ubuntu regression CI and defer
  functional Linux/confinement work. Hosted native proof is not Windows10/11
  or PowerShell5.1 acceptance.

## Fail-closed raw Git diff file-mode validation (2026-10-21)

- The raw `-z` diff header parser rejects non-octal file-mode digits (`8`, `9`) instead of treating impossible Git modes as verified provenance. Synthetic regressions verify both old/new invalid modes and legitimate symlink/gitlink octal formats. No Git arguments, API schema, permission, timeout, process or sandbox changes. Native Windows and Linux CI on the exact head remain the acceptance proof.


## Git log malformed record fail-closed correction (2026-10-21)

- The bounded Git log parser now rejects empty record rows rather than silently skipping them. A blank intermediate record would otherwise make parsed entry counts disagree with Git's offset-based cursor progression. Synthetic regression covers valid consecutive commits, a legitimately empty result, and blank leading/intermediate/trailing rows. No change to Git arguments, permissions, cursor format, limits, process or sandbox behavior. Exact-head native CI remains the acceptance gate.


## Verified FileRead hard-link identity safeguard — PR #314 (2026-10-21)

- [#314](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314) squash-merged at `7bc17c16a52383bf6fb7ba68dc4b9348b259be9a`: prepared `fs_read` now conservatively requires an explicit sensitive lease for an opened file identity having multiple hard-link names, even through an ordinary-looking alias. Synthetic deny/grant/ordinary-file native regressions cover Windows and Linux. Harmless multi-linked files may be denied absent extra authority.
- Accepted source SHA `fbf7c9f40bbae18d4017f237b8783a19964620c8` passed [CI #39576724127](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39576724127) and three specialized Windows workflows; independent [main CI #39577461324](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39577461324) passed first attempt at the squash SHA. Initial superseded source-head rustfmt failure is retained. [Evidence](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314#issuecomment-6312779381).
- [Read-only GitRead review](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182#issuecomment-6312922336) records branch/ref labels and historical/staged Git blobs as unresolved contract/authority boundaries. #176, #182 and Windows timeout #185 remain open; no runtime change beyond FileRead alias check, feature, permission, sandbox or operator-PC change.

## Verified security read-boundary increment — PR #312 (2026-10-21)

- Squash-merged [#312](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/312) at `30295539fc6911f221cee6ac3c59785398c7a050`: deterministic sensitivity guard for direnv `.envrc` and `.envrc.*` names in the shared classifier. Synthetic Rust core, MCP FileRead, Git status and Git diff tests verify denial without explicit sensitive authority and permitted reads with the matching lease. ADR-0034 updated.
- Exact accepted head `4e61a8f0aa0b65872d76ea9a2584b4ab79b89f33` passed [PR CI #39571281997](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39571281997); independent [main push CI #39571875338](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39571875338) passed first attempt on the squash SHA, hosted Windows/Linux/dependency checks. First superseded [CI #39570589090](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39570589090) failed formatting and remains preserved.
- Open boundaries unchanged: secrets inside ordinary names, Git provenance/races (#176/#182), the Windows timeout (#185), independent first MCP response timeout, recovery/durability and soak limits. No default permission, timeout, sandbox, transport, feature or operator-PC change.

## Verified security integration through #310 (2026-10-21)

- **Merged:** [#308](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/308) shared deterministic sensitive-name classifier for FileRead/fs_list, [#309](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/309) checked Git status protected names, and [#310](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/310) checked bounded `--raw -z --patch` from the same hardened Git invocation with strict extra-NUL framing and explicit additional FileRead content/sensitive authorization on classified paths.
- **Verified:** #310 squash commit `bd2915c3a9833c1da5a8fc5005445d1884260d4f`, [first-attempt green post-merge Windows/Linux/dependency CI #39552523257](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39552523257); exact accepted source `154d7ea6ab6b10aa967ab0a2f2034f2f06400fe9` passed #39551993433 and #39551993151. Earlier formatting and failed Git-protocol tests remain retained as historical evidence.
- **Open:** #176 ordinary-named secret content / native filesystem identities, #182 residual Git-read coverage, provenance and races, and existing Windows/recovery/soak risks. #176 was accidentally closed by a GitHub keyword on #310 merge and reopened; do not infer a completed security gate from issue auto-closure. Older candidate entries below are historical only. No remote real-workspace guarantee, new feature, self-hosted run or PC execution.

## Git diff sensitive-content guard candidate (2026-10-21)

- Operator-approved Policy B, next #182 increment: request raw NUL-delimited Git changed-file metadata and ordinary patch in one hardened Git diff call; verify each changed path, refuse unparseable outputs, and return only the bounded patch.
- For a classified sensitive path in a patch, require explicit active FileRead with ReadContent and ReadSensitive scopes while the session admission guard is held; no GitRead privilege expansion. Native Windows/Linux Git and MCP regressions for whole/path/staged/ordinary and malformed outputs.
- Candidate until exact source CI, reviewed squash and independent main CI. Ordinary filenames may contain arbitrary secrets; #176/#182 remain open. No features, release, self-hosted runner, workstation execution, network/timeout or security policy loosening.

## Git status sensitive metadata gate — PR candidate (2026-10-21)

- Validate the complete Git porcelain-v2 `-z` status record stream before
  exposing it through MCP; unknown, malformed or unrepresentable native
  paths fail closed.
- Enforce the existing shared sensitive-name classifier and explicit
  sensitive read lease for tracked/staged/untracked/renamed status entries.
  No new default authority is granted; ordinary status remains usable.
- Add Windows/Linux-targeted unit and MCP regressions; update ADR-0034
  and stabilization handoff. This remains a **candidate** pending exact
  SHA CI, squash and independent main CI.
- **#182 still open:** Git diff bytes are not yet classified; #176 also
  remains open. Do not enable real project remote exposure.

## Security stabilization candidate: first sensitive read/list boundary — 2026-10-21

- Operator approved Policy B: a shared deterministic sensitive classifier across
  filesystem and Git output, fail-closed for unverifiable results (ADR-0034).
- Candidate `security/sensitive-file-authority-scope`: classify common sensitive
  path components in Rust core; require both ordinary and sensitive scopes for
  FileRead; deny fs_list pages with protected names unless a separate active
  sensitive scope exists. Add Windows/Linux-targeted core, policy and MCP regressions.
- **Not yet integrated:** verify exact PR-head CI, review code, squash and then
  verify independent main CI. GitRead status/diff **remain unfiltered**, arbitrary
  ordinary files may contain secrets, and #176/#182 still block remote real
  project exposure. No features, broader grants or workstation activity.

## Verified stabilization integrations through #305 — 2026-10-21

- [#303](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/303) merged on `main` at `9c6c91096590f9f005dabc51253b9b5f8b1abcca`: authorize `fs_list` on canonical target, with alias-scope denial regression and [first-attempt green post-merge CI #39534614408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39534614408).
- [#304](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/304) merged at `9d90485fce2a044a71fb7d6986a591f36cd4a60f`: reserve exact case-insensitive `.git` control components from generic file/directory mutation; Windows/Linux regressions and [first-attempt green post-merge CI #39536541153](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39536541153).
- [#305](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/305) merged at `43d3a4aeedee04dd03e895e27e81d3bc5af6652c`: refit `git_log` output after adding continuation cursor, or refuse oversized response; [first-attempt green post-merge CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) on exact merge SHA. The earlier source-head formatting/lint failures remain visible and were corrected in that **same** PR before final green acceptance.
- #301 and #302 were also previously merged (exact integration evidence in [STATUS](STATUS.md)). Sections below calling #303–#305 “candidate” are **historical pre-merge observations**, not the current branch state.
- Remaining: Windows 10s Node AppContainer and independent ~30s MCP initial-response intermittencies, recovery/ancestor/power-loss limits, sensitive file/GitRead exposure decisions. Green CI is not a full security or stability certification. No runtime change in this documentation reconciliation.


## Git log cursor-aware response fitting

- Correct bounded `git_log` response fitting: if trimming an end-of-history page requires adding a continuation cursor, the response is now checked again with that cursor included. Return a bounded error rather than emit oversized JSON when even one entry plus cursor cannot fit; retain forward-progressing offsets without overflow. Add a deterministic byte-budget regression. Existing GitRead isolation, authority, cursor-head pinning and public tool surface remain unchanged. CI and merge status must be verified live.


## Git metadata authority segregation — bounded stabilization candidate (2026-10-21)

- Prevent generic workspace file mutation from addressing canonical `.git` control metadata (including linked worktree pointer files and ASCII case variants) without affecting ordinary dotfiles; keep dedicated Git integration authority separate. Deny generic creation of Git-control directories and provide a stable MCP error. Add native Windows/Unix regressions for requested and canonical paths. This entry describes a **PR candidate**, not a verified merge; require exact-head CI and independent main CI before claiming final delivery. Open risks: concurrent namespace races, hardlinks, sensitive GitRead/content policy and remote access.


## Canonical fs_list metadata authorization hardening — 2026-10-21

- A prefix-scoped search previously authorized the requested lexical directory but enumerated its resolved target. The focused stabilization change canonicalizes the directory before `Effect::FileSearch` policy evaluation and lists canonical returned paths. A Unix symlink-prefix negative test and portable normal-directory test cover the behavior. No default authority widening or permission change. Integration must be verified against GitHub live PR/CI and `main` SHA; sensitive-content classification and concurrent directory-race hardening remain open in [security issue #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176).


## Recovery unpublished-evidence preflight — 2026-10-21

- [PR #301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) squash-merged as `05b5a928c862601926e8b0cc11540ce020caf271`: preflight every unpublished temporary recovery record before removing the first, then revalidate native identity at removal. Cross-platform negative hard-link regression and post-repair success were verified. No mutation policy, scope, timeout, grant or journal format changed.
- Accepted source `089e64a2b6c95dc0705eeaaa22f90194dfb7be66` passed [CI #39529419994](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529419994) on attempt **2**, after a native Windows Node/AppContainer 10s first-attempt timeout (retained). [Native preparation #39529420240](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529420240) passed first attempt. Independent [main CI #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) **passed on its first attempt** on exact squash SHA, Windows Rust, Linux Rust and dependency policy. Multi-file cleanup is still non-atomic on later actual deletion failure and the bridge remains pre-alpha.



## Windows launcher timing diagnostics — 2026-10-21

- [PR #299](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/299) squash-merged as `6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea`: opt-in fixed-label monotonic `stderr` phases for the native Windows isolation launcher, exact-`1` flag forwarding and a native test requiring selected phases without relaxing Node loopback denial. [CI #39526324369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324369) on exact source `aca382535f24b6f6b1c26c2313302d315099ed4e` and [preparation #39526324493](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324493) passed on first attempts. Independent main [CI #39526677098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526677098) passed the exact merged SHA on its first attempt (Windows, Linux, dependencies).
- One successful native loopback test observed `target_created_suspended=14ms`, `target_resumed=14ms` and `target_exited=361ms` relative to the *outer helper entry*. No conclusion about the earlier missing-child-output ~10s failure or separate MCP 30s response timeout. No permissions, network grants, process budgets or UI features changed. Core remains pre-alpha.


## Verified integration and handoff closure — 2026-10-21

- [#293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/293) merged `de973989`: recovery validates stable journals and staged content before discarding unrelated unpublished evidence.
- [#294](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/294) merged `da3ab6f1`: bounded opt-in Windows MCP child timing collection. **Independent post-merge CI passed on attempt 2**, retaining the first Windows Node/AppContainer 10s failure.
- [#295](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/295) merged `615e6219`: further fixed Git/session/read/process-lease timing boundaries.
- [#296](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/296) merged `72966f92`: measured canonicalization/pinned-open/reading+hashing durations. One Windows source run measured `hash_ms=4869`; this includes file reads, not BLAKE3 algorithm CPU alone.
- [#297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) merged `83329d48789c20b1fc235d3096c3199dc9dc5176`: duplicate and global/per-session capacity rejection now happens before pinned executable capture, with the same checks repeated under the final insertion lock. Negative nonexistent-file and concurrent worker regressions passed on native Windows and Linux. [Exact-source CI #39514352067](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514352067), [native preparation #39514352140](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514352140), and independent [main CI #39514832814](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514832814) all passed on their accepted exact SHAs, first attempt. Original rustfmt-only draft failure retained.
- Live pre-documentation checkpoint: `main=83329d48789c20b1fc235d3096c3199dc9dc5176`, 0 open PRs, final main CI green first attempt. Feature freeze remains active, with Node/AppContainer 10s, MCP first response 30s, recovery atomics/ancestor/power-loss and Cargo/Electron compatibility unresolved. Historical 'candidate' entries below predate their later merges; do not treat them as open PRs.

## Historical changelog entries (retain dated evidence; candidate status may have since merged)

## Verified stabilization — Windows diagnostics and recovered staging evidence (2026-10-20)

- [PR #290](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/290) merged as `0c2a125537d020e3a3aff4124d12745d7a8c5c4c`: the existing native Windows Node/AppContainer loopback test records bounded monotonic host phases and an early child marker. [Source CI #39392262806](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392262806) and independent [main CI #39392829136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392829136) passed. Historical Node and MCP startup timeouts remain unresolved; this is instrumentation, not a fix.
- [PR #291](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/291) merged as `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`: on Windows, recovery validates **all** staged artifacts before deleting any, then reopens and revalidates exact native delete handles for cleanup. Native two-journal regression protects earlier staging evidence when a later staging mismatch blocks recovery and confirms normal cleanup after repair. [Source CI #39393849343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849343), [native preparation #39393849287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849287) and independent [post-merge CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) passed (hosted Windows/Linux/dependencies, main first attempt). This does **not** make a series of deletions atomic, pin directory ancestors, or establish power-loss durability.
- Stabilization-only policy continues: existing reliability and security corrections, no new features. Older changelog entries containing 'candidate' describe their historical state and are not current assignments. No operator-PC tests or self-hosted CI, releases, permission expansions or bulk deletion.

## Stabilization-only release discipline — 2026-10-20

- Operator directs a temporary pause on **new functionality**. Continue only
  proved bug fixes, security/recovery hardening, reliability investigations
  and regression coverage for already-integrated behavior; no broad refactor,
  new GUI/integration or speculative PR. Feature roadmap remains deferred.
- [PR #288](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/288) is
  **merged** as `b1edc83438571867bd164ea0b2b9653304eed752`, not
  merely a candidate. Final PR CI #39388280621 and native preparation
  #39388280521 succeeded; post-merge main CI #39388749406 passed
  Windows, Linux and dependency policy on its first attempt.
- The scoped fix retains unpublished temp evidence if later stable recovery
  validation fails. Multi-file cleanup atomicity, concurrent ancestor
  replacement, Windows power-loss directory durability and the known Windows
  Node/AppContainer/MCP intermittencies remain open investigations.

## Unpublished recovery evidence preservation — 2026-10-20

- Proposed targeted fix `fix/recovery-inspect-preserve-temp-on-failure`:
  delay removal of valid unpublished `*.prepared.tmp` files until all
  stable recovery records have been classified without error.
- Cross-platform negative test preserves both earlier temporary and later
  corrupt stable bytes on failed inspection, then confirms normal cleanup
  after removing the corruption. This does not change journal formats or
  restore authorization, nor provide atomic multi-file deletion.
- PR #287 merged as `0ca2305`; its post-merge CI 39387133506 passed
  Windows, Linux and dependency policy on first attempt. Verify the new
  candidate PR's exact-head checks and merge before claiming integration.

## Linux prepared-journal directory synchronization — 2026-10-20

- Candidate `fix/recovery-linux-directory-sync`: after no-replace publication,
  sync the already-open Linux GNU recovery directory before reporting success.
- Injected failure test proves a failed directory sync returns an error while
  the newly published `prepared` journal stays available for reconciliation.
- Does not alter Windows behavior, filesystem grants or journal schema, nor
  claim complete power-loss durability. Check exact-head CI and PR merge.
- Previous #286 was merged at `67c65a5`; its post-merge main CI #39385303625
  succeeded on its first attempt.

## Atomic no-clobber prepared journal publication — 2026-10-20

- Candidate `fix/recovery-journal-publish-no-replace`: replace ordinary
  potentially overwriting rename with native no-replace publication on
  Windows and Linux GNU. Use the already-locked `nix` Unix wrapper.
- Existing stable evidence must survive a publication attempt. Concurrency
  regression allows exactly one publisher to win; no intermediate two-link
  journal state is introduced.
- Other platforms fail closed until dedicated support is validated.
  This does not prove namespace ancestor pinning or power-loss durability.
  Verify exact-head PR checks and squash before claiming integration.


## Recovery state-root creation side-effect review — 2026-10-20

- Pending narrow branch `fix/recovery-state-root-preflight`: reject absent
  recovery state roots under the workspace **before** creating their ancestors.
  Keep the existing post-create canonical check.
- Negative test preserves an otherwise empty workspace when the chosen
  nested state-root is forbidden; outside-root missing-directory creation
  remains supported. No journal format, policy grant or filesystem effect API
  changes. Concurrent directory replacement and full durability remain out
  of scope. Verify exact-head GitHub CI and PR merge before claiming delivery.


## Recovery journal append alias hardening — 2026-10-20

- [PR #282](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/282)
  squash-merged as `bf79b320` after final head `c837cead` passed Windows
  and Linux Rust, dependency policy and native preparation (CI 39375905332,
  39375905331). Its first head's Windows Clippy failure was corrected, not
  suppressed.
- Reject hard-linked external journal evidence before append. Windows reuses
  the exclusive no-reparse state handle; Unix checks actual opened file
  identity and single link. The file ceiling is based on the opened handle.
  The new negative regression confirms neither linked file is modified.
- No change to schema, state machine, permissions, timeouts or runtime
  dependencies. Parent pinning, concurrent Unix namespace/size races, staging
  cleanup and power-loss durability remain separate debt.

## Opt-in Windows MCP startup evidence — 2026-10-20

- [PR #280](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/280) merged as `8b6ccdc`: bounded diagnostics for the intermittent Windows TEMP-only doctor initialization, without altering production permission/isolation or timeouts. Exact head `ce15d03` passed hosted Windows/Linux Rust and dependency policy in CI 39373512931.
- The bridge emits only static startup-phase names and monotonic elapsed milliseconds to stderr when the host explicitly sets `OPTIC_STARTUP_TIMING=1`; the existing Windows CI smoke enables the diagnostic only on its spawned child and records host-side process/initialize timing.
- The instrumentation is integrated, **not a causal fix**. Passing Windows doctor host-side timings were 7ms process start, 30ms initialize sent, 4823ms initialize response. Preserve earlier failed Windows CI attempts and do not infer a root cause from a single good run. Post-merge main CI 39373992973 requires a separate observed conclusion.


## Recovery journal evidence boundary — 2026-10-20

- [PR #277](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/277)
  merged as `c2282ff`: validate actual opened journal file identity before
  recovery parsing and reject hard links (Windows exclusive no-reparse/single
  link, Unix descriptor identity and link count). Exact head `825bcdb`
  passed hosted Windows/Linux Rust, dependency and native preparation.
- [PR #278](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/278)
  merged as `b15fe76`: reject linked files before journal retirement and
  delete the opened file via existing native Windows handle; Unix continues
  to unlink by path after descriptor checks. Exact head `9778b49` passed
  the same four checks. Earlier rustfmt failures were corrected and preserved.
- Post-merge #278 `main` CI
  [39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540)
  initially failed the known intermittent Windows Node/AppContainer loopback
  test (10s `TimedOut`, no output). Three targeted diagnostics passed, then
  a failed-Windows-job-only rerun on the same SHA passed. The first failure
  remains an unresolved reliability finding, not a waived gate.
- These are bounded recovery evidence improvements, not new public MCP
  capabilities, disk quotas, power-loss guarantees, Windows cold-start fixes
  or general Unix pathname-race elimination. Next source review concerns
  append-open aliasing and, independently, staged-artifact cleanup.


## GitHub-only continuity — 2026-10-20

- Add agent entry instructions, a repository-owned GitHub handoff and a complete reusable French continuation prompt.
- Link current docs to hosted validation and the operator's no-PC-testing direction; preserve native Windows gates and known research failures.
- Refresh source/package provenance after #259, #255 and alpha alignment #260; mark the older read-boundary audit as historical.

## Fixed offline Cargo research — 2026-10-20

- Retain the test-only offline fixture, bounded known-output collector and exact-profile disposable NUL comparison; no installed core grant or general build tool.
- Separate normal/prepared research acceptance jobs from standard direct-Rust CI; keep normal failure visible and never use a fallback.
- Add an unrelated-profile denial control and retain the proposed optional Windows preparation/recovery contract.


## Native path boundary hardening — 2026-10-20

- Reject unrepresentable native Windows paths during containment comparisons; prevent distinct UTF-16 root names from collapsing to the same replacement spelling.
- Preserve exact native Git path/hooks arguments and refuse ambiguous operator-policy placement checks.
- Add an actual native read regression, representation/argument/policy tests and a scoped reliability-debt register. No additional permissions or runtime dependency.

## Approved branch retirement — 2026-10-20

- After explicit operator approval, remove the 244 historical branches in the published ledger using exact expected heads; verify every listed branch absent afterward.
- Confirm all preserved pull/archive refs, backup hash and the four reference branches unchanged; record approval and results in the existing consolidation ledger.
- Retire the merged #257 maintenance branch separately. No runtime or permission change.

## Repository consolidation — 2026-10-20

- Replace stale root phase summaries with current overview, measured status and delivery order; retain exact historical snapshots in Git.
- Add documentation navigation, a branch/PR lifecycle and exact preservation/retirement ledger.
- Close 41 superseded PRs; preserve divergent historical heads with archive tags.
- Record the proposed 244-branch retirement as pending explicit operator approval; no remote branch deletion or runtime change.

## Unreleased — exact isolated module reads

- Candidate: derive bounded directory metadata rights from exact launcher read
  grants, pin intermediate directories, and test real nested Node resolution
  alongside content/list/create denial. No recursive read or write grant.

All notable project changes are recorded here.

## [Unreleased]

- Add explicit local CLI supervision of existing finite session/lease windows,
  with terminal revoke/expiry and transport-EOF cleanup; default launches remain timed.

- Read recovery journal records without write rights, preserving exclusive native
  identity/alias checks and separate write requirements for locking and appends.

- Retain validated start-intent fingerprints in host-only private-task recovery
  reports, including legacy records, without new durable data or replay authority.

- Add opt-in fixed std Rust app proof with copied explicit CRT/SDK resources,
  compiler-owned linker descendant and real Vec/parse/assert/println execution.
  Test-only ADR0029, exact validation pending; no Cargo or general execution grant.

- Opt-in fixed no_std Rust native roundtrip: COFF object, direct lld with explicit
  kernel32 import, and produced application exit42 under owned native Jobs.
  Test-only ADR0028, exact validation pending; no Cargo/CRT/general build grant.

- Add exclusive read-only native artifact opening, retaining regular-file,
  no-reparse, single-link and identity checks without requesting write access.
  The fixed Rust proof now collects an actual read-only compiler output;
  native validation pending, no new application permission.

- Extend the optional fixed Rust proof with one compiler-written private LLVM
  file and bounded collection after owned-tree exit, including alias/writer/size
  denial cases. Test-only; exact native validation pending, not build authority.

- Extract existing bounded process-output storage and pipe drains into a private
  child module. Cursor authorization, aggregate reservations, overflow handling
  and termination ownership are unchanged; no new authority or dependency.

- Operator JSON permission reductions and protected startup policy sources,
  with default-off present sections and order-independent process-read denial.
  No implicit scopes or live reload; exact stdio/Windows/Linux validation passed
  in PR237 CI39041285058.
- Reject duplicated operator JSON authority fields before generic map parsing can
  collapse them. Valid version1 configuration and capability scopes are unchanged.

- Opt-in fixed public Rust compiler compatibility probe: isolated no_std LLVM IR
  and real type diagnostics, bounded runtime copying and output. Test-only;
  no required runtime, linker/Cargo or new execution grant. Validation pending.

- Separately scoped absent-only directory creation for Windows project workflows,
  with operator CLI/JSON provisioning and a conditional strict MCP tool. Native
  validation pending; no recursive creation, overwrite or implied file authority.

- Bounded diagnostic private-task event times, with compatible legacy recovery
  and unchanged lock/capacity limits. Timestamps never authorize, reorder events
  or restore tasks; native and Windows/Linux validation pending.

### Added
- Authorized private project input preparation under the existing read/ownership boundary, with finite capacity and nonrecursive recorded-only cleanup. No process, MCP or dependency permission is added. Exact-head validation pending; see ADR-0018.


- Bounded session/task/epoch-bound project input manifests and immutable snapshots, with exact path/version/size validation. Windows capture preflights typed read authority, preserves native path identity and existing ceilings, and discards data on revocation or deadline expiry. No process or MCP authority is added; native branch validation pending (ADR-0012).


### Security hardening candidate — 2026-10-12 (unmerged)

- #183: file read/list/Git read retain session admission and transport/I/O capacity inside their worker through the effect, including caller cancellation/timeout.
- #181: observed native paths use lossless segment conversion for read/list/mutation observation; Unix literal backslashes cannot become different authority targets.
- #176 (pagination only): oversized nonempty list pages fail instead of returning a non-progressing cursor; final cursor size is rechecked.
- See `docs/security/READ_BOUNDARY_HARDENING_2026-10-12.md` for tests and deferred decisions. Classifier, canonical list authorization, Git sensitivity and repository-control namespace remain unresolved.

### Added
- Authorized private project input preparation under the existing read/ownership boundary, with finite capacity and nonrecursive recorded-only cleanup. No process, MCP or dependency permission is added. Exact-head validation pending; see ADR-0018.

- B-02A direct `FixedTool` TCP/IPv4 loopback characterization (PR #109, merge `0d05959a`, exact green head `4a96ff09`, PR CI #426, post-merge CI #427): the existing zero-capability AppContainer Node regression still reaches its network attempt without any accepted loopback connection, while a direct `FixedTool` launched outside AppContainer with only server-allowlisted `SystemRoot` restored reaches `FIXED_TOOL_NETWORK_CONNECTED`, exits 0 and is accepted by the host listener. The earlier empty-environment Winsock 10106 result is explicitly not treated as containment evidence. No network capability, scope or enforcement changed; B-02B must add truthful direct-path containment.
- Phase 3E3 bounded heavy-workload admission (PR #107, merge `826f8cb3`, final head `203d729c`, final-head CI #421, post-merge CI #422): `TaskLease` now carries application-owned `WorkloadClass::{Standard, Heavy}` metadata independent of caller budgets and `ProcessExecutionClass`. Production process leases are `Heavy`; mutation/Git-integration leases remain `Standard`. `HardLimits` defaults to 2 active heavy process jobs bridge-wide and 1 per session; `Running` / `TerminationUncertain` retain a slot and proven-terminal state releases it. MCP derives workload class from the active server-owned lease and cannot choose or downgrade it. Global/per-session heavy-capacity failures have distinct stable MCP codes. No filesystem/network authority, process eligibility, sandbox semantics or public MCP schema was widened.
- Phase 3E2B Windows host-memory emergency-headroom admission (PR #105, merge `aaaf4e17`, exact green head `9cb0516f`, PR CI #414, post-merge CI #415): Windows `ProcessManager` now admits a process only when the validated host snapshot can cover `max(1 GiB, 10% total physical RAM)` plus all active/uncertain Phase 3E1 declared-memory reservations plus the new job budget. The provider is testable, malformed/unavailable telemetry fails closed, 3E1 capacity errors retain precedence, and headroom denial cannot evict retained terminal history. Distinct MCP codes report unavailable telemetry versus insufficient headroom. No MCP schema or process authority changes; heavy-task slots/classes and optional I/O/pressure feedback remain separate A-02 work.
- Phase 3E2A Windows host-memory observation (PR #103, merge `e2b37be0`, exact green head `3c765d9e`, PR CI #410, post-merge CI #411): `optic-bridge-windows` now exposes a validated point-in-time `GlobalMemoryStatusEx` snapshot with total/available physical bytes while keeping the Win32 unsafe boundary in the platform crate. Zero total or available greater than total fails closed as invalid telemetry. This tranche is observation-only: process admission, MCP schemas and authority are unchanged; 3E2B remains the separate emergency-headroom admission gate.
- Phase 3E1 aggregate declared process-memory governor (PR #101, merge `097b6a37`, exact green head `b8447deb`, PR CI #406, post-merge CI #407): `HardLimits` now caps declared process-memory reservation at 16 GiB bridge-wide and 8 GiB per session around the unchanged 8 GiB per-job budget; `Running` / `TerminationUncertain` retain reservation and proven terminal state releases it. Global/session capacity errors remain distinct through MCP. Windows continues independent per-job `JOB_OBJECT_LIMIT_JOB_MEMORY`, and Job Object creation/configuration now occurs before JobRecord publication so setup failure cannot leak output/CPU/memory reservations. This is declared-budget admission, not physical-RAM telemetry or a host-headroom guarantee.
- Phase 3D adversarial multi-session closure (PR #99, merge `d9068334`, exact green head `fdccf3a4`, PR CI #401, post-merge CI #402): one composition-level A/B/C runtime test proves cross-session task-lease denial (`WrongSession`), opaque JobId read/stop denial (`UnknownJob`), per-session active-job pressure without foreign damage, owner-scoped revoke/reap, and safe reuse of reclaimed session/lease/process capacity by C while B remains active. No public session creation/renewal or new authority is introduced.
- Phase 3C3C2C5G real ChatGPT Desktop Node validation is complete: PR #93 (`c6bba66`) added the exact CI-built validation bundle and real plugin-manager preparation path; PR #94 (`694ee3f`) tightened the installed process surface; PR #95 (`1f8449e`) preserved hidden plugin metadata through artifact round-trip; and PR #97 (`4e6f21a`, exact green head `4e27bce`, PR CI #397, post-merge CI #398) fixed the real target's `TEMP`-present/`TMP`-absent Windows launcher environment without widening authority. The exact #398 artifact was installed on the target Windows machine. After a full ChatGPT Desktop restart, a normal Chat displayed explicit approval for exactly canonical `node.exe`, args `["--version"]`, `network=false`, `process_count=1`; approving only that call completed through MCP `process_start/result/read` and returned `v22.15.1`, exit code 0. No process workspace-read grant, workspace file-tool call, mutation, network request or other executable was used.
- Phase 3C3C2C5F installed Node operator profile (PR #92, merge `b4412b08`, exact green head `df6742f3`, PR CI #386, post-merge CI #387): the Windows installer adds explicit `-EnableIsolatedNode` with optional absolute `-NodePath`; the default profile remains process-free. Opt-in auto-discovers/canonicalizes `node.exe`, emits only the existing exact `Interpreter` + `--allow-isolated-node` startup authority, exposes `process_start/read/result/stop`, prompt-gates `process_start`, and never invents workspace read grants. The installer doctor performs a real MCP isolated `node --version` through the installed sibling helper. Installer-profile CI proves default absence, `-NodePath`-without-opt-in rejection, exact generated args/tool allowlist/prompt policy, no automatic `--allow-process-read-file`, doctor success and uninstall cleanup.
- Phase 3C3C2C5E selected Node operator eligibility (PR #91, merge `1cab83f4`, exact green head `f89cd7f6`, PR CI #383, post-merge CI #384): Windows startup adds repeatable `--allow-isolated-node=<absolute-node.exe>` as the first deliberately profile-specific production minting path for `ProcessIsolationEligible`. The exact canonical executable must already be authorized as `--allow-executable=interpreter:<same-path>`, must resolve to `node.exe`, requires the installed sibling isolation helper, and cannot be supplied by MCP. Eligible high-risk leases are capped to one logical process. Native real-binary MCP CI proves wrong-class/non-Node/missing-helper startup rejection, no-profile denial (`optic.process_isolation_unavailable`), caller spoof resistance, multi-process rejection, exact Node admission, exact granted-file read and ungranted-sibling denial. Python/Java/Cargo/rustc remain excluded.
- Phase 3C3C2C5D helper packaging (PR #90, merge `5d3f64a7`, exact green head `815a73c8`, PR CI #380, post-merge CI #381): the Windows release bundle now includes `optic-bridge-isolation-launcher.exe` beside `optic-bridge.exe`; the installer requires/resolves the canonical helper, copies it into the installed `bin` directory, the doctor verifies sibling layout, and installer-profile CI proves byte-exact SHA-256 install/reinstall behavior plus owned-root uninstall cleanup. Helper presence alone grants no `ProcessIsolationEligible` authority.
- Phase 3C3C2C5C exact eligibility policy gate (PR #89, merge `f5bf9e0d`, exact green head `a96fc58f`, PR CI #377, post-merge CI #378): `Interpreter` / `RepositoryCode` can pass policy only when the same active lease has exact `ProcessExecutable` plus exact matching `ProcessIsolationEligible`; missing/path-mismatched/class-mismatched markers stay `ProcessIsolationRequired`, marker-only authority stays scope-denied, and network authority remains independent. Production startup still mints zero eligibility markers.
- Phase 3C3C2C5B internal eligibility provisioning (PR #88, merge `d7aca659`, exact green head `02f977f5`, PR CI #375, post-merge CI #376): application-owned process lease provisioning can attach exact high-risk isolation eligibility only after complete pre-registration validation; FixedTool, unknown and non-Windows non-empty selections fail closed. Real startup intentionally passes an empty eligibility set, so CLI/MCP behavior remains unchanged.
- Phase 3C3C2C5A exact eligibility domain marker (PR #87, merge `c5cafd0`, PR CI #371, post-merge CI #372): adds `ProcessIsolationEligible { executable, class }` as an exact lease scope that grants no process authority by itself and cannot replace `ProcessExecutable`.
- Phase 3C3C2C4 capability-free AppContainer network proof (PR #85, merge `bb649833`, exact head `6fcc25d1`): native Windows CI proves a normal host TCP loopback baseline succeeds, then the same listener accepts no connection from proven-compatible Node launched through the real zero-capability high-risk AppContainer path. The script reaches `NETWORK_ATTEMPT`, never reaches `NETWORK_CONNECTED`, and exits only via bounded socket denial/timeout. PR CI #365 and post-merge `main` CI #366 passed. No network authority or public high-risk admission is added, and the claim is not generalized to every protocol/address family, loopback-exempt host configuration or the direct `FixedTool` path.
- Phase 3C3C2C3 runtime-dependency characterization (PR #83, merge `b5a53b08`, exact green final head `25d2eed7`, CI #361): distinguishes rustup proxy binaries from direct pinned toolchain binaries, proves direct Cargo startup inside the AppContainer, records direct rustc loader failure and bounded Python/Java runtime DLL candidates, and deliberately mints no runtime-directory/stdlib/JDK/sysroot authority.
- Phase 3C3C2C3 representative toolchain characterization (PR #82, merge `8ea01de3`, exact green final head `915b38f4`, CI #357): Node starts through the real helper/AppContainer path, reads exactly one granted file and is denied an ungranted sibling; Python/Java fail earlier at `STATUS_DLL_NOT_FOUND`, while rustup proxy failures are classified separately. Compatibility failures are diagnostic and never broaden workspace authority automatically.
- Phase 3C3C2C2C1 operator-owned exact-file process read provisioning (PR #78, merge `70cde40f`, exact green final head `3b871bdf`): Windows startup can explicitly bind `--allow-process-read-file <absolute-executable> <workspace-file>` to an already-authorized `Interpreter` / `RepositoryCode` executable. Only the matching application-owned process lease gains `FileRead` plus exact `WorkspacePrefix(file)` scopes after existing bounded regular-file/containment/duplicate/count validation; `FixedTool`, `WorkspaceAll`, non-Windows provisioning and MCP-controlled grant selection fail closed. CI #347 passed Ubuntu, Windows, native real-binary smoke, installer profiles and dependency policy. High-risk policy admission remains disabled; validation exposed `cmd.exe` / `findstr.exe` ACL-consumption differences versus the controlled Rust probe, so representative toolchain compatibility is intentionally the next separate gate instead of broadening the grant.
- Phase 3C3C1 fail-closed high-risk execution gate (PR #63, merge `148f4720`, exact green final head `3b63e05d`): otherwise-valid `Interpreter` and `RepositoryCode` process effects are now denied with deterministic `ProcessIsolationRequired` until stronger confinement exists, while `FixedTool` remains available under the existing bounded Job Object runtime. MCP maps this specific denial to stable `optic.process_isolation_unavailable`; ordinary policy denials remain `optic.policy_denied`. Exact-head CI #291 and post-merge `main` CI #292 passed Ubuntu, Windows tests/build/real MCP smoke/installer profiles and dependency policy. This is a fail-closed safety gate, not a sandbox claim.
- Phase 3C3B explicit process execution classification (PR #61, merge `3a907f50`, exact green final head `506e1f7e`): operator-owned executable authority is now classified as `FixedTool`, `Interpreter` or `RepositoryCode`, and that class is carried through the process lease, runtime authority and `Effect::ProcessRun`. Policy requires exact path + class agreement; MCP cannot choose or override the class. Startup now requires `--allow-executable=<fixed-tool|interpreter|repository-code>:<absolute-path>`, rejects legacy unclassified/unknown/empty forms, and rejects duplicate canonical executable paths so one binary cannot receive competing classes in one lifecycle. Exact-head CI #287 and post-merge `main` CI #288 passed Ubuntu, Windows tests/build/real MCP smoke/installer profiles and dependency policy. Classification is an operator assertion only; stronger isolation, truthful network containment and Windows sandboxing remain later gates.
- Phase 3C3A pinned executable identity (PR #59, merge `050a4244`, exact green final head `85a7d92c`): absolute process-executable task-lease scopes now capture a canonical, byte-bounded BLAKE3 content identity at registration. Windows retains a read-shared-only executable handle for the lease-record lifetime, denying concurrent write/delete/rename replacement until physical reap while still allowing normal launch; active lease resolution revalidates identity. MCP schemas are unchanged but identity drift/unavailability receive distinct stable error codes. Exact-head CI #282 and post-merge `main` CI #283 passed Ubuntu, Windows tests/build/real MCP smoke/installer validation and dependency policy. Non-Windows currently relies on bounded content revalidation and does not claim the same persistent-handle guarantee.
- Phase 3C2 bounded process CPU governor (PR #57, merge `fda0f24`, exact green final head `444dab2`): operator-owned hard limits reserve at most 25% CPU per active process job, 75% aggregate Optic process CPU and 50% per session by default. Native Windows process trees receive `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` before resume; `TerminationUncertain` continues to hold its CPU reservation, terminal retained history releases it, and no caller-controlled CPU field or new MCP tool is added. Final exact-head CI #270 and post-merge `main` CI #271 passed Ubuntu, Windows tests/smoke and dependency policy.
- Phase 3C1 bounded process termination (PR #55, merge `bacac76`, exact green head `bfb38a5`): stop/timeout/output-overflow and process-observation failures now request tree termination and require bounded exit confirmation. Failure to prove termination within the confirmation deadline becomes explicit `termination_uncertain`; that state retains process ownership/quota, cannot be evicted or session-reaped as terminal history, and aborts output drains so inherited pipe handles cannot restore an unbounded monitor wait. Exact code/test head `7c01b78` passed Ubuntu, Windows and dependency-policy CI.
- Phase 3B3 lifecycle closure (PR #53, merge `25a40e7`, exact green head `2337651`): `TaskLeaseRegistry` gains hard global/per-session capacities, unpublished partial authority provisioning reclaims its lease slot, startup session creation uses `SessionLifecycleManager::provision()`, and a bounded internal expiry supervisor reuses the admission/revoke/quiescent-reap path. Exact code/test head `5d39f5b` passed Ubuntu, Windows and dependency-policy CI. No public session-minting or renewal protocol is added.
- Phase 3B2 admission/revocation barrier and safe reap: sensitive session-scoped sinks use RAII admission permits; revoke closes new admission and waits for admitted effects before lease/job cleanup; owner-scoped quiescent reap removes terminal process records, owned leases, then the inactive session record. Adversarial Windows tests prove pre-revoke admitted process starts are observed/cancelled and another session remains intact. Direct lease `remove_expired` is removed, and MCP `session_cancel` remains callable after natural session expiry. The automatic expiry invocation path is added by Phase 3B3.
- Phase 3B1 application-owned session lifecycle foundation: `SessionLifecycleManager` provisions opaque server-generated session handles from bounded grants and centralizes fail-closed revoke ordering. Revocation invalidates the session first, then revokes owned task leases and requests termination of owned process jobs. Existing MCP `session_cancel` now routes through this lifecycle manager without changing its public response schema. No physical session/lease/process-record reap is exposed by B1; Phase 3B2 must serialize new session-scoped admission against revoke before safe capacity reclamation can be added.
- Phase 3A bounded multi-session resource-isolation foundation: `HardLimits` now caps registered sessions and adds per-session active-process, retained-process-record and reserved-output-RAM ceilings beneath the existing bridge-wide ceilings. `ProcessManager` applies owner-only terminal-record eviction so one session cannot free capacity by deleting another session's retained process result. Adversarial Windows tests prove active-job and output-reservation exhaustion stay session-local and record pressure preserves another session's terminal result; the MCP adapter exposes distinct per-session resource-limit error codes.
- Phase 2D3 final ChatGPT Desktop validation: the installed default-off Git-integration profile successfully performed `git_integration_status` → prompt-gated exact-head fast-forward → refreshed status, then rejected reuse of the stale precondition with `optic.precondition_failed`. Independent post-smoke verification confirmed the caller `main`/working tree stayed unchanged, the operator-owned internal ref alone advanced, no operation worktree remained, and mutation recovery state was empty. This closes Phase 2D3 and moves current development to Phase 3 multi-session isolation.
- ChatGPT Desktop exact-head Git integration preview packaging: integration remains off by default; explicit `-EnableGitIntegration` provisions the fixed Optic internal ref/runtime, allowlists read-only `git_integration_status` plus prompt-gated `git_integrate`, and extends the MCP doctor. Bootstrap disables prompts/system+global config/replacement objects/hooks, rejects symbolic integration refs, and uses create-only `update-ref --no-deref` CAS without resetting an existing direct ref. Windows CI validates default denial, read-only incompatibility, unsafe custom InstallRoot rejection, symbolic-ref rejection, generated args/tools/approval policy/ref bootstrap/doctor, existing-ref preservation, and explicit uninstall cleanup entirely inside the runner workspace. New installs write an Optic ownership marker so recursive uninstall refuses unrelated non-empty custom paths; normal uninstall never mutates repositories, and `-Workspace ... -RemoveGitIntegrationRef` is required for CAS ref removal.
- Phase 2D3D integration-target observation foundation: typed read-only `Effect::GitIntegrationObserve` reuses the application-owned repository-scoped `GitIntegrate` authority, and conditional MCP `git_integration_status()` returns only the exact current `target_head`. The internal ref/repository/path/executable/argv/lease remain hidden; the real-binary smoke uses this snapshot as `expected_target_head` before integration and confirms the observed head advances afterward.
- Phase 2D3 closure smoke harness: a Windows PowerShell test launches the real `optic-bridge.exe` over stdio MCP against a disposable Git repository, verifies integration-only authority does not imply GitRead, performs an exact-head fast-forward, then proves reuse of the stale target precondition is rejected without moving the ref. Windows CI builds the application binary and runs this smoke after the Rust test suite.
- Phase 2D3C thin conditional MCP `git_integrate` adapter: public input is restricted to exact `source_head` + `expected_target_head`; unknown fields are rejected; ActionId/lease/repository/ref/executable/worktree/argv authority remains server-owned; the tool is registered only with application-owned `GitIntegrate` authority and routes solely through `AuthorizedGitIntegrationService`.
- Phase 2D3B3 startup/operator Git integration wiring: separate `--git-integration-executable` / root / internal-ref configuration, recovery-before-serve without mutation authority, explicit `--allow-git-integrate` provisioning of the application-owned repository lease/capability, fail-closed runtime/authority pairing, and no public `git_integrate` MCP tool yet.
- Phase 2D3B2 bounded Git worktree recovery foundation: machine-readable `git worktree list --porcelain -z` with byte/entry ceilings, complete-snapshot validation before cleanup, ActionId/direct-child/locked/canonical containment checks, no global `worktree prune`, locked removal through double `--force`, pre-cleanup path revalidation, and forced-process-termination recovery coverage; startup wiring and MCP exposure remain disabled.
- Phase 2D3B1 internal Git integration authorization foundation: application-owned `GitIntegrate` task lease restricted to `LeaseScope::Repository`, canonical bounded resource budget, and `AuthorizedGitIntegrationService` requiring active session + exact active lease + deterministic `PolicyEngine` approval before the 2D3A runtime is reachable; no startup flag or MCP mutation exposure yet.
- Phase 2D3A exact-head Git integration runtime foundation: fast-forward-only integration to operator-owned direct refs under `refs/optic/integration/`, exact source/target object binding, detached locked `--no-checkout` worktree validation/cleanup, atomic `update-ref --no-deref <new> <expected>` compare-and-swap, symbolic-ref/hook/replacement-object hardening, and no MCP mutation exposure yet.
- ADR-0010 documenting the internal-ref, exact-head, fast-forward-first Git integration boundary and its remaining recovery/authority/MCP gates.
- Validated local ChatGPT Desktop integration through the compatibility plugin package: normal Chat starts the `optic` stdio MCP server and can invoke the intended file/Git tool surface.
- Developer-preview Windows quick installer (`Install-OpticAIBridge.ps1`), MCP doctor, uninstaller, canonical compatibility-plugin template, and under-three-minute release-bundle installation path.
- Tag-driven Windows release workflow that builds the pinned Rust binary with `--locked`, packages the user installer/plugin, publishes a ZIP, and emits a SHA-256 checksum.
- Phase 2D1 bounded `GitReadService` with exact canonical repository binding, bounded status/diff/log, pinned reachable log cursors and read-only Git hardening; merged in PR #32 (`1cb3cc03`, validated head `ed3b5c43`).
- Phase 2D2 operator-owned MCP Git read adapter exposing conditional `git_status`, `git_diff` and `git_log` without caller-controlled repository/executable/argv authority; merged in PR #33 (`aee4f168`, validated head `54951225`).
- Canonical product, architecture, security, specification, engineering, operations and research documentation.
- Multi-session and same-repository worktree isolation design.
- Deterministic policy/capability model.
- Bounded output and crash-recovery design.
- Living roadmap, project status, security policy and maintenance policy.
- Security invariants and AI-native safety architecture.
- ADRs for application-owned session handles, action envelopes and source/sink containment.
- Phase 0 Rust workspace with `optic-bridge-core`, `optic-bridge-policy` and `optic-bridge-app`.
- Opaque random SessionHandle, TaskLeaseId, ActionId and JobId primitives with redacted Debug output.
- Strict workspace-relative path primitive and BLAKE3 content versions.
- Explicit ResourceBudget/HardLimits types where zero is never interpreted as unlimited.
- Typed ActionEnvelope, SessionGrant and TaskLease.
- Initial deterministic PolicyEngine with negative security tests.
- Windows/Linux CI and cargo-deny dependency policy.
- Phase 0.1 typed `Effect` model so target/precondition/reversibility semantics are derived from one valid variant rather than independently combinable fields.
- Explicit `ExpectedState::{Absent, Content}` mutation preconditions and validated Git object ids for integration preconditions.
- Scope-bearing task leases for workspace prefixes, repositories, process executables and network destinations.
- Monotonic runtime deadline primitive for session/task expiry checks.
- Security tests for lease-scope escape and process-network authorization.
- Phase 1 `optic-bridge-runtime` crate for application-owned runtime services independent of MCP.
- Real monotonic `StdClock` adapter and revocable/expiring `SessionRegistry`.
- `TransportGuard` enforcing request/response byte ceilings, request deadlines and bounded concurrent requests independently of SDK defaults.
- Bounded read-only filesystem service for chunked `fs_read` and deterministic paginated `fs_list`.
- Hard filesystem read/list/scan ceilings and transport concurrency/request-duration ceilings in `HardLimits`.
- Runtime tests covering session revocation/expiry, transport byte/concurrency/deadline bounds, filesystem chunking/pagination and directory scan ceilings.
- Phase 1B `optic-bridge-mcp` adapter using pinned RMCP 3.4.0.
- Optic-owned bounded JSON-line stdio transport with request framing bounded before JSON deserialization and response size bounded after final JSON serialization.
- Read-only MCP tools `fs_read`, `fs_list` and `session_info`, routed through application session lookup, typed effects, policy and runtime services.
- Revocable `TaskLeaseRegistry` for application-owned process authorization.
- Structured `ProcessManager` with absolute canonical executables, args arrays, workspace-contained cwd, cleared environment and explicit inherited-environment allowlist.
- Operator-only process startup allowlists through `--allow-executable` and `--allow-env`.
- Phase 1C MCP tools `process_start`, `process_read`, `process_stop`, `process_result` and `session_cancel`.
- Bounded active process jobs, retained job history, output RAM, process output reads and execution timeout.
- Session-owned cursor-readable stdout/stderr capture with opaque JobIds instead of arbitrary PIDs.
- Native Windows regression coverage for process output, cross-session rejection, timeout, explicit stop and output overflow.
- Phase 1D `optic-bridge-windows` crate as the narrow Win32/unsafe containment boundary.
- Custom Windows `LimitedJobObject` with suspended creation, configure-before-resume ordering, kill-on-close, active-process and total job-memory limits.
- Native Windows Phase 1D fixtures proving process-count enforcement, memory enforcement, descendant-tree timeout cleanup and kill-on-close behavior.
- Phase 2A bounded streaming BLAKE3 content observation for mutation preconditions.
- `HardLimits::max_fs_mutation_bytes` so mutation observation has an explicit hard byte ceiling.
- Runtime `MutationObservation` / `MutationError` foundation for canonical target observation and exact `ExpectedState` conflict detection without exposing durable writes.
- Regression coverage for stale/blind-overwrite preconditions, oversized mutation targets, leaf symlink denial and parent-symlink workspace escape handling.
- Phase 2B Windows handle-first filesystem primitives using no-reparse opens, final paths from handles and `FILE_ID_INFO` identity.
- `AtomicMutationService` / opaque `PreparedMutation` foundation with double expected-state + identity revalidation around same-directory staging.
- Windows existing-target commit through `ReplaceFileW` and absent-target create-only commit through hard-link semantics.
- Explicit `CommitVerification::{Verified, CommittedButUnverified}` so a post-commit verification failure is not misreported as a pre-commit rollback.
- Native Windows mutation tests proving replacement, create-only semantics, same-content delete/recreate identity rejection and parent-directory recreation rejection.
- Phase 2C1 `MutationRecoveryJournal` with operator-owned recovery state stored outside the canonical workspace.
- Append-only recovery state machine `prepared → committing → verified|ambiguous`, keyed by opaque `ActionId`.
- Dedicated `HardLimits::max_mutation_journal_file_bytes` and `max_mutation_recovery_records` ceilings.
- Bounded deterministic recovery scan and reconciliation against exact prior/intended content state.
- Recovery tests proving prepared-only abort classification, intended/prior-state reconciliation, third-state fail-closed retention, outside-workspace state placement and verified-terminal retirement.
- Phase 2C2 `JournaledMutationService` binding the same operation `ActionId` to journal state and deterministic same-directory staging.
- Phase 2C2 process-crash fixtures covering durable prepared, durable committing, post-atomic-service return before terminal state, and terminal state before retirement.
- Recovery-owned staging validation/cleanup with regular-file, byte-ceiling and journaled-intended BLAKE3 checks before journal retirement.
- Negative staging-tamper regression proving mismatched staging fails closed while retaining recovery evidence.
- Phase 2C3A `TransactionalFileService` layered over the journaled mutation boundary for explicit-state whole-file writes and deterministic single-range byte patches.
- Bounded patch snapshot assembly with exact base `ContentVersion` re-hash before deriving and committing the result.
- Cross-platform patch range/size tests plus native Windows write/patch coverage through the real journaled commit path.
- Phase 2C3B1 recovery journal schema v2 with explicit intended `ExpectedState::{Absent, Content}` while retaining strict compatibility with surviving v1 content journals.
- Strict persisted `ContentVersion` hex parsing plus fail-closed mixed-schema, unknown-field and invalid-hash recovery handling.
- Intended-absent recovery tests proving exact committed/not-committed classification before the Windows delete primitive is introduced.
- Phase 2C3B2 dedicated Windows delete-capable no-reparse handle with read + `DELETE`, final-path containment, `FILE_ID_INFO` identity and handle-based `FileDispositionInfo` deletion.
- `PreparedDelete` / `DeleteCommit` runtime primitives requiring an exact existing `ContentVersion`, rejecting stale bytes and same-content file recreation by identity, and explicitly verifying final absence.
- Journal-wrapped transactional delete with `intended = ExpectedState::Absent`, no write-staging artifact and recovery-required semantics after durable `committing`.
- Native Windows forced-process-crash delete fixtures covering durable prepared, durable committing-before-delete, post-delete-service return and terminal-before-retirement, with deterministic restart reconciliation and idempotent second recovery.
- `TransactionalFileService::delete` layered over the same journaled mutation service, plus non-Windows fail-closed coverage.
- Phase 2C3C1 `AuthorizedFileMutationService` as a transport-agnostic authorization boundary: active application session + exact active task lease + deterministic `PolicyEngine` decision before `TransactionalFileService` may execute.
- Phase 2C3C1 negative mutation authorization coverage for missing/cross-session leases, missing capabilities, stale policy epoch, workspace scope escape, primitive/effect mismatch, stale expected state, mutation size bounds and symlink containment.
- Caller-bound recovery-journal operation IDs so an authorized `ActionEnvelope.action_id` can own the same journal, deterministic write staging, commit result and recovery identity end-to-end.
- Active `ActionId` collision protection for recovery journal/temp paths, failing closed instead of reusing live operation state.
- Phase 2C3C2 application-owned mutation authority provisioning from operator startup configuration, with separate `FileWrite` / `FileDelete` capabilities and exact workspace-scoped task leases that MCP cannot mint, widen or freely select.
- Canonical mutation resource budget shared by authority provisioning and normalized action envelopes, plus fail-closed startup parsing for mutation scopes and absolute recovery-state placement.
- Phase 2C3C3 thin MCP mutation tools: conditional `fs_write`, `fs_apply_patch` and `fs_delete` adapters that generate ActionIds server-side, resolve application-owned authority internally and invoke only the authorized transactional runtime.
- Recovery-only startup through `--mutation-state-dir` without mutation authority, with bounded deterministic recovery before MCP serve and fail-closed unresolved conflicts.

### Changed
- CI now uses `--locked` for Clippy/tests and validates PowerShell installer syntax on Windows, preventing dependency-manifest drift from silently rewriting `Cargo.lock`.
- Refreshed the checked-in lockfile so `optic-bridge-runtime` records its declared `optic-bridge-policy` dependency.
- Windows manual smoke commands now preserve an absolute bridge path instead of changing into the disposable repository and then referencing a nonexistent relative `target\release` path.
- Architecture updated for MCP 2026-07-28 stateless protocol semantics.
- Security goal changed from impossible “100% secure” wording to testable invariants plus defense in depth.
- Project lifecycle advanced from documentation-only through executable Phase 0/0.1, completed Phase 1A–1D, Phase 2A/2B mutation foundations, Phase 2C1 recovery state, merged Phase 2C2 journal-wrapped crash recovery (`0297406c`), Phase 2C3A transactional write/patch (`4415a65c`), Phase 2C3B1 intended-state recovery (`f5eafc3a`), Phase 2C3B2 transactional delete (`b346a7d9`), Phase 2C3C1 internal mutation authorization/ActionId binding (`ac381002`, `1b5a3393`), Phase 2C3C2 application-owned authority provisioning (`75477c3b`) and Phase 2C3C3 conditional MCP mutation adapter (`624e88da`); Phase 2D Git read/integration is complete; Phase 3 multi-session isolation is current.
- Phase 2C3B2 merged through PR #24 after exact final head `e74cc0cf` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.
- Phase 2C3C1 authorization merged through PR #26 as `ac381002` after exact final head `fcd1a6aa` passed the full Ubuntu/Windows/`cargo-deny` gate; ActionId binding merged through PR #27 as `1b5a3393` after exact final head `ae05d6a2` passed the same gate.
- Phase 2C3C2 authority provisioning merged through PR #29 as `75477c3b`.
- Phase 2C3C3 MCP mutation adapter merged through PR #30 as `624e88da` after exact final head `f70e520b` passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`.
- Process network access requires NetworkAccess at both session and task-lease level plus an explicit network lease scope at the policy layer; the Phase 1 runtime itself still refuses network-enabled process starts.
- Workspace prefix authorization is segment-aware (`src` does not authorize `src2`).
- Existing read-only filesystem targets are canonicalized and verified to remain under the canonical workspace root before I/O.
- A session receives `ProcessRun` only when the bridge operator explicitly authorizes at least one executable at startup.
- Process output overflow is classified after stdout/stderr drains complete, closing a Windows race where a fast child could exit before the overflow flag was observed.
- Windows process lifecycle uses the Optic-owned Windows adapter so authorized memory/process-count budgets become explicit kernel Job Object limits.
- Phase 2 is split into narrow gates: canonical/bounded observation, Windows mutation-time containment and atomic namespace commit, recovery state machine, journal-wrapped forced-crash gate, transactional file services, authorization/adapter, then Git read/integration.
- Non-Windows durable file commit remains explicitly unsupported until an equivalent containment/commit boundary is designed and proven.
- Production Phase 2C2 recovery now separates journal inspection from retirement so operation evidence survives until staging validation/cleanup completes.
- Phase 2C3 patch is modeled as a deterministic transformation under existing `FileWrite` authority rather than introducing a broader patch-specific capability.
- Recovery intent is no longer content-only: new v2 records encode explicit `Absent|Content`, while surviving v1 content journals remain readable under strict validation.
- Transactional delete uses the absent-intent recovery representation through the same durable lifecycle and is exposed to MCP only when application-owned `FileDelete` authority is explicitly provisioned.
- Started MCP durable mutations retain their transport execution permit and are awaited to a known transaction/recovery result rather than being wrapped in an unsafe timeout around non-cancellable `spawn_blocking` work.

### Security
- Phase 3C3C2C2B2 wires the exact-file read grant lifecycle into the internal isolated process path without changing public policy/MCP admission: the launcher protocol carries at most 32 exact read-grant paths plus the canonical workspace root; `ProcessManager` accepts grants only on the isolated path and resolves only exact regular files; MCP derives candidates only when both the active session and server-owned task lease carry `FileRead`, selecting only `WorkspacePrefix` scopes and ignoring `WorkspaceAll`. The Windows grant primitive now opens workspace root + target, rejects final reparses, resolves both handles with `GetFinalPathNameByHandleW`, verifies final-path containment, then edits the DACL on that same target handle. Grant guards live for the child lifetime and revoke before ephemeral profile destruction. Current operator provisioning still mints no `FileRead` process scopes, so no public high-risk re-admission occurs.
- Phase 3C3C2C2B1 adds the first explicit Windows AppContainer workspace grant primitive without changing MCP/policy admission: only cryptographically ephemeral profiles may mint external ACL grants; the grant is handle-bound to one exact final non-reparse file, adds one non-inheritable `FILE_GENERIC_READ` ACE for the profile Package SID, refuses null/absent DACLs, and revokes explicitly or best-effort on `Drop`. Native Windows CI proves the same profile cannot read before grant, can read only the granted file after grant, cannot append, loses access after revoke/Drop, and predictably named profiles cannot mint persistent grants. Helper/runtime grant selection, directory/recursive grants, write grants, network authority and high-risk re-admission remain future gates.
- Phase 3C3C2C2A wires the proven internal AppContainer launcher into the Windows `ProcessManager` path without changing policy/MCP admission. The launcher path is canonicalized and pinned, `FixedTool` keeps the direct spawn path, direct internal `Interpreter` / `RepositoryCode` starts use the bounded helper protocol when the sibling helper is present, and the Job Object kernel process ceiling reserves exactly +1 process for the helper while the authorized logical workload `process_count` remains unchanged. The helper runs under `env_clear` with only the operator allowlist plus the minimal Windows AppContainer baseline (`SystemRoot`, `LOCALAPPDATA`, `TEMP`, `TMP`); exit code 126 is treated fail-closed. Native Windows CI proves the real runtime routing with logical `process_count = 1`. At that C2A gate the public MCP path still denied high-risk classes, the helper remained non-distributed, and workspace/network grants remained future gates.
- Phase 3C3C2C1 adds a separate, non-distributed Windows internal isolation launcher without re-admitting high-risk execution. It accepts one bounded internal JSON request, recanonicalizes absolute executable/cwd paths, creates a fresh zero-capability AppContainer, gives the target `NUL` stdin, forwards only stdout/stderr through the proven explicit-handle primitive and verifies `TokenIsAppContainer` before resume. Native CI executes the real helper and proves stdout/stderr forwarding, preservation of ungranted-file denial and oversized-request rejection. `ProcessManager` wiring, workspace grants, network semantics and `Interpreter` / `RepositoryCode` re-admission remain separate future gates; future Job Object integration must budget the helper as one additional kernel process without widening the logical workload budget.
- Phase 3C3C2B2 promotes AppContainer process creation into a reusable Windows-only explicit-stdio primitive without re-admitting high-risk execution: stdin/stdout/stderr are duplicated into dedicated inheritable handles, inheritance is restricted with `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, zero-capability `SECURITY_CAPABILITIES` are applied in the same suspended launch, token verification remains available before resume, and unfinished children terminate fail-closed. Native Windows CI proves both ungranted-file denial and captured stdout. The primitive is not selected by `ProcessManager`; workspace grants, network semantics and `Interpreter` / `RepositoryCode` re-admission remain separate future gates.
- Phase 3C3C2B1 carries the server-resolved execution class into `ProcessManager` and adds an independent runtime `IsolationUnavailable` guard, preserving fail-closed high-risk denial even for direct internal runtime calls while leaving MCP unable to choose the class.
- Phase 3C3C2A adds a Windows AppContainer isolation foundation without re-admitting high-risk execution: fresh per-user profiles carry zero capability SIDs, safe Rust lifetime-binds `SECURITY_CAPABILITIES` to the owning profile, and native CI verifies `TokenIsAppContainer` before resume plus denial of an ungranted user-owned file.
- Phase 3C3C1 closes the unsafe interim classification-only state: `Interpreter` and `RepositoryCode` process effects now fail closed with `ProcessIsolationRequired` until stronger confinement is proven. `FixedTool` remains admitted under the existing bounded Job Object runtime. This gate intentionally does not relabel Job Objects as a sandbox or claim restricted-token/AppContainer containment.
- Phase 3C3B makes execution semantics part of operator-owned process authority: the same canonical executable path cannot be simultaneously provisioned under competing classes, policy matches path + class exactly, and MCP never supplies the class. This classification is intentionally not automatic binary inspection.
- Phase 3C3A closes the Windows same-path executable substitution window for an active registered process lease by retaining a read handle that shares reads but denies write/delete replacement, while bounded BLAKE3 revalidation remains mandatory. The guarantee is deliberately platform-scoped: non-Windows does not claim equivalent namespace pinning, and network/sandbox containment remain separate gates.
- Model/repository/process output explicitly treated as untrusted for authorization.
- Transport limits must be enforced by Optic AI Bridge even when an upstream SDK also has limits.
- Initial policy rejects cross-session leases, missing leases for mutating/process actions, stale policy epochs, resource-budget overflow, normal policy mutation and privilege elevation.
- Task leases deny mutations/process/network operations outside their explicit scope.
- Network permission is not inherited from a broad session capability alone for leased process execution.
- Phase 1 runtime state remains application-owned rather than MCP-session-owned.
- Directory listing has both a page bound and an independent deterministic scan ceiling.
- Filesystem reads allocate at most the caller request bounded by the compiled per-call hard ceiling.
- MCP cannot create process capabilities or leases; `process_start` must match an exact canonical executable lease created from operator startup configuration.
- Process environment is empty by default and can inherit only names explicitly allowlisted by the operator.
- Process control is by opaque session-owned JobId; no arbitrary PID kill API exists.
- On Windows, authorized `memory_bytes` and `process_count` map to Job Object `JOB_OBJECT_LIMIT_JOB_MEMORY` and `JOB_OBJECT_LIMIT_ACTIVE_PROCESS`; native CI proves both limits are enforced.
- Phase 2A mutation observation rejects blind overwrite/stale expected state, rejects leaf symlinks, resolves absent-target parents canonically and enforces workspace containment.
- Mutation precondition hashing is both memory-bounded and byte-bounded.
- Phase 2B binds prepared Windows mutations to `FILE_ID_INFO`, so same-path delete/recreate with identical content is still detected as stale.
- Existing and absent targets are revalidated immediately before the namespace commit; replacement data is staged in the same directory and synced before commit.
- Final-component reparse points are denied in the Windows handle boundary, and opened final paths must remain under the canonical workspace root.
- Phase 2B does not claim compare-and-swap semantics: `ReplaceFileW` still names the final target by path after revalidation, leaving a documented external-writer TOCTOU window.
- Phase 2C1 recovery state is kept outside the project workspace, is byte/count bounded, ignores only a torn trailing transition, and retains unresolved third-state conflicts instead of retrying blindly.
- A `prepared` journal record alone cannot authorize a namespace commit; the durable `committing` transition is the protocol boundary before commit may be attempted.
- Phase 2C2 errors after durable `committing` are recovery-required rather than safe retry signals.
- Phase 2C2 keeps journal evidence until any surviving operation staging artifact is validated as regular, bounded and content-equal to the journaled intention; tampering fails closed.
- Phase 2C2 crash tests prove process-termination/restart reconciliation at documented service boundaries, but do not prove sudden-power-loss ACID durability.
- The Phase 2C2 post-commit crash point is after the atomic service returns, not between the raw namespace syscall and its verification.
- Phase 2C3A patch refuses a mixed/changed bounded snapshot unless its complete BLAKE3 still equals the exact expected base; the eventual commit then performs the Phase 2C2 expected-state and Windows identity revalidation again.
- Phase 2C3B1 rejects mixed journal schemas and invalid persisted hashes, keeps the recovery directory stable for upgrade compatibility, and compares observed state against exact intended/prior state without blind retry.
- A surviving write-staging artifact associated with `intended = Absent` fails closed instead of being silently removed.
- Phase 2C3B2 delete requires exact content and exact Windows identity, validates and deletes through the same no-reparse handle, and never falls back to a path-based delete after validation.
- Phase 2C3B2 forced-crash gates prove restart classification at the documented delete service boundaries; they do not expand the project’s durability claim to sudden power loss.
- Phase 2C3C1 keeps authorization outside the MCP adapter: mutation requires application-owned session + exact lease resolution and deterministic policy before the transactional runtime is reachable.
- The authorized envelope ActionId is the durable transaction/recovery identity; active operation-id reuse fails closed, while a general retained replay/idempotency ledger remains deferred to Phase 3.
- Phase 2C3C2 keeps mutation authority application/operator-owned: callers cannot mint/widen capabilities or choose arbitrary mutation leases, and write/delete authority remains structurally scoped and separate.
- Phase 2C3C3 public mutation schemas expose no authorization IDs; server-generated ActionIds and internally resolved leases flow through the existing policy/runtime boundary. Mutation tools are registered only when matching operator-owned authority exists.
- Startup recovery runs before MCP serve whenever a mutation state directory is configured; unresolved recovery fails startup closed.
- Blocking durable mutation execution is cancellation-safe at the adapter boundary: once started, it is followed to a known transaction/recovery outcome instead of returning a timeout while a filesystem effect may still commit.
- Git integration recovery never invokes broad `git worktree prune`: only registered, locked, ActionId-owned direct children of the canonical Optic integration root are eligible, and the full bounded recovery snapshot must validate before cleanup begins.
- Git read and Git integration startup authority use distinct executable options: `--git-executable` alone provisions `GitRead`, while `--git-integration-executable` participates only in integration recovery/authority setup; neither silently grants the other.
- Git integration recovery executes before MCP serve when the complete integration runtime configuration is present; `--allow-git-integrate` is separately required before a `GitIntegrate` capability/lease exists, and the MCP server rejects runtime/authority mismatch.
- Git integration MCP execution retains its transport permit until a started blocking integration reaches a known outcome; stale target and non-fast-forward failures are explicit, while a successful ref update followed by unverifiable post-update state is reported as `optic.git_integration_outcome_uncertain` rather than a safe retry.

- Keep private preparation profiles on bounded native owner workers through async caller cancellation; acknowledge cleanup before releasing a successful stage reservation. No new process or MCP authority.

- Add owned suspended-AppContainer tree containment and single-reader bounded stdout/stderr polling, with native lifecycle regressions and no temporary disk spool.

- Add data-only input fingerprints and narrow native state/profile planning primitives; acknowledge private profile close before releasing preparation capacity.

- Add an owned-job watchdog armed before resume, with a nonrenewable deadline and bounded atomic stop observation independent of owner-worker I/O. No new MCP authority.

- Add a bounded explicit Unicode environment for suspended native children, with no implicit inheritance on an empty list and no mutation of the parent's environment.

- Add a host-only journaled read-only private Node controller with bounded job/output ownership, crash-intent evidence, retained unresolved capacity, and no public task replay or new MCP tool.

- Compose bounded same-workspace artifact capture with independently authorized versioned publication and explicit partial progress; no process write permission or new MCP route is added.
