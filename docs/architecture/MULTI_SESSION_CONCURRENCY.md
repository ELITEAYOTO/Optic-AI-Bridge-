# Multi-session and Same-Repository Concurrency

Status: Phase 3 current; Phase 3A resource isolation and Phase 3B1 application-owned lifecycle are merged. Phase 3B2 admission-vs-revoke serialization plus quiescent owner-scoped reap is current; automatic expiry supervision and client-visible multi-session orchestration remain follow-up gates, while detailed same-repository merge coordination remains PROPOSED.

Multi-session is a first-class invariant, not a later optimization.

## Session identity

Optic owns an opaque application `SessionHandle`; transport connections are not session authority and must not become the ownership key. A session owns or binds:
- project/repository grant;
- workspace/worktree root;
- capability set and task leases;
- process jobs / Windows Job Objects;
- output/spool namespace;
- journal/recovery authority where applicable;
- quotas and cancellation tree.

Session A cannot address Session B jobs, spool objects or private worktree by guessing identifiers. IDs remain unguessable and authorization must still validate ownership. Phase 3A enforces owner-bound `JobId` access plus global-and-per-session process quotas and owner-only terminal-record eviction. Phase 3B1 adds application-owned provisioning and coordinated revoke. Phase 3B2 adds the admission barrier for sensitive sinks: new admissions fail once revoke begins, already-admitted effects drain before lease/job cleanup, and physical reap is allowed only after quiescence. No public MCP session-creation tool exists yet.

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

Crash/reconnect must not transfer ownership implicitly. Expired or explicitly revoked sessions first become unauthorized. Phase 3B2 then blocks new sensitive admissions, waits for already-admitted effects, revokes owned leases, requests owned-job termination, and permits owner-scoped reap only when no admission/job remains active. Natural expiry can use the same lifecycle path once invoked; an automatic expiry supervisor is still required before client-visible multi-session orchestration. Recoverable worktrees remain preserved/quarantined according to journal state until safe cleanup.
