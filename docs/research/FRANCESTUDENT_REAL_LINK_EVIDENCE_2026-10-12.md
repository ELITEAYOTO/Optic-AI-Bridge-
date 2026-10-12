# FranceStudent / Real Optic Link evidence ledger — 2026-10-12

## Provenance and scope

This ledger records user-provided real Windows-PC observations, direct FranceStudent admin statements, and existing GitHub evidence reviewed on 2026-10-12 (Europe/Paris). Today's connector is the **PR #178 Real Optic Link**, not the earlier H-01 probe; only `optic_session_info`, `optic_fs_list`, `optic_fs_read` exist there.

The temporary endpoint was unauthenticated and backed only by adapter-generated disposable data. No real workspace path, public tunnel URL, credential or host header value is reproduced here. Invocation markers below are non-secret experimental diagnostics, never authority.

Evidence classes:

- **server-correlated observation** — host result paired with local adapter/probe evidence;
- **admin product-contract statement** — direct answer from FranceStudent administration, useful for architecture but still eligible for harmless wire regression testing;
- **implementation-backed inference** — conclusion from observed behavior plus current source shape, explicitly bounded by its limits.

## Evidence register

| ID | Observation | Evidence / provenance | Accepted conclusion and limit |
| --- | --- | --- | --- |
| RL-01 | Real fixture read | [PR #178 read proof](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/178#issuecomment-6122684750): `public/hello.txt`, instance `20baecc2eeaf`, seq 22, marker `126b81f5327c1b59`; matching local console line; base64 fixture content | Real FranceStudent → adapter → Rust Optic → synthetic filesystem read. No real-project safety claim. |
| RL-02 | Traversal rejected | [Negative result](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/178#issuecomment-6122717333): `../outside.txt`, `optic.invalid_workspace_path`, JSON-RPC `-32602`, host displayed HTTP 400 | Accepted real-host negative observation, consistent with code/CI. Error-shape compatibility remains to audit. Failed calls do not receive the successful-call wrapper marker; do not claim a matching success marker for this denial. |
| RL-03 | Two actual reads in one turn | [Paired proofs](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/171#issuecomment-6122762076): seq 23 `nested/info.json` / `ff1fc559e79dd183`; seq 24 `public/hello.txt` / `e6aeabb6875f6ffb`; both instance `20baecc2eeaf`; matching console lines | Two distinct real tool invocations in one user-reported assistant turn. Temporal overlap is **not measured**. |
| RL-04 | Conversation A session-info baseline | User-provided host JSON + exact local console line: seq 25, marker `16a1e00238f6c7f9`, instance `20baecc2eeaf` | Real server-correlated baseline call on the running adapter. |
| RL-05 | Genuinely new conversation B reaches same adapter | User-provided host JSON + exact local console line: seq 26, marker `5198a949dec91035`, instance `20baecc2eeaf`; local line: `[REAL-LINK] TOOL_CALL instance=20baecc2eeaf tool=optic_session_info seq=26 marker=5198a949dec91035`; same returned upstream fields; adapter/tunnel not restarted | Accepted server-correlated new-chat observation on the same live adapter. This proves shared experimental adapter context, not a host conversation id. |
| RL-06 | Same experimental upstream context across A/B | RL-04/RL-05 plus [adapter source at tested PR head](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/ef5bc84144670135d13f55a59a782611aeae762f/experiments/optic-mcp-link/server.py): one lifespan-owned stdio `ClientSession`, no host-context resolution | Strong implementation-backed inference that both chats use the same experimental upstream application session. `session_info` does not expose the internal `SessionHandle`; matching fields alone are not unique session identity. This demonstrates missing production conversation isolation. |
| UI-01 | Updated auth inputs visible | [Issue #171 UI report](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/171#issuecomment-6122560860): “Autorisation / Bearer token ou secret” and “Headers sécurisés (JSON)” | UI-level capability observation. Earlier “no custom-header field visible” describes an older UI only. |
| ADM-01 | No conversation/chat id delivered to MCP | Direct FranceStudent admin answer: there is currently no identifier sent to MCP servers that distinguishes chats/conversations; admin noted it could be a future addition | Product-contract input. FranceStudent V1 must not depend on a host chat id. Any future such value remains non-authoritative correlation input. |
| ADM-02 | MCP availability is account-global | Direct FranceStudent admin answer: the configured MCP is global to the user's account, so each conversation has access | Confirms multiple chats can reach the same configured server. Does not itself define application-session isolation. |
| ADM-03 | Secure custom headers | Direct admin answer: custom headers are supported; secure JSON headers are attached to **every request** and are **hidden from the model** | Strong H-02 compatibility input. Harmless on-wire regression proof remains desirable. |
| ADM-04 | Bearer carrier | Direct admin answer: the authorization field sends exactly `Authorization: Bearer <value>` | Closes the host-field discovery question for FranceStudent V1. Does not itself prove Optic's missing/wrong/revoke/rotate behavior. |
| ADM-05 | No native confirmation | Direct admin answer: FranceStudent has no per-tool-call confirmation | Reinforces H-05 host-independent local approval/revoke requirement. |
| ADM-06 | Parallel calls allowed | Direct admin answer: OpenAI/Claude decides parallel tool usage; FranceStudent allows it. Admin believes current limit is ~8 calls and intends to remove that limit | Optic must support/bound concurrency independently and must not encode 8 as a security invariant. True overlap remains useful to measure with H-01 v2. |
| ADM-07 | Cancellation unsupported | Direct admin answer: MCP cancellation is not currently supported | Optic cannot rely on host cancellation; local deadlines/revoke/cleanup remain mandatory. |
| ADM-08 | Reconnect context unspecified | Direct admin answer: reconnect/context behavior was not known and should be tested | Still an interoperability measurement, not a reason to bind authority to transport continuity. |
| HOST-01 | Host testing resumed | [Issue #171 recovery report](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/171#issuecomment-6122645214) and RL-01–RL-05 | The earlier outage no longer explains current pending evidence. This does not promise future service availability. |

## Raw session-info observation for A and B

Both returned:

```json
{
  "capabilities": ["file_read", "file_search"],
  "expires_at_monotonic_ms": 1800000,
  "policy_epoch": 1,
  "principal": "local-stdio",
  "project": "local-workspace"
}
```

`expires_at_monotonic_ms` uses that server process's private monotonic origin; it is neither a Unix timestamp nor a portable session id. The runner stopped after its advertised 25-minute window, before the initial 30-minute session expiry. This is expected experimental runner behavior, not evidence of production renewal.

## Proof integrity

In PR #178 `_wrap()` calls `_new_invocation()` **after** `await _forward(...)` succeeds. Sequences order wrapper creation/log emission after upstream success; they do not establish request admission/start order. The logged marker proves a correlated successful result, not duration or overlap.

The automated client uses `asyncio.gather` and proves the adapter accepts simultaneous client tasks and returns both results. It does not instrument actual upstream effect overlap. Its traversal smoke catches any exception; green CI alone does not assert the precise error code. RL-02 adds a concrete manual error observation. Stronger deterministic overlap and exact-denial checks are next experiment candidates.

Admin statements above are recorded separately from server-correlated results. They are sufficient to choose a host-specific compatibility architecture where the product contract is explicit, but security-sensitive request behavior should still be tested harmlessly where practical.

## Architecture conclusions enabled by admin answers

### FranceStudent V1 session continuity

Because ADM-01 says there is **no per-chat identifier delivered to the MCP server**, H-03 no longer needs to wait for discovery of one.

FranceStudent V1 baseline should be:

```text
Authorization: Bearer <connection credential>
        -> authenticated connection principal
        -> Optic-minted explicit opaque application handle
        -> SessionResolver
        -> internal Optic-owned SessionHandle
        -> session-owned capabilities / leases / policy
```

The explicit application handle is not itself project authority. It must be principal-bound, opaque, bounded, expiring/revocable, and resolved through active application state.

### FranceStudent V1 authentication carrier

ADM-03/ADM-04 make the compatibility carrier concrete:

```http
Authorization: Bearer <value>
```

plus optional secure custom headers that are hidden from the model and sent on every request.

This is a **static bearer compatibility mode**, not OAuth. H-02 still needs to prove validation, immediate revoke/rotation, reconnect-after-revoke, redaction, abuse bounds and separation from project/session authority.

### Human control / cancellation

ADM-05/ADM-07 mean FranceStudent cannot be the only consent or cancellation boundary. H-05 local approval/revoke and effect deadlines/cleanup are required even if the host remains connected or stops displaying a generation.

## Excluded, resolved and unknown

### Resolved host-discovery questions

- Whether FranceStudent sends a per-chat id: **resolved by admin — no**.
- Whether secure custom headers exist: **resolved by UI + admin — yes**.
- Whether secure headers are per request and model-hidden: **resolved by admin — yes**.
- Bearer field wire shape: **resolved by admin — `Authorization: Bearer <value>`**.
- Native per-call confirmation: **resolved by admin — none**.
- Whether host permits parallel tool calls: **resolved by admin — yes**.
- Cancellation support: **resolved by admin — not currently supported**.

### Still excluded / unknown

- The earlier model-admitted fabricated H-01 new-chat JSON remains excluded.
- H-01 v1's absent-request-id fingerprint remains ambiguous.
- RL-03 cannot prove actual temporal overlap.
- Exact local checkout/binary hash used for the manual Real Link run is still missing; do not equate the manual adapter instance with CI head solely from matching schema.
- Reconnect/context behavior remains unmeasured.
- Host-side tool-call timeout remains unanswered/unmeasured.
- Duplicate `Authorization` precedence if both auth UI and secure JSON attempt to set it remains unspecified.
- Production authentication, per-chat application isolation, revocation-after-reconnect and renewal policy are not implemented yet.

## Remaining test matrix

There are no remaining admin answers that should block selecting the H-02 bearer carrier or H-03 explicit-handle strategy.

High-value tests still worth running:

1. harmless secure custom header delivery (`X-Optic-Probe`) with local proof;
2. dummy bearer scheme/header-presence wire proof without logging the value;
3. H-01 v2 grouped `probe_parallel` overlap measurement;
4. reconnect characterization;
5. H-02 missing/wrong/valid/revoked/rotated bearer harness;
6. reconnect after revoke/rotation;
7. server-owned deadline/timeout behavior independent of FranceStudent's eventual host timeout.

For project health and coding priorities see [current state](../product/CURRENT_STATE_2026-10-12.md), [project health](../product/PROJECT_HEALTH_2026-10-12.md), and [repository audit](../engineering/REPOSITORY_HEALTH_AUDIT_2026-10-12.md).
