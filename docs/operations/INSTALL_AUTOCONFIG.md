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
- installation roots are ownership-marked and recursive removal refuses unrelated non-empty custom directories;
- uninstall removes only a recognized dedicated marketplace/plugin/install directory.

## Implemented flow

`scripts/Install-OpticAIBridge.ps1` currently:

1. resolves and validates the selected workspace;
2. validates the requested installation root, rejects filesystem roots/files/unrecognized non-empty custom directories, and writes a path-bound `.optic-ai-bridge-install.json` ownership marker for new installs;
3. validates the bundled/prebuilt `optic-bridge.exe`;
4. locates the ChatGPT Desktop/Codex plugin manager;
5. detects Git and only enables Git read when the selected workspace is the exact repository root; optional `-EnableGitIntegration` remains a distinct explicit opt-in;
6. installs the executable under `%LOCALAPPDATA%\OpticAIBridge\bin`;
7. provisions mutation recovery state under `%LOCALAPPDATA%\OpticAIBridge\state` when mutations are enabled;
8. materializes the compatibility plugin from `packaging/chatgpt-plugin`;
9. generates the local `.mcp.json` with the exact executable, Git, state and workspace paths;
10. creates a dedicated local marketplace under `%LOCALAPPDATA%\OpticAIBridge\marketplace`;
11. registers and installs `optic-ai-bridge-local@optic-ai-bridge` through the Codex plugin manager;
12. runs `scripts/Test-OpticAIBridge.ps1`, which performs MCP `initialize` and `tools/list` over real stdio;
13. verifies the installed plugin and the `optic` MCP server are visible;
14. prints the one remaining user action: restart ChatGPT Desktop and open a new Chat.

When `-EnableGitIntegration` is explicitly supplied, the installer additionally requires an exact Git repository root with an existing `HEAD`, allocates a non-overlapping user-scoped integration root, bootstrap-disables prompts/system+global Git config/replacement objects/hooks, rejects a symbolic internal ref, create-only initializes `refs/optic/integration/chatgpt` with `update-ref --no-deref <ref> <head> <zero>` only when absent, preserves any existing direct ref, adds `git_integration_status` + `git_integrate` to the generated allowlist, and configures `git_integrate` for prompt approval. `-ReadOnly` and `-EnableGitIntegration` are intentionally incompatible.

## Default grants

The default quick-install profile exposes:

- read/list for the configured workspace;
- Git status/diff/log only when Git and exact repository-root validation succeed;
- write/patch under the structural `scratch/` prefix;
- delete under the structural `scratch/` prefix;
- no process executable allowlist;
- no network authority;
- no Git integration authority unless `-EnableGitIntegration` is explicitly supplied.

The default ChatGPT plugin itself enables only the intended eight file/Git tools. Mutation tools use prompt approval. The opt-in Git-integration profile adds `git_integration_status` plus prompt-gated `git_integrate`; it does not change the internal target ref or integration root into caller-controlled parameters.

A `-ReadOnly` installer switch omits mutation authority entirely and cannot be combined with Git integration.

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
5. publishes the ZIP and checksum as a GitHub **prerelease** using the repository `GITHUB_TOKEN`.

This removes Rust compilation from the end-user install path.

## CI guardrails

Normal CI validates PowerShell script syntax on Windows and uses `--locked` for Rust Clippy/tests. Installer-profile coverage also proves that a non-empty unmarked custom install root is rejected by both install and uninstall while its sentinel content remains untouched. Windows CI also builds the real bridge, runs the exact-head MCP integration smoke, and exercises installer profiles entirely under the runner workspace: default integration absence, read-only rejection, symbolic-ref rejection, explicit opt-in arguments/tool allowlist/prompt policy/ref bootstrap/doctor, preservation of an existing direct ref across reinstall, and explicit integration-ref uninstall cleanup. This prevents dependency-manifest drift and validates packaging without touching a real ChatGPT user profile.

## Still not claimed

This developer-preview flow is not yet a signed MSI/MSIX installer, automatic updater, enterprise deployment package, or production support commitment. The project remains pre-alpha until its broader roadmap/security gates are complete.
