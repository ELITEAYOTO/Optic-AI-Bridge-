# Optic AI Bridge — project health / engineering checkpoint

**Checkpoint date:** 2026-10-12  
**Purpose:** one place to answer: what is finished, what is still open, what evidence is missing, what is blocked, and what should be coded next.  
**Scope:** current `main`, documentation PR #169, H-01 PR #170, Real Optic Link PR #178, issues #171–#183.

This is an operational checkpoint, not a replacement for historical `STATUS.md` or the long-form `docs/product/SCOPE_AND_ROADMAP.md`.

## 1. Repository truth at this checkpoint

- `main`: `156653a406f01383e2fc14cada513963453b04f3`.
- PR #169: documentation / provider-neutral host-autonomy consolidation, Draft.
- PR #170: H-01 harmless host capability probe (`h01-v2`), Draft, head `d8ebc0b5d7f46ce61a9aad737bc64690b7ca584b`.
- PR #178: synthetic Real Optic stdio <-> Streamable HTTP link, Draft, head `ef5bc84144670135d13f55a59a782611aeae762f`.
- PR #169 exact-head CI #666 was green before this checkpoint documentation update.
- PR #170 exact-head dedicated H-01 workflow #37 and normal CI #650 were green.
- PR #178 exact-head dedicated Real Optic MCP Link workflow #11 and normal CI #653 were green.

No production Rust merge is implied by this checkpoint. `main` has not moved during the H-track experiments.

## 2. Product/security invariant

> **The AI decides what it wants to attempt. The bridge executes only authorized effects. Policy authorizes. Isolation contains.**

Transport, tunnel identity, host metadata, request ids, conversation ids and test markers are never project authority by themselves.

## 3. What is already finished / proven

### Production-core foundations already implemented

The existing Rust core already has substantial security/runtime foundations:

- application-owned opaque `SessionHandle` lifecycle;
- typed `ActionEnvelope` effects;
- scoped capabilities and task leases;
- bounded memory/CPU/output/process admission;
- structured executable + argv process execution;
- Windows Job Object governance;
- AppContainer-backed high-risk execution path where proven;
- transactional filesystem mutation with expected-state preconditions and recovery journal;
- bounded Git read and separate exact-head Git integration authority;
- ToolProfile/executable identity checks;
- reusable current-session approval primitives;
- owner-scoped revoke/reap foundations;
- same-repository isolation/integration foundations from earlier phases.

### A-08 reusable approval

- A-08A through A-08D: implemented and valid.
- Reusable authority is bound to one Optic session + exact resolved ToolProfile fingerprint + policy epoch + monotonic expiry.
- Every invocation still revalidates session/lease/profile/resource/isolation/policy.
- A-08E is **blocked by host interaction UX**, not a failure of the Optic reusable-approval core.
- A-08F wording such as “this conversation” remains behind H-03 evidence. Current truthful wording is “current Optic session”.

### H-00 provider-neutral architecture

H-00 is complete. Accepted direction:

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

Accepted product constraints:

- no mandatory paid Optic backend/relay/API-credit requirement for normal student use;
- Cloudflare/Tailscale/custom HTTPS remain replaceable connectivity providers;
- MCP is an adapter, not the core;
- FranceStudent is the priority V1 remote host;
- local human revoke/approval remains authoritative.

### H-01 harness implementation

The `h01-v2` authority-free probe is implemented and CI-proven in PR #170. It has:

- explicit request-id presence handling;
- credential-like header redaction;
- per-call random invocation proof mirrored in local logs;
- per-process server identity;
- grouped server-side overlap measurement for concurrency;
- bounded correlation state;
- 64 KiB request cap;
- frozen Python dependency graph;
- loopback DNS-rebinding protection by default.

### Real Optic Link automated proof

PR #178 proves the real Rust Optic stdio server can sit behind a narrow Streamable HTTP compatibility adapter against disposable synthetic data.

Remote allowlist is exactly:

- `optic_session_info`;
- `optic_fs_list`;
- `optic_fs_read`.

