# Windows Manual Smoke Test

**Status:** pre-alpha developer smoke guide
**Baseline:** `main` after PR #33 (`aee4f168`), with Phase 2D2 Git read merged
**Purpose:** validate the real Windows machine and MCP host without granting broad authority to an important repository.

## What this smoke test is for

This is a developer validation, not an installer or production-acceptance test. It is intended to prove that the binary builds and starts on the target Windows machine, that an MCP stdio client can connect, and that each authority is exposed only when the operator explicitly enables it.

Use a disposable Git repository. Do not use an important working tree for the first mutation tests.

## Current tested surface

The repository CI already compiles, lints and tests on `windows-latest` and `ubuntu-latest`, and runs `cargo-deny`. The current MCP surface includes:

- always available: `fs_list`, `fs_read`, `session_info`;
- process tools only when an executable was operator-authorized: `process_start`, `process_read`, `process_stop`, `process_result`;
- `session_cancel`;
- durable Windows mutation tools only when matching operator-owned authority is configured: `fs_write`, `fs_apply_patch`, `fs_delete`;
- Git read tools only when the operator supplies one absolute Git executable: `git_status`, `git_diff`, `git_log`.

Not ready yet: `GitIntegrate`, worktree integration ownership, merge/cherry-pick/rebase/reset, installer/autoconfiguration, public multi-session orchestration, and a general replay/idempotency ledger.

## Preconditions

1. Windows 11 or a supported modern Windows environment.
2. Git installed if Git read tools will be tested.
3. Rustup available. The repository pins Rust `1.99.0` through `rust-toolchain.toml`.
4. A disposable directory outside valuable source trees, for example `C:\optic-smoke\repo`.
5. A recovery directory outside that workspace if mutation tools are tested, for example `C:\optic-smoke\state`.
6. An MCP stdio-capable client for the protocol-level tests.

## Low-load local build

From the repository root in PowerShell:

```powershell
$env:CARGO_BUILD_JOBS = "1"
cargo build -p optic-bridge-app --release -j 1
```

Expected binary:

```text
target\release\optic-bridge.exe
```

Optional low-load local verification before protocol testing:

```powershell
$env:CARGO_BUILD_JOBS = "1"
cargo test -p optic-bridge-core -p optic-bridge-policy -p optic-bridge-runtime -p optic-bridge-mcp -- --test-threads=1
```

This local verification is useful, but GitHub Actions remains the exact-SHA CI gate for the repository.

## Prepare a disposable Git workspace

```powershell
New-Item -ItemType Directory -Force C:\optic-smoke\repo | Out-Null
Set-Location C:\optic-smoke\repo
git init
git config user.name "Optic Smoke"
git config user.email "optic-smoke@example.invalid"
"alpha" | Set-Content tracked.txt
git add tracked.txt
git commit -m "initial"
where.exe git
```

Use an absolute path returned by `where.exe git` for `--git-executable`. The Git runtime will additionally canonicalize and validate the executable and requires the configured workspace to be the exact Git worktree top-level.

## Test 1 — safest startup: read-only filesystem

Start with no process, mutation or Git authority:

```powershell
.\target\release\optic-bridge.exe C:\optic-smoke\repo
```

The process uses MCP over stdio and normally waits for its client. The important security expectation is that mutation/process/Git tools are not registered in this configuration.

Through the MCP client, verify:

- `session_info` works;
- `fs_list` can list the disposable workspace;
- `fs_read` can read `tracked.txt`;
- no `fs_write`, `fs_apply_patch`, `fs_delete`, `process_start`, `git_status`, `git_diff` or `git_log` tool is exposed unless its authority is explicitly configured.

## Test 2 — Git read authority only

Stop the bridge, then restart it with an absolute Git path:

```powershell
.\target\release\optic-bridge.exe --git-executable="C:\Program Files\Git\cmd\git.exe" C:\optic-smoke\repo
```

Adjust the Git path to the real canonical executable on the machine.

Expected MCP behavior:

- `git_status` is present and returns bounded porcelain-v2 data as base64;
- after editing `tracked.txt`, `git_diff` returns a bounded diff;
- `git_log` returns bounded commit metadata and a server-validated cursor;
- the caller cannot provide another repository path or Git executable;
- no Git mutation/integration tool exists yet.

