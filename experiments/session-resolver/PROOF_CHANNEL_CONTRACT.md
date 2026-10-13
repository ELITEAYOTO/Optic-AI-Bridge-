# Bounded asynchronous proof sink — synthetic diagnostic candidate

Follow-up to #193 completed host ledger at ac2025482b9821e59119a7f0fec927ef8ac30363.
The first host chain timed out while later logs showed TOOL/HTTP200. Root cause
remains unknown; this change is NOT a demonstrated fix for that incident.

Previously Guard emitted proof synchronously; TOOL used direct flushed stdout.
A blocking sink can stall producers independently of the synthetic resolver.
New bootstrap carrierv2 uses ProofChannel with one daemon writer: producers only
enqueue using put_nowait; sink I/O runs without producer or registry locks. Only
that carrier is wired to the queue; earlier carrier defaults remain unchanged.
Guard routes TOOL proof through the per-request emitter so SDK worker threads do
not bypass it. No production Rust, authority/permission/session contract changes.

Queue128 records plus atmost1 in-flight writer record; each ASCII string <=4096
characters. Accepted record budget4096/process; counters saturate at4096. No files,
disk spool, unbounded retry/backlog or sink-error string formatting. Sink error
marks failed/closed, rejects later producers; queued records can remain undelivered
but bounded. Shutdown waits atmost0.2s for writer, never waits indefinitely for a
blocked sink; daemon terminates with process. This is best-effort diagnostics, not
durable auditing, race-free process containment or a general redaction engine.
Producers still emit only existing schema-controlled observations/markers; no raw
bearer/ticket/reference/body/URL/query or arbitrary exception strings.

Overflow/budget/invalid-line/closed-sink drops increment capped count. TOOL result
adds console_queued bit only in timed mode: accepted into queue does NOT mean
delivered to console. Operator local proof-stats reports bounded counts/flags;
it does not change session/auth state and shares existing16-line console budget.
Matching console marker is still required for manual proof. Lost/partial logs
cannot be treated as negative evidence or reconstructed from counters. A busy
queue may delay/drop ACK visibility; operator must wait for actual ACK before a
dependent step. No automatic retries or silently granting authority on log loss.

Opt-in fs-probe-v3 includes monotonic elapsed_ms and request_elapsed_ms on TOOL,
STATUS (before ASGI send) and COMPLETE (after final body ASGI send returns).
Request clock begins at Guard ASGI entry, not remote dispatch/network ingress.
These stages do NOT prove peer receipt, TCP ACK or host business result. Timed
properties are opt-in; existing fs-probe-v2 fields/behavior stay for other carriers.

Timeout remains cooperative asyncio20s around body/SDK processing. It is NOT a
preemptive wall-clock limit over synchronous header-auth, stalled event loop,
native thread work, OS suspend, filesystem/control lock or network peer. Queue
isolates this stdout sink only; it cannot preempt arbitrary SDK/core/native work.
No host timeout constant or cancellation guarantee follows from a late response.

Six new native tests prove bounded concurrent producers while sink stalls,
saturating counts/input limits, sink-exception secrecy/closure, fixed config,
cooperative timeout HTTP408 still emitted while writer is blocked, and exact
fake-clock TOOL/STATUS/COMPLETE timestamps plus matched marker. Test sink is a
controlled threading.Event stall, not a reproduction of Windows console selection
or the FranceStudent timeout. All resolver tests/real MCP smoke are rerun onv2.
Additional model/host trial is not required to prove this local isolation mechanism.

Helper now runs23 session/bootstrap/channel tests and existing actual MCP smoke;
full inherited regressions ensure default carriers retain their behavior. New
listener uses sameport8006/three harmless tools and local lifecycle grammar plus
proof-stats. No production effect exposure or new remote diagnostic-control tool.
Keep Draft, do not merge; H-01 host overlap/correlation and H-02/H-03 production
storage/permissions/HostContext/real SessionHandle gates remain open.
