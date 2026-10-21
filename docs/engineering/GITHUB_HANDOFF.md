# GitHub-only project handoff

## Approved Policy B — first narrow correction candidate (2026-10-21)

Live baseline before branching: `main=3d9a0f78304578af53fd23b9fed7d08603348e0b` after merged #307, zero open PRs, independent push CI #39542505029 green on first attempt (Windows native/Linux/dependencies). The operator **approved Policy B**: a shared deterministic sensitive classifier for FileRead, FileSearch and GitRead, rejecting unverifiable outputs. The first scoped branch `security/sensitive-file-authority-scope` adds a shared pure core lexical classifier, file-read dual-scope enforcement and `fs_list` sensitive-name page denial with targeted unit/MCP regressions; acceptance depends on exact-head hosted CI and post-merge main CI. See [ADR-0034](../decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md). Do not assume this candidate is merged: inspect GitHub live.

Remaining critical gap: the classifier is a path-name **floor**, not exhaustive content-secret detection, and **GitRead currently remains unfiltered**. #176 and #182 stay open; real-workspace remote/autonomous exposure is prohibited. The next individually demonstrated change should target complete Git status/diff output sensitivity, fail closed on unknown records, preserve existing Git containment and ordinary behavior where proven, with Windows/Linux fixture coverage. No speculative permission broadening or feature development. Other open Windows, recovery/race, soak and code-audit debts are unchanged.

## Current successor handoff — after merged #306 (2026-10-21)

