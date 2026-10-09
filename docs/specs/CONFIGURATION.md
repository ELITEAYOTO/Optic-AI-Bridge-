# Configuration Specification

## Design

Configuration is layered: secure compiled ceilings → operator startup configuration → application-owned session/task authority. Lower layers cannot override hard security invariants.

The compiled `HardLimits` remain the non-overridable security ceiling. Operator authority may be supplied either through the existing command-line startup surface or through one strict versioned JSON configuration file. The two modes are intentionally exclusive; Optic does not merge CLI authority into a persistent config implicitly.

## Current compiled hard ceilings

`HardLimits::default()` currently defines:

- request body: 1 MiB;
- response body: 256 KiB;
- request lifetime: 30 s;
- concurrent requests: 16;
- retained application sessions: 16;
- active/reserved process output RAM: 16 MiB bridge-wide, 8 MiB per session;
- single filesystem read: 256 KiB;
- single filesystem mutation payload/result ceiling: 8 MiB;
- mutation recovery journal file: 64 KiB;
- mutation recovery records scanned at startup: 256;
- `fs_list` page: 256 entries;
- deterministic directory scan ceiling: 4096 entries;
- Git read bytes: 256 KiB;
- Git log page hard ceiling: 256 entries;
- active process jobs: 8 bridge-wide, 4 per session;
- retained process records: 64 bridge-wide, 32 per session;
- single process output read: 64 KiB;
- process timeout: 1 h;
- process output per job: 8 MiB;
- process memory: 8 GiB;
- process count: 32.

Zero is rejected for every hard safety limit; it never means unlimited. Phase 3A additionally requires every per-session process ceiling to fit within its corresponding bridge-wide ceiling, the per-session retained-record ceiling to cover the per-session active-job ceiling, and the per-job output budget ceiling to fit within the per-session reserved-output ceiling.

## Current startup surface

The application accepts one optional positional workspace path. When omitted, the current directory is used.

### Versioned persistent config

```text
optic-bridge --config <absolute-json-file>
```

`--config` is an exclusive startup mode: it cannot be mixed with positional workspace or authority flags. The file is read once at startup from the exact opened handle, must be a regular file, is capped at 256 KiB, and is parsed with unknown-field rejection. Unknown config versions fail closed.

Version 1 requires `version`, `policy_epoch` and `workspace`; authority sections are optional and default to empty. `policy_epoch` must be greater than zero and is reused consistently when provisioning the application session and its read, mutation, process and Git-integration leases. MCP callers cannot set or change it.

The v1 structure mirrors the existing operator surface rather than creating a second authorization model: executable classes, exact process-read grants, isolated Node selections, ToolProfile file, classified environment grants, mutation scopes/state, Git read and Git integration settings are compiled into the same internal startup arguments and therefore pass through the same canonicalization and fail-closed validation as CLI configuration.

Example:

```json
{
  "version": 1,
  "policy_epoch": 2,
  "workspace": "C:\\work\\project",
  "executables": [
    { "class": "interpreter", "path": "C:\\Program Files\\nodejs\\node.exe" }
  ],
  "isolated_node_executables": ["C:\\Program Files\\nodejs\\node.exe"],
  "environment_grants": [
    { "class": "benign", "name": "TEMP" },
    { "class": "sensitive", "name": "API_KEY" }
  ]
}
```

A config version migration is explicit: Optic does not silently reinterpret an unknown version, and changing persistent authority should be accompanied by an operator-selected `policy_epoch` bump when stale authority must be invalidated.

### Process authority

Repeatable:

```text
--allow-executable <fixed-tool|interpreter|repository-code>:<absolute-path>
--env-grant <benign|sensitive|forbidden>:<variable-name>
--allow-env <variable-name>  # legacy alias for benign:<variable-name>
```

