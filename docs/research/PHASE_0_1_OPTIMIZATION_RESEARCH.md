# Phase 0.1 Optimization & Robustness Research — 2026-10-03

Status: RESEARCH. This document records evidence and recommendations; it is not itself an implementation decision.

## Executive result

The current architecture should not be replaced. The strongest opportunities improve freshness, OS containment and retry safety while preserving the existing deterministic policy boundary.

Recommended disposition:

- **ADOPT at the relevant service boundary:** handle-first Windows filesystem validation; real monotonic clock adapter; strict RMCP cache/freshness policy; locked/parseable Git worktree lifecycle; Job Object enforcement.
- **PROTOTYPE / MEASURE:** FILE_ID_INFO-backed file identity; USN-assisted invalidation; ActionId idempotency ledger; AppContainer/LPAC compatibility profile.
- **DEFER:** broad content-addressed cache/dedup, generic WASM extension system, worker-process-per-session by default.
- **DO NOT USE AS AUTHORITY:** filesystem watcher events, USN records, MCP cache TTL, model-generated freshness/risk claims.

## 1. Windows handle-first filesystem operations — high priority

### Evidence

Windows `CreateFile` supports `FILE_FLAG_OPEN_REPARSE_POINT`, which opens a reparse point itself instead of following normal reparse processing. `GetFileInformationByHandleEx` retrieves metadata for an already-open handle.

Sources:
- https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-createfilea
- https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex

### Recommendation

When the filesystem service is introduced, authorization/execution should converge on a validated handle rather than repeatedly resolving a path string:

1. parse project-relative WorkspacePath;
2. open using a policy-appropriate handle strategy;
3. inspect reparse/final identity and root containment;
4. authorize the resolved object;
5. perform the operation against the validated handle where the Windows API permits it;
6. revalidate mutation preconditions immediately before commit/replace.

This reduces TOCTOU exposure compared with `validate path string -> later reopen path string`.

### Important boundary

LeaseScope path matching remains only a domain authorization scope. It is not a substitute for Windows reparse/final-target validation.

## 2. Durable Windows file identity — prototype in Phase 2

### Evidence

`FILE_ID_INFO` contains the volume serial number plus a 128-bit file identifier. Microsoft states that together they uniquely identify a file on one computer and can be used to determine whether two handles represent the same file.

Source:
- https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info

### Candidate

Introduce an internal `FileIdentity` only inside the Windows/filesystem service, not the public MCP contract:

`FileIdentity { volume_serial, file_id_128 }`

A later ObservationStamp may combine:

`project_generation + file_identity + content_version + policy_epoch`

For Git-backed targets it may additionally carry the observed repository/head generation.

### Why prototype instead of adopt now

ContentVersion already gives correct optimistic concurrency for content. File identity adds value for delete/recreate/same-path races, but its practical benefit should be demonstrated with adversarial tests before becoming a permanent cross-platform core abstraction.

## 3. USN Change Journal assisted invalidation — useful accelerator, never authority

### Evidence

NTFS maintains a persistent USN change journal containing records when files/directories are created, deleted or modified. Microsoft also documents that old records may be deleted and clients must recover by re-enumerating/re-indexing. The journal records that a change occurred and why; it is not a reversible transaction log.

Sources:
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/fsutil-usn
- https://learn.microsoft.com/en-us/windows/win32/fileio/change-journal-records

### Recommendation

Prototype USN in Phase 4 as a stale-context accelerator:

`USN/file notification -> mark observed state stale -> mandatory real hash/head/identity revalidation at commit`

Never allow `no observed USN event` to prove that a file is unchanged.

Fallback rules are mandatory when:
- journal records were truncated;
- volume/filesystem does not support the required journal behavior;
- journal id changes;
- watcher reports overflow.

Fallback = bounded targeted re-enumeration/revalidation, not silent trust.

## 4. ReadDirectoryChangesW — low-latency hint, not consistency mechanism

### Evidence

Microsoft documents that `ReadDirectoryChangesW` can lose changes and return `ERROR_NOTIFY_ENUM_DIR`, in which case the caller must recompute changes by enumerating the directory/subtree. Network monitoring also has a 64 KiB buffer limitation.

Source:
- https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-readdirectorychangesw

### Recommendation

If used, treat notifications as a latency optimization feeding the stale-context system. It must never replace expected-state/hash revalidation.

## 5. Monotonic authorization time — adopt

### Evidence

Rust documents `SystemTime` as non-monotonic. `Instant` uses a monotonic OS clock where available; on Windows its current implementation uses `QueryPerformanceCounter`.

Sources:
- https://doc.rust-lang.org/std/time/struct.SystemTime.html
- https://doc.rust-lang.org/beta/std/time/struct.Instant.html

### Recommendation

The Phase 0.1 domain now carries opaque monotonic deadline values. Phase 1 should add a runtime Clock adapter backed by `Instant` for active authorization/session/task TTLs.

Wall-clock timestamps remain appropriate for persisted audit/event timestamps, never as the sole active authorization deadline.

## 6. Windows Job Objects — adopt for process resource enforcement

### Evidence

Windows Job Objects group process trees, propagate membership to children by default, support nested jobs on modern Windows, can apply process/memory limits and can terminate associated processes on last handle close with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.

Sources:
- https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs
- https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information

### Recommendation

