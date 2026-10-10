# ADR-0012 — Provider-neutral remote host boundary

Status: ACCEPTED DIRECTION / IMPLEMENTATION GATED  
Date: 2026-10-10

## Context

Optic already establishes several relevant architectural decisions:

- ADR-0002 keeps MCP as an adapter rather than the application core;
- ADR-0006 makes developer sessions application-owned `SessionHandle`s rather than MCP transport sessions;
- ADR-0007/0009 keep effect authority in typed envelopes and scoped leases;
- ADR-0011 adds bounded reusable session/profile approval without turning transport/chat identity into authority.

The project now needs to support remote AI hosts such as FranceStudent and, where the user's ChatGPT plan/surface exposes a compatible path, ChatGPT itself. The product must remain usable by students without an Optic subscription, maintainer-funded relay/VPS or mandatory paid connectivity service.

A prior harmless FranceStudent prototype proved that MCP Streamable HTTP `/mcp` could be reached through a temporary Cloudflare Quick Tunnel. A-08E separately proved that host interaction UI cannot be assumed even when the server-side approval model is correct.

MCP revision `2026-07-28` is stateless at the protocol layer. It removes the modern protocol-level session/handshake model, so a transport connection cannot truthfully stand in for a user-visible AI conversation or an Optic developer session.

The architectural problem is therefore not “replace stdio with a tunnel”. It is to add host/connectivity adapters while preserving the application-owned authority boundary.

## Decision

Optic will use a **provider-neutral, host-adapter boundary** for remote AI connectivity.

The intended shape is:

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

Connectivity providers are outside the authorization core:

```text
Streamable HTTP endpoint
        |
        +-- Tailscale
        +-- Cloudflare
        +-- custom HTTPS/reverse proxy
        +-- future provider
```

### MCP is not mandatory

MCP remains one external adapter, not the Optic product boundary. If a host/plan does not expose the MCP capabilities Optic needs, Optic may add another adapter rather than weakening the security model or making that host mandatory for the product.

FranceStudent and ChatGPT remain V1 targets, but support claims are surface-specific and evidence-based.

### Zero-cost default path

The normal student path must not require:

- an Optic subscription;
- OpenAI API credits merely to operate the bridge;
- an Optic-maintained paid relay/VPS/database;
- a mandatory paid tunnel provider.

Free user-owned connectivity such as Tailscale or Cloudflare is acceptable. No third-party free tier is assumed permanent, so provider interchangeability is part of the architecture.

### Authentication is not project authority

A remote HTTP adapter must authenticate the remote connection/principal before real Optic effects are exposed.

Successful authentication yields only an application-owned **remote principal identity/context**. It does not mint:

- a project capability;
- a task lease;
- filesystem/Git/process authority;
- an autonomy grant;
- a network grant;
- an internal `SessionHandle` supplied by the caller.

The later `SessionResolver` may use an authenticated principal together with proven host correlation inputs to create/resolve an Optic session. Existing policy/capability/lease checks remain authoritative after that.

Tunnel/provider identity, URL secrecy, TLS reachability or ownership of a provider account is never sufficient Optic authentication or authorization.

### HostContext is bounded observation

Each host adapter may normalize only explicitly understood, bounded metadata into a `HostContext`, such as:

- adapter identity/version;
- authenticated remote principal;
- protocol revision and relevant client capabilities;
- a host conversation correlation value only if real H-01 evidence establishes its semantics;
- reconnect/connection generation for diagnostics;
- locally selected project reference.

Raw caller metadata does not flow directly into the core and is never capability authority.

### Session continuity is adapter-derived or explicit

Because modern MCP has no protocol session, Optic will not bind application authority to an MCP connection or legacy `Mcp-Session-Id`.

For each host adapter, H-03 may choose one of two evidence-backed correlation strategies:

1. use a proven host conversation correlation signal as a non-authoritative mapping key; or
2. if the host exposes no trustworthy conversation signal, use an opaque Optic-minted application handle that the client/model threads across calls.

In both cases Optic owns the real `SessionHandle`, applies lifecycle/capacity/revocation rules, and verifies the authenticated principal/project context before reuse.

Only an adapter with a proven one-to-one mapping may describe a grant as applying to “this conversation”. Otherwise the truthful product term is “current Optic session”.

### Human authority must not depend on host UI

Host confirmation/elicitation/MRTR behavior is an adapter capability, not the sole human-consent boundary.

Optic will provide a local approval/session-control contract suitable for the planned Windows app/widget. A host interaction mechanism may integrate with the same application-owned approval domain where safe, but absence or cancellation of host UI must not force the core to adopt a weaker approval model.

The local approval path is intended for genuine authority-boundary changes. Session autonomy/reusable grants remain responsible for avoiding confirmation spam during normal trusted development.

### Desktop app follows service contracts

The existing Figma product concept is the UX baseline, but the full desktop application will be implemented after host/session/auth/autonomy/local-approval service contracts are stable enough to avoid redesign churn.

The app/widget will consume those contracts rather than become the source of authority semantics.

## Gates

This decision is implemented only through evidence-gated work:

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

H-06 cannot be pulled forward merely because a tunnel or harmless probe works.

## Security consequences

- Remote transport adds an authentication boundary but does not move deterministic policy into the transport layer.
- New host adapters can be added without creating host-specific branches inside filesystem/Git/process services.
- Provider compromise/reconfiguration does not itself mint Optic project authority.
- Session revocation remains local and effective even while the remote host/tunnel stays connected.
- Host metadata spoofing is contained to adapter correlation unless independently authenticated and validated.
- A remote reconnect cannot resurrect a revoked/expired Optic session merely by replaying transport metadata.
- Full/Project Autonomy can later improve UX without changing the rule that every concrete effect is independently revalidated.

## Operational consequences

- Streamable HTTP production support requires TransportGuard-equivalent request/response/concurrency bounds at its own boundary.
- Credentials must not be placed in URI query strings or logs.
- Authentication strategy may differ per host adapter (for example standards-native OAuth versus an explicitly documented compatibility credential) while producing the same internal principal contract.
- Cloudflare/Tailscale process lifecycle belongs to provider adapters/diagnostics, not PolicyEngine.
- The project can continue supporting local stdio while remote adapters mature.

## Rejected alternatives

- **Mandatory Optic-hosted relay/SaaS:** violates the zero-cost/self-hosted student target and creates unnecessary maintainer infrastructure.
- **Make Tailscale or Cloudflare the architecture:** provider-specific and would make tunnel changes affect core design.
- **Trust a tunnel URL/provider identity as authorization:** connectivity is not project authority.
- **Bind Optic session to an MCP connection/session id:** incompatible with MCP 2026 stateless semantics and user-visible conversation lifecycle.
- **Trust a caller-supplied conversation id directly:** forgeable/unproven unless adapter evidence establishes a safe mapping, and still not authorization.
- **Move policy into each host adapter:** duplicates security logic and creates inconsistent hosts.
- **Depend only on host elicitation/confirmation UI:** A-08E demonstrated that host interaction support is not uniform.
- **Build the entire desktop UI before service contracts:** risks encoding the wrong transport/session/approval semantics into the application shell.

## Relationship to earlier ADRs

This ADR does not replace ADR-0002, ADR-0006 or ADR-0011. It specializes their consequences for multi-host remote connectivity:

- ADR-0002 remains the transport-independence rule;
- ADR-0006 remains the application-owned session rule;
- ADR-0011 remains the reusable approval rule;
- ADR-0012 defines how remote hosts/providers/authentication/local consent fit around those existing boundaries.
