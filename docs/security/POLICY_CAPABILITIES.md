# Policy and Capability Model

Status: DECIDED direction.

Policy is deny-by-default and target/capability based, not a command blacklist.

## Decision

Allow | RequireApproval | Deny

## ActionEnvelope

Every executable effect contains typed action, canonical target, SessionHandle/TaskLease identity, project grant, expected hashes/base revision, requested resources/network, reversibility class and policy epoch.

## Capability and task leases

A session receives narrow server-side leases with scope, ceilings, expiry and revocation. The AI may request a lease but cannot mint, widen, renew or approve one itself.

Possession of an opaque handle never bypasses caller/project/policy validation.

## Hard denials

Normal AI tools cannot mutate security policy, elevate privilege, control arbitrary PIDs, escape granted roots or perform destructive system operations.

Network access is a separate capability, not ambient permission inherited by every process.

## Approval

Approval is one-shot and bound to exact normalized action/target/preconditions/time window. A model-generated “yes” is never approval.

## Risk budget

Do not let an AI-generated probability/score authorize actions. Risk is represented deterministically as capability boundaries and resource budgets. Crossing a boundary requires policy/approval.

## Explain / dry-run

policy_explain/dry-run should return normalized action, target class, policy decision/rule and required capability without executing it. Explanations are diagnostic; policy code remains authoritative.

## MCP annotations

Tool annotations such as read-only/destructive hints can improve UX but are not trusted security controls. Optic derives enforcement from its own tool/action definitions and policy.
