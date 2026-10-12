# FranceStudent / Real Optic Link evidence ledger — 2026-10-12

## Provenance and scope

This ledger records user-provided real Windows-PC observations and existing GitHub evidence, reviewed on 2026-10-12 (Europe/Paris). The audit did not rerun FranceStudent or open a tunnel. Today's connector is the **PR #178 Real Optic Link**, not the earlier H-01 probe; only `optic_session_info`, `optic_fs_list`, `optic_fs_read` exist there.

The temporary endpoint was unauthenticated and backed only by adapter-generated disposable data. No real workspace path, public tunnel URL, credential or host header value is reproduced here. Invocation markers below are non-secret experimental diagnostics, never authority.

## Evidence register

| ID | Observation | Evidence / provenance | Accepted conclusion and limit |
| --- | --- | --- | --- |
| RL-01 | Real fixture read | [PR #178 read proof](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/178#issuecomment-6122684750): `public/hello.txt`, instance `20baecc2eeaf`, seq 22, marker `126b81f5327c1b59`; matching local console line; base64 fixture content | Real FranceStudent → adapter → Rust Optic → synthetic filesystem read. No real-project safety claim. |
| RL-02 | Traversal rejected | [Negative result](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/178#issuecomment-6122717333): `../outside.txt`, `optic.invalid_workspace_path`, JSON-RPC `-32602`, host displayed HTTP 400 | Accepted real-host negative observation, consistent with code/CI. Error-shape compatibility remains to audit. Failed calls do not receive the successful-call wrapper marker; do not claim a matching success marker for this denial. |
| RL-03 | Two actual reads in one turn | [Paired proofs](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/171#issuecomment-6122762076): seq 23 `nested/info.json` / `ff1fc559e79dd183`; seq 24 `public/hello.txt` / `e6aeabb6875f6ffb`; both instance `20baecc2eeaf`; matching console lines | Two distinct real tool invocations in one user-reported assistant turn. Temporal overlap is **not measured**. |
| RL-04 | Conversation A session-info baseline | User-provided host JSON + exact local console line: seq 25, marker `16a1e00238f6c7f9`, instance `20baecc2eeaf` | Real server-correlated baseline call on the running adapter. |
| RL-05 | Genuinely new conversation B reaches same adapter | User-provided host JSON + exact local console line: seq 26, marker `5198a949dec91035`, instance `20baecc2eeaf`; local line: `[REAL-LINK] TOOL_CALL instance=20baecc2eeaf tool=optic_session_info seq=26 marker=5198a949dec91035`; same returned upstream fields; adapter/tunnel not restarted | Accepted server-correlated new-chat observation on the same live adapter. This proves shared experimental adapter context, not absence/presence of a host conversation id. |
| RL-06 | Same experimental upstream context across A/B | RL-04/RL-05 plus [adapter source at tested PR head](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/ef5bc84144670135d13f55a59a782611aeae762f/experiments/optic-mcp-link/server.py): one lifespan-owned stdio `ClientSession`, no host-context resolution | Strong implementation-backed inference that both chats use the same experimental upstream application session. `session_info` does not expose the internal `SessionHandle`; matching fields alone are not unique session identity. This demonstrates the missing production conversation isolation, not a host conversation-id signal. |
| UI-01 | Updated auth inputs visible | [Issue #171 UI report](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/171#issuecomment-6122560860): “Autorisation / Bearer token ou secret” and “Headers sécurisés (JSON)” | UI-level capability observation. Header wire shape, precedence, storage, missing/invalid/revoked behavior still untested. Earlier “no custom-header field visible” describes an older UI only. |
| HOST-01 | Host testing resumed | [Issue #171 recovery report](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/171#issuecomment-6122645214) and RL-01–RL-05 | The earlier outage no longer explains the current pending evidence. This does not promise future service availability. |

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

## Excluded and unknown

- The earlier model-admitted fabricated H-01 new-chat JSON remains excluded.
- H-01 v1's absent-request-id fingerprint remains ambiguous.
- RL-03 cannot close H-01 v2 grouped-parallel evidence.
- RL-05 cannot close H-01 v2 host-header/`_meta` correlation tests.
- No reliable host conversation identifier, production authentication, per-chat authority isolation, revocation-after-reconnect or renewal policy has been proven.
- Exact local checkout/binary hash used for the manual run is still missing; do not equate the manual adapter instance with CI head solely from matching schema.
- Fresh sanitized H-01 v2 captures and exact tested local commit/dependency provenance belong in the next evidence bundle.

## Next evidence collection

Keep the current experiment synthetic-only. For H-01 use its own fresh v2 endpoint/process, fresh correlation/parallel run ids, and paired server markers. Collect same-chat/new-chat/reconnect comparisons, grouped `overlap_observed`, harmless custom-header delivery, and auth-field name/scheme/presence using dummy values without logging credentials. H-02 must independently prove rejection, local revoke/rotation and request bounds.

For project health and coding priorities see [current state](../product/CURRENT_STATE_2026-10-12.md), [project health](../product/PROJECT_HEALTH_2026-10-12.md), and [repository audit](../engineering/REPOSITORY_HEALTH_AUDIT_2026-10-12.md).
