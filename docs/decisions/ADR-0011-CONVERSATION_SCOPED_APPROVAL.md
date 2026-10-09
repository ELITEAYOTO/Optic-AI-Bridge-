# ADR-0011 — Conversation-scoped reusable approval

Status: ACCEPTED DIRECTION / IMPLEMENTATION GATED  
Date: 2026-10-09

## Context

Optic already has an application-owned `SessionHandle`, scoped task leases, deterministic policy, `ToolProfile` resolution, and an exact one-shot `ApprovalGrant`. A-07D deliberately binds one accepted human elicitation to one exact normalized action and consumes that approval immediately.

That one-shot property is safe, but it is too interruptive for an interactive development chat where the user wants to approve a bounded development profile once and let the same chat continue using that profile without approving every individual command.

The product target is therefore a user-visible choice equivalent to **“allow for this conversation”**. This must work without weakening the transport-independent authorization core and must remain usable later by non-ChatGPT clients such as FranceStudent through a remote MCP adapter.

A tunnel (Tailscale, Cloudflare or another provider), an MCP connection, a URL, and a caller-supplied conversation identifier are transport facts, not authorization authority.

## Decision

Optic will add a second approval form in addition to the existing exact one-shot approval: a **bounded reusable session/profile approval**.

The reusable approval is application-owned. It authorizes repeated policy-approved invocations only when every invocation independently satisfies the existing session, task-lease, profile, scope, isolation, resource and policy checks.

It does **not** mint capabilities, leases, executable authority, file authority, network authority or isolation eligibility.

### Semantic boundary

The core primitive is session-scoped, not transport-chat-scoped.

A UI or adapter may label it “this conversation” only when that adapter has established a trustworthy one-to-one binding between the user-visible conversation and one Optic application session. Optic must never trust a raw caller-supplied chat/conversation id as proof of that binding.

If such a binding cannot be proven, the product must describe the grant truthfully as applying to the current Optic session rather than claiming conversation isolation.

### What a reusable approval binds to

The first implementation target is profiled process execution. A reusable grant binds at minimum to:

- one application-owned `SessionHandle`;
- one resolved server-owned `ToolProfile` identity plus a stable profile fingerprint/version;
- the active policy epoch;
- a monotonic expiry no later than the owning session expiry;
- the profile's executable/class/cwd/read-file/environment/network contract;
- resource ceilings, never caller-selected resource expansion;
- an application-owned principal/project context.

The grant deliberately does **not** bind to one `ActionId`, because each invocation remains a new action with a new server-generated `ActionId` and normal policy evaluation.

### Per-invocation flow

For every invocation covered by a reusable approval:

1. normalize the request;
2. resolve the unique server-owned `ToolProfile`;
3. resolve active session and task lease;
4. evaluate normal deterministic policy;
5. verify the reusable grant still matches the same session/profile fingerprint/policy epoch and has not expired or been revoked;
6. re-check resource, network, filesystem and isolation constraints for this concrete invocation;
7. execute only after admission succeeds.

A reusable approval suppresses repeated human elicitation only. It never bypasses policy.

### Revocation and invalidation

Reusable approvals are bounded in memory and fail closed. They are invalidated on at least:

- session revoke/cancel/expiry/reap;
- policy epoch change;
- ToolProfile replacement or fingerprint change;
- principal/project mismatch;
- explicit user revocation;
- bridge restart for the first implementation;
- adapter loss when that adapter is the proven conversation/session binding owner.

The initial implementation will not persist reusable approvals to disk.

### Remote MCP / FranceStudent

Remote access must reuse this same application-owned approval model.

The future remote adapter is responsible for authenticated remote principal mapping and trustworthy conversation/session lifecycle. Tailscale, Cloudflare, bearer routing, tunnel URLs and provider process lifetime are not sufficient by themselves to mint or preserve reusable approval authority.

A remote reconnect may resume a reusable approval only if the adapter can cryptographically/authentically prove it is resuming the same application-owned session under the same principal/project and the grant remains active. Otherwise it receives a new session and must ask again.

### Host approval remains independent

ChatGPT or another MCP host may impose its own confirmation UI or safety checks. Optic cannot and must not claim that its reusable approval disables host-level confirmations. Where a host exposes a supported permission mode, Optic packaging may integrate with it separately, but Optic's internal security never relies on that host preference.

## Initial implementation gates

1. **A-08A domain model:** reusable approval id/spec/grant with hard global/per-session bounds, monotonic expiry, profile fingerprint and policy epoch. No MCP behavior change.
2. **A-08B broker lifecycle:** issue/lookup/revoke/session-revoke/profile-invalidation with adversarial tests; existing one-shot broker semantics remain unchanged.
3. **A-08C profiled process authorization:** `RequireApproval` may be satisfied by a matching active reusable profile grant after full policy revalidation. No generic shell and no caller-selected profile id.
4. **A-08D elicitation choice:** standard MCP elicitation can request one-shot or session/profile approval where the host supports the interaction; unsupported hosts stay one-shot/fail-closed.
5. **A-08E Desktop proof:** real ChatGPT Desktop smoke proving one user approval can cover multiple distinct invocations of the same allowed profile while a different profile, changed policy epoch, revoked session and out-of-profile invocation still require approval or fail.
6. **A-08F conversation-binding proof:** only after measuring the real host lifecycle may the Desktop UI/docs truthfully say “this conversation”. Until then use “current Optic session”.

File mutation and Git mutation reusable approvals are explicitly separate later gates; they are not automatically covered by profiled-process approval.

## Consequences

- Existing exact one-shot approval remains the safest fallback and continues to work unchanged.
- Reusable approval state becomes another bounded session-owned collection and must participate in session lifecycle cleanup.
- ToolProfile identity/fingerprinting becomes security-relevant and must be stable and deterministic.
- A change to a profile or policy invalidates old approval rather than silently broadening it.
- Remote connectivity can be added without coupling trust to Tailscale or Cloudflare.
- The AI still cannot approve itself, widen a grant, choose its own profile authority or extend expiry.

## Rejected alternatives

- “Always allow everything in this chat”: too broad and not representable safely across clients.
- Trust a caller-supplied ChatGPT/FranceStudent conversation id: forgeable transport metadata unless independently authenticated.
- Reuse the exact `ApprovalGrant` by removing one-shot consumption: would incorrectly turn an exact action authorization into ambient authority.
- Persist reusable approvals by default: unnecessary replay/recovery risk for the first implementation.
- Treat tunnel authentication as policy authorization: violates transport/core separation.
