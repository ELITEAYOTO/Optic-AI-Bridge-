# A-08 — Reusable ToolProfile approval track

**Status:** LIVING SUBTRACK  
**Last reviewed:** 2026-10-12

This document records the A-08 reusable-approval gates. The security decision remains owned by `docs/decisions/ADR-0011-CONVERSATION_SCOPED_APPROVAL.md`.

Host/session/autonomy research that became necessary during A-08E is tracked separately in [`HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md`](HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md). A-08A through A-08D remain valid implementation work and must not be discarded merely because one tested host does not expose the expected approval interaction.

For the current remote-host state also read:

- [`CURRENT_STATE_2026-10-12.md`](CURRENT_STATE_2026-10-12.md);
- [`../security/REMOTE_AUTONOMY_AUDIT_GATES.md`](../security/REMOTE_AUTONOMY_AUDIT_GATES.md);
- [`../research/REAL_OPTIC_MCP_LINK_EXPERIMENT.md`](../research/REAL_OPTIC_MCP_LINK_EXPERIMENT.md).

## Security boundary

The reusable primitive is scoped to:

- one application-owned Optic session;
- one exact server-resolved `ToolProfile` fingerprint;
- the active policy epoch;
- a monotonic expiry bounded by the session.

It suppresses repeated **Optic** human elicitation only. Every invocation still independently passes session, task-lease, ToolProfile, executable/class, resource, filesystem/network/isolation and deterministic policy checks.

It does not mint capabilities, leases, file/network authority, process eligibility, resource expansion or generic shell authority. It is memory-only in the initial implementation and is invalidated by session lifecycle, policy/profile mismatch or expiry. ChatGPT, FranceStudent or another host may still impose independent confirmation UI.

The product must say **current Optic session** until an adapter-specific lifecycle proof demonstrates a trustworthy mapping between the host's user-visible conversation lifecycle and an application-owned Optic session. A caller-supplied conversation id, MCP connection, request id, tunnel URL or reconnect is not authorization authority.

## Completed gates

### A-08A — bounded domain model — COMPLETE

Reusable approval ids/specs/grants are bounded globally and per session and bind session, exact ToolProfile fingerprint, policy epoch and monotonic expiry.

### A-08B — broker and lifecycle — COMPLETE

The reusable broker has independent capacity from the exact one-shot broker, active-session issue/lookup, owner-scoped revoke/cleanup and lifecycle integration. Server, `session_cancel` and supervisor use the same broker instance. A grant cannot outlive its owning session.

Merged through A-08B1/B2/B3 as `d441ca20`, `1f1cde2a` and `b669ca76`.

### A-08C — profiled-process consumption — COMPLETE

`process_start_profiled` may satisfy `RequireApproval` from a matching reusable grant only after normal request/profile/lease/policy checks. Reuse holds live session admission, revalidates the exact lease/policy decision and falls back to human approval on fingerprint mismatch. Revoked sessions fail before prompt reuse.

A-08C1/C2 merged as `e3d3ff53` and `0034d54d`. Exact C2 head `6305413c` passed Ubuntu, Windows, dependency policy and native product smokes.

### A-08D — bounded MCP choice and issuance — COMPLETE

A-08D was split so untrusted host form content could not become authority before parser/transport hardening.

- **D0 / PR #164 / merge `7a0c5cb6`:** bounded stdio transport rejects contradictory JSON-RPC shapes before RMCP content is treated as security-significant.
- **D1 / PR #165 / merge `7f4b592a`:** one required enum `approval_scope` exposes `once` and `current_session`; `once` is least authority. Missing/malformed/unknown/extra accepted content degrades to one-shot and cannot request reusable authority.
- **D2 / PR #166 / merge `d8c8a545`:** after human scope choice, Optic reacquires session admission and revalidates active lease + deterministic `RequireApproval` reason. `current_session` can issue only the exact server-resolved profile grant under the active session/policy epoch/expiry.

Exact D2 head `30d4d82c` passed Ubuntu/Windows/dependency policy plus AppContainer, isolated Node, profiled approval, Git MCP, ChatGPT installer-profile and Desktop artifact gates before merge.

## A-08E — real-host reusable approval proof — BLOCKED BY HOST INTERACTION

A-08E was intended to prove that a real host could surface the bounded Optic choice and then reuse the grant across distinct invocations.

The direct real-binary/server-side proof remains valid. Tested host behavior did not consistently expose the nested approval interaction needed to close the UX gate. This is a host-integration limitation, not evidence that the reusable broker/security model failed.

The original evidence targets remain useful:

1. allowed exact ToolProfile invocation surfaces a bounded human choice;
2. selecting session-scoped reuse succeeds;
3. second distinct invocation of same exact profile succeeds without another Optic approval;
4. different profile does not reuse the grant;
5. changed policy epoch does not reuse the grant;
6. revoked/cancelled session cannot reuse the grant;
7. out-of-profile invocation remains denied/separately gated;
8. no authority exists outside the intended session/profile boundary.

Independent host confirmations are recorded separately and are not something Optic promises to suppress.

## A-08F conversation wording moved behind H-03

The former A-08F question is now an adapter-level lifecycle problem.

`H-03 — SessionResolver` owns proof of:

- same-chat correlation where available;
- new-chat separation;
- reconnect behavior;
- cross-host separation;
- explicit Optic-handle fallback when a host supplies no trustworthy conversation signal.

Current FranceStudent evidence still does **not** prove a conversation-specific signal. MCP `2026-07-28` is stateless, old v1 request-id evidence is ambiguous, and one host/model sample was admitted to have been fabricated without a tool call.

Therefore user-facing wording stays **current Optic session** until a specific adapter earns stronger wording through real server-corroborated evidence.

## Relationship to H-04 autonomy

File mutation, Git mutation and broader project-autonomy authority are separate from A-08's reusable profiled-process grant.

H-04 may reuse the same principles:

- exact application-owned scope;
- monotonic expiry;
- policy epoch binding;
- independent per-effect revalidation;
- immediate/local revocation;
- no AI self-escalation.

But H-04 must also respect the remote security gates now tracked in `REMOTE_AUTONOMY_AUDIT_GATES.md`, including sensitive-content policy, protected Git-control metadata, path round-trip safety and revoke/quiescence semantics.

## Relationship to local approval broker

H-05 will provide a host-independent local approval path so reusable/session autonomy does not depend on whether a host renders MCP elicitation correctly.

The existing A-08 domain/broker should be reused where its semantics fit; do not fork a second weaker approval model merely for remote transport.