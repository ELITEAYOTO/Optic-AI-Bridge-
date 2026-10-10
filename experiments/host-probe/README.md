# H-01 harmless host capability probe

This experiment characterizes what an AI host actually sends to an MCP Streamable HTTP server **before** Optic production transport/session code is changed.

It is intentionally incapable of accessing project files, Git, processes, the Optic runtime, Windows settings, secrets, or mutation APIs.

## Tools

- `probe_ping` — liveness plus a redacted request observation;
- `probe_host_context` — redacted HTTP/MCP context;
- `probe_session_correlation(run_id)` — compares this request with the first request for the same caller-selected test id;
- `probe_parallel(delay_ms)` — harmless bounded delay for concurrency tests;
- `probe_reset(run_id)` — forgets one in-memory correlation run.

Header names containing credential-like terms are omitted completely. Other unknown header/meta values are represented only by short one-way fingerprints so repeated values can be compared without exposing raw identifiers.

## Local setup on Windows

```powershell
cd experiments\host-probe\scripts
.\Setup-HostProbe.ps1
.\Start-HostProbe.ps1
```

In a second PowerShell window:

```powershell
cd experiments\host-probe\scripts
.\Test-HostProbe.ps1
```

## Temporary Cloudflare Quick Tunnel

For H-01 only, restart the probe with the explicit random-host compatibility switch:

```powershell
.\Start-HostProbe.ps1 -AllowRandomTunnelHost
```

Then in another window:

```powershell
.\Start-CloudflareQuickTunnel.ps1
```

Append `/mcp` to the printed `https://...trycloudflare.com` URL. The DNS-rebinding protection relaxation is deliberately explicit and is acceptable only for this authority-free temporary probe; it is **not** a production Optic default.

## FranceStudent manual protocol

Create a fresh test conversation and connect the temporary `/mcp` endpoint with no authentication **only for H-01**.

If the FranceStudent MCP configuration supports custom headers, add one harmless probe header such as:

```text
X-Optic-Probe: h01-francestudent
```

This value is not a credential. Seeing the header name in `probe_host_context` is useful H-02 evidence that the host can transmit custom HTTP headers. Do not use a real token during H-01.

Choose one random run id, for example `fs-20261010-a1b2c3d4`, and keep that **same run id** until the conversation-separation/reconnect comparisons are finished.

Ask the host to call:

1. `probe_ping` once;
2. `probe_host_context` twice;
3. `probe_session_correlation` twice with the same run id;
4. two `probe_parallel(delay_ms=500)` calls at the same time if the host can dispatch parallel tools.

Then open a **genuinely new conversation** while leaving the probe server running and call `probe_session_correlation` again with the **same run id**. That third call directly compares the new conversation with the first conversation recorded by the probe.

If possible, also test a disconnect/reconnect while keeping the same user-visible conversation and call `probe_session_correlation` again with the same run id.

Only after those comparisons, call `probe_reset` for the run id. Save the sanitized tool results. We care about whether fingerprints/metadata are stable, not their raw values.

## ChatGPT manual protocol

Run the same protocol only on a ChatGPT surface/plan that actually permits connecting the endpoint. Record the surface (normal Chat, Work, desktop, web) separately. A missing MCP connection feature is a host limitation, not a failed probe.

If the connection UI supports harmless custom headers, use a non-secret `X-Optic-Probe` value there as well. Do not put credentials into H-01.

## H-01 interpretation

The probe is successful when we have enough real evidence to describe, per host adapter:

- discovery/call compatibility;
- delivered request metadata;
- whether harmless custom headers can be delivered;
- same-conversation correlation candidates;
- new-conversation separation candidates;
- reconnect behavior;
- parallel invocation behavior;
- cancellation/timeouts if observable;
- any host-owned confirmations or interaction limits.

No value observed here becomes authorization authority. H-03 will define the actual Optic `SessionResolver` contract.
