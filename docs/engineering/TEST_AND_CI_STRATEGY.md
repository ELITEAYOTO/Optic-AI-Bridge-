# Test and CI Strategy

Every security invariant needs an executable test. CI is a security boundary for releases, not proof that software is vulnerability-free.

## Fast PR gates

- cargo fmt --check
- cargo clippy --workspace --all-targets --all-features -- -D warnings
- cargo test --workspace
- dependency/advisory/license/source policy
- schema/contract compatibility
- documentation/status/changelog consistency check

## Security gates

- path traversal and Windows path edge cases;
- reparse/symlink escape and race tests;
- stale hash/TOCTOU simulations;
- cross-session SessionHandle/JobId/spool/worktree denial;
- expired/revoked capability lease denial;
- transaction rollback/recovery;
- child/grandchild orphan tests;
- timeout/cancellation races;
- output flood/backpressure;
- oversized/unterminated MCP transport input;
- oversized HTTP/error response handling;
- malformed MCP/property/fuzz corpus;
- network denied unless explicitly leased;
- environment/secret allowlist tests.

Evaluate cargo-deny, cargo-audit/RustSec and cargo-fuzz during bootstrap and record the chosen roles. Add CodeQL if it provides useful Rust coverage for this codebase rather than treating tool count as security.

## Supply-chain gates

Commit Cargo.lock for the application. Review dependency diffs. Pin third-party GitHub Actions to full commit SHAs for production workflows. Release pipeline should produce checksums plus SBOM/provenance/attestation where supported and sign Windows release artifacts before stable distribution.

## Windows-native CI

Job Object/token/path security behavior must run on native Windows. Wine is not the security oracle. Phase 2D3 closure additionally builds the real `optic-bridge.exe` and drives it over stdio MCP against a disposable Git repository, proving integration-only tool exposure, exact-head fast-forward and stale-target rejection at the application binary boundary.

## Soak and chaos

Run 1h/4h mixed workloads, repeated session create/destroy, high-output jobs and controlled bridge termination/restart. Inject cancellation/crash at transaction state boundaries and verify idempotent recovery.

Track bridge memory separately from child process trees and Windows standby cache.
