# MCP Tool Contracts

Status: initial V1 contract; schemas will be machine-generated/validated in code.

## Files

fs_list(path, cursor?, limit?)
fs_read(path, offset?, max_bytes?)
fs_search(query, paths?, cursor?, limit?)
fs_apply_patch(path, patch, expected_hash)
fs_write(path, content, expected_state)
fs_delete(path, expected_hash)

`expected_state` is explicit: either target must be absent or it must match a specific content version. There is no blind-overwrite mode.

## Git

git_status()
git_diff(path?, staged?, cursor?, max_bytes?)
git_log(cursor?, limit?)
git_worktree_status()
git_integrate(change_set_id, expected_target_head)

`expected_target_head` is a validated Git object id and is checked immediately before integration.

## Processes

process_start(executable, args[], cwd?, env_allowlist?, network?, timeout_ms?, output_budget?)
process_read(job_id, stream, cursor?, max_bytes?)
process_send_input(job_id, data)
process_stop(job_id)
process_result(job_id)

Network is denied by default. Enabling it is a separate authorization concern and cannot be inferred from ProcessRun alone.

## Session/system

session_info()
session_cancel()
system_info()

## Contract rules

- Paths are project-relative in the public contract where possible.
- No arbitrary PID parameter.
- No opaque shell command as the base process API.
- All potentially large responses are bounded/paginated.
- Mutations carry explicit concurrency preconditions.
- Git integration carries an exact expected target head.
- Tool arguments normalize into one typed core `Effect` before authorization.
- Side-effecting effects must fit the active task lease's explicit scope.
- Errors are typed and stable.
- A tool never grants new capabilities.
