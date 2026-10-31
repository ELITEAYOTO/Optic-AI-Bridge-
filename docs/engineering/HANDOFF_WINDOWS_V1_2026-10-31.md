# Windows V1 — canonical GitHub handoff (31 October 2026, after #453)

> This is a dated, verified **pre-documentation PR** checkpoint. The docs
> merge will advance `main`. Always requery live GitHub main SHA, PRs, issues,
> workflow conclusions and artifacts before implementing any next change.
> Do not reset a branch to this SHA.

## Most recent integrated technical state (after #453)

- **Repository:** https://github.com/ELITEAYOTO/Optic-AI-Bridge-
- **Confirmed technical `main`:**
  `bfcd9dcba0f73bf6d8aa4d148fc5c654c819edf6`. Its *independent*
  [push/main CI #41195919444](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195919444)
  completed **SUCCESS 4/4, attempt 1** on exact squash SHA/checkout.
  Windows runtime **424 passed, 0 failed, 11 ignored**; original schema8
  `OPTIC_PHYSICAL_HOST_STOP` reports are `complete=true` with **10
  Windows** scenarios and **4 Unix** characterizations, matching source,
  checkout, run and attempt, artifacts attached.
- **Branch security observation:** `main.protected=false` at the checkpoint.
  Do **not** modify branch protections, security settings, credentials,
  reviews, releases or workflow permissions without owner approval.
- **Current release gate:** [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323)
  OPEN / **Windows 10/11 V1 NO-GO**. #317 closed *administratively*, not
  proven live liveness; #185, #319, #176, #182 OPEN. Open research PRs
  **#327 / #353 / #355 / #358** remain deliberately outside `main`;
  #355 has a negative real Git spawn-after-revoke observation.
- **Scope:** Windows-first Rust/MCP core stabilization. Ubuntu CI remains
  required, but no Ubuntu V1 release. No operator-PC access, new local
  grants, self-hosted runners, Optic Desktop, REL, tag, production native
  GitRead switch or security relaxation.

### Proven completed technical PR sequence — #450–#453

All four PRs were branched/committed/pushed, validated on **exact final head**,
self-reviewed (explicitly **not** independent peer approval), **squash-merged
only after checks**, and compared with the **identical original HEAD Git tree**.
Their independently triggered **push/main CI** completed SUCCESS 4/4 at
attempt 1 on the exact integrated SHA.

| PR | Final tested HEAD | Exact-head CI | Specialized applicable check | Squash `main` | Independent push/main CI |
| --- | --- | --- | --- | --- | --- |
| [#450](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/450) | `fa9e9232a923eba53bf034c7f54b6fc4ac22229b` | [41191441197](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41191441197) 4/4 | [native #41191441170](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41191441170) | `f907ab462704bc7c67c2780f6f9ef2b9bf351f1b` | [41191950148](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41191950148) 4/4 |
| [#451](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/451) | `4e4e9c8698f7cef41465086e0df1022b7273f84e` | [41193250500](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41193250500) 4/4 | [native #41193250425](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41193250425) | `9a3b9b070320d8dd13f8440c0af44be4a90f7d9e` | [41193687286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41193687286) 4/4 |
| [#452](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/452) | `b0fd540f893fcf62e6fdd5eec6185f5092f313d1` | [41194430564](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41194430564) 4/4 | [native #41194430569](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41194430569) | `1d7934de941041afdfa8a347110001221268f94a` | [41194832125](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41194832125) 4/4 |
| [#453](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/453) | `7354f5b0d847861bd6d5b3add611bb8ca603a91e` | [41195558970](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195558970) 4/4 | [#317 late-pipe #41195558883](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195558883) **11/11 per OS**; [native #41195558899](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195558899) | `bfcd9dcba0f73bf6d8aa4d148fc5c654c819edf6` | [41195919444](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195919444) 4/4 |

- **#450:** introduced inert CreateSuspended/Resume in-flight call tickets and
  irreversible stop publication without waiting on blocked modeled calls.
  The first candidate's rustfmt failure was fixed on another head.
- **#451:** the nonexecuting owner binds the tracker to its **original**
  session, TaskLease generation, counted Job and host stop flags and
  nonrenewable absolute `Instant` after watchdog arm. An earlier rustfmt
  failure and separate test-only private-field E0616 failure are preserved.
  This is not an atomic CreateProcessW/ResumeThread fence.
- **#452:** `TrackedNativeCall<'_>` now borrows its tracker inside original
  owner lifetime rather than holding a detachable `Arc` clone; scoped
  concurrent stop test passes. This is *call lifetime*, not physical root/
  descendant termination, retained Job/EOF or an OS launch permission.
- **#453:** actual MCP legacy `HardenedCommandRunner` now captures the
  original `Instant` *before* `Command::spawn()`, input copy and pipe
  setup. No post-spawn timer reset. A real disposable test-child with an
  already elapsed injected start must return `TimedOut`. Windows+Ubuntu
  pass. Tested HEAD and squash have identical Git tree
  `b89c232b7038771a73596d3bbc8a7ea8447b1974`.
  The dedicated #317 pipeline reproduces its controlled late pipe cases
  (11/11 on both systems), **but this does NOT bound joins or prove native
  descendant termination**.

Detailed [#453 receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/453#issuecomment-6542696895)
and [#323 checkpoint](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6542697249).
Preserve previous red CI sources; green reruns are not an explanation of
intermittent Node/AppContainer or first-response failure.

### Blocking acceptance that none of these PRs establishes

**No GO for native GitRead**: the real MCP `git_status` still executes
`HardenedCommandRunner`, whose stdout/stderr `.join()` and stdin writer
may remain blocked if descendants inherit pipes. Capturing the earlier
start in #453 changes the **eventual timeout result**, not the wall-clock
bound during an uninterruptible `spawn`, `wait`, or pipe join.
The inert `git/native_status.rs` + `git/native_inflight_calls.rs`
and native Job/watchdog do not issue actual Git child execution.
Separate session/TaskLease/policy/host snapshots, revoke flags, stop
watchdog and even stopped empty Job cannot seal future Win32 creation or
resume. A call ticket admitted before revoke can still reach the kernel
after revoke. #355 proves a real post-revoke Git spawn on research source.

The next true P0 must supply a **durable service guardian** with original
session/lease generation/typed policy epoch/host authority, original pinned
executable/repository, synchronized future and in-flight CreateProcessW
and ResumeThread admission/stop, with no cross-authority locks held during
potentially blocking Win32 or I/O. Retain original permit/Job/root/thread/
stdio/quotas until separately observing **physical root exit, kernel
Job-empty, both physical pipe EOFs and no future launches**; preserve
native HRESULT/uncertainty and crash/abandoned-caller reconciliation.
Security #176/#182, real startup #319 and AppContainer #185 remain
independent gates; do not inflate timeouts or relax containment.

### Mandatory successor method

1. Read `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`,
   `ROADMAP.md`, `CHANGELOG.md`, this handoff,
   [current successor prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-31.md),
   `ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md`, `RELIABILITY_DEBT.md`,
   `REPOSITORY_WORKFLOW.md` and relevant source/tests/ADRs.
2. Query **live GitHub** main, merged/open PRs, issue statuses, branch
   protection, final runs and exact source/tree/physical evidence. Do
   not assume this docs SHA remains HEAD.
3. Choose one narrow **real behavioral** defect and deterministic
   regression. Avoid stacking inert markers in lieu of native launch
   authority. Branch from actual main, code, commit/push, open ONE PR.
4. Verify exact final-head CI Windows/Ubuntu fmt, strict Clippy, runtime/
   workspace tests, dependency/release and applicable specialized native/
   #317 workflows. Read logs and physical report source/run/attempt, not
   badges alone. Preserve any red head, correct on a new commit; never
   retry-to-green or lower assertions.
5. Review the entire diff, branch base, permissions and safety scope.
   Auto-review is not independent peer approval. Squash only if all
   applicable checks pass and the head/base are verified.
6. Compare HEAD and squash Git **tree IDs**, then check a **separate**
   `push/main` CI on the exact integrated SHA (all four jobs and
   applicable physical artifacts). Comment complete receipts in PR and
   #323; **only then start the next PR**.
7. If anything cannot be safely proven, leave the PR open and describe
   the blocker. No operator PC, self-hosted runner, settings/protection,
   permission or release change without fresh explicit approval.

**Windows V1 remains NO-GO.**

---

## Previous pre-#449 technical checkpoint — historical only


> **Snapshot before this documentation PR.** Live GitHub `main`, open PRs, issues, Actions, and branch rules must be rechecked before any next change. This SHA is evidence, not a checkout/reset instruction.

## Verified integrated checkpoint

- Repository: https://github.com/ELITEAYOTO/Optic-AI-Bridge-. `main=7f1de707c130d61f8a6878728ec2ba8bd6ac23a8` after [PR #448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/448); independent [push/main CI #41188920407](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188920407) **SUCCESS 4/4, attempt 1**, verified exact merge SHA.
- `main.protected=false` at this snapshot. Do **not** change branch protection, releases, tokens, approval or repository settings without explicit owner authorization.
- Only open PRs at this snapshot: [#327](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/327) (research on non-main bootstrap branch; not draft), [#353](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/353) (draft native research), [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) (draft revoke-before-spawn research), [#358](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/358) (draft stacked on #353). **None is integrated into production main**; never bulk-merge or mistake green research CI for runtime readiness.
- [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is **CLOSED administratively**. Its missing behavioral acceptance is tracked through [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323), which remains **OPEN / Windows 10/11 V1 NO-GO**. [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319), [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176), [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) remain **OPEN**.
- Scope: **headless local Rust/MCP Windows V1 stabilization only**. Existing Ubuntu CI remains mandatory verification; it is not a Linux V1 release target. No Optic Desktop, REL, general feature development, remote real-project access, autonomous execution, Windows V1 release/tag or unsupported platform claims.

## Finished PR-by-PR delivery record

Each PR was code-pushed, reviewed in a GitHub comment (not an independent peer approval), checked on **exact head**, squash-merged, and followed by a separate **push/main CI success 4/4 on its exact integrated SHA, attempt 1**.

| PR | Final tested head | Squash `main` SHA | Exact-head CI (4/4) | Specialized native check | Independent push/main CI (4/4) |
| --- | --- | --- | --- | --- | --- |
| [#445](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/445) | `443721d9e5ed` | `6cc47415717f` | [41184970274](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41184970274) | [41184970113](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41184970113) | [41185358982](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41185358982) |
| [#446](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/446) | `5a8546b33664` | `198c33b8b2a1` | [41185962462](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41185962462) | Not applicable | [41186369040](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41186369040) |
| [#447](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/447) | `fdaa9013c0fa` | `b0b68dd1d5e3` | [41187712596](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41187712596) | [41187712569](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41187712569) | [41188103927](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188103927) |
| [#448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/448) | `fc628077c98f` | `7f1de707c130` | [41188629382](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188629382) | [41188629411](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188629411) | [41188920407](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188920407) |

- **#445** — bounded receiver-only startup-age and cumulative pinned-executable read-duration diagnostics. Preserve first earlier Windows failure [#41127771145](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41127771145) and earlier [main #41123153308](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41123153308). Added a same-length partial-stderr-rewrite cursor regression: old records cannot be retimestamped. These diagnostics **do not establish root cause or fix the intermittent pre-listener cold startup**.
- **#446** — MCP GitRead checks the **same original RequestPermit deadline after sensitive-output verification**; deterministic policy-review clock regression passes on Windows and Ubuntu. Earlier first head [CI #41185906029](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41185906029) had Ubuntu `cargo fmt --check` failure and was superseded; not a behavioral GitRead denial failure. Not an atomic spawn fence.
- **#447** — inert non-executing native status owner samples original clock **after** arming the actual original Windows Job watchdog; deterministic expiry regression. Earlier format/borrow/visibility failures remained visible in superseded heads; full final exact-head CI and specialized native checks passed.
- **#448** — inert `RegisteredNativeGitStatus::precreate_native_custody` checks original service clock **after** real bounded Job/pipe precreation for clock-issued owners. Windows-only regression injected an expired post-creation time: original clock identity retained, counted capacity present in the callback then safely dropped, no child created, private ledger Failed. Legacy logical-time preparation remains inert/unmodified.

**Physical source evidence:** final CI for #448 and its independent `main` reports explicitly show `OPTIC_PHYSICAL_HOST_STOP` schema 8, `complete=true`, **10 Windows** scenarios and **4 Unix** characterizations, matching source SHA, run and attempt; 21 offline supervisor tests per OS. The targeted #448 native regression passed on Windows specialized and ordinary CI and on post-squash Windows CI. Hosted Windows runners do not replace tests on supported Windows 10/11 operator devices.

## What is STILL missing (no-go, not a mere TODO)

1. **P0 native #317/#323:** The service-owned native status preparation is **not executable and not wired to real MCP GitRead**. Session, TaskLease, policy/epoch, host-stop, pin, Job and original deadline checks are currently independent observations/receipts. They **do not form one synchronized create/resume gate** controlling in-flight `CreateProcessW` / `ResumeThread` or later process admission. [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) intentionally demonstrates a real post-revoke Git spawn on research source. Do not fix by repeating snapshot checks, a watchdog alone, or by calling `TerminateJobObject` on an empty Job: [#439](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/439) shows this does not seal future creation.
2. **Physical lifetime:** Keep original admission, quota, Job, root and stdin/stdout/stderr until proof of **root exit, native Job-empty, both pipe EOF, and no pending future launch**. Preserve native error code / uncertain outcomes; stop-before-drain, bounded cancellation, abandoned caller and crash/restart reconciliation. The legacy `HardenedCommandRunner` pipe joins are not yet demonstrated bounded for descendant-held streams; see [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323), research [#355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355) and earlier targeted failures.
3. **Independent cold-start regressions:** [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) ~30-second first MCP initialize; [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) AppContainer Node startup/loopback stalls; neither is rooted nor corrected by #445–#448. Do not inflate timeouts, relax network denial, or retry-to-green.
4. **Sensitive read:** [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) and [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) still require actual content, alias, Git status/diff/blob/provenance policy for real workspaces. Filename checks alone are not a proof for ordinary-named secrets.
5. **Windows V1 release:** crash/interrupt/soak evidence, Windows 10/11 host acceptance, PowerShell 5.1 installer/update/uninstall rollback, protected branch/reviews/credentials and exact-main build provenance. Operator sign-off required. Do not issue any V1 tag/release.

## Mandatory next-session sequence

1. Read `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`, `ROADMAP.md`, `CHANGELOG.md`, this handoff, `docs/engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md`, `RELIABILITY_DEBT.md`, `REPOSITORY_WORKFLOW.md` and [the successor prompt](NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-31.md).
2. Use the **connected GitHub plugin** to retrieve **current** main SHA, tree, open PRs, issue statuses, CI checks, specialized workflows, last main CI, open red evidence. Do not trust a dated checkpoint as the current branch. Ask only an essential product/security question; otherwise continue autonomously.
3. Prefer a bounded **real defect and deterministic regression** at the synchronized native launch/revocation boundary (or a necessary prerequisite). Decide the smallest safe PR, review exact source paths, document what it does **not** prove, create one branch directly from live `main`.
4. Implement, commit, push; use `cargo fmt --all -- --check`, Clippy, workspace tests, relevant Windows kernel/native workflows on **hosted** runners. Check logs, job steps, physical evidence, first red attempts. Fix only demonstrated failures on **new commits**; never pretend a cancellation or later green removes earlier red.
5. Review exact final head, check original `main` hasn't drifted, ensure applicable **all checks succeed**, squash merge **only** that PR, compare tested-source and squash **Git tree IDs**, then independently validate the new `push/main` CI (4/4 and applicable physical/native proof) on the **merge SHA**. Publish evidence in the PR and #323. **Finish the whole PR before opening another**.
6. If gates are genuinely red, incomplete or unavailable, do not merge to force progress. Explain the exact blocker and leave one PR open rather than misrepresenting completion.

**Operator safeguards:** no work on user PC, no self-hosted runners, no bypass of cybersecurity prompts or tools, no unauthorized local Codex/Work execution, no repository-wide reset, no branch protection change, no release, no privilege/network/isolation/time-limit relaxation; other repositories out of scope. No asynchronous promises.

## Linked proof owners

- [#323 V1 gate](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) and final [#448 receipt](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/448#issuecomment-6541762588).
- [native authority audit](ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md), [physical shutdown proof](PHYSICAL_HOST_SHUTDOWN_PROOF.md), [reliability debt](RELIABILITY_DEBT.md).
- [continuity prompt](../../PROJECT_CONTINUITY_PROMPT.md), [Codex recovery procedure](CODEX_INTERRUPTION_RECOVERY.md), [source/release gate](RELEASE_SOURCE_GATE.md).

> This handoff fixes the **pre-documentation** checkpoint. The documentation PR itself will move `main` after squash; validate its own `push/main` CI, then requery live GitHub in the next chat.
