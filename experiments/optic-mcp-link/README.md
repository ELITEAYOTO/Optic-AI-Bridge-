# Real Optic MCP link experiment

This experiment connects a Streamable HTTP MCP surface to the **real production `optic-bridge` stdio server** while keeping the workspace disposable and synthetic.

It exists to collect integration evidence before H-02/H-03/H-06 are finished. It is **not** the production remote transport.

## Architecture

```text
MCP host/client
  -> Streamable HTTP /mcp
  -> experiment Python adapter
  -> long-lived MCP stdio client
  -> real target/debug/optic-bridge(.exe)
  -> disposable synthetic temp workspace
```

The Python layer does not reimplement Optic file policy. It forwards the three explicitly allowlisted calls to the real Rust MCP server.

## Exact remote surface

Only these tools are exposed:

- `optic_session_info` -> upstream `session_info`
- `optic_fs_list` -> upstream `fs_list`
- `optic_fs_read` -> upstream `fs_read`

The adapter does **not** expose or dynamically forward:

- `session_cancel`
- `process_start`, `process_read`, `process_result`, `process_stop`
- `fs_write`, `fs_apply_patch`, `fs_delete`
- Git tools
- arbitrary caller-selected upstream tool names

The real upstream server may advertise more local stdio tools, but they remain unreachable from this HTTP adapter.

## Synthetic workspace only

At adapter startup a fresh temporary directory is created with only generated fixture content:

```text
public/hello.txt
nested/info.json
```

`public/hello.txt` contains a random per-start fixture marker. The directory is deleted when the adapter exits.

**Never modify this experiment to accept a caller-supplied or real project workspace path.** Audit issue #176 confirmed that the current typed `ReadSensitiveScope` is not yet backed by an explicit sensitive-path classifier, so this experiment must not expose a real repository even though the current local stdio server is otherwise bounded to its configured workspace.

## What the automated smoke proves

The dedicated workflow:

1. builds the actual Rust `optic-bridge` binary;
2. starts this adapter with that binary;
3. connects over real MCP Streamable HTTP;
4. verifies the remote tool list is exactly the three-tool allowlist;
5. forwards `session_info` to the real Optic application session;
6. lists and reads the generated fixture through real Optic policy/lease/filesystem code;
7. sends two reads concurrently through the same long-lived upstream stdio session;
8. verifies a traversal-style project-relative path is rejected by real Optic.

Every successful adapter tool call also returns an `adapter_invocation` object with a random marker and sequence. The same proof is printed locally as `[REAL-LINK] TOOL_CALL ...` so later FranceStudent output can be checked against the local console.

## Windows setup

From the repository root:

```powershell
cd experiments\optic-mcp-link\scripts
.\Setup-RealOpticLink.ps1
```

This reuses the isolated Python environment from H-01 and builds only the real `optic-bridge` binary.

Start the adapter on loopback:

```powershell
.\Start-RealOpticLink.ps1
```

In another PowerShell window:

```powershell
cd experiments\optic-mcp-link\scripts
.\Test-RealOpticLink.ps1
```

## Temporary FranceStudent/Quick-Tunnel test

Only after the local smoke succeeds, the synthetic adapter can be started with the explicit experiment switch:

```powershell
.\Start-RealOpticLink.ps1 -AllowRandomTunnelHost
```

Then reuse the H-01 Quick Tunnel helper in another window:

```powershell
cd ..\..\host-probe\scripts
.\Start-CloudflareQuickTunnel.ps1
```

Use the printed `https://...trycloudflare.com/mcp` endpoint in the host.

This unauthenticated public exposure is acceptable **only for this generated fixture with the narrow three-tool allowlist**. It must never be reused as H-02 production authentication and must never point at user files.

## Evidence we still do not infer

A successful link does not prove:

- FranceStudent conversation identity;
- production authentication semantics;
- safe real-project sensitive-file filtering;
- multi-session `SessionResolver` behavior;
- production remote transport architecture.

Those remain owned by H-01/H-02/H-03/H-06.
