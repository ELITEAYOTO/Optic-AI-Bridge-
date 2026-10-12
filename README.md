# Optic AI Bridge

**Status:** pre-alpha. Phase 1/2 foundations and the major Phase 3 isolation/resource gates are implemented; Phase 4 same-repository work is active. In parallel, the evidence-gated H-track is adding provider-neutral remote-host connectivity, Optic-owned session resolution and bounded session autonomy without weakening the existing Rust security/runtime core.
**Target:** Windows-first, Rust, local-first, lightweight bridge for AI-assisted development.
**Release:** no production-supported release yet.

> **Core rule:** The AI decides what it needs. The bridge executes only authorized effects. Policy authorizes. OS isolation contains.

Optic AI Bridge is intended to give AI hosts such as ChatGPT, FranceStudent and future clients safe access to developer workflows such as project files, code search, Git, builds, tests and supervised local processes—without embedding an LLM and without requiring an Electron/Node runtime for the bridge itself.

The product remains local-first and is being designed so normal student use does **not** require a mandatory paid Optic backend, paid relay or API-credit path. MCP is one adapter, not the product core.

## Current project state

For the fastest accurate handoff, read:

- [`docs/product/CURRENT_STATE_2026-10-12.md`](docs/product/CURRENT_STATE_2026-10-12.md) — compact current state;
- [`ROADMAP.md`](ROADMAP.md) — active gates;
- [`docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md`](docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md) — H-00 through H-10;
- [`docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md`](docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md) — security blockers before real-project remote exposure;
- [`PROJECT_CONTINUITY_PROMPT.md`](PROJECT_CONTINUITY_PROMPT.md) — generic prompt to resume the project in a fresh AI conversation.

Historical implementation detail remains in [`STATUS.md`](STATUS.md). The [repository audit](docs/engineering/REPOSITORY_HEALTH_AUDIT_2026-10-12.md) distinguishes main from drafts and records documentation drift; the [FranceStudent evidence ledger](docs/research/FRANCESTUDENT_REAL_LINK_EVIDENCE_2026-10-12.md) records today's verified synthetic reads, traversal denial, multiple calls and same experimental adapter across conversations.

## Current remote-host work

The accepted provider-neutral direction is:

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

Connectivity providers such as Cloudflare, Tailscale or custom HTTPS remain replaceable adapters outside PolicyEngine. Transport, tunnel identity, request ids and host conversation ids never become project authority by themselves.

### FranceStudent H-01

Draft PR #170 contains the authority-free `h01-v2` probe. Real Windows testing has already proven that FranceStudent can reach a Streamable HTTP `/mcp` endpoint through a temporary Cloudflare Quick Tunnel and execute harmless tools. Two same-conversation calls were corroborated by local server traffic. No trustworthy conversation-specific signal has yet been proven. Testing resumed on 12 October using the separate #178 Real Optic Link; fresh h01-v2 correlation/reconnect/grouped-overlap and auth/header checks remain pending.

The v2 probe also corrects an important evidence problem: one earlier host/model JSON response was later admitted to have been invented without a real tool call, and v1 request-id fingerprints were ambiguous when the SDK request id was absent. Every v2 call now carries a server-minted invocation proof mirrored in local logs.

### Real Optic MCP link experiment

Draft PR #178 proves an experiment-only path:

```text
Streamable HTTP MCP client
  -> bounded Python compatibility adapter
  -> real Rust `optic-bridge` stdio server
  -> disposable synthetic workspace
```

The remote allowlist is exactly session info, bounded list and bounded read. It exposes no process execution, mutation, Git, `session_cancel` or arbitrary upstream tool forwarding.

At experiment head `ef5bc84144670135d13f55a59a782611aeae762f`, dedicated workflow #11 is green on Linux + Windows and normal repository CI #653 is green on dependency policy + Ubuntu + Windows. The automated experiment proves real Optic session/list/read behavior, traversal rejection, successful paired client tasks, per-call invocation proof and HTTP-client reconnect continuity to the same long-lived upstream Optic context. It does not measure actual upstream effect overlap.

This is transport evidence only; it does not prove host-conversation identity or production remote authorization.

### Real-project remote exposure remains blocked

The synthetic bridge must not be pointed at a real repository until the relevant audit gates are resolved, including:

- #176 sensitive content + metadata/listing classification;
- #180 generic file mutation vs protected `.git` control state;
- #181 Unix native-path round-trip safety into `WorkspacePath`;
- #182 `GitRead` vs sensitive-content policy;
- #183 session admission/quiescence across read/list/Git effects.

See [`docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md`](docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md).

## Quick install - ChatGPT Desktop (developer preview)

The local ChatGPT Desktop path was integration-tested on Windows. The default compatibility profile exposes the intended bounded file/Git tools while process/session routes remain outside the normal ChatGPT tool surface. Optional profiles are explicit: Git integration and the proven isolated Node process path remain opt-in.

The end-user installation target is **under 3 minutes** using a prebuilt Windows release bundle:

```powershell
.\Install-OpticAIBridge.ps1 -Workspace "C:\path\to\your\repository"
```

No OpenAI API key, API credits, public tunnel, Python, Docker, Rust compiler or Node.js is required by the **default** local release-bundle path. The installer is user-scoped, detects Git, generates the local MCP configuration, registers the ChatGPT Desktop plugin and runs an MCP self-test. Mutations are limited by explicit authority and `-ReadOnly` mode remains available. Exact-head Git integration and isolated Node execution remain explicit opt-ins.

There is not yet a production-supported release. See [`docs/operations/CHATGPT_DESKTOP_QUICK_INSTALL.md`](docs/operations/CHATGPT_DESKTOP_QUICK_INSTALL.md).

## Implemented security/runtime foundations

The repository started documentation-first and now contains an executable Rust implementation. Major implemented foundations include:

- application-owned sessions, monotonic expiry and revocation;
- independent hard transport/body/concurrency ceilings;
- bounded MCP stdio framing and responses;
- project-scoped bounded `fs_list` / `fs_read`;
- transactional file mutation with exact expected-state preconditions and durable recovery journal;
- bounded Git read and exact-head integration paths;
- structured process lifecycle with opaque JobIds, bounded output and timeout/stop handling;
- operator-owned executable/environment authority;
- pinned executable identity and explicit execution classification;
- deterministic session + exact task-lease policy before execution;
- dedicated Windows containment crate with a narrow Win32/unsafe boundary;
- Windows Job Objects with kill-on-close, process-count, memory and CPU governance;
- proven AppContainer isolation path for the selected high-risk toolchain case;
- native Windows gates for process cleanup/security boundaries;
- ToolProfile authorization and reusable current-session approval primitives;
- same-repository worktree/integration foundations.

## Mutation and Git safety model

Filesystem mutations are not blind writes. They use explicit expected state/content versions, canonical containment and durable journal/recovery semantics. Delete requires exact existing-state authority rather than a blind delete input.

Git integration remains separate from generic file mutation: the dedicated integration path is exact-head/fast-forward oriented and application-owned. Current audit #180 additionally requires generic broad file mutation to reserve protected `.git` control state so it cannot become an alternate route around dedicated Git authority.

## Process safety model

Process execution remains structured as executable + argv, not an opaque shell string. Application/operator-owned authority classifies executable identity and process class. Resource budgets are bounded and Windows isolation/governance is enforced at runtime.

High-risk interpreter/repository-code execution remains subject to the proven isolation path rather than being admitted merely because an executable was configured.

## Reusable approval / autonomy direction

A-08 reusable approval is scoped to one application-owned Optic session, one exact resolved ToolProfile fingerprint, policy epoch and monotonic expiry. Reuse does not skip per-invocation security checks.

A-08E is blocked by host interaction behavior, not by failure of the reusable-approval core. The H-track therefore adds a host-independent local approval path and future session autonomy rather than depending exclusively on host UI.

Future `Project Autonomy` is intended to support realistic:

```text
read -> edit -> build -> test -> inspect -> fix -> repeat
```

without per-action prompt spam, while every concrete action still passes session/lease/policy/path/resource/network/isolation checks. The AI cannot mint or widen its own grant.

## Canonical documentation

Start with [`docs/README.md`](docs/README.md).

The canonical documents are grouped by purpose:

- `docs/product/` — product charter, scope, roadmap, current state and autonomy track.
- `docs/architecture/` — system, sessions, concurrency, transport, memory and recovery.
- `docs/security/` — threat model, policy, Windows isolation, filesystem safety and remote/autonomy audit gates.
- `docs/specs/` — MCP tool contracts, config and errors.
- `docs/engineering/` — implementation, tests, CI/security gates, dependencies and OpticCode reuse.
- `docs/operations/` — install, observability and benchmarks.
- `docs/decisions/` — Architecture Decision Records (ADRs).
- `docs/research/` — source register, experiments, evidence and open questions.

`AI_HANDOFF.md` is the mandatory guardrail for AI coding agents. `PROJECT_CONTINUITY_PROMPT.md` is the generic context-recovery prompt for a fresh conversation.

## Current V1 direction

V1 stays deliberately narrow: Rust daemon/core, provider-neutral host adapters, deterministic local policy, project-scoped filesystem/search/edit primitives, Git primitives, supervised process execution, bounded output, crash cleanup, strong session isolation, zero-cost remote connectivity and locally revocable session autonomy.

Multi-session support remains a first-class requirement. Two chats must be able to work on different projects safely; parallel work on the same repository must use isolated Git worktrees and explicit integration/conflict gates rather than uncontrolled shared writes.

## Status labels used in the docs

- **DECIDED** — accepted architecture direction.
- **PROPOSED** — strong candidate, still needs ADR/PoC.
- **TARGET** — benchmark or resource objective; not a measured fact.
- **RESEARCH** — sourced information awaiting implementation validation.
- **OPEN** — unresolved question.
- **BLOCKED BY HOST** — missing external host evidence, not a failed implementation.

Current host/autonomy snapshot baseline: **2026-10-12**.
