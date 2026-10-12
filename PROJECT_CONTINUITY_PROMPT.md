# Optic AI Bridge — generic continuity prompt

Use this prompt when a long ChatGPT/Claude/agent conversation reaches its context limit or when handing the project to a fresh AI session.

Copy the entire block below into the new conversation, then attach or connect the repository if the host can access GitHub.

---

## BEGIN CONTINUITY PROMPT

You are taking over ongoing engineering work on **Optic AI Bridge**, repository:

`ELITEAYOTO/Optic-AI-Bridge-`

Do **not** start coding from assumptions. First inspect the repository, open PRs/issues and current CI, then reconcile this handoff with the real repository state. If this prompt conflicts with newer GitHub evidence, the newer repository evidence wins and you must explain the difference before changing code.

### Core product goal

Optic AI Bridge is a Windows-first, Rust, local-first bridge that lets AI hosts safely use developer workflows on the user's own machine. It must remain usable by students without a mandatory paid Optic backend/relay/API-credit requirement.

Primary V1 host target is **FranceStudent**. ChatGPT is supported only on surfaces/plans that expose the required compatible path. MCP is an adapter, not the product core.

### Non-negotiable security invariant

> **The AI decides what it wants to attempt. The bridge executes only authorized effects. Policy authorizes. Isolation contains.**

Preserve:

- transport/core separation;
- application-owned opaque `SessionHandle` authority;
- typed `ActionEnvelope` effects;
- narrow capabilities + scoped task leases;
- exact expected-state mutation preconditions;
- structured executable + argv rather than arbitrary shell strings;
- hard resource ceilings;
- Windows Job Object/AppContainer containment;
- ToolProfile/executable identity checks;
- local revocation;
- provider/tunnel/host metadata as non-authoritative inputs only.

Never treat any of the following as project authority:

- MCP connection/session values;
- request ids;
- host conversation ids supplied by the caller;
- Cloudflare/Tailscale metadata;
- tunnel URLs;
- provider identity alone;
- invocation/test markers.

### Current architectural direction

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

Connectivity providers such as Cloudflare, Tailscale or custom HTTPS remain replaceable adapters outside PolicyEngine.

### Current repository / PR state to re-check

At the time of this handoff:

- `main` was `156653a406f01383e2fc14cada513963453b04f3`;
- PR #169 was the Draft docs/architecture consolidation branch;
- PR #170 was the Draft H-01 `h01-v2` probe, head `d8ebc0b5d7f46ce61a9aad737bc64690b7ca584b`;
- PR #178 was the Draft synthetic Real Optic Link experiment, head `ef5bc84144670135d13f55a59a782611aeae762f`;
- exact-head CI for those branches had been green before/through the documented evidence.

Do not trust these SHAs indefinitely. Re-check GitHub first.

### Current H-track

- **H-00 complete** — provider-neutral architecture and product/security invariants frozen.
- **H-01 active / evidence-pending** — harmless host capability/correlation probe, draft PR #170, issue #171.
- **H-02 pending** — zero-cost endpoint authentication + credential lifecycle, issue #172.
- **H-03 pending** — bounded `HostContext -> SessionResolver -> SessionHandle`, issue #173.
- **H-04 pending** — bounded session autonomy (`Review`, `Project Autonomy`, advanced high-authority mode), issue #174.
- **H-05 pending** — host-independent local approval broker, issue #175.
- H-06+ are gated behind those proofs/contracts.

Do not jump to production H-02/H-03/H-04 code if required host facts are still unknown.

### H-01 FranceStudent evidence

Earlier real H-01 runs proved:

- FranceStudent accepted MCP Streamable HTTP `/mcp` through a temporary Cloudflare Quick Tunnel;
- real POST traffic reached the local Windows probe;
- `probe_ping` succeeded;
- protocol `2026-07-28` was negotiated;
- no modern transport session id was observed;
- two same-conversation correlation calls were corroborated by server activity;
- no per-call host confirmation UI was observed for those harmless calls.

Evidence integrity rules:

- one FranceStudent model response was later admitted to have been invented without a real tool call and is excluded;
- model text/JSON alone is never accepted as proof when local proof can be collected;
- `h01-v2` adds `probe_invocation` + `server_instance` mirrored in local `[H01] TOOL_CALL` logs;
- old v1 request-id evidence is ambiguous because request ids are optional.

