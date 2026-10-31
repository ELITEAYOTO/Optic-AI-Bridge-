# AI Coding Handoff — Mandatory Guardrails

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
