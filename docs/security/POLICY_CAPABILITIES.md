# Policy and Capability Model

Status: DECIDED direction.

Policy is deny-by-default and target/capability based, not a giant command blacklist.

## Decision

Allow | RequireApproval | Deny

## Typed action examples

FileRead, FileSearch, FileWrite, FileDelete, GitRead, GitIntegrate, ProcessRun, NetworkAccess, PolicyChange, PrivilegeElevation.

Each action includes canonical target, session, project grant, relevant hashes/preconditions and resource request.

## Capability grants

A session receives only capabilities needed for its granted project/task. PROPOSED capability leases include expiry, scope and revocation. Capabilities are not bearer strings that bypass policy; possession is one input to authorization.

## Hard denials

Policy/security modification by an AI session, privilege elevation, arbitrary process/PID control, escape from granted roots, and destructive system operations are denied in V1.

## Approval

Approval is one-shot and bound to the exact action/target/preconditions/time window. A model-generated “yes” is never user approval.

## Explain/dry-run

PROPOSED: policy_explain/dry-run returns the normalized action, target, decision and non-sensitive reason without executing it. This improves debuggability without weakening policy.
