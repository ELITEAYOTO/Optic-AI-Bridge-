# Current project status

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
