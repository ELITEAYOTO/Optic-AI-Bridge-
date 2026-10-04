# MCP Tool Contracts

Status: current implemented pre-alpha surface through Phase 2D2. Implemented schemas are generated/validated from Rust types; this document summarizes the public contract and registration/authorization rules.

## Registration is not authorization

Tool visibility and execution authority are intentionally distinct:

- `fs_list`, `fs_read`, `session_info`, `session_cancel` and the process tool router are part of the base MCP surface.
- Process tools can therefore be visible even when no executable was authorized. In that state `process_start` fails closed because no exact operator-owned executable task lease exists; `process_read`, `process_stop` and `process_result` also have no valid owned JobId to act on.
- `fs_write` and `fs_apply_patch` are registered only when application-owned `FileWrite` authority was provisioned.
- `fs_delete` is registered only when distinct application-owned `FileDelete` authority was provisioned.
- `git_status`, `git_diff` and `git_log` are registered only when the operator supplied one valid absolute `--git-executable`, allowing the application to own a `GitReadService`.

A visible MCP tool never grants a capability, task lease or authority by itself.

## Filesystem reads

Implemented on the base surface:

```text
fs_list(path, cursor?, limit?)
fs_read(path, offset?, max_bytes?)
```

- Paths are project-relative and must remain inside the canonical workspace.
- Listing is deterministic, paginated and scan-bounded.
- Reads are byte-bounded and binary bytes are returned as base64.

Reserved for later work:

```text
fs_search(...)
```

## Durable file mutation

Implemented on Windows and exposed conditionally from operator-owned authority:

```text
fs_write(path, content_base64, expected)
fs_apply_patch(path, expected_version, offset, remove_bytes, insert_base64)
fs_delete(path, expected_version)
```

`fs_write.expected` is explicit and tagged:

```text
{ kind: "absent" }
```

or:

```text
{ kind: "content", version_hex: "<exact BLAKE3 content version>" }
```

Rules:

- there is no blind-overwrite mode;
- `fs_apply_patch` is one deterministic byte-range transform and requires the exact base content version;
- `fs_delete` requires the exact current content version; there is no blind delete;
- mutation payload bytes are bounded by `HardLimits::max_fs_mutation_bytes`;
- the server generates the `ActionId`; callers cannot supply one;
- callers cannot supply or choose a `TaskLeaseId`;
- write/patch resolve the application-owned `FileWrite` lease; delete resolves the separate `FileDelete` lease;
- normalized effects flow through `AuthorizedFileMutationService` and the durable transactional/recovery boundary;
- recovery-required/ambiguous outcomes are not reported as success or as a safe blind-retry signal;
- non-Windows durable mutation currently fails closed as unsupported.

## Git reads

Implemented conditionally through Phase 2D2:

```text
git_status()
git_diff(path?, staged?, max_bytes?)
git_log(cursor?, limit?)
```

Rules:

- Git read exists only when the operator provides one absolute `--git-executable` and the configured workspace validates as the exact canonical Git worktree top-level;
- MCP callers cannot supply a repository path, Git executable, raw Git argv, capability or authority identifier;
- `git_status` returns bounded porcelain-v2 bytes as base64 plus the observed HEAD when present;
- `git_diff` supports an optional project-relative literal path and optional staged diff; external diff/textconv, rename detection and submodule traversal are constrained/disabled by the runtime;
- `git_log` is entry-bounded and cursor-based; a cursor contains the snapshot `head` plus an `offset` and must remain the current HEAD or a reachable ancestor;
- Git subprocess output and runtime duration are hard-bounded.

Not exposed through MCP yet:

```text
git_worktree_status()
git_integrate(source_head, expected_target_head)
```

Phase 2D3A now owns a runtime-only fast-forward integration foundation. The normalized `GitIntegrate` effect binds both exact `source_head` and `expected_target_head`; the runtime is restricted to operator-owned direct refs under `refs/optic/integration/`, an isolated locked `--no-checkout` worktree and atomic expected-old-value ref update. It does **not** register a public MCP mutation tool. Application-owned `GitIntegrate` authority, repository-scoped lease resolution, interruption cleanup/recovery and the thin conditional MCP adapter must pass their own gates first.

## Processes

Implemented process surface:

```text
process_start(executable, args[], cwd?, env_allowlist?, network?, timeout_ms?, output_budget?, memory_bytes?, process_count?)
process_read(job_id, stream, cursor?, max_bytes?)
process_stop(job_id)
process_result(job_id)
session_cancel()
```

Reserved for later work:

```text
process_send_input(job_id, data)
```

### `process_start`

- `executable` must be an absolute path and canonicalize to a regular file.
- Canonical executable identity must exactly match a task lease created by the bridge operator at startup with `--allow-executable`; the MCP caller cannot mint or select a lease.
- A session receives `ProcessRun` only when at least one executable was operator-authorized for that server lifecycle.
- `args` is a structured array. An opaque shell command string is not accepted as the primitive.
- `cwd`, when supplied, is project-relative and must canonicalize to a directory inside the workspace.
- The child environment is cleared by default. `env_allowlist` may request inheritance only for names that the operator allowed at startup with `--allow-env`.
- `network` defaults to denied and `network=true` currently fails closed because process network containment is not implemented.
- `timeout_ms`, `output_budget`, `memory_bytes` and `process_count` must be non-zero and fit both application hard ceilings and the active exact-executable task lease resource ceiling.
- On Windows, timeout/output remain Optic-owned runtime limits while `memory_bytes` and `process_count` are additionally enforced by the per-job Windows Job Object.
- Success returns an opaque `job_id`; no operating-system PID is exposed as a control capability.

### `process_read`

- `job_id` must belong to the active application session.
- `stream` is `stdout` or `stderr`.
- Results are cursor-based and byte-bounded per call.
- Data is returned as base64 so arbitrary child bytes remain representable without UTF-8 assumptions.
- Output overflow terminates the owned process tree and is reported distinctly from a normal exit.

### `process_stop` / `process_result`

- Both accept only an opaque session-owned `job_id`.
- Unknown and cross-session JobIds fail closed as not found to the caller.
- `process_stop` requests termination of the owned process tree; no arbitrary PID kill tool exists.
- `process_result` returns bounded lifecycle metadata.

## Session/system

Implemented:

```text
session_info()
session_cancel()
```

Reserved/optional later:

```text
system_info()
```

`session_cancel` revokes the current application session, revokes its task leases and requests termination of all running process jobs owned by it.

## Contract rules

- Paths are project-relative in the public contract where possible.
- Executable paths are an intentional absolute-path exception so operator authorization can bind to an exact canonical executable.
- No arbitrary PID parameter.
- No opaque shell command as the base process API.
- All potentially large responses are bounded/paginated.
- Mutations carry explicit concurrency preconditions.
- Git integration will carry an exact expected target head before any integration effect.
- Tool arguments normalize into one typed core `Effect` before authorization when they represent an executable effect.
- Side-effecting effects must fit the active task lease's explicit scope.
- Errors are typed/stable at the public adapter boundary.
- A tool never grants new capabilities or task leases.
- Transport, session, policy and output ceilings remain Optic-owned even when RMCP also offers protocol helpers.
