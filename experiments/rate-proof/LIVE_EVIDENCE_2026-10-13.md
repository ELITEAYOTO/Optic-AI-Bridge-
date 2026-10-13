# FranceStudent controlled429 and recovery — 2026-10-13

Source: operator-provided Luna outputs and complete local console excerpts in the
guided trial. Tested code #191 head1b9af692d66e8954a2ff95b98f0aacf49c6fc650;
CI698 and Synthetic Rate Admission#2 green. Public dummy auth only, authority none,
single remote rate_probe. No real workspace or credential values recorded here.
Operator confirmed server/tunnel stopped and connector disabled at completion.

## Matched observations

Windows setup succeeded; six rate tests and real loopback HTTP/MCP smoke passed.
Manual server instance fb4d3724b26a, synthetic-rate-host-v1, loopback8005 through
temporary HTTPS tunnel. Host discovery seq1 initialize200, seq2 initialized202,
seq3 tools/list200. All supplied admitted requests matched single dummy Bearer and
both custom dummy headers.

Baseline actual rate_probe: seq7, elapsed123578ms, markera9e23dfe219696e4 matched
Luna and local TOOL/status200, oktrue/authoritynone. Seq8 DELETE returned405 at
stage mcp after authentication; not a cancellation or rate failure.

Local hold-rate ACK heldtrue, same instance. One instructed host call yielded:

```text
connect to MCP server: streamable HTTP: calling "initialize": rejected by transport: sending "initialize": Too Many Requests; HTTP/SSE: failed to connect: Too Many Requests
```

Three safe RATE lines on the same instance reported status429/global_admission,
remaining_log_budget31,30,29. No new Guard DISPATCH/TOOL was present during hold.
The controlled gate rejects before authentication/header/body inspection. Thus
these429 records do NOT prove header delivery on rejected requests. They contain
no HTTP method/request marker/timing, so do not attribute each line to a specific
host method or classify them as automatic retries. Host error mentions both
transport paths, but that does not establish the retry policy or Retry-After use.

Local release-rate ACK heldfalse, same instance; explicit control replenishes
the bucket. Operator sent the same prompt once, without config change or server
restart. Seq9 initialize200, seq10 initialized202, then actual rate_probe seq11
elapsed291797ms, marker3b70855e608ee847 matched Luna and local TOOL/status200.
Result oktrue/authoritynone; all dummy header bits true. Seq12 DELETE405 at mcp.

## Conclusions and remaining gates

FranceStudent surfaced controlled429 as Too Many Requests during initialize;
a subsequent operator-requested call succeeded after local release on the same
server instance/config. No production auth or authority was bypassed. This is a
finite host error/recovery observation, not automatic retry, natural refill,
actual burst-limit exhaustion, per-owner fairness, socket reuse, DDoS protection,
or broad protocol conformity. Natural refill/capacity are local deterministic
tests; forty forced denials/log ceiling32 are separate loopback automation.

This completes the guided controlled429 series. H-02 production fairness,
credential protection/integrity/recovery, and H-03 SessionResolver gates remain
open. H-01 parallel overlap and conversation behavior still require their own
characterization; #178 must not be conflated with H-01. No draft merge authorized.
