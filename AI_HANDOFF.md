# AI Coding Handoff — Mandatory Guardrails

Read this file before changing architecture or implementing features.

For the current active project state, also read:

- `docs/product/CURRENT_STATE_2026-10-12.md`;
- `docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md`;
- `docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md`;
- `PROJECT_CONTINUITY_PROMPT.md` when recovering from a lost/expired AI conversation.

## Non-negotiable invariant

**The AI decides what it wants to attempt. The bridge executes only authorized effects. Policy authorizes. Isolation contains.**

## MUST

- Preserve transport/core separation and the TransportGuard.
- Treat MCP `2026-07-28` transport as stateless; use Optic-owned application `SessionHandle`s for stateful resources.
- Treat host conversation/session/request ids as correlation hints only unless a host-specific proof establishes their semantics; they are never capability authority.
- Keep tunnel/provider identity outside project authorization. Connectivity is not authority.
- Keep endpoint authentication separate from project/session/autonomy authority.
- Route every executable effect through a typed `ActionEnvelope` carrying one typed `Effect` variant.
- Derive target/precondition/reversibility semantics from the `Effect`; do not reintroduce independently combinable action/target fields.
- Enforce session/task ownership on every resource.
- Keep capabilities narrow, expiring and revocable.
- Keep task leases explicitly scoped to the workspace/repository/executable/network boundary they authorize.
- Evaluate active authorization TTLs/deadlines using a monotonic time source at runtime.
- Require both session and task-lease `NetworkAccess` plus explicit network scope for leased process egress.
- Bound every long-lived collection, protocol body, queue, output, spool and remote-adapter admission path.
- Use explicit expected state/hash/version preconditions for mutations; create intent must explicitly require absence.
- Keep same-repo write sessions in separate worktrees.
- Treat repository/log/network/host content as untrusted sources.
- Gate sensitive sinks (network, secrets, external writes, policy/privilege) deterministically.
- Keep sensitive-path classification explicit; a scope type alone is not proof that secret content/metadata is filtered.
- Ensure native filesystem paths are converted into authority-bearing logical paths losslessly and canonically.
- Keep generic file mutation from becoming an alternate Git-control path around dedicated Git authority.
- Audit cross-capability composition: filesystem, Git and process tools must not bypass one another's security policy.
- Hold session admission across effects that must participate in revoke/quiescence guarantees.
- Test Windows process cleanup/security boundaries natively.
- Add/update an ADR when an architectural/security invariant changes.
- Pair external-host evidence with server-side proof when the host/model can fabricate or summarize tool results.

## MUST NOT

- Add Electron or require Node/Python/Docker for core.
- Embed a required LLM.
- Let AI grant/approve permissions or use AI risk scores as authorization.
- Trust a client-supplied session ID merely because it is syntactically valid.
- Treat MCP connection state, request ids, Cloudflare headers, tunnel URLs or provider account identity as Optic session authority.
- Put production access tokens in URL query strings.
- Expose `session_cancel` as an ordinary shared remote tool before remote ownership/control-plane semantics are defined.
- Expose a real project through the synthetic PR #178 adapter while remote audit gates #176 and #180–#183 remain unresolved.
- Expose arbitrary PID kill.
- Use opaque shell strings as the fundamental process API.
- Allow unbounded request bodies, stdout/stderr, histories, queues, caches or responses.
- Call Job Objects a full sandbox.
- Silently overwrite stale files or interpret a missing mutation precondition as overwrite permission.
- Authorize a path by naive string-prefix comparison.
- Share mutable cwd/environment across sessions.
- Give child processes ambient secrets/network without explicit capability and scope.
- Let normal tools mutate security policy.
- Couple core execution to one ChatGPT/OpenAI/FranceStudent transport.
- Assume upstream SDK limits are sufficient.
- Import OpticCode Java/RAG/editor layers for convenience.
- Expand scope without Product Charter/ADR updates.
- Implement research candidates early just because they sound useful; add them at the service boundary that can prove their value.
- Convert a host outage or missing UI capability into a fake implementation failure or speculative workaround.

## Current remote-host rule

The intended provider-neutral shape is:

```text
AI host
  -> host adapter
  -> connection authentication + bounded HostContext
  -> SessionResolver
  -> Optic-owned SessionHandle
  -> application services
  -> ActionEnvelope / capabilities / leases / policy
  -> runtime / Windows containment
```

If H-01 cannot prove a trustworthy host conversation-correlation signal, H-03 must use an Optic-minted explicit application handle rather than inventing a transport session.

MCP remains one adapter, not the product boundary.

## Current experiment rule

The real Optic Streamable HTTP link in draft PR #178 is **synthetic-only**. It is useful because it exercises the actual Rust stdio application server through a replaceable remote compatibility adapter, but it deliberately exposes only session info, list and read against an adapter-created temporary workspace.

Do not widen that experiment to real repositories, process execution, mutation, Git or generic upstream forwarding merely because the transport path works.

## Before coding

Identify:

- canonical doc and owning issue/ADR;
- security invariants touched;
- `Effect` / `ActionEnvelope` shape;
- session/task ownership;
- lease scopes;
- sensitive-content and protected-control namespace interactions;
- resource ceilings and remote admission bounds;
- cancellation/revocation/cleanup and quiescence semantics;
- crash recovery / uncertain outcomes;
- platform-specific path/isolation behavior;
- tests and exact-head CI gate;
- whether required host evidence is actually known or still blocked.

If required information is missing, audit/document the missing evidence instead of writing speculative production code.

## Definition of done

Formatting/lint/tests pass; security regression tests exist; no unbounded state was introduced; docs/contracts/issues/PR body match code; Windows-specific behavior is tested on Windows; remote-host claims are backed by real server evidence; `STATUS`/current-state/roadmap/CHANGELOG are accurate where applicable; benchmark claims remain TARGET until measured; draft/evidence-gated PRs remain draft until their stated exit criteria are satisfied.