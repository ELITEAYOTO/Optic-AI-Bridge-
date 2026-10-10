# H-01 host capability observation

Copy this file for each real host/surface test. Record only probe output/fingerprints and product behavior; do not paste access tokens, cookies or credentials.

## Test identity

- Date/time:
- Host/product: `FranceStudent | ChatGPT | other`
- Surface: `web | desktop | Work | normal Chat | other`
- Account/plan (only if relevant to feature availability):
- Probe commit SHA:
- Probe version: `h01-v1`
- Connectivity provider: `Cloudflare Quick Tunnel | Tailscale | direct/custom`
- MCP endpoint path: `/mcp`

## Discovery and basic call

- MCP server accepted by host: `yes/no`
- Tools discovered: `yes/no`
- `probe_ping` callable: `yes/no`
- Host showed its own confirmation before call: `yes/no/not observed`
- Notes/error text:

## Same-conversation correlation

Run `probe_session_correlation` twice with the same test run id in one conversation.

### Call 1

```json
<paste sanitized probe result>
```

### Call 2

```json
<paste sanitized probe result>
```

- `call_index` observed: `1 -> 2` expected
- `same_transport_session_as_first`:
- Stable header fingerprint names:
- Stable `_meta` fingerprint names:
- Candidate conversation/session signal(s):
- Signals that changed between calls:

## New-conversation separation

Start a genuinely new conversation and use a **new** test run id.

```json
<paste sanitized probe result>
```

- Which candidate correlation signals changed compared with the prior conversation?
- Which remained identical?
- Is there enough evidence to distinguish conversations? `yes/no/uncertain`

## Reconnect behavior

Without intentionally creating a new conversation, disconnect/reconnect the client or MCP connection if the host permits it, then call the probe again with a dedicated run id.

- Reconnect procedure:
- Transport/session signal before:
- Transport/session signal after:
- Candidate conversation metadata before:
- Candidate conversation metadata after:
- Did the user-visible conversation remain the same? `yes/no`
- Notes:

## Parallel calls

Request two `probe_parallel(delay_ms=500)` calls concurrently if the host supports parallel tool dispatch.

- Both dispatched concurrently: `yes/no/uncertain`
- Approximate wall-clock behavior:
- Per-call elapsed values:
- Same candidate context on both calls: `yes/no/uncertain`
- Notes:

## Cancellation / timeout

Only if the host exposes a safe way to cancel a pending harmless probe call.

- Tested: `yes/no`
- Procedure:
- Observable server/client result:

## Host interaction capabilities

- Host-owned confirmation dialogs: `none/per-call/conditional/unknown`
- MCP elicitation exposed to this server: `yes/no/not tested`
- Any visible session/conversation metadata in sanitized output:
- Other host-specific behavior:

## H-01 conclusion for this adapter/surface

- Basic Streamable HTTP compatibility: `proven/not proven`
- Reliable same-conversation correlation candidate: `proven/not proven/uncertain`
- Reliable new-conversation separation candidate: `proven/not proven/uncertain`
- Reconnect semantics understood: `yes/no`
- Parallel semantics understood: `yes/no`
- Safe to design an H-03 adapter mapping from this evidence: `yes/no`

### Important

No observed header, `_meta` field, tunnel identity or transport session id becomes Optic authorization authority. H-03 may use proven signals only as correlation inputs before minting an independent Optic-owned `SessionHandle`.
