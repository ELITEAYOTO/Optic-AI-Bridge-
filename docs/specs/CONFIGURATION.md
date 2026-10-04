# Configuration Specification

## Design

Configuration is layered: secure compiled ceilings → operator startup configuration → application-owned session/task authority. Lower layers cannot override hard security invariants.

There is no general TOML configuration loader yet. The compiled `HardLimits` plus the current command-line startup options are authoritative today.

## Current compiled hard ceilings

`HardLimits::default()` currently defines:

- request body: 1 MiB;
- response body: 256 KiB;
- request lifetime: 30 s;
- concurrent requests: 16;
- active output RAM: 16 MiB;
- single filesystem read: 256 KiB;
- single filesystem mutation payload/result ceiling: 8 MiB;
- mutation recovery journal file: 64 KiB;
- mutation recovery records scanned at startup: 256;
- `fs_list` page: 256 entries;
- deterministic directory scan ceiling: 4096 entries;
- Git read bytes: 256 KiB;
- Git log page hard ceiling: 256 entries;
- active process jobs: 8;
- retained process records: 64;
- single process output read: 64 KiB;
- process timeout: 1 h;
- process output: 16 MiB;
- process memory: 8 GiB;
- process count: 32.

Zero is rejected for every hard safety limit; it never means unlimited.

## Current startup surface

The application accepts one optional positional workspace path. When omitted, the current directory is used.

### Process authority

Repeatable:

```text
--allow-executable <absolute-path>
--allow-env <variable-name>
```

- Each allowed executable is canonicalized and must resolve to a regular file before a process task lease is created.
- `ProcessRun` is added to the application session only when at least one executable was authorized.
- The process MCP tools are part of the base tool router, but `process_start` fails closed unless the requested canonical executable exactly matches an operator-created lease.
- Child environment is cleared by default; only variables allowed by the operator may be requested for inheritance.
- Process network access remains unavailable; a request with `network=true` fails closed.

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
- Phase 2D3B3 still registers no public `git_integrate` MCP tool. Public adapter exposure remains Phase 2D3C.

## Future configuration shape

A layered file-based configuration may later expose smaller operator-selected values while retaining the compiled ceilings as non-widenable maxima. A future shape may include categories such as transport, session, filesystem, output and process settings, but that surface is not implemented today.

## Rules

Paths are never silently widened. Zero does not mean unlimited. Secrets/tunnel credentials do not belong in normal committed config. SDK/library defaults never override Optic-owned hard ceilings. Operator startup configuration may grant only capabilities/scopes that the application can normalize and enforce deterministically.
