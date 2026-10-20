# Current project status

## Atomic no-replace journal publication — scoped PR under review (2026-10-20)

Live GitHub check: `main` `49c55d7ad6e4eee7507e2ded1e3de1f4342f5c77`, zero open PRs before this increment, and post-merge [CI 39382804575](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39382804575) passed hosted Windows Rust, Linux Rust and dependency policy on its first attempt. Publication in `MutationRecoveryJournal::begin_for_action` checked destination absence, then called overwriting `fs::rename`: another creator could install a durable journal between the check and rename. The new scoped candidate replaces publication with an atomic no-replace rename (Win32 `MoveFileExW` without replace/copy flags; Linux GNU `renameat2(RENAME_NOREPLACE)` on one opened parent), with deterministic preexisting-evidence and contended-publisher regressions. It retains the prior durable prepared write and recovery schema. The source PR's exact checks and merge must be verified before considering delivery. Unsupported non-Windows/non-Linux-GNU targets now fail closed instead of silently using a replacing rename; other platforms need separate acceptance. This is not a parent-namespace pinning, crash-power-loss persistence or process-isolation proof.


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
