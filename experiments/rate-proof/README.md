# Bounded global admission proof — H-02 synthetic candidate

Stacked on #189 docs head99bdeeca7fbe7dd88c1c3e4c30756d1b124c8e76,
whose CI696/persistence#2 are green. #190 proves standalone admission; this
follow-up integrates only a new synthetic listener, never production Rust or an
existing credential/persistence server. FranceStudent manual behavior is pending.

H-02 #172 still lacked a rate-admission proof. Fixed global token bucket defaults
to burst8, refill2/second using monotonic time. It admits before auth/body parsing;
limited requests receive fixed429/no-store/retry-after without reading their body
or logging their credential, headers, proxy IP or path. Accepted requests still
pass to the existing Guard; the limiter grants no authentication or authority.
It does not trust tunnel identity, forwarding headers or caller-selected keys.

State is five fixed scalar/reference fields, never a map keyed by arbitrary
input. Synchronous admission has no await and supports exactly one ASGI event
loop; multi-process distributed budgets are not provided. Clock regression cannot
refill, nonfinite time refuses, long idle refill saturates at capacity. Remaining
Guard size/concurrency/request/lifetime ceilings stay separate invariants.

Global admission deliberately sacrifices per-owner fairness: unauthenticated
traffic can exhaust shared capacity and temporarily deny legitimate traffic.
Existing host requests, including initialization and cleanup, share the budget.
This is not a production rate policy, network DDoS protection, proxy connection
containment, or proof of SDK/host retry behavior. Restart resets this ephemeral
bucket; credential state/persistence is separate. No issue is closed by it.

Six deterministic native tests cover refill/capacity, bad clock/limits, Guard401
without auth bypass, no body read/value echo/log amplification for429, simultaneous
100-call burst with input churn (exactly8 admissions), and lifespan passthrough.
They use a fake monotonic clock rather than timing-sensitive sleeps. ASGI-level
proof only: no live FranceStudent or real network rate claim.

Windows with the existing isolated probe dependencies:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\rate-proof\scripts\Test-RateProof.ps1
```

The follow-up includes a real loopback HTTP/MCP smoke and separate synthetic
host_server.py onport8005. Exactly one remote tool, rate_probe, authority none.
Existing dummy A/B Bearer and both custom dummy headers required; Guard allowlist
rejects other tools (including session_cancel). No real workspace/process/Git or
remote control interface. Fixed local stdin commands hold-rate/release-rate allow
reproducible rejection and recovery without model-generated burst timing. Commands
are bounded16/read65; state changes run on the single event loop. Hold forces429
until release or process exit; release explicitly restores the burst budget.
Neither action changes authentication. This controlled recovery is NOT proof of
natural refill/host automatic retry. Natural refill remains unit-test evidence.
Only32 safe429 diagnostic lines are emitted per server, with random instance id,
fixed stage/status and remaining log budget, never header/path/body values.
Operator must not use the controls on a production adapter.

Real HTTP smoke proves exact one-tool discovery, marker-matched successful calls,
40 held valid/invalid requests all429, log ceiling32, release recovery same server,
unknown tool400 and missing auth401, no dummy/canary values in logs. Subprocess
killed on exit, temporary port only; no external URL accepted by smoke.

## Guided Windows host phase — review output after each step

Extract a fresh source-only ZIP; old persistence fixture stays untouched. Scripts:
Setup-RateHostProbe.ps1 installs frozen isolated dependencies;
Test-RateProof.ps1 runs6 tests plus actual HTTP/MCP smoke;
Start-RateHostProbe.ps1 -AllowRandomTunnelHost startsport8005/lifetime25min.
New tunnel targets127.0.0.1:8005; connector uses HTTPS/mcp, full-prefix dummy A,
same two secure headers, rate_probe only. No local initialization needed.

Baseline prompt:
```text
Appelle réellement rate_probe une seule fois. Affiche le résultat complet ou l'erreur réelle. En cas d'échec, arrête-toi sans réessayer.
```
Match tool marker/instance/sequence in server log before local control. Operator
types hold-rate and waits for LOCAL heldtrue ACK. Send the SAME prompt once, retain
actual host error and safe RATE429 logs. Do not remove auth or retry in a loop.
Then operator types release-rate and waits for heldfalse ACK; SAME prompt once,
match successful new marker on same instance. Host automatic retries, error
presentation and discovery behavior are observations, not assumptions. End with
Ctrl+C server/tunnel and connector disabled. Assistant reviews each step separately.
No manual call count limit or cancellation/H-01 claim follows from this phase.

Prior
FranceStudent persistence series is finished; its ledger remains under
experiments/persistent-auth/LIVE_EVIDENCE_2026-10-13.md. H-01 overlap is separate
from #178. Future rate integration requires measured host discovery/retry behavior
and deliberate fairness/capacity policy, before any real project authority.
