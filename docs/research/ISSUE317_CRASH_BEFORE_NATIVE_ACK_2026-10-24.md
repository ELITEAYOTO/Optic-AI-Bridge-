# Crash before native ACK — 24 October 2026

Research only; no production fix, merge, or V1 GO.

Verified live reference branch: research/317-repeated-native-observation-2026-10-24,
47b6a6310c8bd9d494a0facd2aa661cbced9fc75. Reference CI 40050805563 successful.
Verified live main: 16ca67a75fd49c5681a0672773ffc268c3635f6b.
New isolated worktree/branch: research/317-crash-before-native-ack-2026-10-24.
Existing dirty launcher-crash-transfer worktree was preserved. Four draft files
were copied as a starting point, then reviewed and completed here; no claim that
the uncommitted drafts were validated. The 24 October checkpoint is in #317's
latest comments and dated research documents; no dated 24 October engineering
handoff exists at the reference SHA. ADR-0019 and ADR-0031 remain authoritative.

## Schedules and proof boundary

The guardian owns the genuine non-clonable admission and registered reservation
before starting a separate launcher PROCESS. The reservation is conservatively
prearmed. A fixed suspended current-libtest child is created atomically in the
launcher's own kill-on-close Job. No user repository, Git command, child resume,
arbitrary PID termination, or unsafe runtime code is used.

An independent Python supervisor remains outside its newly created outer Job.
The service host is assigned before an outer gate allows launcher creation.
Its descendants inherit outer containment. The launcher is terminated through
the exact owned CreateProcessW process handle, never via PID lookup. No outer
Job termination occurs before survivor-count and inner-cleanup observations.

Schedules: before-create; after-create; offered values without acceptance;
accepted Job alone; accepted Job and process handle before ACK. No ACK is sent.
DuplicateHandle duplicates from the exact owned source process. A Job accounting
query validates the duplicate; IsProcessInJob validates the accepted child.
Values in an offer alone are explicitly not ownership.

Without a duplicate Job, the guardian has no positive native empty-Job receipt.
It retains its actual admission/reservation, publishes no terminal state and
leaves revoke pending. External survivor accounting may prove no synthetic child
remains, but is NOT used to release this service's quota. The host is then ended
by outer recovery. This is conservative quarantine in a fixture, not durable
production recovery, bounded graceful shutdown or automatic reconciliation.

With a duplicate Job, the child must still survive launcher death (outer count 2)
before stop. A deliberately injected query failure retains Running/quota and
blocks synchronous revoke; an independent close request has already returned.
Only actual TerminateJobObject, optional process-handle exit, and a positive
zero-members query on that accepted Job permit Stopped and admission release.
The supervisor checks count 1 BEFORE any outer cleanup. Job-only acceptance
proves tree emptiness without asserting a separate root-handle exit receipt.

The first-red control is deliberately synthesized: missing acceptance returns
from the guardian, discarding genuine admission/worker ownership. The original
assertion `guardian vanished before retaining uncertain owner` is preserved and
must fail in the child host. This is NOT a discovered production execution path
or evidence that the old successful channel-handoff test would behave identically.
The same missing-receipt schedule with retention must remain explicitly pending.

## Validation

Local environment inventory: Windows 10 build 19045; Rust 1.99.0
(b940084d7, x86_64-pc-windows-msvc). Local Rustfmt check and git diff --check.
Native campaign is restricted to GitHub-hosted Windows, not this operator PC.
Hosted workflow records actual Windows version and rustc -Vv; artifact manifest
records exact source SHA, cases, phase receipts, and outer cleanup confirmation.

Commands: cargo fmt --all -- --check;
cargo clippy --locked -p optic-bridge-runtime --all-targets --all-features -- -D warnings;
cargo test --locked -p optic-bridge-runtime --lib;
cargo test --locked -p optic-bridge-runtime --lib --no-run --message-format=json;
python scripts/research/issue317_crash_transfer.py --test-exe <exact-built-libtest> --output crash-evidence.

Results are pending until actual hosted logs/manifest are inspected. Expected
outcomes in source are assertions, not measurements. First failures must remain
visible; no security control, lockfile or test gate may be relaxed.

## Residual risk and architecture

Prefer a service owner that already holds the admission, registration and native
Job before delegating child creation. Cross-process handoff must define durable
acceptance, duplicate ownership, acknowledgement loss, partial handle delivery,
and reconciliation for inaccessible source handles. Crash-before-create with
conservative prearm illustrates retained capacity without a recoverable native
receipt. An offer or timeout cannot resolve that ambiguity. A service process
crash additionally needs external durable quarantine/accounting; this fixture's
in-process guardian does not provide it. Descendant stdio, EOF, persistent real
Win32 failures, host power loss, and production task/lease provenance remain open.