FranceStudent is stable again as of 2026-10-12. Remaining H-01 tests require intentionally reconnecting FranceStudent to the **H-01 v2 endpoint**, not the PR #178 Real Optic Link endpoint:

- clean new-conversation separation;
- reconnect comparison;
- grouped true parallel dispatch with server-side overlap proof;
- optional safe cancellation;
- exact authentication/header behavior.

Conversation identity remains **unproven**. H-03 must keep an Optic-minted explicit application-handle fallback.

### New FranceStudent auth/header UI fact

The current FranceStudent MCP editor visibly exposes:

- `Autorisation` labelled **“Bearer token ou secret”**;
- **“Headers sécurisés (JSON)”**.

This is real H-02 input, but exact wire behavior is still unknown. Do not assume:

- exact `Authorization: Bearer <value>` mapping;
- OAuth semantics;
- custom `Authorization` merge/conflict behavior;
- secure custom-header delivery.

Measure those with harmless values and server proof before implementing production auth assumptions.

### Real Optic MCP Link experiment

Draft PR #178 proves:

```text
Streamable HTTP MCP client
  -> bounded Python compatibility adapter
  -> real Rust `optic-bridge` stdio server
  -> disposable synthetic workspace
```

Remote allowlist is exactly:

- `optic_session_info` -> upstream `session_info`;
- `optic_fs_list` -> upstream `fs_list`;
- `optic_fs_read` -> upstream `fs_read`.

No process, mutation, Git, `session_cancel` or arbitrary upstream forwarding is exposed.

Automated proof on head `ef5bc84144670135d13f55a59a782611aeae762f`:

- dedicated Real Optic MCP Link workflow #11 green Linux + Windows;
- normal CI #653 green dependency policy + Ubuntu + Windows;
- real traversal rejection, bounded concurrent forwarding, per-call proof and HTTP-client reconnect continuity to the same long-lived upstream Optic application session.

### Live FranceStudent -> real Optic proof from 2026-10-12

The live connector used for these tests was a **new/current PR #178 endpoint**, not the old H-01 probe URL.

Accepted server-correlated evidence on adapter instance `20baecc2eeaf`:

- FranceStudent discovered exactly `optic_session_info`, `optic_fs_list`, `optic_fs_read`;
- an absolute Windows path to the user's real Desktop/repository was rejected;
- `optic_fs_read("public/hello.txt")` returned `seq=22`, marker `126b81f5327c1b59`, exactly matching local `[REAL-LINK] TOOL_CALL` output;
- the payload contained the expected synthetic fixture marker;
- `../outside.txt` was rejected with `optic.invalid_workspace_path`, JSON-RPC `-32602` (surfaced as HTTP 400 by FranceStudent);
- two real reads were issued from one FranceStudent turn (`seq=23`, `seq=24`, both matching local markers). This proves two calls in one turn, **not** true temporal overlap;
- one FranceStudent conversation produced `optic_session_info` `seq=25`, marker `16a1e00238f6c7f9`;
- a genuinely new FranceStudent conversation, without restarting the adapter/tunnel, produced `seq=26`, marker `5198a949dec91035`, same `adapter_instance=20baecc2eeaf`, same upstream-visible experimental session-info context.

Safe interpretation:

> Different FranceStudent conversations currently converge on the same long-lived PR #178 adapter/upstream experimental context.

This does **not** discover a FranceStudent conversation id because PR #178 does not instrument H-01 correlation inputs. It does prove that the current experimental bridge itself does not isolate conversations into separate Optic application sessions, which makes H-03 a concrete requirement.

The manual real-link process intentionally stops around 25 minutes, before the current ~30-minute Optic session lifetime.

### Real-project remote access is still blocked

Do **not** expose a real repository through PR #178 until the relevant security gates are resolved:

- **#176** sensitive content + metadata/listing classifier/policy, canonical `fs_list` authorization and cursor progress;
- **#180** generic file mutation must not reach protected `.git` control state and bypass `GitIntegrate`;
- **#181** Unix native path segments must round-trip losslessly into authority-bearing `WorkspacePath`;
- **#182** `GitRead` must not bypass future sensitive-content policy through `git_diff`/`git_status`;
- **#183** `fs_read` / `fs_list` / Git read must hold session admission across the effect so local revoke has truthful immediate/quiescent semantics.

Also keep `session_cancel` out of a shared remote adapter until H-03/H-05 defines ownership/control-plane semantics.

### A-08 reusable approval

