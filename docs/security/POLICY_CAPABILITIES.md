# Policy and Capability Model

Status: DECIDED direction; Phase 0.1 contracts executable.

Policy is deny-by-default and target/capability based, not a command blacklist.

## Decision

Allow | RequireApproval | Deny

## ActionEnvelope and typed Effect

Every executable effect is represented by one typed `Effect` variant. Target, mutation precondition, network intent and reversibility semantics are derived from that variant rather than supplied as independently combinable fields.

`ActionEnvelope` binds the effect to ActionId, SessionHandle/TaskLease identity, requested resources and policy epoch.

Invalid combinations such as a file write targeting a network endpoint should be impossible to represent in the core domain model.

## Capability and task leases

A session receives server-side capabilities. Side-effecting work additionally requires narrow task leases with:

- capability set;
- explicit scope;
- resource ceiling;
- monotonic expiry deadline;
- session ownership;
- policy epoch.

Supported Phase 0.1 scope classes include workspace-all/workspace-prefix, repository, exact process executable and network scopes. Scope matching must be structural/segment-aware rather than string-prefix based.

The AI may request a lease but cannot mint, widen, renew or approve one itself. Possession of an opaque handle never bypasses caller/project/policy validation.

## Mutation preconditions

File writes use an explicit expected state: either the target must be absent or its content must match the supplied ContentVersion. Delete requires the expected current content version. Git integration carries an expected target object id.

There is no implicit blind-write mode in the core effect model.

## Network

Network access is a separate capability, not ambient permission inherited by every process.

For a leased process requesting network access, `NetworkAccess` must be present at both session and task-lease level and the lease must contain an explicit network scope. A session-wide network capability alone is insufficient.

Direct network effects remain approval-gated unless a later ADR defines a narrower deterministic policy.

## Hard denials

Normal AI tools cannot mutate security policy, elevate privilege, control arbitrary PIDs, escape granted roots or perform destructive system operations.

## Approval

Approval is one-shot and bound to exact normalized action/target/preconditions/time window. A model-generated “yes” is never approval.

## Risk budget

Do not let an AI-generated probability/score authorize actions. Risk is represented deterministically as capability boundaries, scopes and resource budgets. Crossing a boundary requires policy/approval.

## Explain / dry-run

`policy_explain`/dry-run should return normalized effect, scope/target class, policy decision/rule and required capabilities without executing it. Explanations are diagnostic; policy code remains authoritative.

## MCP annotations

Tool annotations such as read-only/destructive hints can improve UX but are not trusted security controls. Optic derives enforcement from its own effect definitions and policy.
