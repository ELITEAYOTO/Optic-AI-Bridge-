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

Ask the host to call:

1. `probe_ping` once;
2. `probe_host_context` twice;
3. `probe_session_correlation` twice with the exact same random run id, e.g. `fs-20261010-a1b2c3d4`;
4. two `probe_parallel(delay_ms=500)` calls at the same time if the host can dispatch parallel tools;
5. `probe_reset` for the run id.

Then repeat the correlation call from a **new conversation** with a new run id. Save the tool results. We care about whether fingerprints/metadata are stable, not their raw values.

## ChatGPT manual protocol

Run the same protocol only on a ChatGPT surface/plan that actually permits connecting the endpoint. Record the surface (normal Chat, Work, desktop, web) separately. A missing MCP connection feature is a host limitation, not a failed probe.

## H-01 interpretation

The probe is successful when we have enough real evidence to describe, per host adapter:

- discovery/call compatibility;
- delivered request metadata;
- same-conversation correlation candidates;
- new-conversation separation candidates;
- reconnect behavior;
- parallel invocation behavior;
- cancellation/timeouts if observable;
- any host-owned confirmations or interaction limits.

No value observed here becomes authorization authority. H-03 will define the actual Optic `SessionResolver` contract.
