# Delivery roadmap

## REPRISE P0 — 26 octobre 2026

Documents : [Passation canonique Windows V1 du 26 octobre](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-26.md) et [prompt complet à copier](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-26.md). Avant cette PR documentaire : `main=c058ccf3f9db18c7d67194d117af99cc3e0cea52`, [CI indépendante push/main #40358111878](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40358111878) **SUCCESS 4/4 au premier essai**. Cette PR fera avancer `main` : vérifier le nouveau SHA et la CI `push/main` après fusion. #369/#371/#373/#375/#377 sont des contrats GitRead **inertes** ; production MCP GitRead toujours legacy ; #327/#353/#358 recherches isolées, originaux #352/#355 **ROUGES** ; issues #317/#323/#185/#319 ouvertes ; Windows V1 NO-GO, aucun accès PC/Optic Desktop/REL.

1. Auditer `ReadonlyMcpServer::run_admitted_read`, `authorize_git_read`, `GitReadService` et le contrat `NativeGitReadOwnedPermits` pour UN vrai `git status` MCP, sans double admission ni permission ambiante.
2. Enregistrer Job/root/stdio/quotas et pin identité Git AVANT la création native, synchroniser stop+epoch+TaskLease+deadline juste avant le spawn et préserver `TerminationUncertain`.
3. Réussir **originaux inchangés** #355 (aucun Git réel post-revoke) et #352 (vraie fin stdin/stdout/stderr en temps borné), puis #185/#319, données sensibles, Win10/Win11 physiques, installateur offline/update/rollback #323. **V1 NO-GO.**

---

## 26 October 2026 — after trusted transfer seam #377

[PR #377](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/377) adds a registry-origin-checked handoff of the worker's existing counted session admission and transport permit into an inert GitRead native-owner contract. This is **not live MCP routing**.

1. **Next P0 single-handler wiring:** redesign a tightly scoped MCP `git_status` worker to hold transferable **owned**, not borrowed, session admission and the original request permit. Keep the legacy GitRead path unchanged until a service-owned native Job/root/pipes guardian can take custody of the same admission/lease trio without double-counting or premature drop. Preserve sensitive staged/untracked metadata protections.
2. **Next P0 actual native launch authority:** exact canonical repository and fixed opened/pinned/hashed Git executable, registered Job/guardian and quota reservations **before** process creation, immutable generation-bound session/lease/transport stop witnesses, synchronized last pre-spawn policy/epoch/revoke/deadline checks; `is_stop_required_at == false` is never independent spawn permission. Keep capacity on `TerminationUncertain`.
3. **Acceptance unmodified:** original #355 real post-revoke no Git spawn and #352 true total elapsed stdin/stdout/stderr EOF, root + Job-empty and host-loss. Research #327/#353/#358 stays isolated until integrated honestly; independent #185 AppContainer, #319 MCP first response, #176/#182 sensitive authorization and #323 actual Windows 10/11 install/update/rollback remain required. **#317/#323 OPEN, V1 NO-GO.**

---

## 26 October 2026 — after three-resource native admission contract #375

[PR #375](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/375) now packages an owned session admission, *real owned request transport permit* and exact TaskLease/revocation witness into one inert Rust contract. All three retain their original per-operation deadlines. No Git process is launched by the helper.

1. **P0 real single-tool integration:** route one genuinely admitted MCP `git_status` envelope and sensitive metadata/provenance checks into a service-owned native Windows Job/pipe guardian, transferring the current worker's owned transport permit + counted admission + lease without double-dropping reservations or altering unrelated GitRead legacy behavior.
2. **P0 true pre-spawn authority:** pin/hash the fixed Git executable and canonical repository from operator-owned state, register root/Job/pipes and quotas before child creation, and revalidate policy/epoch/revocation/expiry immediately at the last synchronized spawn boundary. Watcher `false` never licenses launch. Preserve TerminationUncertain capacity.
3. **P0 behavioral acceptance unchanged:** prove original [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) no real Git start after revoke, original [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) actual elapsed stdin/stdout/stderr/EOF, root + Job empty and host-loss recovery. Separate #185 AppContainer, #319 first MCP response, #176/#182 sensitive provenance, #323 installer/offline updates/rollback and real authorized Windows 10/11 tests remain required. **Windows V1 NO-GO; #317/#323 OPEN.**

---

## 26 October 2026 — after registry cleanup witness fix #373

[PR #373](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/373) closes one *registry cleanup* invalidation gap: physical TaskLease removal now publishes original-generation stop, even if the normal session lifecycle's earlier `revoke_session` is bypassed. It does **not** close #317's native-process ownership/race, prove a child has stopped, alter the live MCP GitRead path or change #352/#355.

1. Preserve existing tests and scoped permissions. Next real execution seam remains **one admitted MCP `git status`** with server-owned identity, exact registered scoped TaskLease, sensitive metadata/provenance approval and a pinned executable.
2. Before any native child exists, register its service-owned Job/root/stdio and cancellation/reservations, then revalidate session/lease policy, identity, expiry and revocation at the final synchronized spawn boundary. Do not mistake a watcher reading `false` for launch authorization.
3. Prove the **original unmodified** [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) post-revocation real Git no-start and [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) total wall-clock pipe/EOF gates, including root + Job-empty, crash/restart recovery and uncertain native ownership. #185, #319, #176/#182, real Windows 10/11 and #323 installer/offline update/rollback remain independent gates. **V1 NO-GO, #317/#323 OPEN.**

---

## Next P0 after admitted native GitRead preflight #371 — 25 October 2026

[PR #371](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/371) now binds **genuine owned session admission** to the exact registry-verified TaskLease policy witness, with original session/lease revocation and expiry. See [native preflight contract](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md). This is an inert Rust service prerequisite, **not live MCP wiring or a spawn permit**.

1. Next narrow slice: wire one *real MCP-admitted* `git status` ActionEnvelope through the trusted server application session, task registry, sensitive filename/provenance checks and canonical root into a **registered, retained native owner and Job before child creation**. Keep legacy GitRead behavior unchanged until the new path satisfies all authorization and behavioral acceptance.
2. Validate final pre-spawn generation, policy epoch, deadline, exact opened/hashed FixedTool Git identity and original-stop witnesses at a properly synchronized authority boundary. Negative watcher state alone is insufficient; maintain count/Job/pipe ownership under `TerminationUncertain`.
3. Meet **original unchanged** [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) real Git post-revocation no-spawn and [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) elapsed stdout/stderr/stdin + EOF tests on Windows/Ubuntu. Continue separate #185 AppContainer, #176/#182 sensitive provenance, #319 first MCP response and #323 Windows 10/11 physical/offline install/update/rollback work. Do not bulk-merge #353/#358. **V1 NO-GO, #317/#323 OPEN.**

---

## P0 next seam — 25 October 2026, after integrated registered-lease witness #369

[PR #369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/369) combined the genuine registry TaskLease and original-generation revocation watch with the existing typed GitRead policy preflight. See [inert contract and limitations](docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md). This is **not** a native owner, true MCP admission, or permission to spawn; the legacy `GitReadService` still launches via `HardenedCommandRunner`.

1. **Next smallest tested integration:** make one real MCP `git status` path pass its *admitted* `ActionEnvelope`, sensitive metadata/provenance decision, exact active session and TaskLease, canonical worktree and opened/hash-verified Git executable into a service-owned native Job/reservation. Do not change all legacy GitRead endpoints as a shortcut.
2. **Final native boundary:** register all ownership, pipes, stop/watch and quotas before creating the process; revalidate authorization, deadline, execution identity and generation-bound revocation at the last safe pre-spawn point. Preserve uncertain ownership and capacity. A negative `is_revoked` bit from #369 is never sufficient launch authority.
3. **Unchanged behavioral gates:** original [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) actual post-revocation Git start and [#352](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/352) bounded real stdout/stderr/stdin/EOF remain **RED**; do not modify their assertions, inflate delays, detach unsupervised pipes or release reservations without proof. Also require root + Job-empty + genuine EOF, crash/restart recovery and #185, #176/#182, #319, authorized Windows 10/11 physical tests and offline update/rollback #323. **WINDOWS V1 NO-GO; #317/#323 remain OPEN.**

---

## Immediate next-agent work order — 25 October 2026

Read [complete canonical handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-25.md) and [new-chat prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-25.md). Verify live refs before coding. Baseline before the docs PR: `main=7c72ecfb8886fb34be2cc7ef9f2e1070f73c08da`, [independent CI #40188090038](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40188090038) green 4/4; experimental #358 `f431c3ef…` with native evidence green on exact research SHA **not in main**.

1. **P0 #317 — trustworthy live MCP/native authorization seam.** Existing production typed scopes and `PolicyEngine::evaluate_native_git_read_preflight` are prerequisites only; live MCP `authorize_git_read` uses `task_lease: None` and `GitReadService` still calls `HardenedCommandRunner`. Prefer narrow test-first contract and adapter with real ActionEnvelope, sensitive Git authority, active registry-backed TaskLease, session admission, canonical repo, exact opened/hash-verified Git executable, one bound original revocation witness, pre-spawn native Job/owner and final stop check. Do NOT globally alter legacy GitRead permission before adapter is ready; do NOT wholesale merge research #353/#358.
2. **P0 acceptance #355 and #352.** Re-run existing original tests unmodified after actual candidate wiring: no real Git spawn after published revoke (specialized [#40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203) RED); true bounded stdout/stderr/**stdin** completion when descendants retain handles (specialized [#40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) RED). Never inflate timeouts, skip tests, fake EOF, leak pipes or release reservations early.
3. **P0 lifecycle/release #323.** Require root exit + native Job empty + both real EOFs and stdin writer completion; preserve TerminationUncertain, original Win32 errors, active admissions and quotas; external host-crash/guardian recovery, session close, sensitive reads/provenance #176/#182, #185 AppContainer, #319 first reply, authorized Windows 10/11 and offline installer/update/rollback acceptance. **Windows V1 NO-GO; #317/#323 OPEN.**

---


## Next P0 acceptance after native witness research #366 — 25 October 2026

- The latest **isolated** native owner study lives in [draft PR #358](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/358) at `f431c3efde2e5d9512eb1715ad3475196f18bf1d` after [research #366](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/366) was merged into #358 only. Its independently run CI is green: [general #40187179915](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179915), [native #40187179856](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179856), [private #40187179916](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40187179916), all first attempt. Production `main=288fa176251ac9038a8c6d1b8df74c6eac724d3c` is unchanged.
- The newest verified property is atomic registry verification of the *original* TaskLease + revoke witness, retained through native reservation. A later same-ID lease cannot make a revoked native owner spawn, and capacity is only released on its own never-spawned terminal receipt. This is **research evidence**, not MCP authorisation.
- **Next smallest production slice:** design a real MCP `ActionEnvelope`/sensitive-read/TaskLease authority adapter, with exact current session admission, canonical repository and verified Git executable, the #362 policy preflight, service-owned native Job and a final race-safe check immediately before kernel creation. Do not unconditionally demand a lease on the legacy GitRead runner before the adapter is complete. Reconcile #358 research with production #364 without duplicating native helpers.
- Run existing independent #355 real Git post-revoke gate and #352 stdout/stderr/stdin elapsed/EOF gate *unchanged*; then host crash/Job-empty/recovery, #185 Node/AppContainer and #323 Win10/11 installer/update/rollback. Green isolated owner tests never turn specialized RED acceptance green. **#317/#323 OPEN, V1 NO-GO.**

---

## Next P0 boundary — 25 October 2026 after #362

Code [#362](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/362) merged a *policy-only* `evaluate_native_git_read_preflight` on `main=739bdd5de3a86d72eaa251cb4f9f311e69b33e0e` with first-attempt full [PR CI #40182783889](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40182783889) 4/4. This checks GitRead action + exact TaskLease/session/epoch, root scope, FixedTool pin *scope* and finite watchdog envelope; it is **not** native authorization to spawn and is not called from live MCP yet. Independent [push/main #40183165673](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40183165673) retained separate #185 Windows Node/AppContainer RED; Ubuntu/dependency/release passed.

**Next smallest production step:** join the genuine MCP action/policy decision and active registry-backed TaskLease (including opened-and-hashed executable identity, canonical repository path and revoke watch) to a *service-owned* native reservation **before any process creation**, recheck immediately before spawn, preserve admissions/quotas on uncertainty. Prove #355 post-revoke no-spawn and #352 real wall-clock stdout/stderr **and stdin** independently; then prove root+Job-empty+EOF terminal, host-loss recovery, Win10/Win11 packaging/update/rollback. Research PRs #327/#352/#353/#355/#358 are not a bulk integration plan. **#317/#323 OPEN; Windows V1 NO-GO.**

---

## P0 sequencing — 25 October 2026 after five research PRs

**Verified production `main=d7d4d50adf95b6a3cf4a5df0ef06a6ba526d6810`**, independent [CI #40173338835](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40173338835) 4/4 green. Merged #356/#357 typed common GitRead command construction only; native owner is **NOT live**. #317/#323 remain open, V1 NO-GO.

1. **Behavioral acceptance RED:** [#352 dedicated CI #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) Windows/Ubuntu wall-clock ~1.6 s on 200 ms timeout for stdout/stderr/stdin; ordinary CI green does not supersede it. [#355 dedicated CI #40158288203](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40158288203) Windows/Ubuntu Git spawned after published revoke; its ordinary Windows CI also red independently under #185 Node/AppContainer. Both must be corrected without suppressing/rerunning-to-green.
2. **Unmerged owners:** #353 head `b091dc28…` has staged Windows Job/pipes/watchdog/ledger, real typed Git status, counted admission, required GitRead capability+TaskLease/policy/executable and actual watchdog time-horizon revalidation, native terminal/EOF proof; [CI #40177026825](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177026825) and [native #40177026834](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177026834) green. #358 head `59d1a331…` is **stacked onto #353, not main**; pins exact canonical GitRead repository to TaskLease in addition to generic Repository marker, [CI #40177299174](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299174)/[native #40177299159](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40177299159) green on isolated head. #327 remains independent older research. No wholesale merge.
3. **Next smallest safe production seam:** bind real MCP GitRead policy/action/sensitive-read authorization, nonexpired scoped TaskLease, session/policy epoch, canonical root, exact Git exe, fixed typed builder, owned native admission/Job registered before child creation, and recheck under revocation immediately before spawn. If proof insufficient, first create narrow deterministic test/contract, not monolithic migration. Keep admission/quota/handles under native uncertainty.
4. **Later P0:** actual bounded EOF and cancellation for stdout/stderr **and stdin**, root+Job-empty receipts and guardian crash/restart reconciliation. Separate #176/#182, #185, #319, Windows 10/11/offline installer/update/rollback #323. Release remains NO-GO.

All earlier "latest" sections below are historical. [Detailed handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md), [complete successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md).

---


## Immediate engineering priorities — 25 October 2026

1. **P0 #317 — actual bounded completion:** [#352 dedicated acceptance CI #40087176209](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40087176209) is intentionally RED on Windows and Ubuntu; descendant-retained stdout/stderr and stdin still cost ~1.6 s even though deadline is 200 ms and result is now `TimedOut` after #347. A future production fix must satisfy the actual <900ms test *without* weakening it, detached writers/readers, fake EOF, early resource releases or timeout inflation.
2. **P0 #317 — integrated trusted native owner:** [#353](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/353) has proof-of-concept Windows native Job/pipes/watchdog, admission/ledger/quota, pre-adoption cancellation and `GitRead` capability gate, full [CI #40153942601](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40153942601) and specialized [#40153942636](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40153942636) green. Still no actual `GitReadService` integration or full task lease, exact Git exe/repo provenance, stdin, service restart, persistent OS error.
3. **P0 #317 — avoid overclaiming:** [#350](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/350) fixes registered-job stop-before-drain only; [#349](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/349) preserves Windows native Job query error codes; [#347](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/347) fixes late-result classification, **not real pipe wait time**. Root exit, Job-empty and actual EOF are separate; retain session generation/admissions/quotas on uncertainty.
4. **Other independent gates:** GitRead sensitive authority/provenance #176/#182, Node/AppContainer #185 (preserve independent [main red #40084443275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40084443275)), first MCP response #319, host crash/recovery, Windows 10/11 field acceptance, installer/offline/release/rollback #323. Windows V1 **NO-GO**.

**Baseline:** `main=6a6457155758df3050e2aad81ee9446952d7eedd`, [push CI #40086602634](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40086602634) successful on exact main. At checkpoint, only PRs **#327/#352/#353** open, issues **#317/#323** open. All hashes are dated evidence and must be checked anew. See [latest handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) and [current successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md). Earlier roadmap sections are history.



## Next gates — 24 October 2026, after #349/#350

**Engineering code checkpoint:** `main=e0de79147e04daaea75cf59694d0e791b824fedc`. [PR #349](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/349) preserves genuine native Windows Job query errors; [PR #350](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/350) requests cancellation of **registered** session-owned process jobs before the admission-drain barrier, with a second sweep for late registration. Exact corrected-head full CI [#40085443930](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40085443930) 4/4 green; independent [push/main CI #40085754973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40085754973) **completed SUCCESS on `e0de7914` at first attempt, 4/4 jobs**.

1. **P0 #317 actual finite pipe/process lifetime:** replace direct-root-only `HardenedCommandRunner`'s blocking stdout/stderr/stdin joins with owned native/process-group supervision and cancellable bounded pipe EOF, holding admission/quotas until root and full Job/process tree are independently proven terminal. The prior #347 correction changes eventual results, not elapsed lifetime.
2. **P0 #317 late native registration and stop ownership:** prove process owner registered before any native spawn, stop observed during admission drain even if the job is not yet recorded, retained ownership on failed OS accounting, and true Job-empty/EOF receipts. Two cancellation sweeps (#350) are **not** sufficient for a guardian that holds an admission indefinitely without reacting to revoke. Never bulk-merge #327.
3. **P0 #317 durable and uncertain state:** preserve distinct OS observation phases, original Win32/HRESULT, no release of uncertain reservations, crash/guardian/restart reconciliation, bounded shutdown.
4. **P0 #176/#182** sensitive files and Git provenance; **P1 #185/#319** intermittent Node AppContainer and first MCP response; keep prior failed CI and compare distinct no-output vs post-network-attempt signatures.
5. **V1 #323 NO-GO:** real authorized Windows 10/11 acceptance, offline install/update/rollback and exact-integrated-main CI. No release tag or operator-PC command until explicitly authorized.

---

## Updated sequencing — 24 Oct 2026 after physical Windows 10 validation / Codex recovery procedure

**Release gate remains NO-GO:** #317 and #323 open. Production `main=dce94cc02f178d91b1eec4dc06134ab57df47ff6` after docs-only #343; confirm latest independent push/main CI [#40075855681](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40075855681) on its exact SHA. Earlier red Windows [#40059745546](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40059745546) (`TerminationUncertain` instead of `TimedOut`) must remain visible regardless of newer green research jobs. [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) is an independent unmerged owner experiment.

1. **P0 native uncertainty / owner authority (#317):** distinguish root wait, root exit + Job nonempty, query failure with preserved Win32 code, and proven root + Job-empty; demonstrate bounded negative observation, native owner retention, generation-bound `OwnedSessionAdmissionPermit` / `JobStore` quota, and out-of-band stop-before-draining admission. No automatic terminal publication or dropping owner when uncertain.
2. **P0 bounded pipes / stdin (#317):** design root+Job+all pipe owners as a single auditable lifetime. Invert the already demonstrated regression evidence from closed research [#339 stdout/stderr](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/339) and [#341 stdin/EOF](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/341) into *correctness* tests before a minimal reviewed production change. No detached readers/writers or timeout-only “clean” result.
3. **P0 owner loss / shutdown:** prove crash/guardian loss and restart reconciliation with true native processes and positive OS quiescence. Research #340 and physical Windows 10 #342 verified **normal paths**, not permanent observation failure. Preserve original failed main CI and all negative outcomes.
4. **P0 sensitive Git authorization (#176/#182):** repo/exe/provenance, task leases, secrets/aliases, deterministic GitRead policy after native owner contract proven. Do not wire the entirety of PR #327 into production.
5. **P1 startup / isolation (#185/#319):** real AppContainer/Node and first MCP response controls, kept separate from #317.
6. **V1 freeze (#323):** real Windows 10/11 acceptance, offline installer, update/rollback, exact-integrated-main CI and emergency procedures; V1 only GO after independently demonstrated security/availability gates.

**Operational (not production):** docs-only [PR #343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/343) added [Codex interruption recovery](docs/engineering/CODEX_INTERRUPTION_RECOVERY.md) to preserve checkpoints across interrupted assistant sessions. The procedure cannot override security blocks or survive same-disk failure by itself. Local snapshot reports are not remote CI evidence. Do not create competing handoff documents; use [this 24 Oct canonical handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) and [full successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md).

---


## Updated priority after ten hosted native crash scenarios — 24 Oct 2026

Latest researched and verified **unmerged** code: `research/317-failed-native-create-2026-10-24@b5a91dee`, exact [CI #40057931887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887) fully green with **ten** native externally supervised Windows scenarios. Production `main=16ca67a7` remained unchanged at this checkpoint; **#317/#323 OPEN, V1 NO-GO**. See [latest handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) for the research matrix, negative controls and semantics.

**Next engineering priority:** reproduce and characterize **descendant-held stdout/stderr pipe handles after parent exit**, bounded stdin closure/EOF and asynchronous stop-before-admission-drain under persistent native uncertainty. Keep the original guardian Job, generation-bound permit, JobStore quota and nonterminal status retained until independent native evidence; a timer cannot manufacture a terminal receipt. Prove host-loss/restart recovery or explicitly document why a persistent supervisor/ADR is required. Do not continue generating only near-identical 'suspended child/no-ACK' tests now that those controls are green. Only then extract one small independent production-safe increment from **current `main`** with exact-head Windows/Ubuntu gates; do not bulk merge research. GitRead task-lease/repo/exe and provenance (#176/#182), #185/#319, Windows 10/11 physical device and installer/release/rollback gates remain separate. GitHub-hosted only; Codex local blocked by controls is not a reason to bypass them.


## Current immediate sequencing — 24 October 2026

The [24 Oct canonical Windows V1 handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) supersedes the older 23 Oct research snapshots for the **current** work order. Production already has the *counted owned session admission* from #336, while research has separately proved JobStore commitment, kernel Job association at CreateProcessW, external synthetic creator-host death, combined stop-before-drain and **temporary injected** Win32 accounting uncertainty (latest exact-source [CI #40046563179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40046563179)). None of the combined native research is wired to GitRead in main.

**Next critical acceptance order:** (1) nonblocking stop publication distinct from terminal drain/reap with an accountable retained owner even under *persistently unavailable native observation*; (2) externally supervised Windows native descendant/stdio-EOF, creator/service death and quota/handle uncertainty tests; (3) only then one bounded production-owned read-only Git effect under approved lease/executable/repo provenance and security #176/#182; (4) CI #185/#319, Windows 10/11 devices, release/rollback acceptance. Keep issues #317/#323 OPEN and V1 NO-GO. No user-PC testing, new feature, broad research merge, or release by implication. Treat the 23 Oct sections below as history, and inspect GitHub live.

Reviewed: 2026-10-22. [STATUS.md](STATUS.md) owns current evidence; this page owns
the next delivery order. Earlier phase/H-track snapshots remain historical
planning inputs, not a second statement of current completion.

Execution mode for the current continuation: agent-owned isolated environment
and GitHub-hosted CI only; no operator-PC tests or runtime changes. See the
[handoff](docs/engineering/GITHUB_HANDOFF.md). This does not remove native Windows
acceptance gates or imply they can be established on Linux.

## Current integration priorities — 23 October 2026 after PR 334

Production `main` now includes the **read-only, generation-bound admitted stop
condition** (PR 334) in addition to PR 331's explicit revocation watcher.
The new witness observes the original admission expiry despite renewal; it is
**not** native process ownership, a launch permit or a completion receipt.
Use the [latest measured status](STATUS.md) instead of dated SHAs below.

The immediate next bounded increments are: (1) specify and test one immutable
session-generation-bound registered-owner commit under a shared close
linearization, with actual task/lease policy gates kept independent; (2) ensure
stop request reaches already admitted workers **before** any synchronous permit
drain, including late registration and uncertain Win32 observation; (3) prove
Windows process tree membership **at creation** and full stdout/stderr/stdin,
host crash, deadline and quota retention under an independent native
supervisor; (4) only then migrate one sealed read-only Git path with safe
publication/provenance. Keep legacy session/approval/lifecycle report contracts
until their callers are reviewed. Preserve the real concurrent reaper/ABA
negative control as a separate generation-isolation acceptance case, while
distinguishing forced handle reuse from supported provisioning.

The V1 decision remains **NO-GO** under issues 317/323. Also preserve
sensitive-content/provenance review (176/182), native Windows intermittency
(185/319), real Windows 10/11 operator-authorized testing, release rollback
and offline installer/PowerShell checks. Hosted green tests alone do not
satisfy this matrix. No GitHub ruleset or user-PC changes in this checkpoint.

## Current Windows V1 delivery order — 23 October 2026

[Read current handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-23.md) and [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). **Stabilization, not new features:** #317 remains the foremost Windows reliability blocker, but the new `ReadServiceRoot` research at `1c7dac9` has progressed past inert owner fixtures with real shared JobStore, native cleanup and seal/reap tests (CI #39888522519 + native #39888522200). Do **not** claim production repair: last-known `main=3fe3de0c` still runs HardenedCommandRunner, and PR #327 remains open with a real #185 red run. The new **host-crash-before-Job-assignment** reproduction leaves a suspended child until independent outer supervisor cleans it; revocation admission only protects submit; Drop may wait forever on persistent native observation errors.

**Next:** make an explicit architecture/revoke/shutdown responsibility decision; then prove child containment across host crash and stop-versus-resume linearization; then verified Git authorizer/pins/leases/deadlines and ONE read-only Git path, with #182 patch-provenance policy separately approved. #185/#319 native intermittency, real Windows10/11 support, packaging/recovery and release provenance remain gates. No Electron, remote-project scope or V1 release without owner consent.

## Current Windows V1 stabilization sequencing — handoff 22 October 2026

Owner directive: V1 is a future **release decision following stabilization of existing Rust/MCP code**, not an automatic result of green CI. [Current handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-22.md) is the continuity point; [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) holds the evidence-based V1 freeze/no-go matrix. The highest-priority release blocker is #317: a Windows service-owned Git command process tree, robust stdin/stdout/stderr cleanup, owner-retained uncertainty and session seal/reap must be proven on the *production* runner. The research branch has 13 supervised native cases, real SessionRegistry/Job tests and a **non-wired prototype ledger**, none merged to main. **Next:** review/prove independent native observation and stop (avoid callbacks under a global lock), then staged Windows owned process and session integration before any GitRead migration. No wholesale research merge.

After #317, resolve or fail-closed exclude unproven V1-reachable parts of #176/#182, #319 and #185; certify supported Windows 10/11/PowerShell combinations with operator-approved real-device tests, and prove reproducible release/rollback/CI. Out-of-scope remote H-01–H-05/#177 and Electron remain deferred with enforced exclusions. No V1 tag or feature unfreeze without operator approval.

## Windows V1 first — operator decision, 22 October 2026

Target Windows10/11; verify actual supported versions rather than inferring them
from hosted Windows Server. Prioritize demonstrated Windows command ownership317, keeping sensitive-read
176/182 gates explicit, then MCP startup319, AppContainer/Node185 and proven
recovery/authorization/resource and installation defects. Keep these distinct; doctor error-path diagnostics do not
explain natural startup latency. Linux functional support is post-V1. Do not start
new cgroup/pidfd/PDEATHSIG/confinement research unless shared-code regression
actually blocks Windows; retain existing Ubuntu CI as non-regression checks.
Rust/MCP remains core; Electron can resume only after the relevant foundation
gates and a delivery decision, never as a mandatory Rust runtime dependency.

## Temporary stabilization gate (operator directive, 2026-10-20)

**The feature roadmap below is paused, not cancelled.** Do not start new
features, optional integrations, GUI work, broad refactors or experimental
alpha-to-main merges. Prioritize reproducible defects in the existing
permission/isolation/recovery/process boundaries, reliable diagnostics for
intermittent hosted Windows gates, targeted regressions and documentation
accuracy. A corrective PR requires a demonstrated failure and exact-head
native checks where applicable. If no defect is demonstrated, do not create
a change merely to show activity. Feature work resumes only on a new operator
decision. This freeze does not relax any existing security or CI requirement.

## Stabilization exit criteria — objective gates, not a claim of zero bugs

Two **distinct** decisions are required:

1. **Resume bounded headless development** only after an exact-head Windows/Linux CI baseline and targeted safety checks for any newly exposed effect; the known Windows AppContainer/Node and MCP first-response flakes must be explained, corrected, or explicitly excluded from that narrowly permitted workflow **without weakening a security test**. The operator must explicitly authorize lifting the feature freeze for this limited scope.
2. **Enable real-project remote/autonomous work** only after deterministic classification/enforcement of sensitive filesystem content/list metadata ([#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)) and a matching GitRead authority decision ([#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182)); authenticated principal, session and host correlation proof (H-01–H-03), local approval/revocation contracts (H-04/H-05), process/namespace and recovery boundaries, and appropriately reproducible soak/fault-injection evidence. A green short CI is insufficient.

**Before either decision:** retain actual red-run evidence, avoid speculative timeout increases and redundant PRs, explicitly classify partial Windows power-loss/ancestor/race guarantees, and reconcile code review coverage and handoff docs with the exact `main` SHA. Existing documentation and PR status inconsistencies are a real continuity risk, not evidence of a new runtime defect. Owner approval is required before changing GitHub branch-protection requirements; the last GitHub query reported `main.protected=false` and no rulesets.

## Deferred feature delivery sequence (not currently authorized)

1. **Maintain coherent integration.** Repository consolidation and bounded
   diagnostic scopes are recorded; keep `main` and the generated-project alpha
   aligned through reviewed increments. Track compatibility fixes as delivery
   gates, close completed PRs and retire only preserved exact branch heads.
2. **Prototype optional Cargo preparation on disposable Windows.** Prove the
   fixed NUL/exact-profile contract, trusted local broker identity, revocation and
   interrupted recovery before presenting a concrete administrator change on a
   user's PC. Keep project code and the model unelevated.
3. **Complete an authorized writable build environment.** Define truthful storage
   enforcement, owned-tree stop/quiescence, crash evidence and artifact
   publication. Input-copy and post-exit collection ceilings are not disk quotas.
4. **Add locally selected project and dependency workflows.** Support empty or
   existing workspaces with explicit enrollment, permissions and restore/egress
   policy. Prefer a goal-driven Node or Rust application test over another fixed
   UI exercise once the relevant gates pass.
5. **Close GUI compatibility separately.** Investigate Electron's normal startup
   failure without silently disabling its sandbox. Do not block all headless
   Node/Rust progress on that investigation.
6. **Productize the operator experience.** Add a visual permissions/project/log
   interface over the same validated services. Conversation resume uses retained
   project/diagnostic state; it cannot recreate a host's hidden model state.

Detailed design references:

- [Product charter](docs/product/PRODUCT_CHARTER.md)
- [Scope and long-term requirements](docs/product/SCOPE_AND_ROADMAP.md)
- [Security invariants](docs/security/SECURITY_INVARIANTS.md)
- [Repository workflow](docs/engineering/REPOSITORY_WORKFLOW.md)

The [generated alpha's delivery index](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/alpha/generated-project-editing/docs/product/REAL_PROJECT_DELIVERY.md)
contains its adapter-specific operational boundaries. Historical phase and
reusable-approval plans remain useful references, but do not override the
current evidence and acceptance order above.
