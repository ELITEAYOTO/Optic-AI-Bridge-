# MCP Tool Contracts

Status: initial V1 contract; schemas are generated/validated from Rust types for implemented tools.

## Files

fs_list(path, cursor?, limit?)
fs_read(path, offset?, max_bytes?)
fs_search(query, paths?, cursor?, limit?)
fs_apply_patch(path, patch, expected_hash)
fs_write(path, content, expected_state)
fs_delete(path, expected_hash)

Phase 1 currently implements only `fs_list` and `fs_read` from this section.

`expected_state` is explicit: either target must be absent or it must match a specific content version. There is no blind-overwrite mode.

## Git

git_status()
git_diff(path?, staged?, cursor?, max_bytes?)
git_log(cursor?, limit?)
git_worktree_status()
git_integrate(change_set_id, expected_target_head)

Git tools are not implemented in Phase 1.

`expected_target_head` is a validated Git object id and is checked immediately before integration.

## Processes

Implemented in Phase 1C:

process_start(executable, args[], cwd?, env_allowlist?, network?, timeout_ms?, output_budget?, memory_bytes?, process_count?)
process_read(job_id, stream, cursor?, max_bytes?)
process_stop(job_id)
process_result(job_id)

Reserved for later work:

process_send_input(job_id, data)

### `process_start`

- `executable` must be an absolute path and must canonicalize to a regular file.
- Canonical executable identity must exactly match a task lease created by the bridge operator at startup with `--allow-executable`; the MCP caller cannot mint or select a lease.
- A session receives `ProcessRun` only when at least one executable was operator-authorized for that server lifecycle.
- `args` is a structured array. An opaque shell command string is not accepted as the primitive.
- `cwd`, when supplied, is project-relative and must canonicalize to a directory inside the workspace.
- The child environment is cleared by default. `env_allowlist` may request inheritance only for names that the operator allowed at startup with `--allow-env`.
- `network` defaults to denied. In Phase 1C, `network=true` is rejected because runtime network containment is not implemented yet.
- `timeout_ms`, `output_budget`, `memory_bytes` and `process_count` must be non-zero and fit both the application hard ceilings and the active task lease resource ceiling.
- Phase 1C enforces timeout/output/registry ceilings in runtime. `memory_bytes` and `process_count` are authorized/bounded request values but are not yet claimed as Windows-kernel-enforced limits; Phase 1D owns that gate.
- Success returns an opaque `job_id`; no operating-system PID is exposed as a control capability.

### `process_read`

- `job_id` must belong to the active application session.
- `stream` is `stdout` or `stderr`.
- Results are cursor-based and byte-bounded per call.
- Data is returned as base64 so arbitrary child bytes remain representable without UTF-8 assumptions.
- `truncated=true` indicates the combined stdout/stderr budget was exceeded.

### `process_stop` / `process_result`

- Both accept only an opaque session-owned `job_id`.
- Unknown and cross-session JobIds fail closed as not found to the caller.
- `process_stop` requests termination of the owned process tree; no arbitrary PID kill tool exists.
- `process_result` reports bounded lifecycle metadata: running/exited/stopped/timed_out/output_limit_exceeded/failed, optional exit code and output-truncation state.

## Session/system

Implemented:

session_info()
session_cancel()

Reserved/optional later:

system_info()

`session_cancel` revokes the current application session, revokes its task leases and requests termination of all running process jobs owned by it.

## Contract rules

- Paths are project-relative in the public contract where possible.
- Executable paths are the intentional exception in Phase 1C: they are absolute and canonicalized so operator authorization can bind to an exact executable path.
- No arbitrary PID parameter.
- No opaque shell command as the base process API.
- All potentially large responses are bounded/paginated.
- Mutations carry explicit concurrency preconditions.
- Git integration carries an exact expected target head.
- Tool arguments normalize into one typed core `Effect` before authorization when they represent an executable effect.
- Side-effecting effects must fit the active task lease's explicit scope.
- Errors are typed/stable at the public adapter boundary.
- A tool never grants new capabilities or task leases.
- Transport, session, policy and output ceilings remain Optic-owned even when RMCP also offers protocol helpers.
