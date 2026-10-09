# A-08 — Reusable ToolProfile approval track

Status: LIVING SUBTRACK. Last reviewed: 2026-10-09.

This document records the active A-08 reusable-approval gates while the larger `docs/product/SCOPE_AND_ROADMAP.md` retains substantial historical roadmap material that should not be rewritten wholesale through a contents-API replacement. The security decision remains owned by `docs/decisions/ADR-0011-CONVERSATION_SCOPED_APPROVAL.md`.

## Security boundary

The reusable primitive is scoped to one application-owned Optic session and one exact server-resolved `ToolProfile` fingerprint under the active policy epoch. It suppresses repeated Optic human elicitation only. Every invocation still independently passes session, task-lease, ToolProfile, executable/class, resource, filesystem/network/isolation and deterministic policy checks.

It does not mint capabilities, leases, file/network authority, process eligibility, resource expansion or generic shell authority. It is memory-only in the initial implementation and is invalidated by session lifecycle, policy/profile mismatch or expiry. ChatGPT or another host may still impose independent confirmation UI.

The product must say **current Optic session** until A-08F proves a trustworthy one-to-one mapping between the host's user-visible conversation lifecycle and the application-owned Optic session. A caller-supplied conversation id, MCP connection, tunnel URL or reconnect is not authorization authority.

## Completed gates

### A-08A — bounded domain model — COMPLETE

Reusable approval ids/specs/grants are bounded globally and per session and bind session, exact ToolProfile fingerprint, policy epoch and monotonic expiry.

### A-08B — broker and lifecycle — COMPLETE

The reusable broker has independent capacity from the exact one-shot broker, active-session issue/lookup, owner-scoped revoke/cleanup and lifecycle integration. Server, `session_cancel` and supervisor use the same broker instance. A grant cannot outlive its owning session.

Merged through A-08B1/B2/B3 as `d441ca20`, `1f1cde2a` and `b669ca76`.

### A-08C — profiled-process consumption — COMPLETE

`process_start_profiled` may satisfy `RequireApproval` from a matching reusable grant only after normal request/profile/lease/policy checks. The reusable lookup holds a live session admission and the exact lease/policy decision is revalidated under that permit before the effect. Fingerprint mismatch falls back to human approval; revoked sessions fail before prompt reuse.

A-08C1/C2 merged as `e3d3ff53` and `0034d54d`. Exact C2 head `6305413c` passed Ubuntu, Windows, dependency policy and the native product smoke matrix.

### A-08D — bounded MCP choice and issuance — COMPLETE

A-08D was deliberately split so untrusted host-form content could not become authority before the transport and parser boundary was hardened.

- **D0 / PR #164 / merge `7a0c5cb6`:** Optic's bounded stdio transport parses the already-bounded JSON line before RMCP and rejects contradictory JSON-RPC shapes such as `result + error` or `method + result/error`. This compensates for the known RMCP parser ambiguity before elicitation content becomes security-significant.
- **D1 / PR #165 / merge `7f4b592a`:** standard MCP form elicitation exposes one required enum field `approval_scope` with stable values `once` and `current_session`; `once` is the least-authority default. Missing, malformed, unknown or extra accepted content degrades to one-shot and can never request reusable authority. Unsupported hosts remain fail-closed.
- **D2 / PR #166 / merge `d8c8a545`:** after a human scope choice, Optic acquires a fresh session admission and revalidates the active task lease plus the same deterministic `RequireApproval` reason. `once` keeps exact one-shot issue+consume behavior. `current_session` issues only the exact server-resolved ToolProfile grant, with policy epoch derived from the active session and expiry bounded to the session. Distinct-ActionId integration tests prove current-session approval suppresses the second prompt for the same profile, while one-shot and ambiguous accepted content do not mint a reusable grant.

Exact D2 head `30d4d82c` passed Ubuntu format/Clippy/tests, Windows Clippy/tests, dependency policy, AppContainer loopback proof, isolated Node doctor/admission, profiled-process approval, Git MCP, ChatGPT installer-profile validation and Desktop artifact round-trip before merge.

## Current gate

### A-08E — real ChatGPT Desktop reusable-approval proof — CURRENT

The gate is not satisfied by CI or by earlier one-shot Desktop smokes. It requires a real installed ChatGPT Desktop session to prove the actual host interaction and lifecycle:

1. an allowed exact ToolProfile invocation surfaces the bounded Optic scope choice;
2. selecting `current_session` succeeds;
3. a second distinct invocation of the same exact ToolProfile succeeds without another Optic elicitation;
4. a different ToolProfile does not reuse the grant;
5. a changed policy epoch does not reuse the grant;
6. a revoked/cancelled session cannot reuse the grant;
7. an out-of-profile invocation remains denied or separately approval-gated;
8. independent verification shows no authority was added outside the intended session/profile boundary.

Host-level confirmations imposed independently by ChatGPT are recorded separately and are not treated as an Optic failure or as evidence that Optic can suppress them.

## Later gate

### A-08F — conversation-binding proof — PENDING

Measure the real Desktop host lifecycle, reconnect behavior and plugin/session ownership. Only if a trustworthy adapter-owned one-to-one mapping is demonstrated may UI/docs replace **current Optic session** with **this conversation**. Otherwise the session wording remains permanent.

File mutation and Git mutation reusable approvals remain separate future work; A-08 does not automatically extend session/profile process approval to those sinks.
