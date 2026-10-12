# ADR-0012 — Provider-neutral remote host boundary

**Status:** ACCEPTED DIRECTION / IMPLEMENTATION GATED  
**Date:** 2026-10-10  
**Last reviewed:** 2026-10-12

## Context

Optic already establishes several relevant architectural decisions:

- ADR-0002 keeps MCP as an adapter rather than the application core;
- ADR-0006 makes developer sessions application-owned `SessionHandle`s rather than MCP transport sessions;
- ADR-0007/0009 keep effect authority in typed envelopes and scoped leases;
- ADR-0011 adds bounded reusable session/profile approval without turning transport/chat identity into authority.

The project now needs to support remote AI hosts such as FranceStudent and, where the user's ChatGPT surface exposes a compatible path, ChatGPT itself. The product must remain usable by students without an Optic subscription, maintainer-funded relay/VPS or mandatory paid connectivity service.

A harmless real FranceStudent test proved that MCP Streamable HTTP `/mcp` can reach a local Windows probe through a temporary Cloudflare Quick Tunnel. A-08E separately showed that host interaction UI cannot be assumed even when server-side reusable approval is correct.

MCP `2026-07-28` is stateless at protocol level. A transport connection therefore cannot truthfully stand in for a user-visible AI conversation or Optic developer session.

Subsequent evidence strengthened the original decision:

- two real same-conversation FranceStudent calls were corroborated by local server traffic, but no conversation-specific signal has been proven;
- one host/model JSON response was later admitted to have been invented without a real tool call, so external host output alone is not accepted as invocation proof;
- the old H-01 v1 request-id fingerprint was ambiguous because request ids are optional;
- H-01 v2 now uses explicit request-id presence and per-call server-minted invocation proof;
- draft PR #178 proves a bounded Streamable HTTP compatibility adapter can drive the **real Rust `optic-bridge` stdio application server** against a disposable synthetic workspace on Windows and Linux;
- a completely new HTTP/MCP client connection can reconnect to the same adapter process and continue to reach the same long-lived upstream Optic application session.

The architecture problem is therefore not “replace stdio with a tunnel”. It is to add host/connectivity adapters while preserving application-owned authority and evidence-driven session semantics.

## Decision

Optic uses a **provider-neutral, host-adapter boundary** for remote AI connectivity.

```text
AI host
(ChatGPT / FranceStudent / future client)
        |
        v
Host adapter
(MCP stdio / MCP Streamable HTTP / future protocol)
        |
        v
Connection authentication + bounded HostContext
        |
        v
SessionResolver
(HostContext -> Optic-owned SessionHandle)
        |
        v
Optic application services
        |
        v
ActionEnvelope + capabilities + leases + policy
        |
        v
Runtime + Windows containment
```

Connectivity providers remain outside the authorization core:

```text
Streamable HTTP endpoint
        |
        +-- Tailscale
        +-- Cloudflare
        +-- custom HTTPS/reverse proxy
        +-- future provider
```

### MCP is not mandatory

MCP remains one external adapter, not the Optic product boundary. If a host/plan does not expose the capabilities Optic needs, Optic may add another adapter rather than weaken the security model or make that host mandatory.

FranceStudent and ChatGPT remain V1 targets, but support claims are surface-specific and evidence-based.

### Zero-cost default path

Normal student operation must not require:

- an Optic subscription;
- OpenAI API credits merely to operate the bridge;
- an Optic-maintained paid relay/VPS/database;
- a mandatory paid tunnel provider.

Free user-owned connectivity such as Tailscale or Cloudflare is acceptable. No third-party free tier is assumed permanent; provider interchangeability is part of the architecture.

### Authentication is not project authority

A remote HTTP adapter authenticates the remote connection/principal before real Optic effects are exposed.

Successful authentication yields only an application-owned remote principal/context. It does not mint:

- project capability;
- task lease;
- filesystem/Git/process authority;
- autonomy grant;
- network grant;
- caller-selected internal `SessionHandle`.

The later `SessionResolver` may use the authenticated principal together with proven correlation inputs to create/resolve an Optic session. Existing capability/lease/policy checks remain authoritative after that.

Tunnel/provider identity, URL secrecy, TLS reachability or provider account ownership is never sufficient Optic authentication/authorization.

Production credentials are never placed in URI query strings or normal logs/tool output.

### HostContext is bounded observation

Each host adapter normalizes only explicitly understood, bounded metadata into `HostContext`, such as:

- adapter identity/version;
- authenticated remote principal;
- protocol revision and relevant client capabilities;
- host conversation correlation only if real evidence establishes its semantics;
- reconnect generation for diagnostics;
- locally selected project reference.

Raw caller metadata does not flow directly into the core and is never capability authority.

### Session continuity is adapter-derived or explicit

Because modern MCP has no protocol session, Optic does not bind application authority to MCP connection/request state or legacy `Mcp-Session-Id`.

For each host adapter, H-03 chooses one of two evidence-backed strategies:

1. use a proven host conversation correlation signal as a non-authoritative mapping key; or
2. if no trustworthy host signal exists, use an opaque Optic-minted application handle threaded across calls.

The second strategy is the required fallback until the first is proven.

In both cases Optic owns the real `SessionHandle`, applies lifecycle/capacity/revocation rules and verifies authenticated principal/project context before reuse.

Only an adapter with a proven one-to-one mapping may describe a grant as applying to “this conversation”. Otherwise the truthful term is “current Optic session”.

