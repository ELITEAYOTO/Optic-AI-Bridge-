# ADR-0009 — Typed Effects and Scoped Task Leases

Status: ACCEPTED  
Date: 2026-10-03

## Context

The initial Phase 0 model represented executable intent using independently supplied action kind, target, expected content, network intent and reversibility fields. That shape was flexible but allowed invalid or contradictory combinations to exist in memory and relied on later policy checks to reject them.

The initial TaskLease also carried capabilities, resources, expiry and ownership but did not encode the concrete scope for which the capability was granted. In addition, a process requesting network access could rely on a session-wide NetworkAccess capability while its task lease contained only ProcessRun.

## Decision

1. Replace independently combinable action-kind/target fields with one typed `Effect` enum. Each variant owns the fields that are valid for that operation.
2. Derive required capability and reversibility semantics from the `Effect` definition.
3. Make file write intent explicit with `ExpectedState::Absent` or `ExpectedState::Content(version)`; delete requires an exact current version.
4. Represent Git integration's expected target head as a validated Git object id.
5. Extend TaskLease with explicit scope values for workspace prefixes/all-workspace, repository, process executable and network boundaries.
6. Match workspace scopes structurally by path segment, not naive string prefix.
7. A process requesting network requires NetworkAccess in both SessionGrant and TaskLease and an explicit network scope in the lease.
8. Represent runtime authorization deadlines with a monotonic-time value. The concrete OS/runtime clock adapter is introduced with the session/runtime service rather than into the pure domain crate.

## Consequences

- Invalid target/action combinations become unrepresentable in the core model.
- A capability is no longer sufficient by itself for side effects; the active lease must also cover the target scope.
- Blind file overwrite semantics are removed from the core contract.
- Network access can no longer leak from a broad session grant into an otherwise narrow process lease.
- MCP adapters and future CLI adapters normalize requests into the same effect model before authorization.
- Later filesystem/runtime implementations still need canonical target/handle validation; lexical LeaseScope checks are an authorization contract, not an OS containment mechanism.

## Deferred research

Handle-first Windows filesystem operations, persistent file identity, event-driven invalidation and ActionId-based idempotent replay protection are promising but remain separate ADR candidates until the corresponding service boundaries exist and can be benchmarked/tested.
