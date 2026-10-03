# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / Phase 1 vertical slice gate  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Finish Phase 1D documentation and merge only while the native Windows resource-containment gate remains green. Phase 2 mutation/Git work starts only from that clean baseline.

### Completed
- Documentation ownership and living governance.
- Windows-first Rust direction.
- MCP isolated as an adapter.
- Deterministic deny-by-default policy boundary.
- Bounded-output requirement.
- Multi-session/worktree isolation architecture.
- Phase 0 Cargo workspace merged to `main` via PR #1.
- Core opaque SessionHandle/TaskLeaseId/ActionId primitives.
- WorkspacePath and BLAKE3 ContentVersion primitives.
- ResourceBudget/HardLimits baseline.
- Typed ActionEnvelope, SessionGrant and TaskLease domain model.
- Initial deterministic PolicyEngine and negative security tests.
- Windows/Linux CI plus dependency-policy workflow.
- Phase 0.1 contract hardening merged to `main` via PR #2.
- Typed `Effect` variants replacing independently combinable action/target fields.
- Explicit file `ExpectedState` and Git target-head preconditions.
- Scope-bearing task leases for workspace, repository, executable and network boundaries.
- Segment-aware workspace scope checks.
- Dual session + task-lease authorization contract for process network access.
- Monotonic runtime authorization deadline values.
- Regression tests for scope escape and missing network authorization.
- ADR-0009 for typed effects/scoped leases.
- Research pass on handle-first filesystem I/O, Windows file identity, USN-assisted invalidation, ActionId idempotency, RMCP cache freshness, Job Objects and AppContainer/LPAC.

### Phase 1A — merged
- PR #4 merged to `main` as `d33a1e5`.
- `optic-bridge-runtime` keeps application runtime ownership outside the MCP adapter.
- Real monotonic `StdClock`.
- Revocable/expiring `SessionRegistry`.
- Optic-owned `TransportGuard` with request/response byte ceilings, request deadlines and bounded concurrency.
- Bounded `fs_read` and deterministic paginated `fs_list` with canonical workspace containment and hard directory-scan ceilings.

### Phase 1B — merged
- PR #5 merged to `main` as `681f939`.
- Maintained RMCP 3.4.0 used only as the MCP adapter.
- Optic-owned bounded JSON-line stdio transport enforces an input frame ceiling before JSON deserialization and a final serialized response ceiling before stdout write.
- Application-owned session lifecycle is independent of MCP transport state.
- MCP surface includes bounded `fs_read`, `fs_list` and `session_info` routed through normalization, policy and runtime services.

### Phase 1C — merged
- PR #6 merged to `main` as `adf2e772`.
- Opaque session-owned `JobId`; no arbitrary PID operation.
- Revocable application-owned `TaskLeaseRegistry`.
- Structured process API: absolute/canonical executable + `args[]` + workspace-contained cwd + controlled inherited environment.
- Operator-only startup allowlists (`--allow-executable`, `--allow-env`); MCP tools cannot create capabilities or task leases.
- Session receives `ProcessRun` only when at least one executable is explicitly operator-authorized.
- MCP tools: `process_start`, `process_read`, `process_stop`, `process_result`, plus `session_cancel`.
- `process_start` normalizes to `Effect::ProcessRun` and requires both the active session grant and the exact executable task lease before runtime execution.
- Network remains unavailable in the Phase 1 runtime; `network=true` fails closed.
- Bounded active jobs, retained records, stdout/stderr RAM, per-call output reads and process timeouts.
- Environment is cleared by default; only operator-allowlisted variables may be inherited.
- Native Windows CI caught and closed an output-overflow terminal-status race; timeout, explicit stop, output overflow and cross-session ownership tests pass on Windows and Linux.

### Phase 1D — implemented on PR #7 branch
- New `optic-bridge-windows` crate isolates the narrow Win32/unsafe boundary; core, policy, MCP and cross-platform runtime remain unsafe-free.
- Windows `ProcessManager` jobs use a custom `LimitedJobObject` configured from the already-authorized `ResourceBudget`.
- Child creation is forced suspended; the Job Object is created and configured, the child is assigned, and only then are child threads resumed.
- Kernel Job Object flags enforce kill-on-close, active-process count and total job-memory ceilings.
- Each process job owns its own Job Object; the job itself remains owned by exactly one application session.
- `try_wait`/`wait` do not treat the job as terminal while descendants remain active.
- Removing/unwrapping the containment wrapper is fail-closed: the whole job is terminated, the root child receives a fallback kill request, and the Job Object handle is closed rather than leaked.
- Native Windows tests prove:
  - `process_count = 1` blocks descendant creation;
  - a 128 MiB job-memory ceiling prevents a fixture from reaching a 384 MiB allocation target;
  - timeout terminates the descendant tree before a delayed survival marker can be written;
  - dropping the live Job Object wrapper triggers kill-on-close cleanup;
  - explicitly unwrapping the containment wrapper does not allow the child to survive.
- Ubuntu format/Clippy/tests and Windows Clippy/tests are green on the fail-closed implementation head; the dependency-policy gate is unchanged and remains required on the final documentation head.

### Current / next implementation
- Finalize PR #7 documentation and merge only while the final documentation head remains green.
- After merge, Phase 1 vertical-slice requirements are satisfied at the current pre-alpha scope.
- Phase 2 begins transactional filesystem mutation/Git work with explicit expected-state preconditions and recovery semantics.

### Later validated research candidates
- Phase 2: handle-first filesystem service and FILE_ID_INFO identity PoC.
- Phase 3: bounded ActionId idempotency ledger integrated with recovery state.
- Phase 4: worktree resource lifecycle and USN/notification-assisted invalidation with mandatory commit-time revalidation.
- Hardening research: restricted-token vs AppContainer/LPAC compatibility matrix.

### Not implemented yet
- Filesystem mutation services and Git execution services.
- Transaction journal / crash-recovery implementation for mutations.
- Multi-session public runtime orchestration and same-repository worktree execution.
- Installer/tunnel integration.
- Restricted-token/AppContainer hardening profile.
- Public release.

## Main baseline

`main` includes Phase 1A (`d33a1e5`), Phase 1B (`681f939`) and Phase 1C (`adf2e772`). Phase 1D is under review in PR #7 and has passed its native Windows implementation gates; the final documentation head must remain green before merge.

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
