# Contributing

Optic AI Bridge is currently in Phase 0. Architecture and security invariants take precedence over feature count.

Before changing code, read:
1. `AI_HANDOFF.md`
2. `docs/security/SECURITY_INVARIANTS.md`
3. the canonical document that owns the subsystem being changed.

## Local checks

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
