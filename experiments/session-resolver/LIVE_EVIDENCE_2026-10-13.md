# FranceStudent idempotent open/reusable session/local close — 2026-10-13

Tested code #193 head5f7cad874e60034c5330ecd3bd3ebf701b4510c9. Code CI701 and
all six dependent experiment workflows green. Evidence source: operator-provided
Luna results plus local server console; no raw bearer/ticket/session token values
retained in this ledger. Public dummy auth only, authority none, fixed3-tool
synthetic-session-bootstrap-v1, instanceedcd1ea1d1fc. Operator confirms final
Ctrl+C server/tunnel and disabled connector. Native setup/17tests/MCP smoke passed.

## First aborted chain — preserve ambiguity

Seq7 bootstrap markerc22c1ed451d87346 elapsed137531ms; seq8 open3fd928a58b376b25
elapsed142031ms reusedfalse; seq9 open9691ccb05b5187e0 elapsed147968ms reusedtrue.
Luna reported same session reference and synthetic id for both opens, remaining
TTL116 then110 seconds. All3 markers match local TOOL/status200. Fourth instructed
call returned actual host HTTP2 timeout awaiting response headers; fifth not called.

The first supplied console excerpt stopped atseq9, with no visible fourth-call
arrival. Later fuller excerpt DOES show seq10 HTTP/dispatch at152859ms, then
seq11 DELETE at242109ms/status405, then TOOL session_probe markerb08e4bb439152179
and seq10 HTTP200. Thus seq10 arrived and tool/result emission occurred after
the seq11 log line. At least89.250s separates the seq10/11 admission timestamps;
TOOL itself has no timestamp. Its business payload is not in the console and
was not received by Luna, so no successful resolution or precise execution time
can be claimed for this aborted trial. No close command was issued for it.

Root cause unknown. A late HTTP200 is not evidence of cancellation or completed
effect suppression; there are no real effects in this harness. The observed delay
does not establish a FranceStudent timeout constant, tunnel fault, blocking
console, or exact tool duration. Existing asyncio20s request timeout is cooperative
and cannot by itself establish a wall-clock/preemptive deadline under event-loop
or synchronous output stalls. Diagnostics robustness requires a separate proof.

## Clean chain in operator-reported new Luna conversation

All supplied markers matched local TOOL/status200 on same server instance:

| seq | tool | marker | admission elapsed_ms | result |
|---|---|---|---:|---|
|15|session_bootstrap|7b2fbd52299f07e9|340546|ticket TTL120|
|16|session_open|6eefec915fed0272|346312|new session, reusedfalse, remaining115|
|17|session_open|2109ba4e417172d5|352734|same ticket -> same reference/id, reusedtrue, remaining108|
|18|session_probe|d65887e81037415b|358390|same session id, remaining103|
|19|session_probe|d0a3a38bf567a36e|363062|same session id, remaining98|
|23|session_probe|eae89d3369d0a14e|407656|invalid_session after local close|
|24|session_open|c4bab5bdc068e177|414078|invalid_session for original ticket after close|

Successful synthetic id synthetic_58fa2686cf785077d53e79043689aefa, principal
5161a35cbe7aad13/epoch1. Absolute deadline is not renewed by either open or probe.
Seq15->19 admission delta22.516s. Operator then issued close-a; LOCAL ACK
oktrue/operationclose/ownerA is present before later requests. Seq15->23 is67.110s,
seq15->24 is73.532s. Both denials are well withinTTL120 and timeout margin20s,
so this is a clean close/non-resurrection trial, not expiry-confounded.

Both denials are business JSON okfalse/invalid_session within HTTP200; connection
auth remained accepted. Seq21 initialize200/seq22 initialized202 precede denial.
The fresh initialization is transport observation, not TCP identity proof.
DELETE seq20/25 yielded405 at mcp; not close/cancel semantics. All supplied HTTP
observations have matching single Bearer and both dummy custom headers. Model
argument equality is reported by model/user; logs deliberately omit raw args.

## Conclusions and open gates

Finite FranceStudent proof of explicit ticket/model handoff, same-ticket repeated
open to SAME synthetic session, repeated reference resolution, local close denying
reference AND same-ticket reopen before expiry. User reported a new conversation;
this does not prove per-chat isolation, stable chat id, or absence of correlation.
Exactly-once effects/durable bootstrap/local permission grants are not tested.
Lost BOOTSTRAP reply and lost OPEN reply have distinct contracts; only same-ticket
eligible OPEN is idempotent. Native concurrent20-open test is not H-01 host overlap.
H-02 production credentials/storage/integrity/recovery, H-03 real SessionHandle/
HostContext/authority and H-01 correlation/parallel gates stay open. #178 remains
separate. Keep drafts; no merge or real project exposure authorized by this proof.
