# Delivery roadmap

## Updated priority after ten hosted native crash scenarios — 24 Oct 2026

Latest researched and verified **unmerged** code: `research/317-failed-native-create-2026-10-24@b5a91dee`, exact [CI #40057931887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887) fully green with **ten** native externally supervised Windows scenarios. Production `main=16ca67a7` remained unchanged at this checkpoint; **#317/#323 OPEN, V1 NO-GO**. See [latest handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) for the research matrix, negative controls and semantics.

**Next engineering priority:** reproduce and characterize **descendant-held stdout/stderr pipe handles after parent exit**, bounded stdin closure/EOF and asynchronous stop-before-admission-drain under persistent native uncertainty. Keep the original guardian Job, generation-bound permit, JobStore quota and nonterminal status retained until independent native evidence; a timer cannot manufacture a terminal receipt. Prove host-loss/restart recovery or explicitly document why a persistent supervisor/ADR is required. Do not continue generating only near-identical 'suspended child/no-ACK' tests now that those controls are green. Only then extract one small independent production-safe increment from **current `main`** with exact-head Windows/Ubuntu gates; do not bulk merge research. GitRead task-lease/repo/exe and provenance (#176/#182), #185/#319, Windows 10/11 physical device and installer/release/rollback gates remain separate. GitHub-hosted only; Codex local blocked by controls is not a reason to bypass them.


## Current immediate sequencing — 24 October 2026

The [24 Oct canonical Windows V1 handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-24.md) supersedes the older 23 Oct research snapshots for the **current** work order. Production already has the *counted owned session admission* from #336, while research has separately proved JobStore commitment, kernel Job association at CreateProcessW, external synthetic creator-host death, combined stop-before-drain and **temporary injected** Win32 accounting uncertainty (latest exact-source [CI #40046563179](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40046563179)). None of the combined native research is wired to GitRead in main.

**Next critical acceptance order:** (1) nonblocking stop publication distinct from terminal drain/reap with an accountable retained owner even under *persistently unavailable native observation*; (2) externally supervised Windows native descendant/stdio-EOF, creator/service death and quota/handle uncertainty tests; (3) only then one bounded production-owned read-only Git effect under approved lease/executable/repo provenance and security #176/#182; (4) CI #185/#319, Windows 10/11 devices, release/rollback acceptance. Keep issues #317/#323 OPEN and V1 NO-GO. No user-PC testing, new feature, broad research merge, or release by implication. Treat the 23 Oct sections below as history, and inspect GitHub live.

Reviewed: 2026-10-22. [STATUS.md](STATUS.md) owns current evidence; this page owns
the next delivery order. Earlier phase/H-track snapshots remain historical
planning inputs, not a second statement of current completion.

Execution mode for the current continuation: agent-owned isolated environment
and GitHub-hosted CI only; no operator-PC tests or runtime changes. See the
[handoff](docs/engineering/GITHUB_HANDOFF.md). This does not remove native Windows
acceptance gates or imply they can be established on Linux.

## Current integration priorities — 23 October 2026 after PR 334

Production `main` now includes the **read-only, generation-bound admitted stop
condition** (PR 334) in addition to PR 331's explicit revocation watcher.
The new witness observes the original admission expiry despite renewal; it is
**not** native process ownership, a launch permit or a completion receipt.
Use the [latest measured status](STATUS.md) instead of dated SHAs below.

The immediate next bounded increments are: (1) specify and test one immutable
session-generation-bound registered-owner commit under a shared close
linearization, with actual task/lease policy gates kept independent; (2) ensure
stop request reaches already admitted workers **before** any synchronous permit
drain, including late registration and uncertain Win32 observation; (3) prove
Windows process tree membership **at creation** and full stdout/stderr/stdin,
host crash, deadline and quota retention under an independent native
supervisor; (4) only then migrate one sealed read-only Git path with safe
publication/provenance. Keep legacy session/approval/lifecycle report contracts
until their callers are reviewed. Preserve the real concurrent reaper/ABA
negative control as a separate generation-isolation acceptance case, while
distinguishing forced handle reuse from supported provisioning.

The V1 decision remains **NO-GO** under issues 317/323. Also preserve
sensitive-content/provenance review (176/182), native Windows intermittency
(185/319), real Windows 10/11 operator-authorized testing, release rollback
and offline installer/PowerShell checks. Hosted green tests alone do not
satisfy this matrix. No GitHub ruleset or user-PC changes in this checkpoint.

## Current Windows V1 delivery order — 23 October 2026

[Read current handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-23.md) and [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323). **Stabilization, not new features:** #317 remains the foremost Windows reliability blocker, but the new `ReadServiceRoot` research at `1c7dac9` has progressed past inert owner fixtures with real shared JobStore, native cleanup and seal/reap tests (CI #39888522519 + native #39888522200). Do **not** claim production repair: last-known `main=3fe3de0c` still runs HardenedCommandRunner, and PR #327 remains open with a real #185 red run. The new **host-crash-before-Job-assignment** reproduction leaves a suspended child until independent outer supervisor cleans it; revocation admission only protects submit; Drop may wait forever on persistent native observation errors.

**Next:** make an explicit architecture/revoke/shutdown responsibility decision; then prove child containment across host crash and stop-versus-resume linearization; then verified Git authorizer/pins/leases/deadlines and ONE read-only Git path, with #182 patch-provenance policy separately approved. #185/#319 native intermittency, real Windows10/11 support, packaging/recovery and release provenance remain gates. No Electron, remote-project scope or V1 release without owner consent.

## Current Windows V1 stabilization sequencing — handoff 22 October 2026

Owner directive: V1 is a future **release decision following stabilization of existing Rust/MCP code**, not an automatic result of green CI. [Current handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-22.md) is the continuity point; [issue #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) holds the evidence-based V1 freeze/no-go matrix. The highest-priority release blocker is #317: a Windows service-owned Git command process tree, robust stdin/stdout/stderr cleanup, owner-retained uncertainty and session seal/reap must be proven on the *production* runner. The research branch has 13 supervised native cases, real SessionRegistry/Job tests and a **non-wired prototype ledger**, none merged to main. **Next:** review/prove independent native observation and stop (avoid callbacks under a global lock), then staged Windows owned process and session integration before any GitRead migration. No wholesale research merge.

After #317, resolve or fail-closed exclude unproven V1-reachable parts of #176/#182, #319 and #185; certify supported Windows 10/11/PowerShell combinations with operator-approved real-device tests, and prove reproducible release/rollback/CI. Out-of-scope remote H-01–H-05/#177 and Electron remain deferred with enforced exclusions. No V1 tag or feature unfreeze without operator approval.

## Windows V1 first — operator decision, 22 October 2026

Target Windows10/11; verify actual supported versions rather than inferring them
from hosted Windows Server. Prioritize demonstrated Windows command ownership317, keeping sensitive-read
176/182 gates explicit, then MCP startup319, AppContainer/Node185 and proven
recovery/authorization/resource and installation defects. Keep these distinct; doctor error-path diagnostics do not
explain natural startup latency. Linux functional support is post-V1. Do not start
new cgroup/pidfd/PDEATHSIG/confinement research unless shared-code regression
actually blocks Windows; retain existing Ubuntu CI as non-regression checks.
Rust/MCP remains core; Electron can resume only after the relevant foundation
gates and a delivery decision, never as a mandatory Rust runtime dependency.

## Temporary stabilization gate (operator directive, 2026-10-20)

**The feature roadmap below is paused, not cancelled.** Do not start new
features, optional integrations, GUI work, broad refactors or experimental
alpha-to-main merges. Prioritize reproducible defects in the existing
permission/isolation/recovery/process boundaries, reliable diagnostics for
intermittent hosted Windows gates, targeted regressions and documentation
accuracy. A corrective PR requires a demonstrated failure and exact-head
native checks where applicable. If no defect is demonstrated, do not create
a change merely to show activity. Feature work resumes only on a new operator
decision. This freeze does not relax any existing security or CI requirement.

## Stabilization exit criteria — objective gates, not a claim of zero bugs

Two **distinct** decisions are required:

1. **Resume bounded headless development** only after an exact-head Windows/Linux CI baseline and targeted safety checks for any newly exposed effect; the known Windows AppContainer/Node and MCP first-response flakes must be explained, corrected, or explicitly excluded from that narrowly permitted workflow **without weakening a security test**. The operator must explicitly authorize lifting the feature freeze for this limited scope.
2. **Enable real-project remote/autonomous work** only after deterministic classification/enforcement of sensitive filesystem content/list metadata ([#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)) and a matching GitRead authority decision ([#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182)); authenticated principal, session and host correlation proof (H-01–H-03), local approval/revocation contracts (H-04/H-05), process/namespace and recovery boundaries, and appropriately reproducible soak/fault-injection evidence. A green short CI is insufficient.

**Before either decision:** retain actual red-run evidence, avoid speculative timeout increases and redundant PRs, explicitly classify partial Windows power-loss/ancestor/race guarantees, and reconcile code review coverage and handoff docs with the exact `main` SHA. Existing documentation and PR status inconsistencies are a real continuity risk, not evidence of a new runtime defect. Owner approval is required before changing GitHub branch-protection requirements; the last GitHub query reported `main.protected=false` and no rulesets.

## Deferred feature delivery sequence (not currently authorized)

1. **Maintain coherent integration.** Repository consolidation and bounded
   diagnostic scopes are recorded; keep `main` and the generated-project alpha
   aligned through reviewed increments. Track compatibility fixes as delivery
   gates, close completed PRs and retire only preserved exact branch heads.
2. **Prototype optional Cargo preparation on disposable Windows.** Prove the
   fixed NUL/exact-profile contract, trusted local broker identity, revocation and
   interrupted recovery before presenting a concrete administrator change on a
   user's PC. Keep project code and the model unelevated.
3. **Complete an authorized writable build environment.** Define truthful storage
   enforcement, owned-tree stop/quiescence, crash evidence and artifact
   publication. Input-copy and post-exit collection ceilings are not disk quotas.
4. **Add locally selected project and dependency workflows.** Support empty or
   existing workspaces with explicit enrollment, permissions and restore/egress
   policy. Prefer a goal-driven Node or Rust application test over another fixed
   UI exercise once the relevant gates pass.
5. **Close GUI compatibility separately.** Investigate Electron's normal startup
   failure without silently disabling its sandbox. Do not block all headless
   Node/Rust progress on that investigation.
6. **Productize the operator experience.** Add a visual permissions/project/log
   interface over the same validated services. Conversation resume uses retained
   project/diagnostic state; it cannot recreate a host's hidden model state.

Detailed design references:

- [Product charter](docs/product/PRODUCT_CHARTER.md)
- [Scope and long-term requirements](docs/product/SCOPE_AND_ROADMAP.md)
- [Security invariants](docs/security/SECURITY_INVARIANTS.md)
- [Repository workflow](docs/engineering/REPOSITORY_WORKFLOW.md)

The [generated alpha's delivery index](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/alpha/generated-project-editing/docs/product/REAL_PROJECT_DELIVERY.md)
contains its adapter-specific operational boundaries. Historical phase and
reusable-approval plans remain useful references, but do not override the
current evidence and acceptance order above.
