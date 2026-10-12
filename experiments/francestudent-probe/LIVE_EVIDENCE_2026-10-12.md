# FranceStudent manual evidence — 2026-10-12

Source: user-supplied model results, pasted server logs and screenshots in the
accompanying Codex chat; original logs were not independently accessed on drive E.
Model identified by user as GPT 6 luna. No claim for other models/host versions.
Archive provided at `50fc4af185dd09d1312eea66f99e88c326bf49f0`; extracted files
were not independently re-hashed. User's Setup and Test scripts passed, including
8 deterministic tests and real local SDK smoke. Live instance: `f347353f08ae`.
The tunnel connected to loopback port 8001. Tunnel identity is not authority.

## First session — observed sequence

| Request | Observation / result |
| --- | --- |
| 1–2 | Both custom headers match; Authorization absent; HTTP401. |
| 3–4 | Single Authorization present, exact `Bearer ` prefix/dummy match fails; both custom headers match; HTTP401. Raw format unknown. |
| 5–7 | After entering full `Bearer <dummy>` in dedicated field: initialize200, initialized notification202, tools/list200. All header checks match. Connector screenshot activated. |
| 8–9 | Matching single Bearer; both custom headers absent; HTTP401. Method/source unknown because rejected before DISPATCH. |
| 10–12 | initialize200, initialized202, tools/list200; all header checks match. |
| 13 | `probe_headers`, marker `9d26d5fcd3ad476f`, HTTP200, exact local TOOL/model marker/instance/sequence match; all headers match. |
| 14 | All header checks match; HTTP405; method/source unknown. |
| 15–16 | initialize200, initialized202; all headers match. |
| 17 | `probe_create_handle`, marker `0ac4a2c8decff26e`, HTTP200; model result ok, single_use, TTL120, authority none. Exact local TOOL proof match. |
| 18 | `probe_consume_handle`, marker `658b92867c2a9040`, HTTP200; model result ok/passed_unchanged/consumed=true, authority none. Exact local TOOL proof match. All headers match on both calls. |
| 19 | All headers match; HTTP405; method/source unknown. |
| 20–21 | initialize200, initialized202; all headers match. |
| 22 | `probe_consume_handle`, marker `981c6ee285ef4d36`, HTTP200; model result ok=false/code=probe.invalid_handle. Exact local TOOL proof match. All headers match. |
| 23 | All headers match; HTTP405; method/source unknown. |

## Accepted conclusions and limits

The exact dummy Bearer and both custom headers were present on the listed admitted
setup/tool requests. Do not claim every request: 8/9 are retained counterevidence
with unknown attribution. Do not guess the method/source of 405 requests.

The initial no-prefix UI-entry instruction did not pass the strict parser. Full
Bearer entry did. This qualifies the admin's reported Bearer behavior; the server
does not reveal whether earlier entries were raw, differently cased, whitespace
modified or otherwise transformed. No real credentials or raw header captures
were used, logged, returned or retained here.

Under the audited implementation, successful consume means an active minted
token was passed unchanged for the same synthetic owner before expiry. The user
asked the model to thread it immediately without copying. Both calls have exact
server proof. This demonstrates one model-mediated handoff with zero authority.
The subsequent requested replay was refused, corroborated by server proof. Raw
arguments are not logged: exact replay-input identity relies on the user-supplied
model account; atomic single-use behavior also has deterministic automated proof.
The raw application handle is deliberately omitted from this ledger.

HTTP200 on request22 is successful MCP delivery, not successful token consumption.
No new chat isolation, production SessionResolver, capabilities/leases or H-02
credential lifecycle is implemented or proven. Repeated initialize traffic is
observed; it does not prove a user-directed connector reconnect experiment.

At the first-session checkpoint, still unmeasured live: explicit reconnect, expiry and other-owner denial, header
conflict precedence, timeout and H-01 grouped temporal overlap. Secure-header
invisibility remains an admin statement. #178 is a separate real Rust synthetic
link and provides none of these H-01 proofs. Keep #186 Draft and H-02/H-03 open.

## First-session follow-up

The user confirmed stopping both local server/tunnel and disabling the connector.
The subsequent development tranche added `fs-probe-v2`
safe HTTP-method/endpoint/query-presence/elapsed-time and response-stage fields,
without changing auth acceptance or handle authority. Local GET missing-header
and DELETE SDK-rejection tests distinguish their phases. Future live results must
use a new instance and version; do not backfill these fields into the first
session's sequence or treat local diagnostic tests as host-source attribution.

## Second session — fs-probe-v2

Provided source archive: `54ca313e3a88e8ce61800966c5818efff2de372b`, not
independently re-hashed after extraction. User's Setup and Test scripts passed:
12 deterministic tests and real local MCP smoke. Same user-reported GPT 6 luna;
FranceStudent screenshot shows build `fd43d3c`. New instance `ed6392f76fb4`.
New tunnel to port8001; no old server state is reused.

