# H-01 host capability observation

Copy this file for each real host/surface test. Record only probe output/fingerprints and product behavior; do not paste access tokens, cookies or credentials.

## Test identity

- Date/time:
- Host/product: `FranceStudent | ChatGPT | other`
- Surface: `web | desktop | Work | normal Chat | other`
- Account/plan (only if relevant to feature availability):
- Probe commit SHA:
- Probe version: `h01-v2`
- Connectivity provider: `Cloudflare Quick Tunnel | Tailscale | direct/custom`
- MCP endpoint path: `/mcp`
- Server instance id shown at startup:

## Evidence integrity

For every model-visible probe result used as evidence, verify that its `probe_invocation` object matches the local server console line:

```text
[H01] TOOL_CALL instance=<server_instance> tool=<tool> seq=<sequence> marker=<marker>
```

- Tool result marker matched console: `yes/no`
- Tool result server instance matched current process: `yes/no`
- Server-side invocation line preserved: `yes/no`
- If not matched, exclude that model-visible result from H-01 conclusions.

Ordinary `POST /mcp` access logs prove transport activity only; they do not identify the tool that produced a displayed JSON result.

## Discovery and basic call

- MCP server accepted by host: `yes/no`
- Tools discovered: `yes/no`
- `probe_ping` callable: `yes/no`
- `probe_ping` invocation marker verified: `yes/no`
- Host showed its own confirmation before call: `yes/no/not observed`
- Notes/error text:

## Harmless custom-header delivery

If the host configuration supports custom headers, configure a non-secret value such as `X-Optic-Probe: h01-<host>` before the test.

- Custom-header UI/config exists: `yes/no/unknown`
- `x-optic-probe` header name appears in sanitized probe output: `yes/no/not tested`
- Notes:

This proves only that the host can carry a custom header. It does not prove production authentication yet.

## Same-conversation correlation

Choose one random correlation run id and reuse that **same run id** for the same-conversation, new-conversation and reconnect comparisons until the final reset.

Run `probe_session_correlation` twice in the first conversation.

### Call 1

```json
<paste sanitized probe result>
```

- Invocation marker verified against console: `yes/no`

### Call 2

```json
<paste sanitized probe result>
```

- Invocation marker verified against console: `yes/no`
- `call_index` observed: `1 -> 2` expected
- `same_transport_session_as_first`:
- `request_id_present` call 1 / call 2:
- Request-id fingerprint stable only when actually present: `yes/no/not applicable`
- Stable header fingerprint names:
- Stable `_meta` fingerprint names:
- Stable client-capability observations:
- Candidate conversation/session signal(s):
- Signals that changed between calls:

## New-conversation separation

Start a genuinely new conversation while the probe server remains running, then call `probe_session_correlation` with the **same correlation run id** used above.

```json
<paste sanitized probe result>
```

- Invocation marker verified against console: `yes/no`
- Expected `call_index`: `3` unless another verified comparison was made first
- Which candidate correlation signals changed compared with call 1?
- Which remained identical?
- Is there enough evidence to distinguish conversations? `yes/no/uncertain`

## Reconnect behavior

Without intentionally creating another conversation, disconnect/reconnect the client or MCP connection if the host permits it, then call the correlation tool again with the **same correlation run id**.

- Reconnect procedure:
- Invocation marker verified against console: `yes/no`
- Transport/session signal before:
- Transport/session signal after:
- Candidate conversation metadata before:
- Candidate conversation metadata after:
- Did the user-visible conversation remain the same? `yes/no`
- Notes:

## Parallel calls

Choose a separate random **parallel run id**. Request two `probe_parallel(run_id=<same parallel run id>, delay_ms=500)` calls concurrently if the host supports parallel tool dispatch.

- Parallel run id:
- Both returned `run_id` values match the chosen parallel run id: `yes/no`
- Both invocation markers/server instances verified against console: `yes/no`
- `overlap_observed` values:
- `active_on_entry` values:
- Both dispatched concurrently: `yes/no/uncertain`
- Approximate wall-clock behavior:
- Per-call elapsed values:
- Same candidate context on both calls: `yes/no/uncertain`
- Notes:

`overlap_observed=true` is server-side proof that another `probe_parallel` invocation with the **same explicit parallel run id** was active during that call. Unrelated probe traffic in another run id cannot create this result.

## Cancellation / timeout

Only if the host exposes a safe way to cancel a pending harmless probe call.

- Tested: `yes/no`
- Procedure:
- Observable server/client result:

## Host interaction capabilities

- Protocol version reported by probe:
- Declared client capabilities in sanitized output:
- Host-owned confirmation dialogs: `none/per-call/conditional/unknown`
- MCP elicitation exposed to this server: `yes/no/not tested`
- Any visible session/conversation metadata in sanitized output:
- Other host-specific behavior:

## H-01 conclusion for this adapter/surface

- Basic Streamable HTTP compatibility: `proven/not proven`
- Harmless custom-header delivery: `proven/not proven/not tested`
- Reliable same-conversation correlation candidate: `proven/not proven/uncertain`
- Reliable new-conversation separation candidate: `proven/not proven/uncertain`
- Reconnect semantics understood: `yes/no`
- Parallel semantics understood: `yes/no`
- Safe to design an H-03 adapter mapping from this evidence: `yes/no`

After collecting all correlation comparisons, call `probe_reset` for the correlation run id.

### Important

MCP v2 request ids are optional. H-01 v2 exposes absence explicitly; do not treat a v1 constant request-id fingerprint as conversation identity without independent evidence that a real request id was present.

No observed header, `_meta` field, tunnel identity, request id, test run id, invocation marker, server-instance id or transport session id becomes Optic authorization authority. H-03 may use proven host signals only as correlation inputs before minting an independent Optic-owned `SessionHandle`.
# Separate guarded grouped-overlap supplement

The optional experiments/grouped-parallel carrier measures only grouped temporal
overlap with dummy authentication and minted group handles. It is separate from
this h01-v2 schema and from #178. Retain tested Git head, banner, server instance,
tool markers/request sequences, diagnostic group_id, wait invocation_id,
active_on_entry, completed entry/exit intervals and completed_pair_overlap.
Require matching GROUP enter/exit and TOOL console proof, exactlytwo successful
waits in the SAME group, strict interval intersection. Two displayed results,
native SDK gather tests and unrelated-group activity are not host evidence.
Do not paste bearer values or raw group handles into public evidence. Follow
experiments/grouped-parallel/README.md; correlation/new-chat/reconnect tests above
remain distinct, not completed by this narrower supplement.
