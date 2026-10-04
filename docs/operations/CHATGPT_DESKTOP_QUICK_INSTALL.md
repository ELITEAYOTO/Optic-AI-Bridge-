# ChatGPT Desktop quick install (Windows)

**Status:** developer preview, validated on 2026-10-04 against ChatGPT Desktop and the current Optic Phase 2D2 runtime.

This is the intended end-user path. It does **not** require an OpenAI API key, API credits, a tunnel, Node.js, Python, Docker, or a Rust compiler. The release bundle contains a prebuilt Windows `optic-bridge.exe` plus the local ChatGPT plugin package and installer.

## Target time

The target is **under 3 minutes** on a normal Windows machine once a prebuilt GitHub release bundle has been downloaded. Network/download time and the final ChatGPT Desktop restart are the only variable parts.

Building from source is a developer workflow and is intentionally not part of the 3-minute target.

## Requirements

- Windows with ChatGPT Desktop installed and opened at least once.
- A folder you want Optic to expose as the workspace.
- Git is optional. If Git is installed and the selected workspace is exactly a Git repository root, `git_status`, `git_diff`, and `git_log` are enabled automatically.

## Install from a release bundle

1. Download and extract `Optic-AI-Bridge-windows-x64.zip` from the project Releases page.
2. Open PowerShell in the extracted folder.
3. Run:

```powershell
.\Install-OpticAIBridge.ps1 -Workspace "C:\path\to\your\repository"
```

4. Fully close and reopen ChatGPT Desktop.
5. Start a **new normal Chat** and enter:

```text
@Optic AI Bridge Inspect the current workspace without modifying anything.
```

The installer performs its own MCP handshake/tool self-test before reporting success.

## Default security profile

The quick installer is deliberately conservative:

- `fs_list` and `fs_read` are enabled for the configured workspace.
- Git read tools are enabled only when Git is detected and the workspace is the exact repository root.
- `fs_write` and `fs_apply_patch` are limited to the structural workspace prefix `scratch/`.
- `fs_delete` is also limited to `scratch/`.
- ChatGPT is configured to prompt for mutation tools.
- No process executable is allowlisted, so `process_start` remains unusable from the plugin.
- The plugin exposes only the 8 intended file/Git tools even though the bridge has additional internal session/process routes.

The installer creates `scratch/` when the default mutation profile is used.

### Read-only install

To expose only file reads and optional Git reads:

```powershell
.\Install-OpticAIBridge.ps1 -Workspace "C:\path\to\your\repository" -ReadOnly
```

## What the installer does

The installation is user-scoped under:

```text
%LOCALAPPDATA%\OpticAIBridge
```

It:

1. validates the workspace and bundled executable;
2. detects the ChatGPT Desktop/Codex plugin manager;
3. detects Git and verifies whether the workspace is the exact Git root;
4. copies the bridge executable into the user-scoped install directory;
5. creates the local compatibility plugin package and `.mcp.json` with absolute paths;
6. registers a dedicated local marketplace named `optic-ai-bridge`;
7. installs/enables `optic-ai-bridge-local@optic-ai-bridge`;
8. performs a real MCP `initialize` + `tools/list` doctor test;
9. verifies that the `optic` MCP server is visible to ChatGPT Desktop.

No administrator service is installed.

## Updating

Install a newer release by extracting the new bundle and running the same command again. The installer replaces its user-scoped binary/plugin source and reinstalls the local plugin.

## Uninstall

From a release bundle:

```powershell
.\Uninstall-OpticAIBridge.ps1
```

Then restart ChatGPT Desktop.

## Developer/source install

Contributors can build the binary themselves and point the installer at it:

```powershell
$env:CARGO_BUILD_JOBS = "1"
cargo +1.99.0 build --locked -p optic-bridge-app --release -j 1

.\scripts\Install-OpticAIBridge.ps1 `
  -Workspace "C:\path\to\repository" `
  -BinaryPath ".\target\release\optic-bridge.exe" `
  -PluginTemplatePath ".\packaging\chatgpt-plugin"
```

This path is for development and is not expected to fit the end-user 3-minute target.

## Verified integration evidence (2026-10-04)

The current Windows smoke validated:

- ChatGPT Desktop started `server_name=optic` over stdio;
- MCP negotiated protocol `2025-06-18` with RMCP `3.4.0`;
- normal Chat successfully called `fs_read` and returned `tracked.txt` content `alpha`;
- the plugin exposed exactly the intended 8 tools: `fs_list`, `fs_read`, `fs_write`, `fs_apply_patch`, `fs_delete`, `git_status`, `git_diff`, `git_log`;
- direct smoke calls proved file listing/read and Git status/diff/log;
- scoped write, exact-version patch, and exact-version delete succeeded under `scratch/`;
- a stale content version failed with `optic.precondition_failed`;
- a write outside the configured scope failed with `optic.policy_denied`;
- an unallowlisted process failed with `optic.process_executable_not_allowed`;
- the mutation recovery directory was empty after successful retirement.

This validates the developer-preview local integration. It does **not** change the project lifecycle to production-supported or claim power-loss ACID durability.
