# Main integration ledger — 2026-10-15

## Integrated

PR184, reviewed exact head1e7695f12d214c504f84870dfe1f861f5d7d6792,
squash-merged to main as18a8ec665907454863cb4e911d3a57baccca3416.
Premerge CI680/run38256421685 and postmerge CI/run38751500494 passed.
The complete eight-file diff was inspected; no unresolved review threads or
requested changes existed. Scope: effect-spanning read/list/Git admission,
lossless native path conversion and advancing bounded list pagination.
It does not close sensitive-content, canonical list, Git-control or remote
project-enrollment gates. Older candidate-status prose is historical and needs
reconciliation in the next documentation integration, not a runtime claim.

## Inventory and next disposition

GitHub API inventory after merge:41open PRs, only169currently targets main.
The other40target branches in the experimental stack. Therefore merging only
the top alpha PR into its current base would not deliver the alpha to main.

| Group | Next action |
|---|---|
|169 documentation | Reconcile current evidence/roadmap with merged main; retain useful history, remove stale current-status assertions; rerun exact-head checks before merge |
|170/178/186–195 transport/auth/session/rate probes | Review and collect reusable tests as a coherent experimental batch; preserve unsupported-host and negative-evidence qualifications |
|196–200/202–203/209–222 native public persistence probes | Separate useful regression coverage from unclosed storage design; no production credentials promoted by passing negative characterization tests |
|201 SDK characterization;204scope | Include accurate architectural/evidence documents in documentation batch, with later observed facts clearly separated |
|205–208 fixed read admission/runner/evidence | Review into alpha support batch; generated fixture only |
|223 editing/Node/viewer/resume | Consolidate after prerequisites; keep optional generated alpha explicit, include shutdown diagnostics and fresh main corrections before targeting main |

These are dispositions to review, not approvals of all individual diffs. Retain
or close superseded PRs only after the corresponding code/evidence is preserved.
Do not force-push or delete dependent branches as part of inventory cleanup.

## Current alpha and delivery identity

Alpha runtime ea23de41a8753062462faaf7915b113fcceb072a passed all nine
PR workflows (CI793/run38751320267 and Read Link45/run38751320456).
It includes the shutdown correction and generated connection diagnostic. Artifact
11980608896 has merge-source ef9653bbc62201ffa17435db898fc8149355d761.
The downloaded inner ZIP SHA256 is
180a85ccd4be71ca4ce4c6342d69346194c007ec4a415f154f66773068e14a70;
outer digest, archive paths/bounds and four binary/viewer manifest hashes verified.
Later resume-report invalidation and evidence documentation are follow-up changes
and must be validated separately before claiming them in an updated package.

Alpha contains main's184commit through merge85df742. The overlapping read path preserves effect-spanning admission and alpha full-read content versions. Local Python suites passed29alpha and26session tests after integration; Rust formatting passed. Local Cargo check is blocked by missing msvcrt.lib; native Windows/Linux integration validation remains assigned to GitHub CI.

Follow-up d8d739f adds operator-selected1024/4096-byte file-read pages. Default1024,8192-byte file limits and mutation authority stay unchanged.30alpha tests passed, including UTF-8 boundary and request-ceiling coverage. Exact-head CI is pending; this source is not yet a validated replacement ZIP.
No claim that the current experimental ZIP is built from main or production-ready.
