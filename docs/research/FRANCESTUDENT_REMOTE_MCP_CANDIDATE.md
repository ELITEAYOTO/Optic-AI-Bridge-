# FranceStudent / Remote MCP candidate

**Status:** active H-01 research target — transport feasibility proven in an isolated prototype; production auth/session semantics not yet proven.
**Recorded:** 2026-10-08. **Updated:** 2026-10-10.

## Motivation

Optic may expose the same application-owned security/runtime core through a remote host adapter so a cloud client such as FranceStudent (FS) can use the Optic instance running on the student's own Windows PC.

The intended product property is local-first and zero mandatory maintainer infrastructure: the user's PC keeps the workspace, CPU/RAM, state and Optic authority. A tunnel, if used, transports HTTPS only and must never become an authorization boundary.

FranceStudent and ChatGPT remain V1 product targets where the host surface actually supports the required connection, but MCP itself is an adapter rather than an Optic core requirement.

## Existing external prototype evidence

A separate harmless prototype supplied by the project owner was tested against FranceStudent with only `ping` / `hello` tools and no file, Git, process or Optic authority.

Observed path:

```text
FranceStudent
  -> HTTPS
  -> MCP Streamable HTTP /mcp
  -> Cloudflare Quick Tunnel
  -> 127.0.0.1:8000/mcp
  -> harmless prototype MCP server
```

Observed result for `ping`:

```text
pong - Streamable HTTP /mcp fonctionne
```

This is accepted evidence that the tested FranceStudent custom-MCP flow can discover and call a tool through Streamable HTTP `/mcp`. It is **not** evidence that production Optic remote access, authentication or high-authority tools are safe or compatible.

## H-01 reproducible probe

Draft PR #170 replaces the one-off prototype as the current characterization instrument. The probe remains authority-free and contains no project/filesystem/Git/process/Optic-runtime mutation path.

It exposes:

- `probe_ping`;
- `probe_host_context`;
- `probe_session_correlation`;
- `probe_parallel`;
- `probe_reset`.

It deliberately redacts credential-like headers and returns one-way fingerprints for non-secret correlation candidates. It also records the negotiated protocol version and the client capabilities exposed by the MCP SDK.

Dedicated H-01 CI is green on commit `2645e6b7` for:

- redaction/correlation unit tests;
- Windows PowerShell helper syntax;
- a real local MCP Streamable HTTP server/client integration smoke.

Real FranceStudent evidence is tracked in issue #171. The same run id is kept across same-conversation, new-conversation and reconnect comparisons so the probe can directly report signal changes.

## What is still not proven

Before any production implementation/support claim, validate at least:

- whether FranceStudent can deliver a harmless custom header;
- the exact FranceStudent "Token ou clé d'accès" request/header/auth behavior;
- invalid/revoked credential behavior;
- reconnect after local bridge/tunnel restart;
- same-conversation versus new-conversation correlation signals;
- multiple clients and application-principal/session mapping;
- request/response byte ceilings, timeouts and concurrent calls;
- real Optic read/Git tools only after the earlier gates permit them;
- cancellation/progress behavior where relevant;
- provider lifecycle/ownership and failure handling;
- remote threat model, rate limiting and audit/redaction;
- no authority widening caused by transport or tunnel configuration.

Do not assume a FranceStudent-specific header format until issue #171 measures it.

## Authentication constraints

The production remote endpoint must authenticate independently from Cloudflare/Tailscale/provider identity.

For standards-based HTTP MCP authorization, bearer access tokens belong in the `Authorization` request header and must not be placed in URI query strings. Optic therefore will not adopt `?token=...` URLs for production access.

H-01 may optionally configure a harmless `X-Optic-Probe` header to determine whether the host can transmit custom HTTP headers. This is not a credential and does not authenticate anything. H-02 will decide the production mechanism after that host capability is measured.

If FranceStudent can carry the required authorization header, a simple locally generated/revocable credential or standards-compatible OAuth path can be evaluated without any paid Optic backend. If it cannot, H-02 must evaluate another compatible adapter/auth flow rather than moving secrets into the URL or trusting the tunnel.

## Architecture direction

```text
FranceStudent / other remote host
        |
        | HTTPS + Streamable HTTP /mcp
        v
Remote host adapter (loopback locally)
  - endpoint authentication
  - transport hard limits
  - normalized HostContext
  - HostContext -> Optic-owned SessionHandle resolution
        |
        v
Existing Optic normalization / PolicyEngine / leases / runtime
        |
        +--> filesystem / Git / isolated process paths
```

Rules:

1. The security/execution core stays transport-independent.
2. The local HTTP listener binds loopback by default.
3. A public/tunnel URL is never treated as a secret or authority.
4. Real remote Optic access requires authentication.
5. The remote client cannot mint sessions, capabilities, task leases, scopes or process grants.
6. Host-provided session/conversation ids are correlation inputs only; Optic mints the real `SessionHandle`.
7. Session-scoped autonomy never removes per-action policy/resource/isolation revalidation.
8. `Interpreter` / `RepositoryCode` re-admission remains subject to the existing isolation gates regardless of remote connectivity.
9. Tunnel providers are replaceable adapters outside PolicyEngine.
10. No mandatory Optic-owned VPS/relay/database is introduced for the free path.
11. Removing one tunnel provider must not require changes to core policy, leases or tool semantics.

## Transport candidate

Primary candidate: MCP **Streamable HTTP `/mcp`**.

Do not implement legacy `/sse` only because an example/placeholder mentions it. Add compatibility transports only when a real important client requires them.

## Connectivity / zero-cost candidate

Interchangeable options remain the goal:

- Cloudflare Quick Tunnel for authority-free prototype/onboarding experiments only;
- Tailscale Funnel as a possible stable user-owned free path, subject to real validation;
- named/custom tunnel or user-provided reverse proxy for advanced users;
- future provider adapters without changes to the Optic authorization core.

No third-party free tier is promised to remain free forever. The durability goal is provider interchangeability and absence of mandatory maintainer-hosted infrastructure.

## Current gates

1. **H-00 complete** — product/architecture freeze.
2. **H-01 current** — harmless real-host characterization; automated probe is green, FranceStudent evidence remains issue #171.
3. **H-02 pending** — endpoint authentication/revocation/rate-size-replay proof behind a zero-cost provider.
4. **H-03 pending** — evidence-driven `HostContext -> SessionResolver -> SessionHandle` contract.
5. Later gates add autonomy/local approval before production Streamable HTTP exposes real Optic effects.

Remote mutation/process exposure remains a later independent decision and must reuse the existing Optic authority model rather than weakening it.
