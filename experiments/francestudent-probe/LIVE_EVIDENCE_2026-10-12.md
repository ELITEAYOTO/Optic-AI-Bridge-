# FranceStudent manual evidence — 2026-10-12

Source: user-supplied model results, pasted server logs and screenshots in the
accompanying Codex chat; original logs were not independently accessed on drive E.
Model identified by user as GPT 6 luna. No claim for other models/host versions.
Archive provided at `50fc4af185dd09d1312eea66f99e88c326bf49f0`; extracted files
were not independently re-hashed. User's Setup and Test scripts passed, including
8 deterministic tests and real local SDK smoke. Live instance: `f347353f08ae`.
The tunnel connected to loopback port 8001. Tunnel identity is not authority.

## Observed sequence

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

Still unmeasured live: explicit reconnect, expiry and other-owner denial, header
conflict precedence, timeout and H-01 grouped temporal overlap. Secure-header
invisibility remains an admin statement. #178 is a separate real Rust synthetic
link and provides none of these H-01 proofs. Keep #186 Draft and H-02/H-03 open.

## Follow-up disposition

The user confirmed stopping both local server/tunnel and disabling the connector.
No further live calls are claimed. The next development tranche adds `fs-probe-v2`
safe HTTP-method/endpoint/query-presence/elapsed-time and response-stage fields,
without changing auth acceptance or handle authority. Local GET missing-header
and DELETE SDK-rejection tests distinguish their phases. Future live results must
use a new instance and version; do not backfill these fields into this ledger's
historical sequence or treat local diagnostic tests as host-source attribution.
