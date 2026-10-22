# Delivery roadmap

Reviewed: 2026-10-22. [STATUS.md](STATUS.md) owns current evidence; this page owns
the next delivery order. Earlier phase/H-track snapshots remain historical
planning inputs, not a second statement of current completion.

Execution mode for the current continuation: agent-owned isolated environment
and GitHub-hosted CI only; no operator-PC tests or runtime changes. See the
[handoff](docs/engineering/GITHUB_HANDOFF.md). This does not remove native Windows
acceptance gates or imply they can be established on Linux.

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