It excludes process, mutation, Git, `session_cancel` and arbitrary upstream forwarding.

Automated proof includes:

- real Rust upstream reached;
- real session/list/read path reached;
- exact three-tool discovery;
- traversal rejection;
- bounded concurrent forwarding;
- per-call adapter proof;
- transport-client reconnect continuity to one long-lived upstream Optic application session;
- Linux + Windows execution.

## 4. Real FranceStudent evidence verified on 2026-10-12

FranceStudent is stable again and was connected to a **new/current PR #178 Real Optic Link endpoint**, not the older H-01 probe endpoint.

### FranceStudent UI / authentication surface

The current MCP server editor visibly exposes:

- `Autorisation` labelled **“Bearer token ou secret”**;
- **“Headers sécurisés (JSON)”**.

This is useful H-02 input. It does **not** yet prove the exact wire shape of the authorization field or custom-header delivery.

### Real tool surface

FranceStudent discovered exactly:

- `optic_session_info`;
- `optic_fs_list`;
- `optic_fs_read`.

A caller-supplied absolute Windows path to the user's real Desktop/repository was rejected. The host then observed only the adapter-owned synthetic fixture (`nested/`, `public/`).

### Server-correlated real read

A real FranceStudent `optic_fs_read("public/hello.txt")` returned:

- `adapter_instance = 20baecc2eeaf`;
- `sequence = 22`;
- `marker = 126b81f5327c1b59`.

The local console printed the exact same instance/sequence/marker. This is accepted end-to-end proof:

```text
FranceStudent
  -> public Streamable HTTP
  -> PR #178 adapter
  -> real Rust optic-bridge stdio
  -> synthetic fixture read
```

### Real negative containment proof

FranceStudent was instructed to read `../outside.txt` without correction/retry. The call was rejected with:

- HTTP 400 as surfaced by the host;
- `optic.invalid_workspace_path`;
- JSON-RPC code `-32602`.

The security denial is correct. The HTTP/error-shape mapping should still be reviewed before H-06 production compatibility is frozen.

### Two calls in one FranceStudent turn

FranceStudent issued two distinct `optic_fs_read` calls from one assistant turn:

- `seq=23`, marker `ff1fc559e79dd183` for `nested/info.json`;
- `seq=24`, marker `e6aeabb6875f6ffb` for `public/hello.txt`.

Both matched local Real Optic Link logs.

This proves **two real tool invocations in one model turn**. It does **not** yet prove temporal overlap/concurrent dispatch because the current Real Optic Link host log has no overlap metric/start-end timing.

### New FranceStudent conversation against the same live adapter

Conversation A produced a verified `optic_session_info` call:

- `seq=25`;
- marker `16a1e00238f6c7f9`;
- adapter instance `20baecc2eeaf`.

A genuinely new FranceStudent conversation, without restarting the adapter/tunnel, produced:

- `seq=26`;
- marker `5198a949dec91035`;
- the same adapter instance `20baecc2eeaf`;
- the same upstream-visible session-info fingerprint: capabilities `file_read` + `file_search`, `policy_epoch=1`, principal `local-stdio`, project `local-workspace`, monotonic expiry value `1800000`.

Safe conclusion:

> Different FranceStudent conversations currently converge on the same long-lived Real Optic Link adapter/upstream experimental context.

This is **not** proof that FranceStudent exposes no conversation identifier, because PR #178 does not record the H-01 correlation inputs. It is strong evidence that the current experimental bridge itself does not isolate FranceStudent conversations into distinct Optic application sessions. H-03 is therefore required before multi-conversation real-project authority can be considered correct.

The adapter process then stopped via its intentional ~25-minute experiment lifetime, before the current ~30-minute Optic session lifetime.

## 5. What is not finished

### H-01 — real host characterization

Status: **implemented harness; remaining real-host measurements open**.

