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


## Exact alpha integration validation

Head8ecd2bb24bd1afd8096bcb56f7b9bdfc30fed2ba passed all nine workflows,
including CI38761152794 and Read Link38761152706. Artifact11986381235 has
merge-source a1b7f34c2c242b7c79cfaec35b5c1de3e575915c. Outer digest
2a0efc1af4ddd5c794d017c1943d2e0ce7922559d5352de765b9914da34255ae;
inner ZIP SHA2566ee3c5fb5ec6b6b51a52c2534d23089243f84b476e698de0026887290c73715f.
Archive path/count/expanded-size bounds and four manifest hashes verified.
Real Windows Node runner passed write/versioned read/failing test/conditional
fix/check/test/stop/resume against that binary. Following source improvements
(default selected page cap, visible limits, versioned search offsets) passed
32alpha tests and the4KiB UTF-8 two-page native runner proof; exact CI for those
source changes is required before packaging a newer delivery.

## Project input capture integrated — 2026-10-16

PR224 independent main-based head72c06c4681769bf873b817cc67d976407c14a81a
passed CI38836808832 (Windows, Linux and dependency policy). Seven-file diff
reviewed locally; no reviews requesting changes or unresolved review threads.
Squash merge bcd26aac9602d30f7461b5d7f303a23456005970 confirmed by GitHub.
The native input capture service adds no new MCP/process/GUI authority.
It inherits the existing scope policy; unresolved sensitive-content classification
is not solved by input hashing. Non-Windows capture remains fail closed.

Alpha incorporates main's implementation; the conflict in a wrong-effect test
uses FileSearch, which exists on both branches, rather than alpha-only directory
creation. Alpha status/history is preserved. Post-merge main CI and this alpha
integration require their own checks; prior exact-head success is not a claim
about a later merge head.
Main post-merge CI38837205821 passed forbcd26aa. The alpha provider boundary
ADR is renumbered0017 to avoid collision with main's input capture ADR0012;
its contents/authority are unchanged. External source classification remains open.

## Main private-preparation integration — 2026-10-16

PR225 exacthead78a45aa passed CI38899856537, preparation38899856368 and local
three-test native reproduction. Squash merge3a89f4060fb9ba2512db211e9922e28654980924
confirmed. Sourcecapture/readpolicy remains mandatory; no public process, GUI,
dependency or artifact permission is added. This alpha retains its newer owner
worker/cancellation controller; main's SID-query negative test and ADR0018 remain.
Equivalent native storage helpers/imports were deduplicated during integration.
Exact merge-head validation remains required; prior CI is not its validation.
Main post-merge CI38900962379 passed for3a89f40. Experimental managed Node first proof compiled and passed six preparation tests, then failed with Node FileSystemRead denial on an extended Windows input path; corrected path spelling and lifecycle checks require their own native result.

## Owner worker and managed Node proof — 2026-10-16

PR226 head2baa138 passed CI38905910740, native38905910736 and six local tests
from verified artifact12047922027 (outerb33d67521d588df4a67d21f1d5f475efa3834569205929d2507aaaf40edb84f9).
Seven-file diff reviewed, no change requests or unresolved threads. Squash
fab63d9f85a3d3730b4f822d1ca565831f290871 merged; mainCI38906519699 passed.

Alpha5406a4f passed CI38906210858 and native38906210466 with five managed Node
cases plus six preparation cases. Earlierc7436b4 three-node/six-preparation
cases reproduced locally, verified artifact12047926705, outer
f841317266a5c41366edc81b026d9aba82ca0a39e6dc948bb365fe3c83342092,
binary956fd34df9730f32fe3b538090c56e4ebbda2f38de2ee2ffbf1337fb1fdaf23d.
Initial extended-Windows-path Node permission failure is corrected with matching
local-drive spelling; UNC/device paths are refused. The native Node API remains
unwired. Normal Electron inner-sandbox startup remains a separately failing probe.

PR227 exacthead4121515 passed CI38906725340, native process/pipe38906725527 and preparation38906725329; both platform tests reproduced locally from verified artifact12048850903. Nine-file diff reviewed, no change requests or unresolved threads. Squash abff6389af2fb082972dcef18a80092e8df5fcbd merged. Alpha integration preserves its newer Node controller, native profile close and experimental-only helpers; main ADR0018/0019 and focused native workflow retained. Exact integration checks pending.

## Private state and independent stop integration — 2026-10-16

PR228 exactdc0bc181d43d743818bf8b5c8473259d4367448f passed CI38953227075,
native38953227163 and preparation38953227214. Both new platform cases reproduced
locally from verified artifact12069975356. Squash07efdf5afb6cc9818f2c008a9aa0dcc91f623a18
merged. Its postmerge CI38955478146 failed the existing Node loopback proof:
TimedOut, no stdout/stderr, no listener accept. This is not network-denial evidence
and no cause is assigned. The subsequent main head below passed the full suite.

PR229 exact017242ea96574ef3882effe9ed9e1f2aa898fc71 passed CI38955900602,
native38955900228 and preparation38955900348. The watchdog regression reproduced
locally0.63s from verified artifact12070108340. Six-file diff reviewed; no pending
change requests or unresolved threads. Squashbcd0bcc9293c782441ff792c5173f53fae66e58d
merged; postmerge CI38956441204 passed Windows/Linux/dependency jobs and smokes.

Alpha integration3b1457c preserves the journal/Node controller and test-only
compatibility probes. Focused38956512499 passed10 preparation,8 journal,9 Node
cases plus the CI-only actual native-owner abrupt exit/quarantine. The local
blocked-owner proof1.95s and revocation-before-admission-drain0.01s passed from
verified artifact12070842559. No local native crash was induced. Exact complete
alpha CI is tracked separately; these facts do not validate a later merge head.
