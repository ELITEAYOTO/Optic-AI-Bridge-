# Contributing

Optic AI Bridge is an executable Rust pre-alpha. Architecture and security invariants take precedence over feature count. Read [current status](STATUS.md) and the [branch/PR workflow](docs/engineering/REPOSITORY_WORKFLOW.md) before choosing a target branch.

Before changing code, read:
1. `AI_HANDOFF.md`
2. `docs/security/SECURITY_INVARIANTS.md`
3. the canonical document that owns the subsystem being changed.

## Current continuation: GitHub only

Read [the agent handoff](docs/engineering/GITHUB_HANDOFF.md). Do not run tests,
builds, installers, servers or permission changes on the operator's PC. Use an
agent-owned isolated environment or GitHub-hosted CI; do not dispatch self-hosted
jobs. Inspect applicable workflows and exact-head results before merging.

## Contributor checks in your own development environment

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

When `cargo-deny` is installed:

```text
cargo deny check
```

A change that modifies an architectural/security invariant requires documentation and normally an ADR. A discovered security bypass requires a regression test.