Phase 1 process runtime should translate authorized ResourceBudget values into enforceable Job Object limits where Windows provides an equivalent.

Candidate hierarchy:

`bridge/session containment -> per-job/process execution job`

Do not enable breakaway flags for ordinary execution. Test nested-job interaction against developer tools before relying on hierarchy in production.

Job Objects are lifecycle/resource containment, not a complete security sandbox.

## 7. Restricted token vs AppContainer/LPAC — AppContainer deserves a PoC

### Evidence

`CreateRestrictedToken` can remove privileges, disable SIDs and add restricting SIDs. Microsoft also warns that restricted applications sharing the default desktop can attack unrestricted applications through window messages unless desktop isolation is handled.

AppContainer explicitly isolates credentials, files, registry, network, processes and windows; network/resource access is capability-based. LPAC is more restrictive still.

Sources:
- https://learn.microsoft.com/windows/win32/api/securitybaseapi/nf-securitybaseapi-createrestrictedtoken
- https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation
- https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer

### Recommendation

Keep restricted-token execution as a compatibility experiment, but add an AppContainer/LPAC PoC to the hardening research matrix. Compare real compatibility with:

- cargo/rustc;
- npm/node toolchains;
- Git;
- Java/Maven/Gradle;
- native compilers/build scripts;
- access to granted workspace paths;
- opt-in networking.

Do not make AppContainer a V1 dependency until compatibility and setup cost are measured.

## 8. MCP/RMCP freshness and response caching — correction required before adapter implementation

### Evidence

The official Rust MCP SDK implements stable MCP 2026-07-28, including stateless operation, long-running tasks, subscriptions, MRTR and response caching. The protocol explicitly treats requests as stateless and persistent application state must use explicit identifiers.

RMCP client caching stores responses with positive TTL hints and invalidates matching entries via notifications. The SDK documentation notes that its default `serve_stale_on_error` behavior can return an expired cached response as successful when a refresh fails.

Sources:
- https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/index.mdx
- https://github.com/modelcontextprotocol/rust-sdk
- https://github.com/modelcontextprotocol/rust-sdk/discussions/969

### Recommendation

Optic policy:

- do not expose positive cache TTLs for security-sensitive/freshness-sensitive resource state unless a concrete safe contract is defined;
- never let cached MCP state satisfy file/Git mutation preconditions;
- configure any Optic-owned RMCP client path with `serve_stale_on_error(false)` when a failed refresh must be observable;
- partition all private caches by explicit principal/session authorization context;
- notifications may invalidate caches early but are not proof of freshness;
- application SessionHandle remains explicit and independent of transport connection/session semantics.

## 9. ActionId idempotency ledger — strong candidate for Phase 3

### Evidence

MCP 2026-07-28 is stateless and supports multi-round-trip retries carrying opaque requestState. Long-running tasks and reconnect/retry behavior make duplicate delivery/execution an engineering concern even when protocol semantics are correct.

Sources:
- https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/index.mdx
- https://github.com/modelcontextprotocol/rust-sdk

### Candidate model

Use the already server-generated ActionId as an execution idempotency key for side-effecting operations:

`Prepared -> Executing -> Committed(result/reference) | Failed`

Rules:
- same ActionId + different normalized effect/preconditions = deny as protocol/state corruption;
- committed action replay returns its recorded result/reference rather than re-executing;
- executing action replay returns status/reference;
- retention is bounded by TTL/quota and integrated with the recovery journal;
- the ledger is not a global forever-cache.

### Why this may be a differentiator

It can provide effectively-once behavior for Optic-owned side effects across retries without pretending the underlying transport is stateful.

## 10. Git worktrees — adopt stricter lifecycle details

### Evidence

Git supports `worktree add --lock`, machine-readable `worktree list --porcelain -z`, explicit lock/unlock, prune and repair operations.

Source:
- https://git-scm.com/docs/git-worktree

### Recommendation

Represent a worktree as an Optic-owned resource, not merely a filesystem path:

`WorktreeId + owning SessionHandle + repository identity + base head + path + lifecycle state`

Operational rules:
- create and lock atomically where supported (`worktree add --lock`);
- parse `--porcelain -z`, never human output;
- journal creation/removal;
- run repair/prune only through bounded maintenance operations;
- integration always checks expected target head immediately before applying changes.

## Proposed priority

### P0 — before/inside Phase 1
1. real monotonic Clock adapter;
2. RMCP stale-cache policy and TransportGuard integration;
3. Job Object resource/lifecycle enforcement;
4. strict application-session binding independent of connection.

### P1 — Phase 2
1. handle-first filesystem service;
2. FILE_ID_INFO identity PoC;
3. transactional write/replace + expected-state revalidation.

### P2 — Phase 3/4
1. bounded ActionId idempotency ledger;
2. worktree resource lifecycle;
3. USN/notification-assisted invalidation with mandatory revalidation.

### Research / hardening
1. AppContainer/LPAC compatibility matrix;
2. restricted-token comparison;
3. signed release/update provenance.

## Explicit non-goals from this research

- no USN watcher as authorization authority;
- no probabilistic freshness score;
- no automatic stale-cache fallback for mutation-relevant state;
- no claim that Job Objects alone sandbox hostile code;
- no global unbounded content-addressed cache;
- no new generic plugin system before a concrete extension need exists.
