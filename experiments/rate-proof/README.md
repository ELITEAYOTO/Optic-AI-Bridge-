# Bounded global admission proof — H-02 synthetic candidate

Stacked on #189 docs head99bdeeca7fbe7dd88c1c3e4c30756d1b124c8e76,
whose CI696/persistence#2 are green. No production Rust changes, listener or
FranceStudent manual test added. This is a candidate ASGI layer tested locally,
not installed into any existing probe server or production transport.

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

No startup/setup/tunnel flow is needed for this local-only candidate. Prior
FranceStudent persistence series is finished; its ledger remains under
experiments/persistent-auth/LIVE_EVIDENCE_2026-10-13.md. H-01 overlap is separate
from #178. Future rate integration requires measured host discovery/retry behavior
and deliberate fairness/capacity policy, before any real project authority.
