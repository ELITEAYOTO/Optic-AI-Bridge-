# Multi-session and Same-Repository Concurrency

Status: DECIDED concept; detailed merge protocol PROPOSED.

Multi-session is a first-class invariant, not a later optimization.

## Session identity

Each connection receives a SessionId and a scoped SessionContext. A session owns:
- project/repository grant;
- worktree or workspace root;
- capability set/lease;
- process Job Object(s);
- output/spool namespace;
- journal namespace;
- quotas and cancellation tree.

Session A cannot address Session B jobs, spool objects or private worktree by guessing identifiers. IDs must be unguessable and authorization must still validate ownership.

## Different projects

Sessions receive disjoint roots and process ownership. No shared mutable global current-directory/environment state is permitted.

## Same repository

Two sessions MUST NOT edit the same physical checkout concurrently.

Preferred flow:
1. coordinator snapshots base HEAD;
2. create one Git worktree + branch per session/task;
3. each session writes only inside its worktree;
4. changes carry expected base/blob hashes;
5. integration checks current target HEAD, overlapping paths and semantic/test gates;
6. clean changes are integrated through an explicit merge/apply gate;
7. conflicts become a structured result, never last-writer-wins.

## Optimistic concurrency

Writes/patches should support expected content hash/version. If the file changed since it was read, return Conflict/StaleBase rather than overwriting.

## Coordinator

The coordinator is deterministic infrastructure, not an AI agent. It manages leases, worktrees, locks only where unavoidable, conflict metadata and integration state.

## Cross-session visibility

Default: private. A session may receive minimal repository events such as TargetHeadChanged or IntegrationConflict when relevant. It must not receive another session's prompts, arbitrary logs or unrelated file contents.

## Failure

Crash/reconnect must not transfer ownership implicitly. Expired sessions are cancelled, process trees terminated, and recoverable worktrees preserved/quarantined according to journal state until safe cleanup.