### Invocation evidence is not authority

Remote host/model output can be fabricated, transformed or summarized. Diagnostic invocation markers may therefore be used to prove that a request reached a test server, but they carry **zero authority**.

Experiment/probe evidence should pair host-visible results with local/server-side invocation proof where practical.

### Human authority must not depend on host UI

Host confirmation/elicitation behavior is an adapter capability, not the sole human-consent boundary.

Optic will provide a local approval/session-control contract suitable for the planned Windows app/widget. Host interaction mechanisms may integrate with the same application-owned approval domain where safe, but absence/cancellation of host UI must not force a weaker model.

Session autonomy/reusable grants remain responsible for avoiding confirmation spam during normal trusted development.

### Existing stdio application server can remain useful behind adapters

PR #178 proves this experiment-only shape is viable:

```text
remote Streamable HTTP client
  -> bounded compatibility adapter
  -> real Optic stdio application server
  -> real Optic sessions/policy/leases/runtime
```

That proof means production H-06 does not need to be rushed merely to gather real-core host evidence. The final Streamable HTTP boundary may still move into Rust for production, but that is an operational/security decision rather than a prerequisite assumption.

The experiment remains synthetic-only and does not alter this ADR's authority boundary.

### Desktop app follows service contracts

The existing Figma product concept is the UX baseline, but the full desktop application is implemented after host/session/auth/autonomy/local-approval contracts stabilize enough to avoid redesign churn.

The app/widget consumes those contracts rather than defining authority semantics ad hoc.

## Gates

Implementation remains evidence-gated:

- **H-01:** harmless host capability/correlation probe;
- **H-02:** zero-cost endpoint authentication/revocation proof;
- **H-03:** `HostContext -> SessionResolver -> SessionHandle` contract;
- **H-04:** bounded session autonomy without per-action approval spam;
- **H-05:** host-independent local approval broker proof;
- **H-06:** production Streamable HTTP adapter beside stdio;
- **H-07:** FranceStudent V1 end-to-end closure;
- **H-08:** ChatGPT adapter on compatible surfaces;
- **H-09:** provider setup adapters/automation;
- **H-10:** desktop app/widget implementation.

H-06 cannot be pulled forward merely because a tunnel, harmless probe or synthetic bridge works.

## Security consequences

- Remote transport adds an authentication boundary but does not move deterministic policy into transport.
- New host adapters can be added without host-specific branches inside filesystem/Git/process services.
- Provider compromise/reconfiguration does not itself mint Optic project authority.
- Host metadata spoofing is contained to correlation unless independently authenticated/validated.
- Remote reconnect cannot resurrect revoked/expired Optic sessions merely by replaying transport metadata.
- Full/Project Autonomy may improve UX without changing the rule that every concrete effect is independently revalidated.
- A shared remote adapter must not expose local control-plane tools such as `session_cancel` casually; ownership/revoke semantics belong to H-03/H-05.
- Real-project remote exposure remains blocked until cross-capability filesystem/Git/path/revocation audit gates are closed or replaced by an equivalent documented design.

## Additional audit consequences discovered after acceptance

The provider-neutral boundary does not by itself make the existing effect surfaces remotely safe. Before real-project exposure, separate implementation gates now cover:

- sensitive content + metadata/listing classification (#176);
- generic file mutation vs protected `.git` control state (#180);
- lossless native-path -> `WorkspacePath` authorization representation (#181);
- `GitRead` interaction with sensitive-content policy (#182);
- session-admission/quiescence coverage for read/list/Git effects (#183).

These are implementation/security gates under this ADR, not reasons to move policy into the remote adapter.

## Operational consequences

- Production Streamable HTTP requires TransportGuard-equivalent request/response/concurrency/admission bounds at its own boundary.
- Credentials stay out of query strings/logs.
- Authentication strategy may differ per host adapter while producing the same internal principal contract.
- Cloudflare/Tailscale lifecycle belongs to provider adapters/diagnostics, not PolicyEngine.
- Local stdio continues to work while remote adapters mature.
- Test-only Quick-Tunnel host relaxation must remain explicit and synthetic/authority-free.

## Rejected alternatives

- **Mandatory Optic-hosted relay/SaaS:** violates zero-cost/self-hosted student target.
- **Make Tailscale or Cloudflare the architecture:** provider-specific and couples connectivity to core design.
- **Trust tunnel URL/provider identity as authorization:** connectivity is not project authority.
- **Bind Optic session to MCP connection/session/request id:** incompatible with modern stateless semantics and user-visible chat lifecycle.
- **Trust caller-supplied conversation id directly:** unproven/forgeable and still not authorization.
- **Treat reconnect continuity as conversation identity:** a long-lived application session behind reconnect says nothing about the host's user-visible conversation mapping.
- **Move policy into each host adapter:** duplicates security logic and creates inconsistent hosts.
- **Depend only on host elicitation/confirmation UI:** A-08E shows host interaction support is not uniform.
- **Expose the user's real repository through the synthetic adapter just because transport works:** violates the independent security gates above.
- **Build full desktop UI before service contracts:** risks encoding wrong transport/session/approval semantics into the shell.

## Relationship to earlier ADRs

This ADR does not replace ADR-0002, ADR-0006 or ADR-0011.

- ADR-0002 remains the transport-independence rule.
- ADR-0006 remains the application-owned session rule.
- ADR-0011 remains the reusable approval rule.
- ADR-0012 defines how remote hosts/providers/authentication/local consent fit around those boundaries.