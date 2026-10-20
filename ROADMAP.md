# Delivery roadmap

Reviewed: 2026-10-20. [STATUS.md](STATUS.md) owns current evidence; this page owns
the next delivery order. Earlier phase/H-track snapshots remain historical
planning inputs, not a second statement of current completion.

Execution mode for the current continuation: agent-owned isolated environment
and GitHub-hosted CI only; no operator-PC tests or runtime changes. See the
[handoff](docs/engineering/GITHUB_HANDOFF.md). This does not remove native Windows
acceptance gates or imply they can be established on Linux.

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
