# ADR0015 — Operator Electron viewer for generated app acceptance

Status: IMPLEMENTED / CI PENDING
Date:2026-10-15

## Decision

The next FS fixture develops an agenda/budget renderer and tests pure logic in
the already authorized isolated Node profiles. A fixed external Electron viewer
is opened by the human operator after confirmed core session stop. Electron is
user-installed tooling; neither a core dependency nor a new MCP process grant.
No arbitrary generated Electron main/preload script is run with user privileges.
The AI can modify renderer sources only. General Electron process eligibility,
package/build/npm/GUI permissions remain separate future runtime gates.

After successful local session_cancel, the runner writes a create-only version1
readiness marker outside the remotely reachable project, with full versions of
five bounded UTF8 files. It is a stop/source snapshot marker, not proof of tests
or signature/authenticity. Viewer startup requires the marker and matching
source hashes. Generated run/root checks, no final symlinks, bounded exact reads
and in-memory copies avoid continued reads of mutable renderer sources.
Same-user local malicious replacement is outside this operator-fixture claim.

## Viewer boundary

Hashed package-owned main/policy files are outside generated file-write scope.
PowerShell launches an explicitly selected installed electron.exe without shell
evaluation or inherited NODE_OPTIONS/ELECTRON_RUN_AS_NODE. This is a normal
operator desktop process, NOT Optic AppContainer containment or leased GUI task.
Renderer: sandbox=true, contextIsolation=true, nodeIntegration=false, webSecurity
enabled, no preload/IPC surface/webview. Exact four immutable assets through a
custom standard secure scheme; tests.js and all other paths denied. Restrictive
CSP denies connections/frames/forms/objects/inline scripts. Requests outside the
assets and same-origin blob export are denied; navigation/new windows/permissions
are denied. Only trusted main code has Node access.

LocalStorage persists under this generated run's electron-data directory, outside
MCP project scope. Optional JSON blob downloads are limited to8 per opening,
2–65536bytes, application/json and fixed fresh .json paths under run/exports.
Unknown-size or external downloads denied. No arbitrary output path selector.
Chromium profile/cache writes are normal operator runtime storage, NOT a bounded
Optic task spool or hard filesystem quota claim. Viewer lifetime25minutes.
Import uses the user's normal file picker/Blob APIs; model must validate data,
size and schema. No private-file or privileged IPC access is added.

## Acceptance

Local Electron44.7.2 hidden native probe passed: renderer require/process absent;
reachable owned listener denied from renderer; file/out-of-asset access and new
window refused; localStorage survives actual close/reopen; fixed bounded JSON
export succeeds. Three policy tests and23 Python alpha tests pass. Windows CI
must repeat the native viewer test with pinned external Electron before package
delivery. Actual agenda/budget feature acceptance remains a manual FS exercise.

[Electron security guidance](https://www.electronjs.org/docs/latest/tutorial/security).
