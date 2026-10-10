# Host compatibility, zero-cost connectivity and session autonomy track

Status: LIVING SUBTRACK. Last reviewed: 2026-10-10.

This subtrack records the product and architecture work required to make Optic AI Bridge usable by large numbers of students across multiple AI clients without coupling the core runtime to one host, one transport provider, one paid service or one host-specific approval UI.

The existing core security/runtime work remains authoritative. This track changes how hosts connect, how host activity is mapped into Optic-owned sessions, and how a human can grant bounded autonomy without being prompted for every normal development action.

## Product invariants

1. **Optic itself must remain free to use.** The project must not require an Optic subscription, paid relay, paid hosted backend, OpenAI API credits or another developer-funded server for normal operation.
2. **The student must not need a paid connectivity service.** Free third-party connectivity such as Tailscale or Cloudflare is acceptable. No single provider is mandatory.
3. **MCP is an adapter, not the product boundary.** FranceStudent AI and ChatGPT are V1 host targets, but Optic core must remain usable through additional adapters if a host does not expose the required MCP capabilities on a given plan/surface.
4. **The bridge remains local-first.** Project data and execution stay on the student's machine unless an explicitly authorized action requires otherwise.
5. **Connectivity is never authority.** A stdio connection, HTTP endpoint, tunnel URL, bearer token, host conversation id or provider account must never mint project/process/file authority by itself.
6. **Trust is session-scoped for V1.** A human may grant autonomy for the current Optic session/chat mapping. The internal model must remain extensible so a later release can add longer-lived grants without changing the execution-policy core.
7. **Normal trusted development must not become a prompt loop.** Once a bounded autonomy grant is active, repeated reads, edits, builds, tests and approved toolchain use inside its scope should not require repeated human confirmation.
8. **Revocation must remain immediate and local.** The user must be able to stop/revoke a session from Optic even if the remote host remains connected.

## UX direction from the existing Figma product concept

The current Figma concept already contains the product structure needed by this track:

- onboarding with project/client/connection selection;
- a provider-independent connection concept and advanced custom HTTPS path;
- FranceStudent connection management;
- project permissions and security profile screens;
- session/activity views and explicit session revocation;
- diagnostics and provider recovery;
- a floating Windows companion capable of surfacing approval-required, connection and project states.

The key product rule expressed by the mockup remains canonical:

> **The connection transports requests. The profile decides rights.**

The desktop application is therefore not a prerequisite for proving the transport/session architecture, but its contracts must be considered now. Implementation of the full visual application remains later work after the host/session/autonomy service boundaries are stable.

## Architecture direction

```text
AI host
(ChatGPT / FranceStudent / future client)
        |
        v
Host adapter
(MCP stdio / MCP Streamable HTTP / future adapter)
        |
        v
Connection authentication + HostContext
        |
        v
SessionResolver
(host context -> Optic-owned SessionHandle)
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

Transport providers sit beside the host adapter, not inside the authorization model:

```text
MCP Streamable HTTP
        |
        +-- direct/local
        +-- Tailscale
        +-- Cloudflare
        +-- custom HTTPS provider
        +-- future provider
