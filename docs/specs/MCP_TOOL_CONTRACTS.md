# MCP Tool Contracts

Status: initial V1 contract; schemas will be machine-generated/validated in code.

## Files

fs_list(path, cursor?, limit?)
fs_read(path, offset?, max_bytes?)
fs_search(query, paths?, cursor?, limit?)
fs_apply_patch(path, patch, expected_hash)
fs_write(path, content, expected_hash?)
fs_delete(path, expected_hash)

## Git

git_status()
git_diff(path?, staged?, cursor?, max_bytes?)
git_log(cursor?, limit?)
git_worktree_status()
git_integrate(change_set_id, expected_target_head)

## Processes

process_start(executable, args[], cwd?, env_allowlist?, timeout_ms?, output_budget?)
process_read(job_id, stream, cursor?, max_bytes?)
process_send_input(job_id, data)
process_stop(job_id)
process_result(job_id)

## Session/system

session_info()
session_cancel()
system_info()

## Contract rules

- Paths are project-relative in the public contract where possible.
- No arbitrary PID parameter.
- No opaque shell command as the base process API.
- All potentially large responses are bounded/paginated.
- Mutations accept concurrency preconditions.
- Errors are typed and stable.
- A tool never grants new capabilities.
