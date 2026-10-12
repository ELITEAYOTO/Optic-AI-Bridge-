# Real Optic stdio <-> Streamable HTTP MCP link experiment

**Status:** experiment-only / draft PR #178  
**Last reviewed:** 2026-10-12  
**Purpose:** prove a real end-to-end host integration path without exposing a real user workspace or rewriting the production Rust transport prematurely.

## Question

Can the existing real Rust `optic-bridge` stdio MCP application server be exercised by a remote Streamable HTTP MCP client through a narrow replaceable compatibility adapter, while keeping Optic's real session/policy/lease/filesystem runtime authoritative?

## Answer so far

Yes, for a synthetic read-only fixture.

The tested path is:

```text
Streamable HTTP MCP client
        |
        v
bounded Python compatibility adapter
        |
        v
real `optic-bridge` stdio MCP server
        |
        v
Optic session + leases + policy + bounded filesystem
        |
        v
disposable generated workspace
```

This is not the final production transport design. It is an evidence tool that lets H-01/H-02/H-03 learn from the real Optic core before production remote transport decisions are frozen.

## Safety boundary

The adapter creates the temporary workspace itself and does not accept a workspace root from the remote caller.

Remote surface is exactly:

- `optic_session_info` -> upstream `session_info`;
- `optic_fs_list` -> upstream `fs_list`;
- `optic_fs_read` -> upstream `fs_read`.

It does **not** forward:

- process tools;
- mutations;
- Git tools;
- `session_cancel`;
- arbitrary upstream method names.

The upstream real stdio server may advertise more tools internally, but the remote adapter allowlist is deliberately smaller.

## Synthetic fixture

Each adapter start creates a new temporary workspace containing generated harmless data such as:

```text
public/hello.txt
nested/info.json
```

`public/hello.txt` contains a random per-start fixture marker. The client smoke checks that marker through the remote HTTP tool result, proving the data came through the real Optic filesystem path rather than being reimplemented in the Python adapter.

The temporary workspace is deleted when the adapter stops.

## Invocation proof

Every successful adapter call creates a random proof:

```text
adapter_invocation = {
  tool,
  sequence,
  marker,
  adapter_instance
}
```

The same values are emitted locally as a `[REAL-LINK] TOOL_CALL ...` line.

This is evidence-integrity instrumentation only. The marker is never authority.

It prevents a host/model-generated JSON response from being accepted as proof of a real tool execution unless it matches local adapter activity.

## Proven automated behavior

On current experiment head `ef5bc84144670135d13f55a59a782611aeae762f`, dedicated workflow **Real Optic MCP Link Experiment #11** is green on Linux and Windows.

The smoke proves:

- real Rust `optic-bridge` starts behind the adapter;
- only the three intended remote tools are discoverable;
- upstream `session_info` reports read/search-only synthetic authority;
- upstream `fs_list` returns the generated fixture;
- upstream `fs_read` returns the generated random fixture marker;
- two remote reads can be forwarded concurrently through the same long-lived upstream stdio application session;
- `../outside.txt` is rejected by the real Optic workspace boundary;
- every successful adapter tool result includes a fresh invocation proof;
- a completely new HTTP/MCP client connection can reconnect to the same adapter process and reach the same upstream Optic application-session fingerprint;
- Windows and Linux both run the same behavioral smoke.

Normal repository CI #653 is also green on that head for dependency policy, Ubuntu and Windows, including the existing Windows isolation/AppContainer, isolated Node, reusable approval, Git integration and ChatGPT packaging smokes.

## Reconnect interpretation

The reconnect test proves:

> HTTP/MCP transport reconnection does not require the underlying Optic application session to be destroyed.

It does **not** prove:

- FranceStudent conversation identity;
- ChatGPT conversation identity;
- a host-provided session key;
- a production `SessionResolver`;
- remote authority continuity after credential/session revocation.

H-03 still owns those semantics.

## Admission and transport bounds

The experiment adapter currently limits:

- request body to 64 KiB;
- Uvicorn admitted HTTP tasks to 32;
- upstream forwards to 8 concurrent calls;
- forward admission wait to 250 ms;
- upstream call duration to 15 s;
- keep-alive to 5 s.

Normal loopback mode keeps DNS-rebinding protection enabled. Random tunnel-host acceptance is available only through an explicit experiment switch for the disposable synthetic fixture.

These bounds are useful evidence for H-06, but they are not production authentication or final production rate limiting.

## Dependency reproducibility

The Python compatibility environment uses:

- direct `mcp==2.3.0` requirement;
- a complete tested transitive constraints file shared with H-01;
- `pip check` on Windows and Linux;
- `pip freeze --all` evidence in CI.

This prevents implicit dependency drift from being mistaken for host/transport behavior changes.

## Why the experiment does not use a real repository

The current audit found unresolved security-composition gates:

- no complete sensitive-path classifier for content + metadata/listing (#176);
- generic broad file mutation must not reach repository-control metadata (#180);
- Unix native path segments must round-trip losslessly into `WorkspacePath` (#181);
- Git read needs an explicit sensitive-content relationship (#182);
- read/list/Git effects need lifecycle admission held through the effect for immediate revoke semantics (#183).

The synthetic fixture intentionally avoids those unresolved real-project questions while still exercising real Optic code.

## Manual Windows path prepared

PR #178 contains:

- `Setup-RealOpticLink.ps1`;
- `Start-RealOpticLink.ps1`;
- `Test-RealOpticLink.ps1`.

The intended manual order is:

1. reproduce the local synthetic loopback smoke;
2. only then enable the explicit temporary random-tunnel-host compatibility switch;
3. expose only the disposable synthetic fixture through a Quick Tunnel;
4. connect FranceStudent when its service is available;
5. pair every host result with local invocation proof.

Do not point this experiment at a real project workspace.

## Architecture consequence

The experiment supports this provisional deployment shape:

```text
host
  -> replaceable transport/host adapter
  -> existing real Optic stdio application service
  -> sessions / policy / leases / runtime
```

Therefore H-06 does not need to rewrite the core simply to prove remote-host behavior. The eventual production Streamable HTTP boundary may still move into Rust, but that should be an evidence-driven operational/security decision rather than a prerequisite assumption.

## Non-claims

This experiment does not claim:

- H-02 authentication complete;
- H-03 SessionResolver complete;
- remote real-project safety;
- process/mutation/Git remote safety;
- provider-specific production support;
- a production release.

It is a controlled bridge for gathering the next layer of evidence.