```

A provider can make an endpoint reachable. It cannot decide what that endpoint may do.

## HostContext and session mapping

Every host adapter should normalize its observable, non-authoritative metadata into a bounded `HostContext`. The exact shape is still a design task, but the abstraction should be able to carry values such as:

- adapter kind;
- authenticated connection principal;
- host-provided conversation/session correlation when available;
- connection instance/reconnect generation;
- target project selected locally;
- capability metadata advertised by the host.

`SessionResolver` then maps this context to an opaque Optic-owned `SessionHandle`.

Host-provided ids are correlation inputs only. They are not capabilities, leases or bearer authority.

## Autonomy model

V1 should support human-selected session policy modes rather than asking for every action individually.

### 1. Review

Conservative mode. Read-only activity can be broadly allowed within the selected project. Mutations, execution or other sensitive effects may require a local review according to policy.

### 2. Project Autonomy

Recommended trusted-development mode. The current session may autonomously perform the normal development loop inside one selected project and within bounded toolchain/policy limits:

- read/search project files;
- write/patch/delete where the project profile permits it;
- Git read and approved integration workflows;
- start approved project toolchains;
- build/test/read outputs;
- repeat those operations without per-action confirmation while all policy/profile/resource/isolation checks continue to pass.

This mode does **not** mean unrestricted machine access.

### 3. Full Access (advanced / dangerous)

An intentionally high-authority session mode may be offered later for users who explicitly opt in, but the label must never imply that Optic disables its own hard safety boundary.

Even in this mode, some effects remain hard-gated or unavailable unless separately designed and proven, including at minimum:

- changing Optic's own security policy from the remote AI;
- silently escalating administrator privileges;
- modifying protected Windows/security/credential boundaries without a dedicated contract;
- bypassing isolation/resource ceilings;
- minting new authority from the AI itself;
- disabling audit/revocation mechanisms.

The UI should clearly communicate the increased risk. A disclaimer is not a substitute for technical containment.

## Tool execution: hybrid model

Exact ToolProfiles remain useful, but they should not force a confirmation maze.

The planned direction is a hybrid execution catalog:

1. operator/application-owned executable identity stays pinned and classified;
2. known development toolchains can be discovered/configured locally;
3. structured executable + argv remains the process primitive; opaque shell strings do not become the fundamental API;
4. session autonomy can authorize repeated invocations of an approved toolchain envelope within its project, resource and isolation limits;
5. a genuinely new executable/toolchain can require one local decision to add it to the current session's bounded catalog, after which normal repeated use does not prompt again;
6. high-risk interpreters/repository code still require the proven isolation path.

This preserves the security work already completed while allowing realistic `build -> test -> inspect -> fix -> repeat` loops.

## Approval UX direction

MCP elicitation remains a compatibility option, not the only approval mechanism.

The long-term human authority should be able to live locally in Optic (desktop app/widget, with a later headless fallback). This avoids depending on whether a specific AI host renders nested elicitation UI correctly.

A future local approval broker may expose an opaque pending request contract such as:

```text
remote action requires authority
        -> Optic creates bounded pending request id
        -> local Optic UI displays the request
        -> human accepts/rejects / chooses session scope
        -> Optic revalidates session + lease + policy + profile + expiry
        -> grant is issued only after successful revalidation