| Request | Monotonic elapsed ms at HTTP admission | Observation / result |
| --- | --- | --- |
| 1–3 | 134750–134860 | POST initialize200 / initialized202 / tools/list200. |
| 4–6 | 199610–199719 | Second POST initialize200 / initialized202 / tools/list200 group; reason not inferred. |
| 7–9 | 298641–298844 | POST initialize200 / initialized202 / tools/list200 preceding the requested tool call. |
| 10 | 302594 | POST `probe_headers`, marker `0f11509383ee60c5`, HTTP200; exact local/model proof match. |
| 11 | 309672 | DELETE, stage=mcp, HTTP405. |
| Toggle | No new rows in supplied excerpt | User disabled/re-enabled connector without changing settings or restarting server/tunnel. Excerpt still ended at11. |
| 12–13 | 533594–533657 | POST initialize200 / initialized202 after the next requested tool call. |
| 14 | 533735 | POST `probe_headers`, marker `a87b4d482fc6d15d`, HTTP200; exact local/model proof match. |
| 15 | 540141 | DELETE, stage=mcp, HTTP405. |
| 16–17 | 639329–639407 | POST initialize200 / initialized202. |
| 18 | 639500 | POST `probe_create_handle`, marker `ffbf007a2ea836f4`, HTTP200; exact local/model proof match, ok/TTL120/single_use/authority none. User requested no consume yet. |
| 19 | 648813 | DELETE, stage=mcp, HTTP405. |
| 20–21 | 948922–949000 | POST initialize200 / initialized202 after intentional delay. |
| 22 | 949063 | POST `probe_consume_handle`, marker `d571c767d1260bc5`, HTTP200; exact local/model proof match, ok=false/probe.invalid_handle. |
| 23 | 956438 | DELETE, stage=mcp, HTTP405. |

All supplied v2 requests1–23 have a single exact dummy Bearer and both exact
custom-header matches, endpoint match and no query; response stage=mcp. No
missing-custom-header request was reproduced in this supplied window. This does
not erase the first session's requests8/9 or establish delivery for every future
request. Methods/headers alone do not authenticate caller source or intent.

The v2 405 rows are specifically authenticated DELETE requests refused inside the
MCP app, after the guard checks; they are not auth/header failures. Do not infer
why the client sends DELETE or retroactively assign this method to v1 rows.

After the user-reported connector toggle, a new real call succeeds on the same
live instance with unchanged headers and a fresh matching marker. This is useful
host compatibility proof after reactivation, not proof of TCP reconnect,
application-handle continuity across toggle, per-chat isolation or revocation.

For expiry, admission timestamps differ by **309563ms (309.563s)** between create
and attempted consume, comfortably beyond TTL120. Same instance, no intermediate
consume supplied, successful create followed by matching refused consume. Exact
submitted token identity is user/model account because raw arguments are not
logged. The timestamps mark ingress, not exact mint/effect time; this supports
delayed expired-handle denial under the audited implementation, not an exact
120.000-second boundary measurement. Deterministic tests independently prove
that boundary, owner denial, capacity, replay, collision and racing single-use.
The raw token is omitted from this ledger.

The user confirmed stopping both v2 server/tunnel and disabling the connector.
No remaining public manual process is claimed active. No production endpoint,
real project/runtime, process/Git/mutation or session_cancel authority was added.

## Current evidence disposition

- Accepted live: strict dummy Bearer/custom delivery on the enumerated setup/tool
  requests; one model-mediated unchanged handoff and requested replay refusal in
  session1; successful new call after connector toggle and delayed handle refusal
  in session2; v2 DELETE405 occurs after authentication inside MCP.
- Accepted automated: owner binding, exact TTL boundary, single-use/race/collision,
  bounded state/streamed ingress, timeout/cancel cleanup, log canary absence,
  actual local SDK reconnect and exact remote tool allowlist on Windows/Linux.
- Unknown/unmeasured: original v1 missing-header caller/method; universal header
  delivery; raw format of initially rejected Authorization; actual TCP reconnect,
  handle continuity across toggle; live other-owner denial; secret-field model
  invisibility beyond admin statement; credential revoke/rotate/storage/OAuth;
  H-01 host temporal overlap and production SessionResolver authority provisioning.

Keep #186 Draft, #171/#172/#173 open, and all real-project gates applicable.
No additional user input is required to consolidate these findings. The next safe
engineering work can build on this evidence without relaxing admission, granting
authority or waiting for a host chat-id. Production promotion requires its own
reviewed credential lifecycle/session contract; these probe handles are not it.
