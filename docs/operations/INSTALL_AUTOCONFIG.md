# Install and autoconfiguration

## Current status

A developer-preview Windows/ChatGPT Desktop install flow is now implemented and was integration-tested on 2026-10-04.

The end-user target is a **prebuilt release bundle + one PowerShell command**, with a target elapsed setup time under 3 minutes. Source compilation remains a separate contributor workflow.

See [`CHATGPT_DESKTOP_QUICK_INSTALL.md`](CHATGPT_DESKTOP_QUICK_INSTALL.md) for the user-facing instructions.

## Design goals

- user-scoped install; no administrator service;
- no mandatory Node.js, Python, Docker or Rust toolchain for release users;
- no OpenAI API key, API billing, or public tunnel for the local ChatGPT Desktop path;
- absolute local paths generated automatically rather than hand-editing `.mcp.json`;
- deny-by-default runtime authority preserved;
- self-test before success is reported;
- reinstall/update is idempotent enough for developer-preview use;
- uninstall removes the dedicated marketplace/plugin/install directory.

## Implemented flow

`scripts/Install-OpticAIBridge.ps1` currently:

1. resolves and validates the selected workspace;
2. validates the bundled/prebuilt `optic-bridge.exe`;
3. locates the ChatGPT Desktop/Codex plugin manager;
4. detects Git and only enables Git read when the selected workspace is the exact repository root;
5. installs the executable under `%LOCALAPPDATA%\OpticAIBridge\bin`;
6. provisions mutation recovery state under `%LOCALAPPDATA%\OpticAIBridge\state` when mutations are enabled;
7. materializes the compatibility plugin from `packaging/chatgpt-plugin`;
8. generates the local `.mcp.json` with the exact executable, Git, state and workspace paths;
9. creates a dedicated local marketplace under `%LOCALAPPDATA%\OpticAIBridge\marketplace`;
10. registers and installs `optic-ai-bridge-local@optic-ai-bridge` through the Codex plugin manager;
11. runs `scripts/Test-OpticAIBridge.ps1`, which performs MCP `initialize` and `tools/list` over real stdio;
12. verifies the installed plugin and the `optic` MCP server are visible;
13. prints the one remaining user action: restart ChatGPT Desktop and open a new Chat.

## Default grants

The default quick-install profile exposes:

- read/list for the configured workspace;
- Git status/diff/log only when Git and exact repository-root validation succeed;
- write/patch under the structural `scratch/` prefix;
- delete under the structural `scratch/` prefix;
- no process executable allowlist;
- no network authority.

The ChatGPT plugin itself enables only the intended file/Git tools. Mutation tools use prompt approval.

A `-ReadOnly` installer switch omits mutation authority entirely.

## Packaging

The canonical plugin source is:

```text
packaging/chatgpt-plugin/
  .codex-plugin/plugin.json
  assets/optic-ai-bridge.png.b64
```

The compatibility-only package is intentional. The MCP server configuration is generated at install time because executable/workspace/state paths are machine-specific.

The logo is stored as base64 text in Git so release/install automation can materialize the PNG without requiring image tooling on the target machine.

## Release automation

`.github/workflows/release-windows.yml` runs for `v*` tags. It:

1. installs the pinned Rust 1.99.0 toolchain on a Windows runner;
2. builds `optic-bridge-app` with `--locked --release`;
3. creates a Windows x64 bundle containing the executable, installer/doctor/uninstaller scripts, and plugin template;
4. writes a SHA-256 checksum;
5. publishes the ZIP and checksum as a GitHub Release using the repository `GITHUB_TOKEN`.

This removes Rust compilation from the end-user install path.

## CI guardrails

Normal CI validates PowerShell script syntax on Windows and uses `--locked` for Rust Clippy/tests. This prevents dependency-manifest drift from silently regenerating `Cargo.lock` during CI.

## Still not claimed

This developer-preview flow is not yet a signed MSI/MSIX installer, automatic updater, enterprise deployment package, or production support commitment. The project remains pre-alpha until its broader roadmap/security gates are complete.
