# H-01 harmless host capability probe

This experiment characterizes what an AI host actually sends to an MCP Streamable HTTP server **before** Optic production transport/session code is changed.

It is intentionally incapable of accessing project files, Git, processes, the Optic runtime, Windows settings, secrets, or mutation APIs.

Current probe schema: `h01-v2`.

## Tools

- `probe_ping` — liveness plus a redacted request observation;
- `probe_host_context` — redacted HTTP/MCP context;
- `probe_session_correlation(run_id)` — compares this request with the first request for the same caller-selected test id;
- `probe_parallel(delay_ms)` — harmless bounded delay for concurrency tests;
- `probe_reset(run_id)` — forgets one in-memory correlation run.

Header names containing credential-like terms are omitted completely. Other unknown header/meta values are represented only by short one-way fingerprints so repeated values can be compared without exposing raw identifiers.

Fingerprints are intentionally keyed with a fresh random secret each time the probe starts. They are stable only while that one probe process remains alive and cannot be compared across restarts. Keep the same server process running for same-conversation, new-conversation and reconnect comparisons.

Each test run is bounded to 16 observations and the process retains at most 32 run ids. Reaching the event limit fails explicitly so the original correlation baseline is never silently replaced.

## Verifying that a model really called the tool

A model can sometimes answer with JSON that looks plausible without actually invoking an MCP tool. H-01 v2 therefore adds a server-minted proof marker to every successful probe call.

Each real tool result contains:

```json
"probe_invocation": {
  "tool": "probe_session_correlation",
  "sequence": 7,
  "marker": "<16 hex chars>"
}
```

At the same moment, the local server prints a line like:

```text
[H01] TOOL_CALL tool=probe_session_correlation seq=7 marker=<same marker>
```

Treat a manual host result as **verified** only when the tool name, sequence and random marker match the local console. Ordinary Uvicorn `POST /mcp` lines prove transport activity, but they do not prove which tool produced a model-visible result.

The marker is random per invocation, has no project authority and is not derived from host/user metadata.

## Request-id interpretation

MCP v2 request ids are optional. H-01 v1 always fingerprinted `str(request_id)`, so an absent request id (`None`) could look like a stable fingerprint across calls. H-01 v2 fixes that ambiguity with:

- `request_id_present: true|false`;
- `request_id_fingerprint: <fingerprint>|null`.

Do not interpret a v1 constant request-id fingerprint as a conversation identifier unless independent evidence proves the underlying id was actually present.

## Parallel-call proof

`probe_parallel` now records:

- `active_on_entry` — how many parallel probes were active after this call entered;
- `overlap_observed` — `true` when this invocation genuinely overlapped another `probe_parallel` call in the same server process.

The server tracks only its own random invocation markers; no host/user identifier is used for this proof.

## Local setup on Windows

Requirements:

- Windows PowerShell;
- Python 3.10 or newer available through `py.exe`;
- Internet access during first dependency installation.

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

The setup creates an isolated `.venv` and installs only the dependency set declared by `requirements.txt`; it does not upgrade the user's Python or global `pip` installation.

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

Keep the Quick Tunnel open only for the test window and stop it afterwards. H-01 intentionally has no production authentication because one of its goals is to determine which authentication/header mechanisms the host can actually carry before H-02 is designed.

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

For each returned JSON object, compare `probe_invocation` with the matching `[H01] TOOL_CALL` console line before treating it as evidence.

Then open a **genuinely new conversation** while leaving the probe server running and call `probe_session_correlation` again with the **same run id**. That third verified call directly compares the new conversation with the first verified conversation recorded by the probe.

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
