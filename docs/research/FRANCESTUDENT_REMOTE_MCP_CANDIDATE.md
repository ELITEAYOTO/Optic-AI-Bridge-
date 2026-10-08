# FranceStudent / Remote MCP candidate

**Status:** research candidate only — not an implementation commitment, not production support.
**Recorded:** 2026-10-08.

## Motivation

Optic may later expose the same application-owned security/runtime core through a remote MCP adapter so a cloud client such as FranceStudent (FS) can use the Optic instance running on the student's own Windows PC.

The intended product property is local-first and zero mandatory maintainer infrastructure: the user's PC keeps the workspace, CPU/RAM, state and Optic authority. A tunnel, if used, transports HTTPS only and must never become an authorization boundary.

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

This is evidence that the tested FranceStudent custom-MCP flow can discover and call a tool through Streamable HTTP `/mcp`. It is **not** evidence that production Optic remote access, authentication or high-authority tools are safe or compatible.

## What is not proven yet

Before any implementation or support claim, validate at least:

- the exact FranceStudent "Token ou clé d'accès" request/header format;
- invalid/revoked token behavior;
- reconnect after local bridge/tunnel restart;
- multiple clients and application-principal/session mapping;
- request/response byte ceilings, timeouts and concurrent calls;
- real Optic read/Git tools;
- cancellation/progress behavior where relevant;
- provider lifecycle/ownership and failure handling;
- remote threat model, rate limiting and audit/redaction;
- no authority widening caused by transport or tunnel configuration.

Do not assume `Authorization: Bearer ...` until measured.

## Architecture direction if this candidate is accepted

```text
FranceStudent / other MCP client
        |
        | HTTPS + Streamable HTTP /mcp
        v
Remote MCP adapter (loopback locally)
  - remote authentication
  - transport hard limits
  - remote principal -> application-owned session mapping
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
6. First remote preview should be read-only / Git-read oriented; mutation and process authority remain separate local opt-ins.
7. `Interpreter` / `RepositoryCode` re-admission remains blocked by the existing Phase 3C gates regardless of remote connectivity.
8. Tunnel providers are replaceable adapters outside PolicyEngine.
9. No mandatory Optic-owned VPS/relay/database is introduced for the free path.
10. Removing one tunnel provider must not require changes to core policy, leases or tool semantics.

## Transport candidate

Primary candidate: MCP **Streamable HTTP `/mcp`**.

Do not implement legacy `/sse` only because an example/placeholder mentions it. Add compatibility transports only when a real important client requires them.

## Connectivity / zero-cost candidate

The current research package proposes interchangeable options rather than a permanent dependency:

- Cloudflare Quick Tunnel for prototype/onboarding only;
- Tailscale Funnel as a possible more stable user-owned free path, subject to real validation;
- named/custom tunnel or user-provided reverse proxy for advanced users.

No third-party free tier is promised to remain free forever. The durability goal is provider interchangeability and absence of mandatory maintainer-hosted infrastructure.

## Suggested future gates

This candidate must not interrupt the current Phase 3C isolation/toolchain work. If later promoted, split it into narrow gates:

1. documentation / threat model / ADR;
2. loopback-only local Streamable HTTP adapter, off by default;
3. remote principal + token lifecycle (generate/verify/revoke/rotate), with no-secret logs;
4. read-only real Optic MCP smoke locally over HTTP;
5. one provider adapter with owned process/PID lifecycle and doctor;
6. real FranceStudent authenticated smoke;
7. reconnect/concurrency/DoS/adversarial gates;
8. optional second provider to prove provider abstraction;
9. installer/wizard only after the security contract is stable.

Remote mutation/process exposure is a later independent decision and must reuse the existing Optic authority model rather than weakening it.