**Verified live base before this handoff update:** `main=dadaf48539788235b689dd4e09adcd2fb94904f3`, zero open PRs, [docs-only PR #306](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/306) squash-merged. Its [source CI #39540522392](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540522392) and independent [main CI #39540940136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540940136) **both succeeded first attempt on their exact SHAs** (Windows Rust, Linux Rust and dependency policy). A new handoff-only PR may have changed main after this snapshot: **requery live GitHub**, do not reset to this SHA.

The operator wants the **existing repo stabilized**, not new features. One PR at a time, scoped correction + regression + docs, no artificial PR. Never use the operator PC or self-hosted runners. No Electron GUI, generalized Cargo, production remote MCP, alpha-wide merge, release, permission broadening or disabled protection. Follow `AGENTS.md`, `AI_HANDOFF.md`, `STATUS.md`, `ROADMAP.md`, `RELIABILITY_DEBT.md` and [the full reusable prompt](../../PROJECT_CONTINUITY_PROMPT.md) for instructions and invariants.

**Immediate next investigation:** read-only inspection of the current sensitive-read authority composition, [issue #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) and [issue #182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182), across `ReadSensitiveScope`, default `ReadAuthoritySpec::workspace_all_non_sensitive()`, `fs_read`/`fs_list`, `GitRead`, `git_status`, `git_diff`, `git_log`, canonical path resolution, policy and lease checks. **Do not assert a functioning sensitive-file classifier:** it has not been implemented/proved, and Git status/diff can independently reveal sensitive material. Decide a deterministic cross-capability contract *before* code changes or real workspace remote exposure; a trust-model choice may require the operator. The existing canonical-`fs_list` and `.git` mutation fixes are already in main (#303/#304).

**Next outstanding reliability gates:** natural native Windows Node/AppContainer 10s startup intermittent [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) with original red evidence, separate MCP ~30s first response, non-atomic multi-delete recovery after a real delete fails, directory ancestor races and Windows crash/power-loss persistence, incomplete 1h/4h fault-soak and file-level audit. Existing #180/#181/#183 issues remain open despite prior targeted safeguards—reconcile acceptance evidence before creating duplicates or closing issues.

**Already integrated:** #293–#297, #299, #301–#306 (source/merge and CI evidence in STATUS); particularly #301 preserve recovery evidence, #303 canonical listing policy, #304 Git control namespace, #305 cursor-aware Git-log response, #306 documentation reconciliation. Green first-pass CI is **not** a certificate of production correctness. `main` was unprotected and the ruleset listing returned none at last observation; owner approval is needed before changing settings. Treat historical sections below as dated accounts, not active tasks.


## Latest verified handoff — 2026-10-21 after merged PR #305

**Previously verified live `main`:** `43d3a4aeedee04dd03e895e27e81d3bc5af6652c`; **zero open PRs**. Independent [CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed first attempt **on the exact merge SHA**, hosted Windows Rust, Linux Rust, dependencies. Requery GitHub before acting: this is a timestamped checkpoint, never a reset target.

**Do not redo merged increments.** [#301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) preserved unpublished recovery evidence (source Windows first attempt exposed the separate known timeout; independent main CI passed). [#302](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/302) delivered earlier handoff docs. [#303](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/303) authorized canonical `fs_list` directory scopes. [#304](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/304) protected canonical `.git` control namespace against generic mutations. [#305](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/305) refit bounded `git_log` output after insertion of a continuation cursor. All five were merged; their independent main CI results are linked in [STATUS.md](../../STATUS.md).

**Actual remaining stabilization investigations:** (1) natural Windows AppContainer/Node 10s no-output failure [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), *not fixed by opt-in timings*; (2) distinct 30s first MCP response timeout; (3) non-atomic recovery cleanup, ancestor races and unproven power-loss persistence; (4) sensitive read classification [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) plus GitRead content/status sensitivity [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182). These last two need a written policy/product decision **before remote access to a real workspace**, not an improvised broad grant. Cargo and sandboxed Electron remain deferred, not production support. Issues #180/#181/#183 are administratively open although code/tests contain scoped fixes; reconcile criteria with evidence before closure, and avoid speculative duplicates.

**Completion discipline:** preserve one PR at a time (fix + focused regression + docs), exact-head native Windows tests for Windows claims, the first failed CI attempts, review and post-squash independent main CI. The [quality audit](CODE_QUALITY_REVIEW.md) is partial. No user-PC commands, self-hosted runners, releases, new features, permission expansion or whole-alpha merge.

**Governance observation:** GitHub branch lookup indicated `main.protected=false` and ruleset listing returned no rules at the above checkpoint. This is *not* authorization to change branch protections. Operator approval is needed for any protection-policy change; continue enforcing exact-head CI and review in the meantime.

Below are **historical checkpoints**. Their “active PR”, “candidate”, and old main SHAs describe *that date* only.


## Focused Git-log response-budget regression

Source review of `ReadonlyMcpServer::fit_git_log_response` identified that adding a continuation cursor **after** clipping an end-of-history page could grow serialized JSON beyond `max_response_bytes`. The scoped correction refits the **final** entries+cursor combination and rejects a page if no entry plus cursor can fit. It also checks offset addition instead of using saturating addition. Regressions cover insufficient budget, sufficient budget with exactly one advancing entry, and a complete page that stays cursor-free.

The correction belongs to a single GitHub PR with docs and tests; live GitHub source SHA and CI **must** be rechecked before merge. No new commands/capabilities, policy changes, timeout increases, MCP transport additions or operator PC access. The active Linux/Windows test results and the post-merge `main` verification, once known, are authoritative in the PR/Actions history. Keep Windows Node/AppContainer and MCP first-response timeout investigations separate, and preserve their evidence.


## Active single-PR stabilization: reserve .git metadata (2026-10-21)

At the start of this increment GitHub `main=9c6c91096590f9f005dabc51253b9b5f8b1abcca`, zero PRs open and [independent #303 post-merge CI #39534614408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39534614408) passed on attempt 1. [Issue #180](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/180) documents an actual separation-of-authorities defect: generic FileWrite/FileDelete could target Git-owned `.git` data without GitIntegrate authority.

The focused branch implements an explicit reserved Git-control namespace at transactional **canonical** mutation preparation and directory creation, rejecting any exact case-insensitive `.git` segment while leaving `.gitignore` etc. usable. Tests cover existing/absent `.git` file content, linked worktree pointer `.git` file, alias into Git metadata (Unix), Windows authenticated WorkspaceAll write/delete and directory denial, and stable MCP invalid-request error. Do not treat the branch as merged until exact-head native Windows/Linux and policy CI confirms it; integrate documentation **in the same PR**; afterward check independent main CI on the squash SHA. Keep first failed attempts.

Scope limits: no expanded Git integration ability, no sensitive-read classifier, no remote transport, no general Cargo or Electron. Concurrent directory replacement races, alternate hardlinks and power-loss persistence are not globally solved. Continue one PR at a time, preserve the Windows Node/AppContainer #185 evidence and use hosted GitHub runners only.


## Security stabilization candidate: canonical fs_list scope — 2026-10-21

Source audit identified a **real** `fs_list` authority mismatch: an alias path inside an allowed `SearchMetadataScope::Prefix` could resolve to a different workspace directory, because policy had used the lexical alias while the bounded filesystem resolved the actual target only after authorization. The scoped candidate adds `BoundedFileSystem::canonical_list_root`, evaluates `Effect::FileSearch` on that canonical directory, and enumerates/returns canonical-path entries. Regressions cover a Unix in-workspace symlink out of a permitted prefix, whole-workspace canonical listing, and an ordinary directory on native Windows/Linux.

This is **not** a fix for simultaneous directory swaps (there is no lifetime-pinned directory handle), unclassified sensitive-file reads, or remote transport safety. [Security audit #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) retains those independent requirements. Verify the actual PR, exact source head, all CI results and independent `main` status live before claiming integration. Use one PR at a time; no operator-PC execution or runner self-hosting, no GUI/Cargo features. The earlier #301/#302 integration and preserved Windows #185 first-failure evidence remain canonical.


## Most recent recovery correction — 2026-10-21, after PR #301

**Live at post-merge inspection:** `main=05b5a928c862601926e8b0cc11540ce020caf271`, zero open PRs. [PR #301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) squash-merged from source `089e64a2b6c95dc0705eeaaa22f90194dfb7be66`. [Source CI #39529419994](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529419994) passed **on attempt 2** after an unrelated but significant initial native Windows Node/AppContainer timeout; [native preparation #39529420240](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529420240) passed first attempt. The **exact same new recovery regression** passed on Linux and on the Windows second attempt. [Independent post-merge main run #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) **passed first attempt** on exact squash SHA `05b5a928c862601926e8b0cc11540ce020caf271`, Windows Rust, Linux Rust and dependency policy.

**Concrete source bug corrected:** `MutationRecoveryJournal::discard_unpublished` formerly deleted sorted `*.prepared.tmp` files one at a time. A valid first file disappeared before a later dangerous hard-link alias failed the deletion check. Now the whole list is preflighted via existing identity/link/regular-file checks before any delete; a second pass reopens/revalidates each handle immediately before deleting. A cross-platform negative regression preserves earlier temporary bytes, aliased bytes, and the external hard-linked evidence after rejection, then repairs and confirms successful cleanup. These are scoped preflight-order guarantees, **not** transactional multi-file atomicity or race/durability proof.

**New real Windows failure evidence from the #301 source CI first attempt:** parent outer-helper launch returned in 9ms, then 10s `TimedOut` without captured stderr or Node marker. On the same SHA, diagnostic #1 reached inner `target_resumed=14ms` but only `target_exited=8952ms`, normal network-denied exit 42. Two later diagnostics exited ~360ms from helper entry. See [issue #185 diagnostic entry](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185#issuecomment-6306039365). No cause established; do not infer loader/scheduling fault as fact, weaken the 10s bound, or treat retries as a fix. MCP first-response 30s issue is distinct.

**Next allowed step:** recheck live branch, PRs and exact-head post-merge CI, then investigate the next *demonstrable* isolation/recovery/security defect. Do not repeat #299/#301 or create speculative PRs. Stabilization only, GitHub-hosted Windows for Windows claims, no operator PC, self-hosted runners, release, new features or alpha-wide merge. Historical checkpoints below are not reset targets.



## Handoff verified through PR #299 — 2026-10-21

The first GitHub inspection after [PR #299](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/299) squash merge found `main=6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea`, **zero open PRs**. The accepted source `aca382535f24b6f6b1c26c2313302d315099ed4e` passed [CI #39526324369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324369) on its **first attempt** (native Windows Rust, Linux Rust, dependency policy) and native-preparation [#39526324493](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324493) on attempt 1. **Independent main CI:** [#39526677098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526677098) **passed on the first attempt** on exact merge SHA `6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea` (hosted Windows Rust, Linux Rust, dependency policy). Do not reset to the old #297/#298 checkpoint.

**Delivered:** exact-value opt-in `OPTIC_LAUNCHER_PHASES=1` allows fixed-label, monotonic stderr timestamps inside the outer Windows isolation launcher without printing paths, arguments, SID, handles or credentials; the existing native test and diagnostic-on-failure CI step enable the flag, no new runner workflow. Observed in a successful Node/AppContainer loopback denial proof: relative to launcher entry, `entered=0ms`, `request_read=0ms`, `profile_created=10ms`, `target_created_suspended=14ms`, `token_verified=14ms`, `target_resumed=14ms`, `target_exited=361ms`. Node emitted `NODE_PROBE_STARTED` and `NETWORK_ATTEMPT`, normal denied `exit=42`, no accepted connection. Parent `launch_returned_ms=13`, `terminal_observed_ms=407` uses a separate clock origin. An earlier failure after only 14-17ms *outer* launch has no proof of inner `CreateProcessW`/resume completion: do not equate launch return with Node execution.

**Still open:** native Windows Node/AppContainer intermittent 10s no-child-output failure [issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185); separate MCP TEMP-only 30s first response; recovery multi-delete non-atomicity, ancestor races and Windows power-loss durability. No root cause/fix claimed from a passing instrumented run. The next useful step is to correlate a naturally occurring failed Windows diagnostic with the last captured **launcher** phase, always preserving initial red failures. If the first marker is absent, investigate outer helper spawn/stdio/entry; if only `entered`, request delivery; if `profile_created` absent, profile setup; if `target_resumed` present but no Node marker, investigate loader/init/forwarding. A shared stderr marker is observational data, not an unforgeable attestation. Avoid synthetic CI loops without a hypothesis. Do not weaken timeout, network denial, native sandbox, or required tests.

**Operator direction unchanged:** stabilization only. No Electron GUI, general Cargo, alpha merge, refactor for appearance, operator PC execution, self-hosted runners, release, privileged/permission change, or evidence deletion. Use GitHub-hosted Windows for Windows claims, preserve exact-source and independent post-merge evidence, squash only after applicable checks/reviews. Reconfirm GitHub live state and read `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`, `ROADMAP.md`, this handoff, `CONTRIBUTING.md`, `REPOSITORY_WORKFLOW.md`, `RELIABILITY_DEBT.md`, `CODE_QUALITY_REVIEW.md`, `PROJECT_CONTINUITY_PROMPT.md` before selecting work.


## Canonical agent takeover — verified 2026-10-21, after PR #297

**Last independently observed GitHub state before this documentation PR**: `main` = `83329d48789c20b1fc235d3096c3199dc9dc5176`, **0 open PRs**. [#297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) is merged by squash. Independent [post-merge CI #39514832814](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514832814) **passed on attempt 1 on that exact SHA**, covering hosted native Windows Rust, Linux Rust and dependency policy. The accepted #297 source head was `30ed0d6c6d28df18351a7165dcdd63d2616cf7f7`; [PR CI #39514352067](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514352067) and [native preparation #39514352140](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514352140) passed on it first attempt. Windows and Linux logs explicitly record both new regressions passing. An earlier draft head failed *rustfmt*; the failure remains in Actions history. Do not treat the branch names or candidate paragraphs below as open work.

**Integrated, not merely proposed:** [#293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/293) (`de973989`, journal/staging cross-phase evidence retention); [#294](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/294) (`da3ab6f1`, bounded successful Windows MCP child startup phase reporting); [#295](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/295) (`615e6219`, bounded Git/session/read/process-lease timing); [#296](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/296) (`72966f92`, opt-in path/pin/read+hash identity capture timing); [#297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) (`83329d48`, preflight duplicate and capacity checks before expensive identity capture **plus race-safe revalidation under insertion lock**). Source and independent main CI are successful for all; **#294 post-merge needed attempt 2** after repeatable first-attempt Windows 10s Node/AppContainer failure. Preserve that failed attempt and serial diagnostic failures; a passing rerun does not establish a fix.

**Concrete measurement:** On a passing native Windows #296 source CI, capture consumed `path_ms=0 pin_ms=0 hash_ms=4869 total_ms=4869`, with process-leases provisioned ~4871ms after read authorities. Here `hash_ms` includes reading 64KiB chunks and BLAKE3; it does **not** isolate algorithm CPU cost from filesystem/disk/cold-cache/host effects. No hash cache, bypass or faster production startup has been delivered. #297 eliminates this expense only for duplicate/exhausted **rejected** lease registrations; valid ones still require pinned identity.

**Remaining verified/unresolved boundaries:** (1) native Windows Node/AppContainer loopback gate sporadically times out at ~10s, sometimes with no `NODE_PROBE_STARTED` marker despite rapid parent-side start return; (2) separate MCP first-response 30s intermittency; (3) recovery cleanup can partially remove multiple files if a later deletion fails, and ancestor namespace races/Windows directory power-loss guarantees are not proven; (4) ordinary Cargo and normal Electron sandbox acceptance remain separate incomplete research gates; production credentials/auth are not established. Do not infer production readiness from CI green.

**Current operator authority**: *stabilization-only feature freeze*. No optional GUI/Electron/Cargo product/new integration, architecture expansion, global refactor or whole-alpha merge. No operator-PC build/test/install/tunnel/permission change, no self-hosted CI, no release or mass deletion. Core Rust independent of Electron/Node/Python. Security rule: AI proposes, deterministic policy authorizes, native isolation contains. Never skip executable pin/identity check; preserve task/session ownership, expiring revocable grants, monotonic deadlines, bounded queues/logs/resources, typed effects/ActionEnvelope, exact filesystem preconditions, recoverable evidence and default-deny network.

**Resume procedure**:
1. Read `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`, `ROADMAP.md`, this file, `CONTRIBUTING.md`, `REPOSITORY_WORKFLOW.md`, `RELIABILITY_DEBT.md`, `CODE_QUALITY_REVIEW.md` and `PROJECT_CONTINUITY_PROMPT.md`.
2. Use the GitHub connector **live**: verify default branch SHA, PRs, reviews, branch protections, latest and first-attempt CI. Dated notes are not a live state or a reset instruction.
3. Select **one demonstrated** reliability/safety defect in the existing code. Good candidates: distinct Node/AppContainer and MCP intermittent diagnostics, bounded recovery identity/persistence failure, or an actual uncovered registry/resource bug. A measurement alone is not a bug. Do not run CI unnecessarily.
4. If a fix is justified, choose a short branch, minimal diff, meaningful negative regression, hosted Windows for Windows-specific guarantees, exact-source checks and reviews. Merge squash only after gates; independently verify post-merge main. Preserve failures, and checkpoint reproducible evidence in GitHub.
5. If no defect is shown, record the scoped no-finding without an artificial PR. End the session with exact SHAs, CI attempts, open limitations and a next actionable step.

**Historical material below** records observations made *before* their later merges. It is archived context, not an active candidate queue; [STATUS.md](../../STATUS.md) and GitHub live state outrank any stale heading.

## Historical handoff checkpoints — dated evidence, not active PR candidates

## Canonical current handoff — verified after PR #291 (2026-10-20)

GitHub `main` was independently confirmed at `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`, with **zero open PRs**. [Post-merge CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) completed successfully on that **exact SHA**, first attempt: hosted Windows Rust, Linux Rust and dependency policy. This supersedes candidate/pending language in the dated history below.

- [PR #290](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/290): merged as `0c2a125537d020e3a3aff4124d12745d7a8c5c4c` from `f7b4207395b495f5a70aaa87d441fb075e991b94`. Hosted [source CI #39392262806](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392262806) and independent [main CI #39392829136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392829136) passed. Windows Node/AppContainer test-only monotonic parent timing and `NODE_PROBE_STARTED` marker are integrated, **not a causal fix**. Historical first-attempt 10-second timeout with no output and the separate 30-second MCP timeout both remain unresolved.
- [PR #291](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/291): merged as `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02` from `cdb8407bd22bb91babd9fbe3386da10443d458c1`. Hosted [source CI #39393849343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849343), [native preparation #39393849287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849287) and independent [main CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) all passed. Windows `JournaledMutationService::cleanup_recovered_staging` now preflights all staged artifacts before any deletion and verifies each again through its exclusive native handle before delete. Two-journal Windows regression proves earlier valid staging and both journals remain when later staged bytes mismatch; repair allows normal cleanup. Multi-file deletion **remains non-atomic** if a later removal fails; ancestor namespace races and power-loss durability are still open.
- No feature expansion, new scopes, changed timeouts, disabled tests, local-PC execution, self-hosted runners, releases, archive deletion or whole-alpha integration. The connector has no individual merged-branch deletion action; no mass cleanup attempted.

**Next allowed step:** after reconfirming live `main` and CI, inspect one *demonstrable* remaining reliability/security defect or correlate a naturally occurring Windows Node/MCP failure using existing phase diagnostics. Fix only with a negative regression and exact-head hosted native checks. If no defect is proven, record the finding and stop. Read older handoff entries below as historical observations even where they once said “candidate”.

## Current operator handoff — stabilization freeze (2026-10-20)

**Temporary delivery policy (operator decision, 2026-10-20): stabilization only.** Pause new features, product expansion, experimental merges, architectural reshuffles and broad refactors. Work exclusively on verified defects, security and recovery boundaries, flaky-test root causes, focused regressions, and accurate status/debt documents. Do not create code changes merely to keep PRs moving; when no defect can be demonstrated, record the evidence and stop. Preserve all existing permissions, isolation, deadlines, budgets and CI gates. Resuming feature development requires fresh operator direction.

Live GitHub `main` at this checkpoint: `b1edc83438571867bd164ea0b2b9653304eed752`. [PR #288](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/288) was squash-merged at `b1edc83438571867bd164ea0b2b9653304eed752` from source `60206dcadbf2efb57800ca93f0bd226acf16f6fb`. Its hosted [CI #39388280621](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280621) passed Windows Rust, Linux Rust and dependency policy; [native preparation #39388280521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280521) passed too. Independent [post-merge CI #39388749406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388749406) succeeded on the same merge SHA, first attempt (Windows, Linux, dependencies). GitHub showed zero open PRs at verification. No open PRs at this checkpoint.

#288 completes the bounded first-phase recovery-scan ordering fix: stable records are classified before unpublished temp cleanup, tested on hosted Windows and Linux; it is **not a multi-file atomic-delete guarantee**. Do not repeat #277–#288. The checked-in old headings labelled “candidate” are historical checkpoints, not live status or unfinished assignments.

**Next permitted work:** examine one existing defect or the recorded Windows Node/AppContainer / MCP first-response intermittencies. Keep an evidence-first threshold: identify a reproducible failure (or bounded log-based evidence), define a negative regression, correct the smallest existing boundary, and run the exact-head hosted Windows/Linux checks. If the cause is unknown, retain debt and avoid speculative fixes, timeout increases, disabled checks or new features. No PC actions, self-hosted runners, releases, archive/evidence deletion, massive branch cleanup, or whole-alpha integration.

## Historical handoff — #288 proposal, already integrated

The previously pending `fix/recovery-inspect-preserve-temp-on-failure` branch was delivered by merged PR #288; source, merge SHA and post-merge CI are recorded above. Remaining cleanup atomicity and directory-identity limitations are still open.

## Integrated Linux directory sync — PR #287 (2026-10-20)

[PR #287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/287) was squash-merged as `0ca2305e7f01d5263330b67c4a0bb0520d452c8a` from final head `66161c9dee9e1a565b074d1cf105cf8961a712c2`. Hosted [PR CI #39386566516](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566516) and [native preparation #39386566570](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566570) passed; independent [main CI #39387133506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39387133506) passed Windows Rust, Linux Rust and dependency policy on its first attempt. This fixes one Linux directory-entry persistence barrier after no-replace journal publication; it does not certify sudden-power-loss durability or Windows directory flush.

## Integrated no-clobber journal publication — PR #286 (2026-10-20)

[PR #286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/286) was squash-merged as `67c65a5d78a2921cee826184a2ad8bc573e4fb88` (exact head `7007ad3d202e42f642cece5f8df131035806ff35`). Hosted Windows/Linux Rust, dependency policy and the three native checks passed on that head; independent [post-merge CI #39385303625](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39385303625) passed Windows/Linux/dependencies on first attempt. The source code and proof establish no replacement of an existing journal during publication on hosted Windows/Linux GNU; they do not establish directory flush or power-loss durability on Windows, nor full ancestor identity guarantees.

## Current scoped state-root setup preflight (2026-10-20)

On GitHub `main` was `adeed077215c2103991254e4b5e2c64fa2d632de`, with zero open PRs. [Post-merge CI #39381445867](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39381445867) finished green on exact main: Windows Rust, Linux Rust, dependency policy, attempt 1. Work branch `fix/recovery-state-root-preflight` corrects an actual side-effect-before-denial in `MutationRecoveryJournal::open`: when the chosen state-root did not yet exist within a workspace, `create_dir_all` previously created it before detecting and rejecting the workspace location. The candidate probes the canonical nearest existing ancestor first, retains the post-create canonical check, and tests refusal without project directory creation plus valid external directory creation. This is a **pending targeted PR** until exact-head CI and squash are observed; refresh live GitHub state.

Parent-directory races between preflight and creation are still possible. No blanket security or durability claims. #277/#278/#282/#284 journal/staging hardening is already merged; do not redo. The Windows AppContainer/Node and MCP first-response intermittencies remain separate unexplained reliability debt. Next after validating this increment: narrow examination of journal publication no-clobber semantics or root identity, without speculative broad native changes. Preserve alpha ancestry and all evidence refs. No operator-PC tests, self-hosted runners, releases or mass deletions.


## Scoped Windows staging recovery evidence — 2026-10-20

GitHub bootstrap: `main` `d11d27c8efb1048f504d96024e4015ec60fd1535`, zero open PRs, and [post-merge CI #39376986248](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376986248) success on first attempt for Windows Rust, Linux Rust and dependency policy. The current work is [PR #284](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) on `fix/recovery-staging-handle-delete`. **Its eventual merge and checks are not pre-asserted**: re-open GitHub and validate exact head, reviews, hosted Windows/Linux and post-merge `main` before declaring success.

The defect was demonstrated in source: Windows `cleanup_recovered_staging` accepted a staging hard link if bytes matched the journal, closed the validation file handle then independently deleted the pathname. The targeted replacement uses the existing native exclusive, non-reparse, single-hard-link delete opener; verifies bounded content and deletes with the same live handle. One native regression asserts hard-linked external evidence, staging bytes and durable journal all survive a rejected recovery. No change to mutation contracts, new rights or byte ceilings.

Remain explicit: the root/parent directory is not pinned by this increment; full ancestor-reparse or namespace replacement resistance and power-loss durability are not proved. Separate Windows Node/AppContainer/MCP cold-start intermittencies remain active debt. Next follow-up should investigate only a demonstrable flaw in remaining temporary journal publication or root identity, not another broad refactor. No PC execution, self-hosted runner, release or mass branch cleanup. Preserve main/alpha separation.


## Current GitHub continuation — journal append integrated, 2026-10-20

Fresh verified `main`: `bf79b320b4e7aa04a02c87fe448d589296bc8bb2`. [PR #282](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/282) was squash-merged from final head `c837cead816fb59c5d3a94b8d9b5fe1926987644`. That head passed all hosted Windows Rust, Linux Rust and dependency policy checks in [CI #39375905332](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905332), plus Windows [native preparation #39375905331](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905331). Its first head failed a Windows-only Clippy `unused_mut` warning; the final head corrected it without suppressing Clippy. The post-merge [main run #39376336498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376336498) had started on that SHA and was still executing when this note was drafted. **Verify live status**, not this time-stamped text, before describing post-merge CI as passed. GitHub showed zero open PRs after #282.

The scoped fix to `MutationRecoveryJournal::append` rejects hard-link aliases before appending, using a checked native exclusive Windows state-file handle or Unix opened file identity/link check; length is bounded against the actual opened file and the existing sync-before-effect contract is unchanged. The regression proves both outside and in-journal bytes unchanged after a rejected `mark_committing`. Do not redo #277 (read), #278 (retire), #280 (MCP timing), or #282 (append). Parent-directory identity, Unix concurrent path/size races, Windows staged-artifact cleanup and power-loss durability remain separate unproven bounds.

**Next focused choice:** inspect the distinct Windows staged-artifact cleanup / opened-handle identity path for a demonstrated flaw with a small native regression, or investigate an actual timed-out AppContainer/Node or MCP first-response run using the opt-in #280 diagnostic markers. Do not invent the root cause or weaken timeouts. Keep `main` and `alpha/generated-project-editing` distinct. No tests, builds, installs, servers, tunnels or permission changes on the operator's PC; hosted CI only, no self-hosted runners, release or bulk deletion.

## Integrated startup timing probe and follow-up — 2026-10-20

A new chat confirmed `main` `0f857b81a0715b2fcb7cb8f1e2a4250bf9221fa6`, no PRs open before work and [post-merge CI 39370890039](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39370890039) completed successfully on attempt 2. Windows attempt 1 of that CI failed in the existing TEMP-only MCP doctor with a 30-second response timeout and an alive bridge, while Linux and dependency checks passed; the initial failure is retained. The separately recurring AppContainer/Node loopback `TimedOut` (10 seconds, no output) also remains a separate investigation.

[PR #280](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/280) squash-merged as `8b6ccdc2270f8fa130df5d182d728d07466df908` from exact final head `ce15d036332d314e7f951042285d4f442d0219a7`. [CI 39373512931](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373512931) passed Windows/Linux Rust and dependency policy. The new opt-in smoke produced process-started=7ms, initialize-sent=30ms, initialize-received=4823ms on a passing runner (elapsed from doctor launch). A previous head's rustfmt failure was corrected, not waived. Separate post-merge [main CI 39373992973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373992973) was still in progress when written; check actual conclusion before claiming post-merge success. The bounded diagnostics increment adds a static opt-in startup clock (`OPTIC_STARTUP_TIMING=1`) to the Rust bridge's stderr and measures spawn/initialize handoff with monotonic time in the existing hosted Windows doctor. The scoped CI step passes `-StartupTiming` only to its child process. There is no additional timeout, fallback, capability, secret disclosure, stderr-to-stdout redirect, network grant, test skip, runner type, or operator-PC action. The phase markers separate pre-main OS startup from argument/policy/server/transport preparation and MCP handshake. If the failure repeats, correlate marker last-seen with host-side timestamps; do **not** infer causation from timing alone.

**Next agent action:** Verify `main` and post-merge run 39373992973, then investigate an actual failed Windows timing trace when available. First check whether the stalled stage was before Rust entry, during configuration, before MCP service creation, or during the first response. Do not interpret the 4.8s passing sample as proof of a defect, or reduce isolation/extend watchdogs. Investigate the 10s AppContainer Node-loopback intermittency separately from 30s MCP first response, then return to journal append hard-link boundaries as a different increment. Preserve the alpha branch, existing evidence and historic failed attempts.


## Current handoff: recovery journal read and retirement integrated — 2026-10-20

At live inspection, `main` became
`c2282ff5eba85d2a7480dd353db638a0623bd06d` after
[PR #277](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/277)
was squash-merged. Exact source `825bcdb963d80ac8419df887e8b8c307aa497caf`
passed four hosted checks in
[CI 39368379416](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379416)
and [native preparation 39368379422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379422).
The distinct post-merge `main` CI [39368741554](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368741554)
completed successfully on exact post-merge `main` (Windows, Linux and dependency policy).

[PR #278](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/278)
was also squash-merged into `main` as
`b15fe7636dcb31033d71ecdfea9ed51bdc786a2b`
from exact head `9778b497320d82db493bfbe4c02ff13d113c8ad5`.
Its [CI 39369331802](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331802)
passed hosted Windows/Linux Rust and dependency policy;
[native preparation 39369331816](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331816)
passed. The independent `main` CI
[39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540)
failed Windows Rust Test on attempt 1 for the known intermittent native
AppContainer/Node network probe (`TimedOut` at 10s, no output); three serial
reruns of the single diagnostic passed. The original failure remains retained
and the failed-Windows-job-only second attempt on exact `main` SHA `b15fe76`
**passed** its native tests and smokes. Linux/dependency passed on the first
attempt. CI 39369706540 completed successfully on attempt 2, but its original
failure remains relevant and recorded; the unknown cause is not fixed.
The original
#278 head failed rustfmt, corrected on a new exact head without waiver.
There were zero open PRs following the observed merge. Previous handoff
checkpoints below are historical and must not be treated as reset targets.
Branch retirement tooling is unavailable through this connector; the integrated
work branches remain without bulk cleanup.

**Next specific review:** prioritize the now-repeated Windows
`capability_free_appcontainer_denies_node_loopback_connection` failure
(run 39363540900 and again 39369706540 attempt 1). Inspect bounded isolation
launch/scheduling/start-of-script timestamps and deterministic diagnostics;
do not extend the 10s watchdog, disable the native gate or interpret diagnostic
passes as a production fix. In a separate subsequent increment, review the
journal append path in
`crates/optic-bridge-runtime/src/mutation/recovery.rs`: it checks pathname
metadata then opens for append separately, without the equivalent alias/
opened-file identity protection now used for reading or retirement.
Demonstrate a defect with a hard-link fixture, then isolate the correction with
native tests. Do not bundle this with Windows staged-artifact cleanup, directory
identity, process-smoke intermittency or research compatibility gates.

Unresolved: the separate journal append and staging-artifact cleanup boundaries,
Unix pathname deletion race, directory pinning, power-loss durability, and the
Windows Node/AppContainer timeout. Keep alpha branch and archive/proof refs
untouched. No PC testing, self-hosted runner, release or broad cleanup.


Reviewed: 2026-10-20. Repository:
[ELITEAYOTO/Optic-AI-Bridge-](https://github.com/ELITEAYOTO/Optic-AI-Bridge-).
This document makes continuation independent of the previous chat and its local
files. [STATUS](../../STATUS.md) owns measured delivery; [ROADMAP](../../ROADMAP.md)
owns acceptance order; contracts and ADRs remain normative in their own scopes.
## Latest verified handoff checkpoint — 2026-10-20

At the time of this final handoff verification, `main` was
`e222ff912ac784eb7ae29d09e74ae1780f8a051b`; there were **zero open PRs**. Documentation reconciliation
[PR #275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/275)
had been squash-merged, and exact-`main` CI
[run 39364949397](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39364949397)
was **successful** (hosted Windows Rust, Linux Rust and dependency policy).
The preceding #274 post-merge run 39364345041 also completed successfully.
Verify GitHub again in a new chat; do not treat this dated SHA as a reset target.

The physical mutation-domain move and blank-journal-record correction (#273/#274)
are **finished**. Do not redo them. The first-attempt Windows Node/AppContainer
timeout in the original #274 CI (run 39363540900) remains an open reliability
finding despite same-head success on rerun; preserve both attempts. Cargo normal,
Electron sandboxed, disk quota enforcement, real power-loss ACID and production
auth in the generated-project alpha remain **unproven**.

**Next specific work candidate:** inspect mutation recovery journal file-open,
read and retirement boundary for a demonstrable safety or evidence-preservation
defect with regression tests, or isolate the Windows AppContainer cold-start
timeout in a separate PR. Choose only one bounded increment with native checks;
do not manufacture a bug, expand capabilities or perform a mass refactor.

## Current operator direction

Continue development, hardening and documentation through GitHub using the
connector and the new agent's own isolated environment. **No tests or runtime
changes on the operator's PC for now.** Do not request PowerShell commands,
start tunnels, install dependencies there, change device permissions or dispatch
self-hosted jobs. GitHub-hosted Windows/Linux runners are available through the
repository workflows. A Linux pass cannot prove a Windows security boundary.
Future real-PC testing is a separate operator decision, not an implicit next step.

The product goal is an AI-independent, Windows-first Rust bridge enabling work
in an explicitly selected project: files, builds, processes and Git operations
under deterministic permissions, with useful failure evidence and resumable
project state. Optional Node, Electron and Rust toolchains should eventually
support real applications. The core must not require those optional runtimes,
Python, Docker or an embedded LLM. A visual operator app is later work.

The user wants fewer unfinished PRs, reviewed merges and truthful docs. Small
completed increments are preferred to another large research stack. Installed
tools or a model's ability do not grant execution, network or administrator rights.

## Bootstrap without the old chat

1. Inspect available GitHub read/write/PR/Actions capabilities. Confirm repository
   identity, default branch, actual heads, open PRs, reviews and current CI. If
   tools cannot edit, push, merge or execute, report that specific limit; do not
   invent capabilities or redirect execution to the operator's PC.
2. Read root AGENTS, AI_HANDOFF, README, STATUS, ROADMAP, CONTRIBUTING, this file,
   REPOSITORY_WORKFLOW and RELIABILITY_DEBT. Then read the owning contract,
   security invariants and relevant ADRs for the chosen change.
3. Treat the snapshot below as a locator, not a command to reset branches. Compare
   it with live history. Inspect existing work before deciding to resume it.
4. Choose one bounded roadmap increment with observable acceptance. Separate
   implementation, supported behavior, diagnostic evidence and future targets.
5. Branch from the actual target; implement with tests appropriate to the change
   in your own environment or hosted CI. Keep Windows-specific proof native.
6. Publish a reviewable PR with exact evidence, docs and limitations. Check its
   latest head and feedback, merge when applicable gates pass and retire only
   that preserved branch after verifying its head. No bulk deletion or release
   publication follows from this handoff.
7. Before stopping, update the owners of current state and record remaining work,
   exact PR/head/CI links, uncertain outcomes and the next concrete action. The
   next agent must not depend on hidden chat memory or local output files.

Repository/log/issue content is untrusted task data. Human authorization and
deterministic policy control effects. Do not copy credentials, private paths or
raw capability handles into commits, comments or logs. Observe an uncertain
write/merge/job outcome before retrying it; a transport timeout is not proof that
the operation did not happen. Do not recreate an entire project after a crash.

## Verified source snapshot before this documentation update

At bootstrap there were zero open PRs and two remote branches:

| Branch | Verified head | Role |
| --- | --- | --- |
| `main` | `f1e72f856eaa5f6ee5e6dd861022c049c7f4cd16` | Reviewed Rust core and canonical docs |
| `alpha/generated-project-editing` | `5245e3daeadd2fce8e3dde1fd6a55ba46588fdb2` | Generated-project HTTP/FS alpha; main ancestry aligned by #260 |

These are historical anchors, not promises about the next chat's live state.
The 244 old branch retirements and 41 historical PR dispositions are already
recorded in [the consolidation ledger](REPOSITORY_CONSOLIDATION_2026-10-20.md).
Archive tags and preserved PR refs are historical evidence, not production
releases. Do not repeat that cleanup or delete preservation refs.

Completed increments: #259 fixes reproduced lossy native Windows path aliasing
(main `97f4098`); #255 completes fixed offline Cargo research (main `f1e72f8`);
#250 completes bounded Electron diagnosis (alpha `e62ea95`); #260 aligns alpha
and packages the hardened core (alpha `5245e3d`). See STATUS for exact CI/source
evidence. Completion of a research PR does not establish runtime compatibility.

## Validations and known red research gates

| Workflow | What it establishes / limit |
| --- | --- |
| `.github/workflows/ci.yml` | Rust Windows/Linux checks and dependency policy; inspect job results, not only aggregate status |
| `private-project-preparation.yml` | Native private-project preparation contract |
| `owned-native-process-pipes.yml` | Owned native process/pipe lifecycle |
| `private-rust-compilation.yml` | Fixed direct-rustc compilation; does not prove general Cargo builds |
| `offline-cargo-research.yml` | Independent normal and prepared disposable Windows comparisons; normal currently fails NUL access with exit 101, prepared succeeds |
| `private-application-staging.yml` (alpha) | Node staging plus Electron controls; normal Electron fails before app startup (`0x80000003`), explicit outer-only diagnostic succeeds |
| `real-link-admission.yml` (alpha) | SDK/generated adapter acceptance and source-identified alpha bundle |

Inspect workflow source, event/path filters, permissions and actual runner labels
before triggering it. Never run the NUL research grant outside its disposable
hosted fixture, change a self-hosted machine, silently disable Electron isolation,
add `continue-on-error` to hide unsupported behavior or relabel a comparison as
ordinary support. Distinguish an unchanged, documented research baseline from a
new regression; preserve evidence when retrying a proven infrastructure failure.

The latest previously verified package is an experimental Actions artifact, not
a permanent release. Artifact `12229445645`, run `39340735266`, source merge
`ac76efd6ea833e46dfced7bee3148af968754566`, PR260 head `a4091bc`.
Payload ZIP SHA256:
`3563bb8e9666455e6559cf6e30362b99849eb437fe91769fc1b6691c2113bc00`.
Its 85 entries/80 source files and packaged HTTP/Node fail-fix checks were
previously verified. Actions artifacts expire; inspect availability and rebuild
with provenance if needed. Do not assume the new chat can access old local ZIPs.

## Scope that remains open

- General writable builds need actual containment, storage enforcement, owned
  stop/quiescence, crash evidence and artifact publication. Input and post-exit
  byte limits are not runtime disk quotas.
- Optional Cargo preparation needs trusted broker/lease provenance, explicit
  operator preview/approval, revoke and interrupted recovery. The user approved
  developing that option, not applying administrator changes to their PC.
- Dependency restore/package scripts/egress/secrets need scoped authorization.
  General project enrollment and normal Electron startup remain unproven.
- The FS adapter uses generated projects, shared authority and public dummy
  authentication. It is not production authentication or conversation isolation.
  Host conversation IDs are not authority. FS backend logs and hidden model
  state cannot be reconstructed by Optic.
- Small paginated reads were a workaround for observed host truncation, not a
  universal 1024-byte security rule. Preserve bounded output and version/offset/
  EOF metadata; measure total serialized responses before increasing budgets.
- Native path hardening is scoped; Windows case/identity semantics and other
  remaining findings live in [RELIABILITY_DEBT](RELIABILITY_DEBT.md).

Prioritize the roadmap's next provable contract, preferably enabling headless
real-project work without waiting for Electron. Do not expand into every runtime
at once. Useful cross-cutting improvements include clear bounded diagnostics,
version-safe resume, efficient tool responses and reliable shutdown; implement
only after inspecting current code/tests so closed defects are not reopened.

## Reporting and completion

Report what changed, exact PR/commit and CI evidence, what remains unsupported
and the next action. State explicitly which validations were not run and why.
Use GitHub links rather than personal filesystem paths. Never say “all tests
pass” when normal Cargo/Electron research is red. Documentation-only work needs
accurate links/state, not a claim of runtime verification. Keep historical
audits dated and annotate their scope instead of rewriting measured history.

## Latest verified quality cycle — 2026-10-20

This is a **newer checkpoint** than the bootstrap snapshot above. GitHub
confirmed `main` at `5a8157ea2fff0b19c4b47e075eeefa457947efc1`,
zero open PRs after completing:

- [PR #265](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/265):
  merged as `a9d9a31b4e1635c35ac53c8225a4c5b4348aeb05`. Moved 1,115
  lines of tests from `process.rs` into `process/tests.rs` without
  altering the production prefix. Final head `f65f69b` passed four hosted
  checks (Windows/Linux Rust, dependency policy, native preparation);
  post-merge main CI run 39355110640 passed.
- [PR #266](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/266):
  merged as `5a8157ea2fff0b19c4b47e075eeefa457947efc1`. Corrected silent
  process stdout/stderr read errors: incomplete output is flagged, and an
  otherwise successful exit becomes `Failed`; existing owned stop,
  deadline and termination-uncertain handling remain distinct. Two
  synthetic failure/EOF tests were added. Exact head `4e4c712` passed
  four hosted checks. Previous head failed rustfmt and was corrected.
  Post-merge main run 39356012838 was still in progress at documentation
  creation; verify its actual result rather than inventing a pass.

Both operations used hosted Windows/Linux runners only; no user-PC
process, install, permission change, tunnel or self-hosted runner. The
normal offline Cargo and sandboxed Electron research gates remain red.
The separate generated-project alpha remains unmerged into main and is
not production authentication. None of this establishes general writable
builds or runtime disk quota enforcement.

[CODE_QUALITY_REVIEW](CODE_QUALITY_REVIEW.md) now tracks the two scoped
source reviews; this is **not** a full review of all 212 baseline files.
Next review target: `process/output.rs` panic/JoinHandle observability,
the remaining spawn/launcher/lock paths, then operator CLI parsing and
session lifecycle. Only add fixes backed by specific tests and native
Windows evidence; avoid unrelated reorganizations.

The connected GitHub actions available in this chat do not include a
delete-branch operation. Previously merged short-lived branch heads
remain until individually verified and retired; preserve archive tags,
historical proof refs and `alpha/generated-project-editing`. Do not start
another mass cleanup. New agent should inspect current heads/checks/PRs,
especially the post-merge run and any later changes, before proceeding.

## Latest runtime Git layout consolidation — 2026-10-20

This supersedes the earlier source snapshot. At inspection `main` was
`24a40e1` with no PRs, and the Windows job in [CI 39356540735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39356540735)
had failed only the TEMP-only Node MCP doctor smoke after 30 seconds with the
bridge alive. An exact-source failed-Windows-job rerun (job 118412524278)
succeeded. This remains an unresolved intermittent failure; do not dismiss
or weaken that acceptance.

[PR #268](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/268)
has **merged into main** at
`d79d60b88b60b2e95c85039c2d31427d0954a865`. The source head
`b2b5207834efe27b131aa1e7d1b46372e55194c0` passed all four
applicable GitHub-hosted checks (Windows/Linux Rust, dependency policy and
private native preparation). Ten `src/git_*.rs` files moved to
`src/git/{blob_batch,change_set,integrate,integration_authority,merge_commit,merge_plan,read,tree_manifest,tree_write,worktree}.rs`
with identical file blob SHAs. `lib.rs` uses explicit `#[path]` to preserve
all previous `git_*` module names, public reexports and private test
namespaces. No authorization, resource ceilings, contracts or functionality
were altered. Post-merge `main` CI run 39358651176 must be inspected
separately.

Next action: verify actual post-merge CI, then continue **one domain at a
time** (session and lease lifecycle or file mutation/recovery) only if module
dependencies and tests justify it. A second priority is investigating the
Windows MCP first-response intermittency with deterministic, bounded
diagnostics; no workaround is approved. The file-by-file audit is ongoing,
not complete. The separate generated-project alpha remains isolated,
no broad Cargo or sandboxed Electron support was proven, and no runtime disk
quota is claimed.

Do not touch the operator's PC, request PowerShell, change local permissions,
start tunnels or use self-hosted runners. The connected GitHub action surface
does not provide deletion of old integrated branches: retain them until a
specific verified retirement capability is available, and never delete
archive/proof references or conduct another mass cleanup.

## Latest completed stabilization cycle — 2026-10-20

The previous session/Git layout work is now superseded by these exact
GitHub observations:

- [PR #270](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/270)
  merged on `main` as `636a8b5e917fd731dbd8566e118958f141f67542`
  from source `4d5ff0a4372c465b7e65b7d1bd581c0d44fe9f3b`.
  The four session-domain Rust source blobs were unchanged; paths and
  `lib.rs` declarations only. Four hosted checks passed on exact PR head;
  post-merge main CI 39360825623 passed.
- [PR #271](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/271)
  merged as `8610c8dc2558695842412c5a0fce993d2420d6fb`
  from source `fb9ffdb8b6489b09653e3a8ea5ed6c71634b08dc`.
  A failed stdout/stderr Tokio drain task join now sets the existing
  incomplete-output signal and closes the stream, preventing clean
  `Exited` reporting for incomplete collection. Deliberately aborted
  drains following `TerminationUncertain` retain their original uncertainty
  semantics. Two synthetic tests were added. Exact source passed four hosted
  checks; initial head `bed3ca9` failed formatting and was corrected.
  Post-merge main run 39361585398 was still in progress when this handoff
  was drafted and must be checked live.

The source inspection of session/lease admission/revoke/renew is **scoped**;
no exhaustive per-file or platform security audit was completed. Follow
[CODE_QUALITY_REVIEW](CODE_QUALITY_REVIEW.md) and keep known research
Cargo/Electron and Windows MCP TEMP-only intermittency evidence intact.
No PC tests, installation, permission change, tunnel or self-hosted runner
was used.

**Next action:** verify actual latest main post-merge CI. Then select one
bounded code-quality increment (e.g. process monitor shutdown/cancellation,
session lock-order tests, or mutation/recovery layout) based on a demonstrated
problem or structural benefit. Do not group many unrelated moves. Keep
`alpha/generated-project-editing` isolated and its ancestry intact.
No branch-deletion action is available through this connector; only
individually verified integrated work branches may later be retired, not tags,
archive/proof refs or a bulk set.

## Mutation/recovery stabilization checkpoint — 2026-10-20

This section **supersedes previous in-progress observations**. The latest
verified `main` immediately after the merges was
`58a72a202bbdb57da425fba461a7b1a97b2f9d92`, zero open PRs.

1. [PR #273](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/273):
   source `97584927ba438efdd843b7bd4ad488d3c321e43f`, squash
   `58ebc5c1a4f0fd015b26c04f8cd83cf4255d5782`. Six exactly unchanged
   mutation/recovery Rust blobs moved under `src/mutation/`, with module
   identities and exports preserved by six explicit `#[path]` declarations.
   Four hosted Windows/Linux checks passed; post-merge main CI 39363288153
   succeeded. No semantic file/journal change.
2. [PR #274](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/274):
   exact final source `283c9e0e699a50470fab2183dc9945b9d4559c23`, squash
   `58a72a202bbdb57da425fba461a7b1a97b2f9d92`. The recovery parser
   rejects completed blank newline-delimited records (`CorruptJournal`)
   while preserving the contractual torn-final-line recovery. Regression
   tests cover corrupt interior/trailing blank lines, evidence retention and
   permitted torn tail. The initial `8d51d19` head failed `rustfmt`, fixed
   without silencing checks. Linux, dependency and native preparation
   passed. Native Windows run 39363540900 first attempt failed a separate
   Node/AppContainer loopback probe because the child `TimedOut`; three
   serial diagnostics passed, and an exact-source failed-Windows-job-only
   rerun passed. **This is an unresolved intermittent CI failure**, not
   fixed compatibility. Preserve the first attempt and study the native
   startup path separately.
3. Post-merge `main` CI run 39364345041 later completed successfully.
   The following documentation reconciliation PR #275 was merged at
   `e222ff912ac784eb7ae29d09e74ae1780f8a051b`; exact-`main` run 39364949397 passed. See the
   latest verified checkpoint at the top; re-check live state after this date.

Follow [CODE_QUALITY_REVIEW](CODE_QUALITY_REVIEW.md) for reviewed files and
[RELIABILITY_DEBT](RELIABILITY_DEBT.md) for unresolved native timing evidence.
Next priority: focused source/test review of mutation-journal file-open and
retirement boundaries or the Windows Node/AppContainer cold-start timing,
with one regression-backed increment at a time. Do not rewrite all mutation
services just to reduce length; their contracts and recovery state machine
remain normative. No all-files audit is complete.

No operator-PC work, self-hosted CI, broad Cargo/Electron compatibility
claim, physical disk-quota claim, alpha merge, new release or mass branch
retirement was authorized. The connected GitHub action surface exposed
no delete-branch operation; preserve archive/proof refs and verify exact
integrated heads before any future individual retirement.
