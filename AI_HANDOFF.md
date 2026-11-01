# AI Coding Handoff — Mandatory Guardrails

## Passation active — 1 novembre 2026

**Jalon vérifié avant cette PR documentaire :** `main=a22d32e94b742eaece9321d7991208ce89edd9cf`, arbre `7dd56c6a08f49866a250178af43e9a5b472628b9`, [push/main #41287712377](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41287712377) **SUCCESS 4/4 tentative 1**, Windows **433/0/11**, Ubuntu **318/0**, preuves schema8 `complete=true` **10 Windows / 4 Unix**, source=checkout exact et trois artefacts conservés. **La fusion de cette PR documentaire changera `main`; re-vérifier les SHA et la nouvelle CI push/main.**

**Portée :** [#460](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/460) à [#464](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/464) ont solidifié le registre d'appels **natifs inertes** (dépassement compteurs, arrêt interne, propagation vers watchdog Job original, mutex empoisonné observé, ticket `Drop` premier observateur). #464 : test-only [RED Windows #41287059165](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41287059165) 32/1 ; [HEAD CI #41287184189](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41287184189) 4/4 et [native #41287184201](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41287184201) 33/33 ; [squash/main #41287712377](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41287712377) 4/4. **Ce ne sont ni une activation de GitRead natif ni une clôture de la course CreateProcessW/ResumeThread.**

**Blocage V1 :** [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) **OPEN / Windows 10-11 V1 NO-GO** ; MCP GitRead utilise encore le legacy `HardenedCommandRunner`. Pas de révocation-linéarisable sur original session/TaskLease/policy/host et Win32 create/resume déjà en cours ou futurs ; pas de vraie preuve root exit + Job-empty + EOF physiques stdout/stderr + no future spawn. Recherche [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) **négative**, PR #327/#353/#355/#358 non fusionnées ; issues #176/#182/#185/#319 toujours ouvertes. Aucun GO, release, activation, self-hosted CI, modification sécurité, PC opérateur, Desktop ou REL.

**Reprise obligatoire :** [passation canonique 2026-11-01](docs/engineering/HANDOFF_WINDOWS_V1_2026-11-01.md) et [prompt du prochain chat](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-11-01.md). Une PR complète à la fois : test-only behavioral RED quand faisable, correction minimale, CI final HEAD Windows/Ubuntu & workflows spécialisés, schema8/artefacts, revue COMMENT, squash exact, arbres identiques, nouvelle CI `push/main` 4/4, reçus PR + #323.

---


## Historical / superseded checkpoint — Active handoff after documentation PR #458 — 31 October 2026

**Verified GitHub checkpoint before this additional documentation PR:** `main=a9b6de0467fe2cdb0d9a74276e0ea0ec1fc40b8a` (Git tree `6d4988c300740f3cc0aefec285b9284a83169366`). [PR #458](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/458) is squash-merged: eight README/status/handoff/reliability Markdown files only; final PR-head CI [#41228285220](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41228285220) **SUCCESS 4/4** and identical tested/merged trees. Independent [push/main CI #41228767639](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41228767639) **SUCCESS 4/4, attempt 1, exact integrated source and checkout SHA**, hosted Windows **428 passed / 0 failed / 11 ignored**, Ubuntu **318 passed**, physical schema8 `complete=true` **10 Windows + 4 Unix** cases/characterizations, original evidence artifacts retained. This SHA is a dated **pre-this-PR checkpoint**; always requery `main` and CI after any documentation merge. See [PR #458 receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/458#issuecomment-6547380732) and [#323 receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6547381263).

**Technical state:** #455 makes the *live legacy* runner deny already-expired original deadlines **before spawn**, without interrupting blocked OS calls or atomically fencing revoke. #456 stops **future** output-reader reads once the shared ceiling is crossed, but an already blocked peer, `wait()`, `join()` or inherited stdin can still hold the operation. #457 fails closed on a poisoned ledger in the **inert** Windows-native call tracker; this is not an issuer or Git process launch. Native GitRead/Job/watchdog are still unconnected to real MCP `GitReadService::status()`. The verified next P0 is a service-owned, original-generation revocation-linearizable authorization/guardian for session, TaskLease, policy epoch, host stop, pinned repo/exe, **in-flight and future** Win32 CreateProcessW/ResumeThread, followed by actual root exit + kernel Job empty + both physical stdout/stderr EOF + no future spawn + crash/unknown custody reconciliation. Research [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) preserves a real **Git spawned after revoke** negative and must not be treated as passed; #327/#353/#355/#358 remain unmerged. Issue [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN; #176/#182 sensitive read authority, #185 Node/AppContainer and #319 first MCP response also OPEN. **Windows V1 NO-GO.** No GitRead native activation, release/tag, policy/security relaxation, user-PC commands, self-hosted runner, Desktop or REL changes.

**Continuation method:** verify *live* SHA, branches, issues, exact run event and logs before coding. One bounded PR **all the way through**: scoped deterministic regression and observed RED when feasible (preserve early format/compiler/cancelled runs as **not** behavioral RED), minimal fix, commit/push, exact final-head Windows+Ubuntu hosted CI and applicable specialized CI, real physical `schema8` source/checkout/run/attempt, full diff and authority review (self-review only **COMMENT**, never independent peer approval), squash with unchanged expected HEAD/base, identical HEAD and squash Git **tree** IDs, independent post-squash `push/main` **4/4** on exact SHA, and durable receipts in PR + #323 before starting another PR. Do not rerun-to-green, inflate timeouts or weaken assertions, coverage or sandbox policy.

---

## Historical handoff after PR #457 (superseded)

`main=291c152cdd132753668563031cf34bcfe0810940` **before the current documentation PR**, with independent [push/main #41226531396](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41226531396) **SUCCESS 4/4, first attempt** on exact integrated SHA and checkout. Windows runtime **428 passed/0 failed/11 ignored**. Schema8 physical reports `complete=true`, **10 Windows/4 Unix** scenarios/characterizations, SHA/run/attempt concordant and artifacts saved. This is a historical verified checkpoint, not an immutable current branch: always requery GitHub after this docs PR.

| PR | Final verified scope | Squash main | Independent push/main |
| --- | --- | --- | --- |
| [#454](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/454) | Docs after #453; not runtime acceptance | `50607a1b72373c6058a46c74968f80fc6165335d` | [#41197559450](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41197559450) 4/4 |
| [#455](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/455) | Live `HardenedCommandRunner`: reject known expired original deadline **before spawn** | `0bdf3621afdccb812d2165021a4788128b4f989f` | [#41202989935](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41202989935) 4/4 |
| [#456](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/456) | Live stdout/stderr reader: stop **new** reads after shared output ceiling | `ef403d14890dc15127431560ebda1fdfb7cdc8d0` | [#41224958029](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41224958029) 4/4 |
| [#457](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/457) | Inert native call tracker: poisoned mutex implies stop-required, counter remains unknown | `291c152cdd132753668563031cf34bcfe0810940` | [#41226531396](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41226531396) 4/4 |

**Not GO:** production `GitReadService::status()` continues using `HardenedCommandRunner`. Known expired deadlines now fail before spawn, but no atomic create/revoke fence or interruption of blocked `spawn`, `wait` or pipe `.join()`. Post-overflow read-stop does not cancel another reader already blocked on an inherited handle. The native GitRead owner/Job/watchdog/stop-tracker remains **non-executing**, does not issue native launch authority, and poison-as-stop is only unknown-accounting fail-closed. A durable service guardian needs joint original session/TaskLease generation/policy epoch/host/pinned-target *future and in-flight* CreateProcessW/ResumeThread synchronization and independent proof of root exit, kernel Job-empty, stdout and stderr EOF, no future spawn, crash/revocation uncertainty and retained quotas. Research [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) demonstrates negative **real Git spawn after revoke**; #327/#353/#355/#358 remain unmerged. #323 OPEN / Windows V1 **NO-GO**; #319/#185/#176/#182 distinct. No tag, release, native switch, security changes, self-hosted runner or user-PC commands without new authorization.

**Workflow:** live main/PR/issue/CI audit; one bounded PR; deterministic test-first RED when feasible, preserve misfires; exact-final-head Windows+Ubuntu and applicable special CI, real logs and physical reports; self-review is not independent peer approval; squash only if green; match tested and integrated Git trees; separate **push/main 4/4** on exact squash and receipt PR/#323. Update stale documentation but never claim docs prove release acceptance.

---

## Historical handoff after PR #453 (superseded)

GitHub-verified pre-documentation `main=bfcd9dcba0f73bf6d8aa4d148fc5c654c819edf6`;
independent [push/main #41195919444](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195919444)
SUCCESS 4/4 attempt1. [PR #453](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/453)
is merged after earlier #450/#451/#452; all exact-head and post-squash CIs
verified with identical trees. New [canonical handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-31.md)
and [successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-31.md)
**supersede** earlier after-#448 and after-#412 'latest' labels below.

Actual live command runner timeout now starts before process spawn and pipe
preparation, but its descendant-held `.join()` and other native OS stalls
are still **potentially unbounded**. Inert Git status owner/Job/watchdog
and stop trackers **do not execute Git or grant launch/revoke permission**.
No cross-authority session/TaskLease/policy/host CreateProcessW/ResumeThread
linearization or root/Job/both-EOF proof. #323 OPEN, Windows V1 NO-GO;
#317 administratively closed, #355 negative research, #185/#319/#176/#182 open.

Apply single-PR GitHub-only workflow: source verification, regression,
commit/push, exact-head Windows/Ubuntu and specialized CI, self-review
labeled as such, squash if green, Git-tree equivalence, separate push/main
CI on squash and receipt in PR/#323. Preserve red runs. No PC access
without fresh explicit authorization; no self-hosted CI, release, security
relaxation, Desktop/REL changes. Requery current GitHub state after this
docs PR, not a dated SHA.

---


## CURRENT PRIORITY — 31 October 2026 (supersedes dated continuation links)

Read [canonical 31 Oct handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-31.md) and [copy-ready GitHub-only prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-31.md). Pre-doc baseline `main=7f1de707c130d61f8a6878728ec2ba8bd6ac23a8`, [independent main CI #41188920407](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188920407) SUCCESS4/4 attempt1; #445–#448 squash-integrated. Native GitRead is **NOT live**; P0 synchronized create/resume/revoke ownership, physical Job/root/pipe EOF and legacy joins still block V1. #317 administratively CLOSED, #323/#185/#319/#176/#182 OPEN, Windows V1 NO-GO. One **complete** targeted PR at a time through exact-head checks, squash and independent main CI. No standing authorization for local operator-PC work: earlier task-specific permissions are NOT transferable. Historical sections below are not the latest `main`.

---


## REPRISE ACTIVE — CHAT CLASSIQUE — 27 octobre 2026

Lire la [passation Windows V1 pour Chat classique](docs/engineering/HANDOFF_WINDOWS_V1_CHAT_2026-10-27.md)
et le [prompt complet à copier](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md).
Reprise demandée dans un **Chat classique, GitHub uniquement** ; ne pas créer de
tâche Codex/Work ou accéder au PC sans demande explicite. Les entrées datées
ci-dessous sont historiques pour leurs références ; les invariants restent
obligatoires.

Avant cette PR documentaire : `main=75947c9b4fb72fab2f3d45f34f5b750f09bc31b2`,
[CI indépendante #40477277322](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40477277322)
SUCCESS 4/4, tentative 1. #380–#383 sont intégrées, mais GitRead utilise encore
le runner legacy, `task_lease: None` et aucun propriétaire Git natif actif.
Vérifier le nouveau main et sa CI push après fusion de cette documentation.
#352/#355 ROUGES ; #317/#323/#185/#319 ouvertes ; Windows V1 NO-GO.
Aucun accès PC, Optic Desktop ou REL.

---

**CURRENT 25 OCTOBER 2026: Read [new canonical Windows V1 handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-25.md) and [complete new-chat prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-25.md) before older snapshots.** Verified pre-doc baseline `main=7c72ecfb8886fb34be2cc7ef9f2e1070f73c08da` has independent [CI #40188090038](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40188090038) 4/4 success at attempt 1; the docs merge may advance main. #358 research `f431c3ef…` is stacked on #353 and NOT in production; #352/#355 behavior RED; #317/#323 OPEN; Windows V1 NO-GO. Preserve all security invariants, first red evidence, recovery checks and no operator-PC access without explicit request.


**Latest 25 October 2026:** [canonical status](STATUS.md), [Windows handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) and [successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md) take precedence. `main=d7d4d50…` [CI #40173338835](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40173338835) green, #356/#357 typed GitRead spec merged; native GitRead still research. Open PRs #327/#352/#353/#355/#358, #352 elapsed-pipe and #355 revoke-before-spawn acceptance RED; #353/#358 native owner/lease green in research. Issues #317/#323 OPEN; V1 NO-GO. No standing permission to access operator computer or bypass safety controls.



**25 Oct 2026 checkpoint:** see [newest `STATUS.md`](STATUS.md), [current Windows technical handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md), and [complete successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md). Integrated main `6a645715…` green at [#40086602634](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40086602634), open #327/#352/#353; #352 actual wall-clock pipeline acceptance RED, #353 native owner research GREEN but not wired to GitRead. #317/#323 OPEN, V1 NO-GO; refresh GitHub before action. Codex recovery does not authorize device access or safety bypass.



**Latest session-entry order (24 Oct):** start with [AGENTS.md](AGENTS.md), [Codex interruption recovery procedure](docs/engineering/CODEX_INTERRUPTION_RECOVERY.md) when local checkpoints are actually accessible, [current Windows V1 handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md), and [full successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-24.md). GitHub-only environments must not claim local checkpoint verification. This is a documentation/recovery practice, not new authorization to access the user's PC or bypass safety controls; #317/#323 V1 NO-GO remains in force.

Read this file before changing architecture or implementing features.

For a fresh chat, first read [AGENTS.md](AGENTS.md) and the
[GitHub-only handoff](docs/engineering/GITHUB_HANDOFF.md). The current operator
direction excludes tests/runtime changes on their PC; use an agent-owned isolated
environment or GitHub-hosted CI. [Reusable prompt](PROJECT_CONTINUITY_PROMPT.md).

## Non-negotiable invariant

**The AI decides what it wants to attempt. The bridge executes only authorized effects. Policy authorizes. Isolation contains.**

## MUST

- Preserve transport/core separation and the TransportGuard.
- Treat MCP 2026-07-28 transport as stateless; use Optic-owned application SessionHandles for stateful resources.
- Route every executable effect through a typed ActionEnvelope carrying one typed `Effect` variant.
- Derive target/precondition/reversibility semantics from the `Effect`; do not reintroduce independently combinable action/target fields.
- Enforce session/task ownership on every resource.
- Keep capabilities narrow, expiring and revocable.
- Keep task leases explicitly scoped to the workspace/repository/executable/network boundary they authorize.
- Evaluate active authorization TTLs/deadlines using a monotonic time source at runtime.
- Require both session and task-lease NetworkAccess plus explicit network scope for leased process egress.
- Bound every long-lived collection, protocol body, queue, output and spool.
- Use explicit expected state/hash/version preconditions for mutations; create intent must explicitly require absence.
- Keep same-repo write sessions in separate worktrees.
- Treat repository/log/network content as untrusted sources.
- Gate sensitive sinks (network, secrets, external writes, policy/privilege) deterministically.
- Test Windows process cleanup/security boundaries natively.
- Add/update an ADR when an architectural/security invariant changes.

## MUST NOT

- Add Electron or require Node/Python/Docker for core.
- Embed a required LLM.
- Let AI grant/approve permissions or use AI risk scores as authorization.
- Trust a client-supplied session ID merely because it is syntactically valid.
- Expose arbitrary PID kill.
- Use opaque shell strings as the fundamental process API.
- Allow unbounded request bodies, stdout/stderr, histories, queues, caches or responses.
- Call Job Objects a full sandbox.
- Silently overwrite stale files or interpret a missing mutation precondition as overwrite permission.
- Authorize a path by naive string-prefix comparison.
- Share mutable cwd/environment across sessions.
- Give child processes ambient secrets/network without explicit capability and scope.
- Let normal tools mutate security policy.
- Couple core execution to one ChatGPT/OpenAI transport.
- Assume upstream SDK limits are sufficient.
- Import OpticCode Java/RAG/editor layers for convenience.
- Expand scope without Product Charter/ADR updates.
- Implement research candidates early just because they sound useful; add them at the service boundary that can prove their value.

## Before coding

Identify canonical doc, security invariants touched, Effect/ActionEnvelope shape, session/task ownership, lease scopes, resource ceilings, cancellation/cleanup, crash recovery, tests and ADR requirement.

## Definition of done

Formatting/lint/tests pass; security regression tests exist; no unbounded state was introduced; docs/contracts match code; Windows-specific behavior is tested on Windows; STATUS/CHANGELOG are accurate; benchmark claims remain TARGET until measured.
