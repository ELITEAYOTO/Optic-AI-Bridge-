# Reliability and implementation debt

## Bounded FileRead multi-link alias safeguard — source-level finding (2026-10-21)

Path-based sensitive authorization alone does not distinguish hard-linked
names for the same native file. A protected `.env` and ordinary
`README.txt` hard link resolve as separate canonical pathnames but the same
opened file identity; without a shared-link check, FileRead through the
ordinary alias could bypass the lexical sensitivity floor. The scoped safeguard
treats multi-linked opened files as potentially sensitive and requires the
existing matching sensitive scope in addition to ordinary FileRead. It is
conservative for harmless hard links. Tests use **synthetic bytes** on native
Windows/Linux; they must pass on exact CI head before this can be considered
integrated. Staged Git blob metadata, arbitrary benign-name secrets,
concurrent identity changes and cross-tool GitRead proof remain open under
#176/#182. No timeout/sandbox/network/process or feature change.


## Verified after #312; Windows transport evidence remains open (2026-10-21)

[PR #312](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/312) extended the shared lexical sensitive-path floor to direnv files. Exact accepted-head [CI #39571281997](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39571281997) and independent post-merge main [CI #39571875338](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39571875338), at `30295539fc6911f221cee6ac3c59785398c7a050`, each passed first attempt on native Windows, Ubuntu and dependency policy. This strengthens a **filename** guard only: #176 and #182 remain open for content/alias/provenance problems; remote real-workspace use is still blocked.

**Focused source observation for #185, without a code change:** `crates/optic-bridge-runtime/src/process.rs` schedules writing and shutting down the Windows isolation launcher's stdin via a detached Tokio task. Both returned errors are currently discarded; the outer `start()` completing in ~9–17 ms does not establish inner launcher request delivery or child startup. See the [recorded source review](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185#issuecomment-6312184351). This is an observability shortfall, **not a proven cause** of the intermittent no-output 10s failure. An earlier passing 9.005s diagnostic measured inner target resumed at ~14 ms and exit near 8.952s, showing a *different* slow phase after resume in that one execution. The separate first MCP reply ~30s issue is not merged with the loopback issue. Keep original red attempts and strict sandbox, network, termination, output and timeout assertions.

No speculative functional patch, new privilege, runner-specific exemption, forced retry-to-green, release or user-PC action is justified by this review. Recovery multi-file deletion partiality, namespace identity, persistence and longer soak proof remain open.

## Verified shared read/Git sensitive-path floor and residual gates (2026-10-21)

At this historical checkpoint `main=bd2915c3a9833c1da5a8fc5005445d1884260d4f` after #310, with zero open PRs and first-attempt successful [independent native Windows/Linux/dependencies main CI #39552523257](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39552523257). #308 (core, FileRead and fs_list), #309 (Git status `-z` metadata), #310 (Git diff same-command NUL raw names + bounded patch and explicit FileRead dual-scope gate for flagged paths) are **fully merged**. #310 accepted-source CI #39551993433 and native preparation #39551993151 were successful on the exact PR head, following preserved earlier failing commits. Historical *candidate* entries below describe the steps leading to integration, **not current unfixed status**.

**Open limitations:** filename-sensitive floor cannot inspect secrets embedded in ordinary filenames; Git diff worktree/index races remain non-atomic despite one command; alternate names/identity, branch metadata and canonical ancestor/pinned-handle concerns need targeted audit (#176, #182). #176 was unexpectedly auto-completed by GitHub as part of #310's wording and has been explicitly reopened. No real project workspace remote exposure. Other unreconciled risks remain #185 Windows intermittent Node/AppContainer, separate MCP initial response, crash recovery and durable multi-file cleanup, fault/soak tests and incomplete file-by-file audit. No new features or broad grants.

## Candidate Git diff output verification and sensitive content lease (2026-10-21)

Historical starting point: `main=ccaa7c05af27cfaab02d5dcb93369f60a0c01f53`, [#309 independent post-merge CI #39549267903](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39549267903) succeeded first attempt. Proposed #182 increment, **not yet CI-proven**: one Git diff invocation with paired `--raw -z --patch` output, parse/validate raw path records before releasing patch, deny unverifiable formats, and authorize typed FileRead dual scopes for flagged sensitive patch paths; native Windows/Linux regressions required.

Remaining: lexical classifier fails to recognize secrets in ordinary-named content (#176); Git worktree/index concurrent edits may not form a single atomic snapshot; pathname aliases and native identities need additional investigation, as do Windows Node and recovery/soak open issues. Neither #176 nor #182 can be closed or real project remote access enabled solely on this change.

## Git porcelain status classification — candidate, issue #182 still open (2026-10-21)

Baseline: `main=bfba4b97a531440de08012ca50106797d995646b`
after #308; post-squash CI #39546153987 fully green first attempt.
Candidate status-only enforcement uses the shared core sensitivity rules
on every Git porcelain-v2 `-z` pathname and checks a matching active
sensitive lease before releasing response bytes. Unknown/malformed/native
unrepresentable records are refused. Regression fixtures include tracked
and untracked sensitive names, rename/original, ordinary status, missing
grants and malformed records. This paragraph is not evidence of CI success.

**Unresolved #182:** raw `git_diff` may still disclose protected paths or
content (including staged and repository-wide diffs). The path-name floor
does not classify arbitrary secrets inside normal-looking files. Keep #176,
#182 and real-workspace remote access gated until a complete checked
Git-diff output policy and relevant Windows/Linux regressions are verified.

## Policy B classifier implementation boundary — candidate (2026-10-21)

Operator approved common deterministic sensitive classification for file content,
directory metadata and Git with unknown/unverifiable output denial. The first
candidate increments *only* file read/list: a pure conservative lexical
`requires_sensitive_authority` classifier, policy-enforced FileRead
ReadContent+ReadSensitive dual-scope, and fs_list whole-page denial for
protected names without separately provisioned sensitive scope, plus unit/MCP
tests. This is a **candidate**, not evidence of completed CI/integration.
Decision: [ADR-0034](../decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md).

**Still blockers:** ordinary-looking files can contain secrets; Git status/diff
can independently reveal protected names/patches and are not yet classified;
unknown Git output representation requires fail-closed design; canonical
ancestor/identity races remain. Keep #176/#182 open, no real-project remote
exposure, no claim of full classifier or blanket data confidentiality.

## Active stabilization risk register — live evidence snapshot 2026-10-21

At inspection `main=43d3a4aeedee04dd03e895e27e81d3bc5af6652c`, no PRs open; [post-merge CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) green first attempt, exact SHA. Scoped PRs #301–#305 are **merged**, not candidates. Do not infer global safety from this CI.

| Risk / boundary | Evidence-backed status | Exit criterion |
| --- | --- | --- |
| Native AppContainer Node 10s failure [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) | **OPEN**; earlier failed attempts and later green retries recorded, cause unknown | Reliable failure-phase capture and root-cause fix with native regression, or documented safe scoped restriction; never accept timeout as network denial |
| MCP first response ~30s | **OPEN** and independent from Node loopback | Reproduce/locate actual stage, validate bounded targeted fix on hosted Windows |
| Recovery and filesystem identity | **OPEN**: multi-file cleanup is not atomic after a later actual deletion failure; ancestor swaps and Windows power-loss persistence unproved | Fault-injection/native proof and explicit handling of uncertain outcomes; no deletion of remaining evidence |
| Sensitive read authority [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) | **BLOCKER before real remote workspace**: no enforced sensitive classifier | Decide classifier/path policy and test canonical ordinary vs sensitive content/list metadata |
| GitRead sensitivity [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) | **BLOCKER before real remote workspace**: status/diff can expose secret-bearing content or names | Choose explicitly trusted GitRead vs sensitive-path enforcement or narrow capabilities, then prove cross-capability separation |
| Long-running soak/fault injection | **NOT fully evidenced**: design specifies 1h/4h mixed workloads and interruption/restart | CI-based resource, drain and recovery evidence appropriate to allowed hosted runners |
| Full file-by-file code quality audit | **PARTIAL**, as [review tracker](CODE_QUALITY_REVIEW.md) states | Record exact code SHA, input/authority/side-effect/error/test status for each reviewed area; avoid unjustified refactors |
| GitHub required protections | **Governance gap**: observed `main.protected=false`, no rulesets reported | Owner decides if required status/review protections should be enabled; no unilateral change |

**Reconciliation, not fresh bugs:** #303 fixed the lexical-versus-canonical `fs_list` authorization mismatch; #304 blocked generic `.git` mutation; #305 fixed pagination overflow. #180/#181/#183 remain open as issues but have documented bounded protections/regressions; check whether their *acceptance criteria* are truly satisfied before closing. Deferred product gates (normal Electron sandbox, generalized Cargo, authentication, host session mapping, operator approval) are not part of the active stabilization work. The dated sections below may retain pre-merge candidate wording and should not be used as a live work queue.


## Git-log serialization cap and cursor progress

A bounded output bug was demonstrated in `fit_git_log_response`: when a final page originally had no `next_cursor`, trimming its entries to fit the structured response budget required synthesizing a continuation cursor. The old code did not remeasure the page **after** synthesizing that cursor. The narrow correction refits entries and cursor together and fails closed if retaining one entry is impossible, without changing the existing maximum response budget. Numeric cursor overflow must not produce a non-progressing saturated offset. Dedicated synthetic-budget tests cover these constraints.

This is a size/progress correctness fix, **not** implementation of sensitive GitRead classification (#182), and does not affect Git history snapshot pinning, security capabilities, Windows startup, filesystem ancestor races, or recovery atomicity. Validate exact-head GitHub CI live before calling it delivered.


## Git control namespace separation — scoped correction under test (2026-10-21)

[Audit issue #180](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/180) establishes that normalized generic FileWrite/FileDelete and DirectoryCreate have structurally accepted `.git` control paths, while GitIntegrate uses a dedicated session capability/lease and runtime. The incremental correction reserves the **exact `.git` component**, case-insensitively at any subtree depth. Atomic file mutation preparation checks both requested and **canonical observed** paths before any commit/staging; generic directory creation denies Git control names while its existing native ancestor reparse-point protections stay intact. Worktree `.git` pointer files are treated as metadata; ordinary `.gitignore` and `.gitattributes` are unaffected.

This is a dedicated namespace enforcement gate, not Git repository trust policy for arbitrary hardlinks, unsafe directory namespace swaps, or Windows power-loss durability. The missing read sensitivity classifier and GitRead output policy (#176/#182) remain separate. Confirm regression and CI evidence live before closing #180 or declaring any capability safe for unattended/remote project exposure.


## File-search authorization target mismatch — 2026-10-21

Audit of `ReadonlyMcpServer::fs_list` established a deterministic security boundary defect: prefix-scoped search authorized the **unresolved request** before following a directory alias. If `allowed/portal` links internally to `private`, a lease for `allowed` can reach names in `private` under the original code. The corrective increment first canonicalizes the requested directory, then authorizes that scope, then lists using the canonical location (and returns canonical paths), with Unix adversarial alias and portable Windows/Linux regressions.

**Still unresolved:** races replacing canonical directory ancestors between the authorization and enumeration, no directory-handle pinning, and `ReadSensitiveScope`'s missing content classifier. This scoped change cannot justify exposing real project data over remote MCP. Keep [issue #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) open for independent unresolved findings. Do not relax other process/mutation containment invariants.


## Updated recovery and Windows debt after #301 — 2026-10-21

[PR #301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) is **merged** at `05b5a928c862601926e8b0cc11540ce020caf271`. Before the fix, `discard_unpublished` could erase a valid early `*.prepared.tmp` artifact before a later unsafe hard-linked entry failed closed; `scan_pending` classified filenames but did not validate unopened aliases. The new two-pass sequence inspects all unpublished entries with the existing native opened identity/link checks, then independently revalidates every deletion. A regression with sorted entries and an external hard link passed on native Windows and Linux, including post-repair cleanup. The source CI passed on attempt 2 following a known **first-attempt** Node/AppContainer 10s failure, preserved in [issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185).

**Still open:** subsequent *actual* multi-file deletion failures can remain partial; directory ancestor replacements and Unix path-based unlink races are not eliminated, nor are Windows directory/power-loss durability guarantees. The separate 10-second native AppContainer Node intermittency now has both no-helper-output failure and an 8.95-second **passing** inner runtime diagnostic following a rapid `target_resumed`; cause remains unproven. The 30-second MCP first response remains separate. Neither test retries nor instrumentation alone fix them. No feature work is permitted.



## Actual startup debt after PR #299 — 2026-10-21

[PR #299](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/299) merged into `main` at `6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea`. It adds **opt-in, fixed-label, monotonic stderr phase diagnostics in the outer Windows isolation launcher**, forwarded only for an exact `OPTIC_LAUNCHER_PHASES=1` test opt-in. The existing native CI Windows gate requires selected phase markers when enabled; Linux, dependency-policy and hosted native-preparation checks passed on accepted source head `aca382535f24b6f6b1c26c2313302d315099ed4e`, first attempt. The original network denial, 10s process budget, sandbox and output assertions remain unchanged.

A passing hosted Windows proof measured, relative to **launcher entry**, `profile_created=10ms`, `target_created_suspended=14ms`, `token_verified=14ms`, `target_resumed=14ms`, `target_exited=361ms`. Separately, parent-side `launch_returned_ms=13` and terminal observation `407ms` use a **different clock origin**. Those phases demonstrate marker delivery, **not** the cause of earlier intermittent timeouts. In particular, the earlier 10s failure had `stdout="" stderr=""` without instrumentation; the next natural failure with opt-in internal phases should be examined without suppressing original failures. If the only visible marker is `entered` or earlier, distinguish request delivery from profile creation; if `target_resumed` exists without Node entry, investigate inner loader/start/output forwarding, not just outer launch. Markers on shared stderr are diagnostic untrusted observations, not security attestations.

**Unresolved:** Windows Node/AppContainer occasional timeout/no script output (~10s; issue [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185)), independently the MCP TEMP-only first response (~30s), non-atomic multi-file recovery cleanup, ancestor path races and Windows power-loss directory persistence. No demonstrated root cause or correctness fix for these was delivered in #299. No timeout extension, extra network grant, runner self-hosting, operator-PC execution or feature expansion. Independent post-merge [main CI #39526677098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526677098) **passed first attempt**, on exact SHA `6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea` (Windows Rust, Linux Rust, dependencies).


## Current open reliability risks — reconciled after #297 (2026-10-21)

The last independently verified `main` is `83329d48789c20b1fc235d3096c3199dc9dc5176`; [post-merge CI #39514832814](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514832814) passed first attempt on exact SHA (hosted Rust Windows, Linux, dependencies), zero open PRs. **#293–#297 are merged**, not pending candidates. Their corrections and opt-in instrumentation are described in [STATUS](../../STATUS.md) and [handoff](GITHUB_HANDOFF.md).

| Priority | Current status | Constraint on the next agent |
| --- | --- | --- |
| Native Windows Node/AppContainer loopback test (10s) | **OPEN**: intermittent timed-out execution with empty child output; #294 post-merge attempt 1 and first serialized replay failed, later reruns succeeded | Investigate native helper, child start and output boundary; preserve failing runs and the original watchdog, never weaken sandbox or network denial |
| MCP TEMP-only initial response (30s historical) | **OPEN**: separate intermittent first-response timeout | Opt-in timing #294–#296 localizes several seconds of *successful* startup, not the 30s failure; do not conflate |
| Valid executable lease capture cost | **MEASURED, not a correctness defect**: 4869ms read+hash in one #296 hosted Windows sample | Capture remains security-critical on admitted leases; #297 only avoids capture on doomed duplicate/capacity admissions; no unproven cache/identity bypass |
| Recovery durability/identity | **OPEN**: multiple-file cleanup non-atomic under later failure; ancestor path replacement races; unproven Windows directory persistence/power-loss semantics | Preserve evidence and fail closed; require actual defect and native regression before any targeted correction |
| Cargo/Electron/production authorization | **DEFERRED FEATURE GATES**, not a scope for current stabilization | Cargo NUL-preparation is bounded research; Electron with normal Chromium sandbox unvalidated; alpha does not have production authentication |

**Closed narrow findings:** #293 coordinates journal/staging validation before discarding unpublished recovery evidence. #297 performs duplicate/global/session admission preflight before executable identity IO *and repeats all conditions under lock before insertion*; both negative and concurrent regressions passed on hosted Windows/Linux. #294–#296 add only opt-in bounded timing evidence. The first-draft #297 rustfmt failure and initial #294 Windows CI failure are part of the audit trail; do not erase them.

**Operator freeze:** no feature work, no broad refactor, no operator-PC execution/permission changes, no self-hosted runners, releases, or mass deletion. The dated sections below are historical snapshots, including old 'candidate' wording, and must not be mistaken for current unresolved PRs.

## Historical reliability notes — dated candidate states, not current PRs

## Verified closure and remaining limitations — 2026-10-20

`main` was verified at `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02` with no open PRs; [post-merge CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) passed hosted Windows Rust, Linux Rust and dependency checks on its exact SHA, first attempt.

- **Windows Node/AppContainer intermittent startup: OPEN.** [PR #290](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/290) merged test-only phase markers as `0c2a125537d020e3a3aff4124d12745d7a8c5c4c`, with [source CI #39392262806](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392262806) and [post-merge CI #39392829136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392829136) successful. Prior native first-attempt failures [#39363540900](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39363540900) and [#39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540) both timed out at 10s with empty output and no accepted connection; serial diagnostics succeeded with denied exit 42. Root cause unknown. Separate MCP 30s first-response intermittency remains open; successful doctor timing is not a fix.
- **Windows staging scan cleanup-before-full-validation: FIXED, narrowly.** [PR #291](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/291) merged as `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`. [Source CI #39393849343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849343), [native preparation #39393849287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849287) and independent [post-merge #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) passed. A later mismatched staging artifact cannot make the initial preflight erase earlier valid staging evidence; two-journal negative regression passed on native Windows. **Still OPEN:** non-atomic second-pass multi-file deletion when a later removal fails, replacement of ancestor paths between inspections, Windows directory persistence, sudden-power-loss guarantee.
  
Dated technical records below remain as evidence of the original failures and investigations; old `candidate` claims do not override the confirmed merge status above. Do not repeat #290/#291. The operator's stabilization-only restrictions remain binding.

## Recovery scan eager unpublished cleanup — corrected by PR #288

[PR #288](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/288) was squash-merged at `b1edc83438571867bd164ea0b2b9653304eed752` from source `60206dcadbf2efb57800ca93f0bd226acf16f6fb`. Its hosted [CI #39388280621](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280621) passed Windows Rust, Linux Rust and dependency policy; [native preparation #39388280521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280521) passed too. Independent [post-merge CI #39388749406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388749406) succeeded on the same merge SHA, first attempt (Windows, Linux, dependencies). GitHub showed zero open PRs at verification.

The scanner now classifies every stable recovery journal before discarding well-formed unpublished temporary entries. The regression proves a later corrupt stable journal cannot erase an earlier temp entry during failed inspection. This closes only the **cleanup-before-validation ordering** failure; cleanup of multiple temporary files is still not atomic if a later deletion fails, and Windows/Unix path-identity and crash/power-loss guarantees remain separately unproven.

## Current stabilization queue (not a feature backlog)

Only investigate substantiated safety or reliability defects in the existing product. Existing debt includes intermittent native Windows AppContainer Node loopback and MCP TEMP-only first-response timeouts, concurrent ancestor/path replacement, Windows directory persistence and non-atomic cleanup behavior. Preserve original failures and do not relax tests or announce fixes without a demonstrated cause and regression.

## Linux published-journal directory entry synchronization — addressed by #287

[PR #287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/287) was squash-merged as `0ca2305e7f01d5263330b67c4a0bb0520d452c8a` from final head `66161c9dee9e1a565b074d1cf105cf8961a712c2`. Hosted [PR CI #39386566516](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566516) and [native preparation #39386566570](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566570) passed; independent [main CI #39387133506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39387133506) passed Windows Rust, Linux Rust and dependency policy on its first attempt. Same opened directory is fsynced after Linux GNU no-replace rename, with an injected-sync-error regression preserving the stable prepared journal; hardware power-loss guarantees, Windows directory durability and retirement persistence remain separate debt.

## Journal publication no-clobber invariant — integrated by #286

[PR #286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/286) was squash-merged as `67c65a5d78a2921cee826184a2ad8bc573e4fb88` (exact head `7007ad3d202e42f642cece5f8df131035806ff35`). Hosted Windows/Linux Rust, dependency policy and the three native checks passed on that head; independent [post-merge CI #39385303625](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39385303625) passed Windows/Linux/dependencies on first attempt. Preexisting evidence and concurrent publisher regressions passed; Linux uses `renameat2` no-replace, Windows uses no-replace `MoveFileExW`. Other platforms fail closed. Parent directory identity, directory synchronization and sudden-power-loss guarantees were explicitly outside #286.

## Setup-time state-root creation before denial — scoped correction under review

Pre-increment `MutationRecoveryJournal::open` creates the requested state-root directory tree before canonical workspace-confinement rejection. An absent nested path inside the workspace therefore causes an unrequested workspace mutation even though journal creation is denied. Branch `fix/recovery-state-root-preflight` preflights the canonical nearest existing ancestor before creating anything, retains the final post-create check and adds a negative no-directory-creation test and a valid outside-root control. Verify the PR and exact-head CI before calling this delivered. The preflight is intentionally conservative and does not pin ancestors or make a concurrent pathname-swap guarantee.


## Windows recovery staging evidence — focused #284 correction (status via PR)

Pre-fix `JournaledMutationService::cleanup_recovered_staging` validated only pathname regularity and matching file bytes before closing the reader and unlinking a separately resolved path. A hard-linked staging alias with content equal to the durable intent could be accepted and removed without detecting the external alias. [PR #284](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) narrows the operation to one exclusive native single-link, no-reparse delete handle: recheck opened size, bounded-hash it, delete with that handle, retain durable journal on any rejection. The new Windows regression asserts the external link and staging link both remain, with original bytes and durable journal intact. See PR for the latest exact-head CI/merge result; **do not count source proposals as merged**. Parent directory identity/reparse behavior, power-loss durability, and other concurrent namespace races remain unproven. Node/AppContainer loopback and MCP first-response timing defects are separate unresolved items.


## Journal append opened-file identity — addressed by #282 (2026-10-20)

`MutationRecoveryJournal::append` on pre-fix `main` checked only pathname `symlink_metadata` before a distinct append open; an otherwise valid journal hard-linked outside its state directory could receive new bytes. [PR #282](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/282) merged as `bf79b320b4e7aa04a02c87fe448d589296bc8bb2` and rejects a hard-link alias before append: on Windows via the existing exclusive no-reparse, single-link checked state handle, and on Unix through opened regular-file device/inode/single-link comparison with pre-open metadata. The byte ceiling uses the opened file length; a negative regression verifies both links retain identical original bytes. Exact final head `c837cead` passed hosted [CI #39375905332](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905332) and [native preparation #39375905331](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905331). This is delivered **scoped behavior**, not a general journal audit. Unix concurrent namespace/size races, Windows directory pinning, staging cleanup, journal publication and full sudden-power-loss durability remain unproven.

## Windows initialization timing — integrated diagnostic, root cause open

The Windows TEMP-only doctor has intermittently failed on its first MCP response after 30 seconds while the bridge remained alive ([CI 39356540735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39356540735), [CI 39370890039](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39370890039), both successful after exact-head failed-job-only reruns). In a distinct native proof, the AppContainer Node loopback test intermittently returned `TimedOut` at 10 seconds with no child output ([CI 39363540900](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39363540900), [CI 39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540)). Neither is proven related to the journal hardening.

Merged PR #280 (`main` `8b6ccdc`, exact final head `ce15d03`, hosted CI 39373512931 passed) adds an opt-in monotonic stderr-only static phase timeline to the Rust server and corresponding host spawn/initialization timestamps to the existing CI TEMP-only doctor. The objective is to narrow where the **next occurrence** stalls. The hosted Windows TEMP-only diagnostic smoke passed and measured 7ms process start, 30ms initialize send, and 4823ms initialize response on one successful run. This is not an established causal fix. Separate post-merge main CI 39373992973 was in progress at this checkpoint. No timeout increase or relaxed gate is authorized. Other surfaces remain uninstrumented.


## Journal evidence lifecycle — targeted corrections, not a full audit

- **Read #277 (merged)**: `main` `c2282ff5eba85d2a7480dd353db638a0623bd06d`.
  Hard-linked external recovery bytes previously passed pathname-only metadata
  checks. Windows now validates an exclusive no-reparse single-link opened
  handle, while Unix checks descriptor identity and single-link count. Exact
  head `825bcdb` passed all four hosted CI checks. No general power-loss
  recovery or adversarial directory replacement proof follows.
- **Retire #278 (merged)**: the original `remove_regular_file` accepted
  a hard link as regular and then removed its directory entry, losing evidence
  of a foreign alias. Merged `main` `b15fe7636dcb31033d71ecdfea9ed51bdc786a2b`
  rejects linked objects, deletes by validated exclusive handle on Windows
  and has a cross-platform negative preservation regression. Exact head
  `9778b49` passed four hosted checks in CI 39369331802 and 39369331816.
  This does not prove Unix concurrent pathname removal safety.
- **Still open**: journal append uses a pathname metadata check followed by
  a separate append opener; Windows staging cleanup validates, reads, closes
  and removes via separate pathname operations. Both need isolated assessment,
  not bundled into #278. Unix deletion remains path-based after descriptor
  validation, so the concurrent namespace replacement race is not closed.
  Explicit state-root/directory pinning and full sudden-power-loss persistence
  guarantees remain unproven.


Reviewed: 2026-10-20. This is a bounded source review and delivery register, not
a claim of a complete security audit. STATUS owns delivered evidence; this page
owns concrete unfinished implementation/verification work.

Continuation mode: [GitHub-only handoff](GITHUB_HANDOFF.md), with no operator-PC
execution. Hosted native proof remains required where the boundary is Windows.
## Findings addressed in the current hardening change

Native path comparison used replacement-character conversion in runtime scope
checks. A Windows fixture reproduced an actual read from a distinct sibling root.
The fix refuses unrepresentable native paths, audits analogous Git and operator
policy placement checks, and preserves Git's native argument units.
See [ADR-0033](../decisions/ADR-0033-LOSSLESS-NATIVE-PATH-BOUNDARIES.md).
The regression exercises the boundary, not a string implementation detail.

## Remaining work, ordered by the boundary it unlocks

| Work | Current evidence or debt | Completion criterion |
| --- | --- | --- |
| Native path identity | Lossy comparison is addressed; Unicode case folding and Windows case-sensitive directories are not fully characterized | Native fixtures and a documented canonical comparison/identity contract; no guessed equivalence |
| Cargo #255 | Fixed offline prepared fixture passed only on disposable CI; ordinary profile still cannot open NUL | Finish optional broker/profile provenance, preview, exact SID grant, interrupted recovery and revoke before any local privileged setup |
| Electron #250 | Diagnostic scope is merged; normal sandboxed GUI still fails | Prove normal startup compatibility; keep failure visible and never silently select the outer-only comparison |
| Private writable builds | Core has input capture, staging, owned tasks and publication services, but no general MCP build profile | Proven writable-storage/cleanup contract, authorized host composition and real native goal-driven acceptance |
| Dependencies | Installed runtime presence does not authorize downloads, package scripts or egress | Operator-selected restore scope, package/script policy, credentials and network boundaries |
| Private-task journal retention | Finite record/unresolved ceilings intentionally refuse admission when full; no host retention/retirement workflow is delivered | Host-only version-pinned retirement of closed evidence introduced; explicit operator integration/retention UX and operational proof still required |
| Filesystem enumeration and sensitive metadata | Older read-boundary review deferred prepared enumeration and classification | Bind authorization to the observed enumeration target and settle content/listing/Git-control policy with native alias tests |
| Windows native AppContainer Node loopback test intermittency | On PR #274 exact head `283c9e0`, CI 39363540900 attempt 1 failed `capability_free_appcontainer_denies_node_loopback_connection`: process `TimedOut` at 10s with no output, rather than normal probe exit; three serial diagnostics immediately passed, then failed-Windows-job-only rerun on the identical SHA passed. A second, independent recurrence happened on #278 post-merge `main` `b15fe76`: [CI 39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540) attempt 1 repeated `TimedOut` at 10s/no output on the same test; all three targeted serial diagnostics passed, while Linux/dependency checks were green. Failed-Windows-job-only attempt 2 on the identical SHA passed, including the original test and subsequent Node/MCP smokes. Neither first-attempt failure proves a journal regression or identifies the underlying cold-start root cause | Preserve both original failures, all three diagnostic passes, exact-head reruns and logs. Investigate cold startup/isolation launch and scheduling with bounded evidence; do not weaken the isolation proof, bypass the native gate or automatically extend the timeout |
| Windows MCP TEMP-only smoke intermittency | Main CI 39356540735 failed a 30 s first-response wait with the bridge alive; a Windows failed-job-only rerun on the *same SHA* succeeded. No reproducible cause established | Retain failure and rerun as separate evidence; instrument bounded MCP startup/stdio path and add targeted deterministic regression if a defect can be reproduced; never silence the check |
| Research integration follow-up | Bounded measurements from #250 and #255 are integrated on their target tracks; normal compatibility remains unavailable | Implement the support/recovery contracts rather than retaining completed measurement PRs; do not import the whole alpha into main |

## Source review coverage

The original bounded review inspected the then-open #250/#255 deltas (now merged),
native path helpers, project input manifests and
capture, artifact publication, private task launch and journal validation.
Finite byte/entry/event bounds, exact snapshot hashes, staged publication progress
and explicit unknown outcomes already exist in those services; they should be
retained rather than rewritten merely to reduce file count.

Do not equate absence of TODO markers with completion. Internal service APIs are
not a delivered end-to-end project workflow, and test-only compatibility helpers
are not operator installation features. Prefer regression-backed corrections and
one coherent PR over another chain of small unmerged drafts.
