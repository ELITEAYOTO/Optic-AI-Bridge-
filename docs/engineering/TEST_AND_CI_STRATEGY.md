# Test and CI Strategy

Every security invariant needs an executable test.

## Fast PR gates

cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
dependency/advisory/license policy checks
schema/contract compatibility tests

## Security gates

- path traversal and Windows path edge cases;
- reparse/symlink escape tests;
- stale hash/TOCTOU simulations;
- cross-session JobId/spool/worktree access denial;
- transaction rollback/recovery;
- process child/grandchild orphan tests;
- timeout/cancellation races;
- output flood/backpressure;
- malformed MCP/property/fuzz corpus.

Evaluate cargo-deny, cargo-audit and cargo-fuzz during bootstrap; pin their role in CI after validating maintenance/redundancy.

## Windows-native CI

Security/runtime tests that depend on Job Objects/tokens must run on native Windows. Wine is not the security oracle.

## Soak

1h and 4h mixed workloads; repeated session create/destroy; high-output jobs; crash/restart. Track bridge private/RSS separately from child processes and Windows standby cache.