Still needed using the actual `h01-v2` endpoint/process (not PR #178):

- new-conversation correlation comparison with server proof;
- reconnect comparison with server proof;
- grouped parallel dispatch using `probe_parallel(run_id, 500)` and server-side overlap proof;
- optional safe cancellation/timeout behavior;
- harmless custom-header delivery measurement using the new FranceStudent secure-header UI;
- exact semantics of FranceStudent's bearer/secret field.

FranceStudent is no longer service-blocked as of this checkpoint. H-01 is now **evidence-pending**, not host-outage-blocked.

### H-02 — remote authentication

Status: **not implemented**.

Known now:

- FranceStudent exposes a bearer/secret field and secure JSON headers.

Still unknown / must be measured:

- whether the bearer/secret UI emits `Authorization: Bearer <value>` exactly;
- whether a custom `Authorization` header can be supplied separately;
- duplicate/conflict behavior;
- missing/wrong/valid credential behavior;
- revoke/rotation behavior;
- reconnect after revoke;
- rate/abuse/header-size behavior;
- secret redaction in logs/diagnostics.

Do not call this OAuth unless FranceStudent actually performs an OAuth flow.

### H-03 — SessionResolver / multi-conversation application sessions

Status: **not implemented**.

Required work:

- bounded `HostContext` contract;
- authenticated principal -> application session resolution;
- Optic-minted explicit application-handle fallback;
- transactional per-session authority provisioning (session + read/process/mutation/Git leases as applicable);
- deterministic cleanup/revoke mapping;
- no resurrection of revoked sessions;
- exact new-chat/reconnect contract;
- explicit session lifetime model (idle vs absolute, bounded renewal).

Important current limitation: production MCP still fundamentally starts around one application session, and the PR #178 adapter intentionally has one long-lived upstream stdio Optic context.

### H-04 — session autonomy

Status: **design issue open; no production autonomy grant yet**.

Need domain contract + adversarial tests for `Review`, `Project Autonomy`, and any advanced mode while retaining every per-effect security check.

### H-05 — local approval broker

Status: **design issue open; no production host-independent local broker yet**.

Needed before a trustworthy remote UX can depend on local consent/revoke rather than host-specific confirmation UI.

### H-06+ — production remote productization

Not started as production transport. Current HTTP adapter is an experiment.

Needs H-02/H-03/H-04/H-05 evidence/contracts first, plus the security gates below.

## 6. Security blockers before real-project remote exposure

Real project workspaces must remain blocked remotely until the relevant gates are resolved/tested:

- **#176:** sensitive content + metadata/listing classifier/policy; canonical `fs_list` authorization; non-progressing list cursor bug.
- **#180:** generic `FileWrite`/`FileDelete` must not mutate protected `.git` repository-control state and bypass dedicated Git authority.
- **#181:** Unix native path segments must round-trip losslessly into authority-bearing `WorkspacePath`.
- **#182:** `GitRead` must not bypass sensitive-content policy through status/diff.
- **#183:** read/list/Git effects must hold session admission across the effect so revoke/quiescence semantics are truthful.

These gates do not invalidate the synthetic PR #178 experiment because it uses generated disposable data and no mutation/process/Git authority.

## 7. Known facts that were previously stale or under-documented

This checkpoint promotes the following to explicit project facts:

- FranceStudent service is stable again on 2026-10-12.
- FranceStudent now visibly exposes bearer/secret authorization and secure JSON header configuration.
- Today's host tests use PR #178 Real Optic Link, not the old H-01 probe endpoint.
- One real read is server-correlated with exact adapter marker/instance/sequence.
- A real traversal attempt from FranceStudent is rejected by Optic.
- FranceStudent can issue two distinct Optic tool calls in one assistant turn; temporal overlap remains unproven.
- A new FranceStudent conversation can reach the same live PR #178 adapter/upstream experimental context; this demonstrates the need for H-03 session resolution but does not discover a host conversation id.
- The current experiment lifetime is intentionally shorter than the current ~30-minute Optic session lifetime.

## 8. Data still missing

### Host / transport facts

- exact FranceStudent bearer-field wire semantics;
- secure custom-header delivery proof;
- H-01 v2 new-chat correlation signal (if any);
- H-01 v2 reconnect signal behavior;
- true FranceStudent parallel dispatch overlap;
- cancellation semantics;
- production-appropriate MCP error/HTTP status mapping.

### Session/lifecycle facts

- final idle-vs-absolute session lifetime contract;
- production renewal trigger/limits;
- session resolution behavior after credential rotation/revoke;
- multi-conversation simultaneous-first-call behavior;
- exact remote control-plane ownership for cancel/revoke.

### Security/product decisions

- sensitive path/content/metadata classification model;
- GitRead relationship to sensitive data;
- protected repository-control namespace policy;
- autonomy grant shape/toolchain envelope;
- local approval broker retry/resume protocol.

## 9. What can be coded now without waiting on FranceStudent

Highest-value candidates that do not require inventing host semantics:

1. **#176 concrete bugs:** prepare/canonicalize directory listing before authorization and enforce pagination progress.
2. **#183 admission barrier:** deterministic read/list/Git revoke-race tests, then hold admission across the bounded effect.
3. **#180 protected Git control namespace:** focused fixture tests + canonical protected-control policy for generic mutation.
4. **#181 Unix path round-trip:** centralized fail-closed native-path conversion + regression tests.
5. **Synthetic mutation experiment extension:** only after auditing the existing mutation service; if built, keep it adapter-created, disposable, no Git, no process, no caller workspace path, and route writes through real Optic transactional mutation rather than Python file I/O.

## 10. What should wait for additional evidence

- Production H-02 auth implementation should wait until the exact FranceStudent credential/header wire behavior is measured.
- Production H-03 host-specific correlation mapping should wait until H-01 v2 measurements are complete; the explicit Optic-handle fallback can be designed independently.
- Production remote real-project read/write/Git/process exposure must wait for the security gates.
- “This conversation” user-facing wording must wait for proven adapter mapping.
- Full desktop app implementation should wait for H-03/H-04/H-05 service contracts.

## 11. Recommended immediate order

1. Synchronize documentation and issue/PR evidence with this checkpoint.
2. Reconnect FranceStudent intentionally to H-01 v2 and finish the missing host measurements, including harmless secure-header testing.
3. In parallel, implement the narrow host-independent security fixes (#176 concrete list bugs and #183 admission barrier) with deterministic tests.
4. Use the H-01 auth/header facts to implement a minimal H-02 proof harness (not H-06 production transport).
5. Freeze H-03 SessionResolver contract, including explicit Optic handle fallback and session lifetime semantics.
6. Only then choose between expanding the synthetic mutation experiment and moving into production H-06 work.

## 12. Repository hygiene rule

- Keep PR #169/#170/#178 Draft until their evidence gates are satisfied.
- Do not merge experiments merely to reduce branch count.
- Do not rewrite historical `STATUS.md` or `SCOPE_AND_ROADMAP.md` wholesale; synchronize them surgically when a production milestone merges.
- Every accepted host/security fact should live in an owning doc **and** the relevant issue/PR evidence trail.
- Exact-head CI must be green before a branch is treated as validated.


## Audit supplement: exact refs, proof limits and documentation drift

See [repository audit](../engineering/REPOSITORY_HEALTH_AUDIT_2026-10-12.md) for exact main/draft CI provenance and issue-by-issue disposition, and [evidence ledger](../research/FRANCESTUDENT_REAL_LINK_EVIDENCE_2026-10-12.md) for accepted proof levels.

The new-chat seq-26 result is accepted as user-reported evidence; the cached console excerpt is truncated before its matching line, which still needs full retention. Successful markers are emitted after upstream return, so sequences are not admission order. Simultaneous CI client tasks do not measure actual upstream effect overlap; the traversal smoke currently catches any exception, while the manual denial supplies a concrete code.

No broader remote exposure follows from these facts. Applicable security gates vary by platform/effect, and H-02/H-03/local revoke remain prerequisites. The local runtime can receive narrow security fixes while host-specific auth/correlation evidence is pending. Historical docs are preserved; their older baseline/current-next wording remains an explicit reconciliation task.
