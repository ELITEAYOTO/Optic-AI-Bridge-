# Repository consolidation — 2026-10-20

## Verified snapshot and completed actions

The audit found 43 open PRs and 248 actual remote branches (249 remote-tracking
refs including origin/HEAD). It fetched exact refs before classifying them.
41 historical PRs were closed with their preservation/disposition written into
each PR. Electron #250 and Cargo #255 remain active drafts. The closed umbrella
#223 pointed the current alpha backward at an old experiment; closing it does
not merge that stack into main or retire the alpha branch.

39 of the closed heads are exact ancestors of retained alpha
`a553645760544b65e15ad0ed65bf6099b1d3f646`. Two divergent snapshots are published
as `archive/pr-169-2026-10-20` and `archive/pr-178-2026-10-20`. Six additional old
unique branch heads have `archive/2026-10-20/<original-branch>` tags. All eight
archives were pushed; they do not match the version-release tag trigger.

A complete local Git bundle was created and verified before any proposed branch
retirement. Its name/hash and all exact PR/branch heads are in the
[machine-readable retirement ledger](REPOSITORY_RETIREMENT_2026-10-20.json).
The bundle is an additional local recovery copy, not a downloadable release.

## Branch retirement remains pending

The plan covers **244 historical remote branches**. Each is either an exact
ancestor of retained alpha, an exact merged PR head still advertised by GitHub's
pull refs, or one of the published archive tags. The four reference branches are
excluded. A temporary maintenance branch may also exist during this cleanup.

Automatic approval review rejected the attempted bulk operation because generic
cleanup authorization did not explicitly cover deleting 244 remote branches.
**No remote branch was deleted.** The ledger is a proposal requiring the
operator's explicit approval before application. Do not split it into smaller
operations as a workaround for that rejection.

On approval, refresh refs and refuse any branch whose head moved, gained an open
PR or lost its documented preservation ref. Use expected-head checks for each
removal. Keep the ledger and completed results; never infer successful deletion
from a planned list or a failed push. Do not remove main, alpha, the active
research branches, release tags or unrelated repositories.

## Recovery and history

Merged commits remain in main or alpha; GitHub pull refs retain verified merged
heads. Published archive tags preserve the divergent old variants. The exact
branch/head ledger permits deliberate restoration to the original name if
needed; restore only after checking whether that name has since been reused.

The concise current README/STATUS/ROADMAP replace contradictory phase summaries.
The exact earlier main documents remain available at commit
`e1b82888dcf9c079740777be27341ca943758859`:
[README](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/e1b82888dcf9c079740777be27341ca943758859/README.md),
[STATUS](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/e1b82888dcf9c079740777be27341ca943758859/STATUS.md),
[ROADMAP](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/e1b82888dcf9c079740777be27341ca943758859/ROADMAP.md).
No runtime, permissions or security test changed in this consolidation.
