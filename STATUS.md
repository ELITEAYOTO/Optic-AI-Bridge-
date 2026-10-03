# Project Status

**Last updated:** 2026-10-03  
**Lifecycle:** pre-alpha / Phase 2B mutation-time containment next  
**Release:** none  
**Security support:** no production-supported release yet

## Current focus

Phase 2A is merged on `main` via PR #9 (`71bdf082`). The next implementation tranche is Phase 2B: mutation-time OS containment/revalidation and a narrow atomic commit primitive. Durable filesystem mutation remains intentionally unavailable to MCP until the Windows handle/reparse boundary and later recovery journal gates are proven.

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

### Phase 1D — merged
- PR #7 merged to `main` as `69af07a`.
- `optic-bridge-windows` isolates the narrow Win32/unsafe boundary; core, policy, MCP and cross-platform runtime remain unsafe-free.
- Windows `ProcessManager` jobs use a custom `LimitedJobObject` configured from the already-authorized `ResourceBudget`.
- Child creation is forced suspended; the Job Object is created/configured, the child is assigned, and only then are child threads resumed.
- Kernel Job Object flags enforce kill-on-close, active-process count and total job-memory ceilings.
- Each process job owns its own Job Object; the job itself remains owned by exactly one application session.
- `try_wait`/`wait` do not treat the job as terminal while descendants remain active.
- Removing/unwrapping containment is fail-closed: terminate the whole job, fallback-kill the root child and close the Job Object handle rather than leak/detach it.
- Native Windows tests prove:
  - `process_count = 1` blocks descendant creation;
  - a 128 MiB job-memory ceiling prevents a fixture from reaching a 384 MiB allocation target;
  - timeout terminates the descendant tree before a delayed survival marker can be written;
  - dropping the live Job Object wrapper triggers kill-on-close cleanup;
  - explicitly unwrapping the containment wrapper does not allow the child to survive.
- Final PR #7 head passed Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny` before merge.

### Phase 2A — merged
- PR #9 merged to `main` as `71bdf082`.
- Adds bounded streaming BLAKE3 content observation without loading whole files into RAM.
- Adds `HardLimits::max_fs_mutation_bytes` (default 8 MiB) so precondition hashing is bounded in bytes as well as memory.
- Adds `MutationObservation` and isolated `MutationError` runtime types without changing the existing read-only MCP error surface.
- Existing mutation targets must be regular files; leaf symlinks are rejected.
- Absent mutation targets canonicalize the parent first, preventing a symlinked parent from hiding the actual authorization path.
- Canonical targets must remain inside the canonical workspace root.
- `ExpectedState::Absent` and exact `ExpectedState::Content(version)` are checked explicitly; stale/blind-overwrite attempts fail closed.
- Unix regression tests cover leaf symlink denial, canonicalized in-workspace parent symlinks and parent escape denial.
- Oversized mutation targets fail closed at the hard observation ceiling.
- This tranche performs no durable mutation and exposes no MCP write/delete/patch tools.
- It does not claim final race-free Windows mutation containment: handle-first reparse/final-path/file-identity revalidation remains required at commit time.
- The exact final tree was validated green on Ubuntu format/Clippy/tests, Windows Clippy/tests and `cargo-deny`; a temporary CI-only PR #10 was used because PR #9's Actions concurrency group had a cancelled intermediate Ubuntu job stuck without steps.

### Current / next implementation
- Phase 2B should introduce mutation-time OS containment/revalidation, including Windows handle-first reparse/final-target checks and a `FILE_ID_INFO` identity PoC where it materially closes delete/recreate races.
- Revalidate the exact expected state immediately before the durable commit point.
- Add a bounded same-directory temporary write plus atomic create/replace primitive only after the target handle boundary is proven.
- Keep MCP `fs_write` / patch / delete surfaces deferred.
- Phase 2C then adds the durable recovery journal and forced-crash gates before public mutation services.

### Later validated research candidates
- Phase 2: handle-first filesystem service and FILE_ID_INFO identity PoC.
- Phase 3: bounded ActionId idempotency ledger integrated with recovery state.
- Phase 4: worktree resource lifecycle and USN/notification-assisted invalidation with mandatory commit-time revalidation.
- Hardening research: restricted-token vs AppContainer/LPAC compatibility matrix.

### Not implemented yet
- Durable filesystem mutation services and Git execution services.
- Transaction journal / crash-recovery implementation for mutations.
- Windows handle-first final mutation commit/revalidation boundary.
- Multi-session public runtime orchestration and same-repository worktree execution.
- Installer/tunnel integration.
- Restricted-token/AppContainer hardening profile.
- Public release.

## Main baseline

`main` includes Phase 1A (`d33a1e5`), Phase 1B (`681f939`), Phase 1C (`adf2e772`), Phase 1D (`69af07a`) and Phase 2A (`71bdf082`). Phase 2B is the next implementation tranche.

## Health rule

This file describes reality. A design document alone never makes a feature “implemented”.
