# Changelog

All notable project changes are recorded here. Until executable code exists, entries describe architecture/documentation changes only.

## [Unreleased]

### Added
- Canonical product, architecture, security, specification, engineering, operations and research documentation.
- Multi-session and same-repository worktree isolation design.
- Deterministic policy/capability model.
- Bounded output and crash-recovery design.
- Living roadmap, project status, security policy and maintenance policy.
- Security invariants and AI-native safety architecture.
- ADRs for application-owned session handles, action envelopes and source/sink containment.

### Changed
- Architecture updated for MCP 2026-07-28 stateless protocol semantics.
- Security goal changed from impossible “100% secure” wording to testable invariants plus defense in depth.

### Security
- Model/repository/process output explicitly treated as untrusted for authorization.
- Transport limits must be enforced by Optic AI Bridge even when an upstream SDK also has limits.