- Each allowed executable must include an explicit operator-owned execution class. Legacy unclassified `--allow-executable <absolute-path>` values are rejected rather than receiving a default class.
- `fixed-tool` is for an operator assertion that the authorized binary is being used as a fixed tool; this is not automatic inspection or proof that the binary cannot load plugins/scripts.
- `interpreter` is for shells/language runtimes or other binaries whose arguments/files can directly express interpreted code (for example `cmd.exe`, PowerShell, Python or Node when used that way).
- `repository-code` is for direct project/repository code execution or tools whose authorized role causes repository-controlled code to run (for example a project binary, test runner/build execution path, or similar operator-classified execution).
- Each path is canonicalized and must resolve to a regular file before a process task lease is created. The same canonical executable path may appear only once per server lifecycle; duplicate/competing classes fail startup closed.
- `ProcessRun` is added to the application session only when at least one classified executable was authorized.
- The process MCP tools are part of the base tool router, but `process_start` fails closed unless the requested canonical executable exactly matches an operator-created lease. The execution class is resolved from that lease and is not a caller field.
- Child environment is cleared by default. Environment authority is application-owned and classified per normalized variable name.
- Only `benign` environment grants are exportable through the current MCP `env_allowlist`. `sensitive` and `forbidden` grants remain non-exportable and fail closed if requested.
- `--allow-env <name>` is retained as a compatibility alias for `--env-grant benign:<name>`; it does not bypass classification.
- The authority model stores names/classes only, never environment values. Values are resolved from the bridge process only at execution time after authorization.
- Process network access remains unavailable; a request with `network=true` fails closed.
- Phase 3C3B adds operator-owned classification. Phase 3C3C1 now enforces that classification: `fixed-tool` may execute under the current bounded runtime, while `interpreter` and `repository-code` authority may still be provisioned/classified but `process_start` fails closed with `optic.process_isolation_unavailable` until stronger isolation is implemented and proven. Provisioning a high-risk class therefore does not currently grant executable runtime access.

### Durable file-mutation authority

```text
--mutation-state-dir <absolute-path>
--allow-write-scope all|prefix:<workspace-path>
--allow-delete-scope all|prefix:<workspace-path>
```

Write/delete scope options are repeatable.

Rules:

- any write/delete scope requires `--mutation-state-dir`;
- the mutation state directory may also be supplied alone for recovery-only startup;
- the recovery state is validated by the transactional runtime and must remain outside the canonical workspace;
- `FileWrite` and `FileDelete` are provisioned separately and only for explicitly configured structural workspace scopes;
- `prefix:<workspace-path>` uses the safe project-relative `WorkspacePath` grammar, not string-prefix authorization;
- `fs_write` / `fs_apply_patch` are registered only with write authority; `fs_delete` is registered only with delete authority;
- bounded recovery runs before MCP service startup and unresolved recovery fails startup closed.

### Git read authority

```text
--git-executable <absolute-path>
```

The option may be supplied once.

Rules:

- the supplied Git path must be absolute and is canonicalized to a regular file by the Git runtime;
- the configured workspace must validate as the exact canonical Git worktree top-level;
- `GitRead` plus `git_status`, `git_diff` and `git_log` exist only when that runtime is successfully provisioned;
- callers cannot replace the repository root, Git executable or Git argv through MCP.

### Git integration recovery and authority

```text
--git-integration-executable <absolute-path>
--git-integration-root <absolute-path-outside-repository>
--git-integration-ref refs/optic/integration/<name>
--allow-git-integrate
```

Rules:

- `--git-integration-executable`, `--git-integration-root` and `--git-integration-ref` form one complete runtime tuple and must be supplied together; partial configuration fails closed;
- the integration executable is intentionally separate from `--git-executable`: integration/recovery configuration does not provision `GitRead`, and Git-read configuration does not provision `GitIntegrate`;
- the runtime canonicalizes the integration Git executable, requires the integration root to remain outside the canonical repository, and accepts only a direct operator-owned ref under `refs/optic/integration/`;
- supplying the complete tuple without `--allow-git-integrate` performs bounded recovery-only startup and creates no `GitIntegrate` session capability or task lease;
- `--allow-git-integrate` requires the complete tuple and provisions exactly one application-owned repository-scoped `GitIntegrate` lease plus the corresponding session capability;
- bounded orphan-worktree recovery runs before MCP serve and any unsafe/ambiguous recovery state fails startup closed;
- the authorized integration runtime is retained only when authority was explicitly provisioned; the MCP server rejects runtime/authority mismatch;
- `git_integrate` and `git_integration_status` are registered only when this explicit authority exists; recovery-only startup still exposes no integration tools.

## Future configuration shape

A layered file-based configuration may later expose smaller operator-selected values while retaining the compiled ceilings as non-widenable maxima. A future shape may include categories such as transport, session, filesystem, output and process settings, but that surface is not implemented today.

## Rules

Paths are never silently widened. Zero does not mean unlimited. Secrets/tunnel credentials do not belong in normal committed config. SDK/library defaults never override Optic-owned hard ceilings. Operator startup configuration may grant only capabilities/scopes that the application can normalize and enforce deterministically.
