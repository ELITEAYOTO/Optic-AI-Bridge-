# Maintenance Policy

Optic AI Bridge is documentation-first but documentation must never drift from code.

## Source of truth

One canonical owner per subject. Do not duplicate normative limits or security rules across multiple documents. Other files link to the canonical owner.

## Required update matrix

- behavior/tool contract change -> specs + tests + CHANGELOG;
- security boundary/invariant change -> security docs + tests + ADR + CHANGELOG;
- architecture/dependency direction change -> owning architecture doc + ADR;
- milestone/gate change -> canonical roadmap + STATUS;
- measured performance change -> benchmarks, never rewrite TARGET as fact without evidence;
- dependency/protocol update -> dependencies/research ledger + compatibility tests.

## Review cadence

On every meaningful PR: status/changelog/docs-impact check.  
Weekly while actively developing: dependency/advisory review and open security issues.  
Before each release: threat-model delta, dependency lock review, Windows-native security suite, soak tests, artifact provenance/SBOM, docs/status/changelog consistency.  
After an incident: add regression test first, then update threat model/invariant/ADR as applicable.

## Dependency policy

Minimize direct dependencies. Commit `Cargo.lock` for the application. New dependencies require a reason, maintenance/security check and feature-minimization review. Security updates may bypass normal feature batching but never bypass tests.

## Protocol policy

MCP is an adapter, not the domain model. Support is versioned explicitly. New MCP features do not automatically become Optic capabilities.

## Definition of maintained

A project state is not “up to date” merely because dependencies are newest. It is maintained when code, tests, threat model, contracts, roadmap, changelog and supported protocol versions agree.
