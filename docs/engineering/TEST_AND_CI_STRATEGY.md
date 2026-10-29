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

## Local Windows development prerequisites

Local builds and tests on an operator PC require explicit task-specific authorization. Use the pinned Rust toolchain and MSVC build tools. Reparse/symlink fixtures must actually create their disposable links: Windows error 1314 at fixture creation means a missing host prerequisite, not a successful escape-denial test. Use an authorized symbolic-link capability, such as [Windows Developer Mode](https://learn.microsoft.com/en-us/windows/advanced-settings/developer-mode); do not remove or silently skip these assertions.

The execution context is part of native evidence. Additional filesystem, token or network restrictions imposed by a development tool can change observed behavior. Record that context and preserve the first failed logs. When authorized, compare the same source and hashed binaries in a standard Windows execution context with all product assertions intact. This comparison does not authorize evading an application-control decision, changing Windows protections, or claiming a full-suite pass from a targeted pass.

Process tests also obey the real host-memory admission guard. With the current default limits, the required available memory includes the larger of 1 GiB or 10% of total physical RAM, existing counted process-memory reservations, and the new process budget. `HostMemoryHeadroomExceeded` means the probe was refused before launch; it is not evidence that the probe exercised its intended containment behavior. A memory snapshot is only a point-in-time observation, so a preflight reading cannot guarantee later admission. Never lower the guard, process budgets or security assertions just to make a workstation pass.

For a memory-constrained development session, limit build and test concurrency without changing product limits:

```powershell
$env:CARGO_BUILD_JOBS = '1'
$env:RUST_TEST_THREADS = '1'
cargo test --locked --workspace --all-features -- --test-threads=1
```

These environment variables apply to that shell session. Serial execution can reduce peak usage but cannot guarantee sufficient host memory. If admission still refuses a probe, preserve the failure and use the hosted Windows/Ubuntu gates for the exact source instead of retrying until a transient pass occurs. Local serial results supplement the ordinary hosted CI; they do not replace it.

Every receipt must distinguish compilation, Clippy, targeted tests and the complete suite, including passed/failed/ignored counts and targets not reached after Cargo stops. Record the source SHA, command, context and log hashes. Environment preparation or a green subset alone does not close the Windows V1 NO-GO gates.

## Soak and chaos

Run 1h/4h mixed workloads, repeated session create/destroy, high-output jobs and controlled bridge termination/restart. Inject cancellation/crash at transaction state boundaries and verify idempotent recovery.

Track bridge memory separately from child process trees and Windows standby cache.
