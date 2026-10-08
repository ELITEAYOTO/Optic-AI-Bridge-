# ChatGPT Desktop quick install (Windows)

**Status:** developer preview. The default Phase 2D2 ChatGPT Desktop profile was validated end-to-end on 2026-10-04. The optional Phase 2D3 exact-head Git-integration profile was validated end-to-end in a real normal ChatGPT Desktop chat on 2026-10-05, in addition to native-binary and installer-profile CI validation.

This is the intended end-user path. The **default** profile does **not** require an OpenAI API key, API credits, a tunnel, Node.js, Python, Docker, or a Rust compiler. Node.js is required only when the optional isolated-Node process profile is explicitly enabled. The release bundle contains prebuilt Windows bridge/isolation binaries plus the local ChatGPT plugin package and installer.

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
- The default plugin exposes only the same 8 intended file/Git tools even though the bridge has additional internal session/process/integration routes. Git integration is **not** silently enabled.

The installer creates `scratch/` when the default mutation profile is used.

### Read-only install

To expose only file reads and optional Git reads:

```powershell
.\Install-OpticAIBridge.ps1 -Workspace "C:\path\to\your\repository" -ReadOnly
```

### Optional isolated Node process profile

Node process execution is deliberately **off by default**. If Node.js is installed and you want ChatGPT to use the proven isolated Node profile, enable it explicitly:

```powershell
.\Install-OpticAIBridge.ps1 `
  -Workspace "C:\path\to\your\repository" `
  -EnableIsolatedNode
```

The installer auto-discovers the exact `node.exe`. An advanced user may instead provide `-NodePath "C:\absolute\path\to\node.exe"`, but `-NodePath` is rejected unless `-EnableIsolatedNode` is also present.

This profile exposes only `process_start`, `process_read`, `process_result`, and `process_stop`; ChatGPT is configured to prompt before `process_start`. The process lease remains capped to one logical process, no network authority is added, and the installer does **not** grant Node workspace-file access automatically. Exact process file reads still require the separate server-owned exact-file grant mechanism. The installer doctor performs a real MCP `node --version` through the installed AppContainer helper before reporting success.

`-ReadOnly` may be combined with `-EnableIsolatedNode`: in that combination, file mutation authority is absent, but the explicitly enabled isolated Node process can still execute. Treat `-ReadOnly` as a filesystem-mutation setting, not as a promise that no process will run.

### Optional exact-head Git integration preview

Git integration is deliberately **off by default**. On a workspace that is exactly a Git repository root with an existing `HEAD`, enable the preview explicitly:

```powershell
.\Install-OpticAIBridge.ps1 `
  -Workspace "C:\path\to\your\repository" `
  -EnableGitIntegration
```

This option cannot be combined with `-ReadOnly`. It keeps the existing Git read tools and additionally exposes:

- `git_integration_status` — read-only; returns only the exact current internal target commit needed as the optimistic-concurrency precondition;
- `git_integrate` — prompt-gated; fast-forwards only the fixed operator-owned internal Optic ref when the supplied `expected_target_head` still matches.

The client cannot choose or learn the internal ref, repository path, Git executable, worktree path, raw Git argv, lease, or ActionId through these tools. The installer uses a dedicated integration root under the user-scoped Optic install directory, disables Git prompts/system+global config/replacement objects for bootstrap, forces an empty hooks directory, rejects a symbolic `refs/optic/integration/chatgpt`, and creates that direct ref only when absent using `update-ref --no-deref` with a zero old OID. An existing direct ref is never reset by the installer.

## What the installer does

The installation is user-scoped under:

```text
%LOCALAPPDATA%\OpticAIBridge
```

It:

1. validates the workspace, bundled executable and installation root ownership; a new install writes `.optic-ai-bridge-install.json`, while a non-empty custom path without a valid Optic marker is refused rather than adopted;
2. detects the ChatGPT Desktop/Codex plugin manager;
3. detects Git and verifies whether the workspace is the exact Git root;
4. copies the bridge executable into the user-scoped install directory;
5. creates the local compatibility plugin package and `.mcp.json` with absolute paths;
6. registers a dedicated local marketplace named `optic-ai-bridge`;
7. installs/enables `optic-ai-bridge-local@optic-ai-bridge`;
8. performs a real MCP `initialize` + `tools/list` doctor test and, when `-EnableIsolatedNode` is selected, a real isolated `node --version` process smoke;
9. verifies that the `optic` MCP server is visible to ChatGPT Desktop.

No administrator service is installed.

## Updating

Install a newer release by extracting the new bundle and running the same command again. The installer replaces its user-scoped binary/plugin source and reinstalls the local plugin.

## Uninstall

From a release bundle:

```powershell
.\Uninstall-OpticAIBridge.ps1
```

The normal uninstall never mutates a repository. Recursive removal is also fail-closed: a non-empty custom `InstallRoot` must carry the matching Optic installation marker, so an arbitrary unrelated directory cannot be passed to the uninstaller and recursively erased. The historical default `%LOCALAPPDATA%\OpticAIBridge` path retains limited compatibility only when recognizable Optic installation artifacts are present. If the optional Git-integration profile was enabled and you also want to remove its internal Optic ref, request that repository mutation explicitly:

```powershell
.\Uninstall-OpticAIBridge.ps1 `
  -Workspace "C:\path\to\your\repository" `
  -RemoveGitIntegrationRef
```

The cleanup verifies the exact Git root, rejects a symbolic integration ref, disables hooks/prompts/system+global config, observes the direct ref OID, and deletes it with an expected-old-value comparison. Then restart ChatGPT Desktop.

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

## Verified integration evidence (2026-10-04 and 2026-10-05)

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

This validates the default developer-preview local integration. Phase 2D3 CI additionally validates the real Windows bridge binary and generated opt-in installer profile, and the 2026-10-05 real ChatGPT Desktop smoke validated `git_integration_status` → prompt-gated `git_integrate` → refreshed status → stale-precondition rejection on a disposable repository. Independent post-smoke Git inspection confirmed the caller branch/workspace did not move, no integration worktree remained, and recovery state was empty. None of this changes the lifecycle to production-supported or claims power-loss ACID durability.