A-08A through A-08D are valid implemented work. Reusable approval is tied to one application-owned session + exact resolved ToolProfile fingerprint + policy epoch + monotonic expiry. Every invocation is still revalidated.

A-08E is **blocked by host interaction**, not failed. Do not delete or weaken reusable approval merely because tested host UI did not expose the expected nested approval flow.

A-08F wording such as “this conversation” is behind H-03 evidence. Until proven, use “current Optic session”.

### Product/UX constraints

- Optic itself stays free to use; no mandatory paid Optic relay/backend/API-credit path.
- Free third-party providers are acceptable but replaceable.
- Local-first: workspace data/execution stays on the student's PC unless explicitly authorized otherwise.
- Future `Project Autonomy` should allow realistic `read -> edit -> build -> test -> inspect -> fix -> repeat` loops without approval spam, but it never disables per-effect policy/resource/isolation checks.
- Advanced high-authority mode must still preserve hard safety boundaries and cannot let the AI alter policy, mint authority or bypass containment.
- Human authority must have a local Optic path independent of host elicitation UI.
- Full desktop app implementation comes after service contracts stabilize; existing Figma design is the product baseline.

### Important docs to read first

Read these before changing architecture:

1. `AI_HANDOFF.md`
2. `docs/product/PROJECT_HEALTH_2026-10-12.md`
3. `docs/product/CURRENT_STATE_2026-10-12.md`
4. `ROADMAP.md`
5. `docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md`
6. `docs/security/REMOTE_AUTONOMY_AUDIT_GATES.md`
7. `docs/research/FRANCESTUDENT_REMOTE_MCP_CANDIDATE.md`
8. `docs/research/REAL_OPTIC_MCP_LINK_EXPERIMENT.md`
9. `docs/research/RESEARCH_LEDGER.md`
10. `docs/decisions/ADR-0012-PROVIDER_NEUTRAL_REMOTE_HOST_BOUNDARY.md`
11. `docs/product/A08_REUSABLE_APPROVAL_TRACK.md`
12. historical `STATUS.md` / `docs/product/SCOPE_AND_ROADMAP.md` when older implementation detail is needed.

### Important GitHub objects

At minimum inspect current state of:

- PR #169 — docs/provider-neutral host/autonomy consolidation;
- PR #170 — H-01 probe;
- PR #178 — real Optic MCP link experiment;
- issues #171–#176;
- issues #177 and #180–#183;
- current `main` head and current PR heads;
- latest CI attached to the exact head you intend to change.

### Recommended engineering order

1. Audit current GitHub state and exact-head CI.
2. Finish the missing H-01 v2 host measurements, especially secure-header/bearer behavior and true parallel overlap.
3. In parallel, implement host-independent narrow security fixes that do not depend on FranceStudent semantics: #176 canonical list/cursor progress and #183 admission barrier with deterministic tests.
4. Build the H-02 auth proof harness from measured host behavior.
5. Freeze H-03 SessionResolver + explicit handle fallback + session lifetime/renewal semantics.
6. Only then decide whether to extend the synthetic mutation experiment or move toward H-06 production transport.
7. Keep real project remote exposure blocked until the security gates close.

### Working method

- Audit before coding.
- Identify the exact canonical doc/security invariant/issue owning the change.
- Prefer a narrow experiment or regression proof to speculative production code.
- If required host behavior is unknown, document the missing evidence instead of inventing semantics.
- Keep synthetic experiments synthetic until the real-project security gates are closed.
- After every meaningful change, run/inspect exact-head CI and update docs/issues/PR body.
- Keep PRs draft while their evidence gates are incomplete.
- Never silently merge major architecture changes merely to “make progress”.

### User collaboration style

The project owner is comfortable letting the AI manage GitHub, but PC-side testing is done interactively with the user. Give one clear step at a time for Windows tests, wait for output/screenshots, and do not make the user repeat setup unnecessarily.

### First action in the new conversation

Before implementing anything:

1. inspect GitHub/current repository state;
2. confirm current `main`;
3. confirm state/head/CI of PR #169, #170 and #178;
4. inspect issues #171–#176, #177, #180–#183;
5. read `PROJECT_HEALTH_2026-10-12.md` and compare it with newer issue/PR comments;
6. report any stale/conflicting documentation before coding;
7. continue from the newest server-correlated evidence, not assumptions.

## END CONTINUITY PROMPT

---

Update this handoff whenever a major H-track gate closes, a production remote path is merged, or the remote/audit architecture changes materially.