# Security Invariants

Status: DECIDED baseline. Every invariant must eventually have an executable test.

## Identity and authorization

**INV-001 — No AI self-authorization.** Model output can request an action but cannot grant a capability, approve itself, change policy or elevate privilege.

**INV-002 — Explicit application identity.** Stateful resources are addressed by server-minted handles and validated against the authenticated principal/project grant on every call. Transport session state is never the authority.

**INV-003 — Cross-session denial.** A session cannot read, mutate, cancel or enumerate another session's jobs, worktree, journal, spool or capability leases.

**INV-004 — Capability expiry/revocation.** Capability leases have scope, ceiling and expiry; revocation invalidates future use without relying on model cooperation.

**INV-005 — Scope is authoritative.** A capability name alone never authorizes a side effect outside the task lease's explicit workspace/repository/executable/network scope. Scope comparison is structural rather than naive string-prefix matching.

**INV-006 — Runtime expiry is monotonic.** Session/task lease runtime deadlines are evaluated against a monotonic time source so wall-clock correction cannot extend an active authorization window.

## Files and Git

**INV-010 — Canonical target first.** Authorization happens on a canonical target after root/reparse/symlink checks, never on raw user/model path text.

**INV-011 — No stale overwrite.** Mutations require expected version/hash or an equivalent transaction precondition. Mismatch returns a structured conflict.

**INV-012 — Same repo, separate checkout.** Concurrent write sessions on one repository never share a physical working tree.

**INV-013 — Reversible mutation.** Multi-file mutation is journaled/transactional or explicitly classified as irreversible before execution.

**INV-014 — No blind create/update ambiguity.** File write intent states either `ExpectedState::Absent` or an exact expected content version; absence of a precondition is not interpreted as permission to overwrite whatever currently exists.

**INV-015 — Git integration has an exact base.** Integration carries a validated expected target object id and fails on target-head drift.

## Processes and resources

**INV-020 — Structured process launch.** Fundamental process API is executable + args + cwd + explicit environment/network/resource request, not an opaque shell string.

**INV-021 — Owned process trees only.** Stop/cancel operates on bridge-owned JobId/session resources, never arbitrary machine PIDs.

**INV-022 — Bounded everything.** Request bodies, protocol frames, queues, stdout/stderr, MCP responses, caches, completed-job retention, audit logs and spool disk have hard ceilings/TTL/quota.

**INV-023 — Cleanup after cancellation/crash.** Owned process trees terminate or are recoverably quarantined; cleanup is idempotent. A process tree whose termination cannot be proven remains explicitly `TerminationUncertain`, retains ownership/resource capacity and cannot be evicted or session-reaped as if it were terminal.

**INV-024 — Process network is dual-authorized.** A leased process receives network only when both its session and its task lease carry `NetworkAccess` and the task lease also contains an explicit network scope.

**INV-025 — Optic-owned process CPU is bounded before execution.** Active/termination-uncertain process jobs consume application-owned CPU reservation under global and per-session ceilings. On native Windows each owned process tree is hard-capped by its Job Object before resume; failure to establish that cap fails closed rather than silently running uncapped. Proven-terminal retained history releases CPU reservation.

## Data-flow containment

**INV-030 — Untrusted source is not authority.** Repository/process/network text can influence model reasoning but cannot weaken local policy.

**INV-031 — Sensitive sinks are deterministic.** Network egress, credential access, writes outside granted roots, policy changes and privilege changes are denied or require a non-model approval path regardless of textual instructions encountered by the AI.

**INV-032 — No ambient secrets.** Child processes receive only explicitly allowlisted environment variables/credentials required by the action.

## Verification

Each invariant gets: unit/property tests where possible, Windows-native integration tests for OS behavior, adversarial negative tests, and a regression test for every discovered bypass.
