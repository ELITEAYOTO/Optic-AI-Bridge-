# Current project status

## Offline Cargo research integration — 2026-10-20

PR255 completes the fixed research fixture and proposed optional preparation
contract, not general Cargo or administrator setup. Its dedicated normal and
prepared acceptance jobs remain independent; an unsupported normal job stays
red and cannot inherit the prepared result. Standard direct-Rust CI is unchanged.
Earlier21ac09b hosted proof completed prepared check/test, failing assertion,
correction, offline dependency denial, cleanup and explicit revoke. At code head282524c, dedicated CI39339416828 prepared job118356950903 also
proved unrelated-profile denial, cleanup and revoke; normal job118356951094
retained the actual NUL error5. Subsequent integration documentation does not
relabel those observations as normal support.
See docs/research/PRIVATE_OFFLINE_CARGO_NUL_2026-10-19.md.


Reviewed: 2026-10-20. This page describes source and measured acceptance, not a
production release. Detailed historical notes are retained in
[the exact previous status snapshot](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/e1b82888dcf9c079740777be27341ca943758859/STATUS.md).

## Merged and validated

PR259/`97f4098` adds lossless Windows path boundary checks and a native
regression for a reproduced alias read across distinct non-Unicode roots.
Git path comparisons/arguments and operator-policy placement receive the same
representation audit. See [ADR-0033](docs/decisions/ADR-0033-LOSSLESS-NATIVE-PATH-BOUNDARIES.md)
and the [reviewed debt register](docs/engineering/RELIABILITY_DEBT.md); this is not
a claim that every filesystem or compatibility gate is closed. Exact
CI39338661674 and preparation39338661374 passed before merge.

| Track | Latest delivered increment | Evidence |
| --- | --- | --- |
| Main core | Local supervision #252; pinned exact Node module metadata #254 | #254 head `837fa30` passed CI 39286806748, preparation 39286806746, native pipes 39286806756 and Rust compilation 39286806794; merged as `e1b8288` |
| Generated FS alpha | Exact replacement #251; environment repair #256; alignment with native-path fix #259 and completed research #250/#255 | #256 head `d00e212` passed CI 39292953084, adapter/package 39292952943, preparation 39292952592, pipes 39292952775 and direct Rust 39292952891; merged as `a553645` |
| Earlier verified alpha package | Source CI merge `75ace2e`, artifact 12206671614 | 85 archive entries and 80 source files verified; native packaged 32 KiB HTTP editing and Node fail/fix/check/test passed |

The alpha ZIP SHA256 is
`b2b34a47e22357b1e6a49d2f721b471060932ab5c538791a28afca3024ae8b14`.
Its source identity remains fixed; a documentation change does not relabel it.
It uses generated projects, shared connection authority and public dummy auth.

The core includes scoped/versioned filesystem mutation, separately authorized
Git operations, bounded process lifecycle and private-service foundations.
Not every internal service is exposed as a general MCP project/build tool.

## Compatibility evidence and remaining limits

- **Electron #250:** bounded diagnostic scope merged into alpha as `e62ea95`.
  At head `62f8177`, ordinary CI/adapter/preparation/pipes/Rust passed. The
  staging rerun job118357480984 passed both diagnostic controls and observed
  normal exit `0x80000003` / primary result11 before app startup. The separately selected outer-only diagnostic succeeds; no automatic
  sandbox downgrade or public GUI permission follows. The baseline research
  staging workflow remains red, including run 39292952790 on #256.
- **Cargo #255:** ordinary capability-free execution fails opening NUL, exit 101.
  At source `21ac09b`, disposable CI 39291321987/job 118192403768 passed the prepared
  exact-profile comparison, actual checks/tests, output cleanup and explicit
  revoke with restored denial. This is not ordinary Windows Cargo acceptance.
- **Optional Windows preparation:** development direction approved; the proposed
  contract is on #255. No local privileged helper or device permission change
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
reference branches and all preservation refs were verified unchanged. The merged
#257 maintenance branch was retired separately; the completion-report branch is
temporary. Exact heads, approval and observed results are in the
[consolidation record](docs/engineering/REPOSITORY_CONSOLIDATION_2026-10-20.md).

Next delivery steps are owned by [ROADMAP.md](ROADMAP.md). Core and alpha remain
separate until their applicable release gates are satisfied.

## Combined alpha source and package provenance

This alpha integrates main through the native-path correction and completed
Cargo measurement scope, retaining the generated adapter and owned Electron
diagnostics. Rebuild and validate a fresh bundle for this source. The earlier
verified package entry above is historical and does not contain the new path
correction; it must not be relabeled or used as evidence for this source.