```

The pending request id itself carries no authority and must be bounded, expiring, single-owner and replay-safe.

## Existing evidence

### FranceStudent Streamable HTTP proof

A prior isolated test used a deliberately harmless Python MCP server with only `ping` and `hello`, exposed as:

- MCP Streamable HTTP;
- `/mcp` endpoint;
- stateless HTTP mode;
- JSON responses;
- temporary Cloudflare Quick Tunnel for reachability.

That test is accepted as evidence that the basic FranceStudent remote-MCP transport path is viable on the tested environment. It does **not** yet prove Optic authentication, session correlation, reconnect semantics, parallel calls, local approval or autonomy.

### A-08 reusable approval proof

A-08A through A-08D remain valid core/runtime work. The direct real-binary proof shows reusable profile authority works inside Optic. A-08E is now treated as **host-interaction blocked**, not as a failure of the reusable approval model: current host surfaces did not provide the expected reusable approval interaction consistently.

The reusable broker should be preserved and reused by the later local-approval/session-autonomy design where appropriate.

## Gates before production refactor

### H-00 — evidence and architecture freeze — CURRENT

- record zero-cost/product invariants;
- preserve existing core security boundaries;
- record FranceStudent Streamable HTTP evidence;
- record host-interaction limitations discovered during A-08E;
- define the host-adapter/session/autonomy research gates;
- make no production transport/runtime refactor yet.

### H-01 — harmless host capability probe

Build a separate, non-privileged experiment with no filesystem, Git or process authority. Suggested tools:

- `probe_ping`;
- `probe_host_context`;
- `probe_session_correlation`;
- `probe_parallel`;
- `probe_approval_capability`.

Measure on each available host/surface:

- tool discovery and invocation;
- Streamable HTTP compatibility;
- request metadata actually delivered to the server;
- same-chat correlation across distinct calls;
- reconnect behavior;
- new-chat behavior;
- parallel/concurrent calls;
- host confirmations;
- elicitation support/behavior if advertised;
- request cancellation/timeouts.

Targets: FranceStudent first; ChatGPT normal Chat/Work only where the user's plan/surface actually permits connection.

### H-02 — transport/authentication proof

Using the harmless probe:

- prove a local Streamable HTTP server behind at least one zero-cost provider;
- preserve provider replaceability;
- add real endpoint authentication separate from tunnel identity;
- reject missing/invalid/revoked credentials;
- test replay/basic rate/size bounds;
- test provider reconnect without silently preserving invalid authority.

Cloudflare and Tailscale are reference providers, not mandatory dependencies.

### H-03 — SessionResolver contract

Define and test:

- bounded `HostContext`;
- opaque Optic session creation;
- same-conversation/session correlation rules per adapter;
- reconnect behavior;
- new-chat separation;
- cross-host separation;
- revocation/reap;
- no authority from caller-supplied ids alone.

Only after this gate may user-facing wording claim a host conversation maps reliably to an Optic session for that adapter.

### H-04 — session autonomy grant model

Define transport-agnostic authority for `Review`, `Project Autonomy` and the advanced high-authority profile.

Acceptance conditions:

- one explicit human/local policy choice can cover multiple distinct actions in the current session;
- every action is still independently revalidated;
- project/toolchain/resource/isolation drift does not silently inherit authority;
- session revoke immediately blocks later reuse;
- no AI action can widen its own autonomy grant;
- audit records distinguish automatic use from newly granted authority.

### H-05 — local approval broker proof

Before building the full desktop application, prove a minimal local approval service contract independent of MCP elicitation.

The proof may use a minimal developer UI/console harness initially, but the contract must be suitable for the planned desktop app/widget.

### H-06 — production Streamable HTTP adapter

Only after H-01 through H-05 are green:

- add production-bounded MCP Streamable HTTP beside stdio;
- do not replace the existing stdio adapter;
- resolve per-request/per-host activity through `SessionResolver` instead of binding the entire server instance to one startup session;
- keep TransportGuard-equivalent limits at the new boundary;
- preserve the transport-agnostic application/runtime services.

### H-07 — FranceStudent V1 adapter

Close the first supported remote-host path end to end:

- authenticated connection;
- project selection locally;
- session correlation;
- read/mutation/process surface according to the selected profile;
- session autonomy;
- local revoke;
- reconnect and stale-session negative tests;
- zero-cost setup documentation.

### H-08 — ChatGPT adapter

Support ChatGPT only on surfaces/plans that actually expose the required compatible path. Do not make ChatGPT availability a requirement for Optic itself.

If a ChatGPT surface cannot provide the required MCP path, keep the core ready and use/consider another adapter rather than weakening Optic's authorization model.

### H-09 — provider adapters and setup automation

Productize provider setup behind one interface:

- Tailscale reference adapter;
- Cloudflare reference adapter;
- custom HTTPS/manual adapter;
- future providers without core changes.

The desktop app should eventually detect/configure the easy path while still exposing an advanced custom-provider route.

### H-10 — desktop application implementation

Begin the full visual application only after the host/session/autonomy contracts are stable enough to avoid redesign churn.

The existing Figma file remains the product-design baseline. Expected implementation areas include onboarding, projects, connections, sessions, activity, security, diagnostics, settings, notifications and the floating widget.

## Explicit non-goals for the current gate

Do not yet:

- replace stdio with HTTP in production;
- delete A-08 reusable approval code;
- weaken `ActionEnvelope`, lease, policy, ToolProfile or isolation checks;
- add unrestricted shell execution;
- bind authority to a tunnel URL or host conversation id;
- begin the full desktop UI implementation;
- promise ChatGPT Free/Plus compatibility without a real compatible host path;
- require Tailscale or Cloudflare as the only provider.

## Definition of done for this track

This track is complete only when at least one supported remote student host can connect through a zero-cost setup, map into an Optic-owned session, receive a bounded user-selected autonomy profile, perform a realistic development loop without repeated approval spam, and be locally revoked without weakening the existing policy/isolation boundaries.