## Test 3 — process authority, one executable only

Choose one harmless absolute executable, for example the real path of `cmd.exe` or another dedicated test executable. Restart the bridge with only that executable allowed:

```powershell
.\target\release\optic-bridge.exe --allow-executable="C:\Windows\System32\cmd.exe" C:\optic-smoke\repo
```

Verify that:

- `process_start` accepts only the configured executable;
- an unconfigured executable is rejected;
- `process_read`, `process_result` and `process_stop` operate through opaque JobIds rather than caller-supplied PIDs;
- process descendants are contained by the Windows Job Object limits.

Do not enable network access for this smoke test. The current process path remains deny-by-default for network authority.

## Test 4 — narrow write authority

Create the recovery directory outside the workspace:

```powershell
New-Item -ItemType Directory -Force C:\optic-smoke\state | Out-Null
```

Prefer a narrow prefix first:

```powershell
New-Item -ItemType Directory -Force C:\optic-smoke\repo\scratch | Out-Null
.\target\release\optic-bridge.exe `
  --mutation-state-dir="C:\optic-smoke\state" `
  --allow-write-scope="prefix:scratch" `
  C:\optic-smoke\repo
```

Verify that:

- `fs_write` and `fs_apply_patch` are exposed;
- `fs_delete` is still absent;
- writes under `scratch` succeed only with explicit expected state (`absent` or exact content version as required by the tool contract);
- writing outside `scratch` is denied;
- stale expected versions fail without modifying the target.

## Test 5 — delete authority separately

Only after Test 4 succeeds, add a separate delete scope:

```powershell
.\target\release\optic-bridge.exe `
  --mutation-state-dir="C:\optic-smoke\state" `
  --allow-write-scope="prefix:scratch" `
  --allow-delete-scope="prefix:scratch" `
  C:\optic-smoke\repo
```

Verify that delete requires the exact current content version and cannot be performed as a blind delete. After a successful operation, restart the bridge with the same state directory and confirm startup recovery is clean.

## Test 6 — combined developer smoke

After the isolated tests pass, combine only the authorities actually needed:

```powershell
.\target\release\optic-bridge.exe `
  --git-executable="C:\Program Files\Git\cmd\git.exe" `
  --allow-executable="C:\Windows\System32\cmd.exe" `
  --mutation-state-dir="C:\optic-smoke\state" `
  --allow-write-scope="prefix:scratch" `
  --allow-delete-scope="prefix:scratch" `
  C:\optic-smoke\repo
```

Do not broaden to `--allow-write-scope=all` or `--allow-delete-scope=all` until narrow-scope behavior has been observed on the target PC.

## What to capture during the first PC test

Record:

- Windows version;
- `rustc --version` and `cargo --version`;
- absolute Git path used;
- build result and peak CPU/RAM if relevant;
- bridge stderr startup line;
- MCP client name/version;
- tool list with no authorities, then with each authority enabled;
- one success and one expected denial for filesystem read, process, mutation and Git read;
- any Windows Defender/antivirus or permission prompts;
- any recovery journal left after a successful mutation/restart.

Do not include secrets or private source content in bug reports.

## Pass criteria for the first PC smoke

The machine-level smoke passes when:

1. `optic-bridge.exe` builds and starts with one Cargo build job;
2. the MCP client connects over stdio;
3. read-only tools work in the disposable workspace;
4. optional tools appear only when their operator-owned authority is configured;
5. a narrow-scope write succeeds and an out-of-scope write is denied;
6. exact-version delete succeeds only inside its configured delete scope;
7. Git status/diff/log work against the exact configured repository;
8. restart with the recovery state directory produces no unresolved recovery after successful mutations;
9. no unexpected process remains after stopping/cancelling process jobs.

## What this test does not prove

A successful smoke does not mean the project is production-ready. In particular it does not prove sudden-power-loss ACID durability, hostile local-kernel resistance, AppContainer/LPAC confinement, installer quality, multi-session public orchestration, or Git integration/mutation safety. Those remain later gates.
