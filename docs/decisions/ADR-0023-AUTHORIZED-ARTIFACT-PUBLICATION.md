# ADR0023 — Bounded same-workspace artifact publication

Status: EXPERIMENTAL HOST API / NO NEW MCP ROUTE
Date: 2026-10-17

Compose existing authorized versioned input capture and journaled FileWrite against
the same operator-enrolled workspace. This service publishes already present
generated files; it cannot run a build, accept another/private output root, grant
process write access or install a dependency. Separate private output collection
and hard runtime disk bounds remain gates. Do not advertise this as a complete
native build environment or a permission to consume arbitrary filesystem data.

The caller supplies one sorted manifest and aligned FileRead/FileWrite actions.
All actions must share a live application session and policy epoch. Read and write
task leases may differ and remain independently scoped. The service preflights
every declared read and write before source I/O, then seals all expected bytes and
versions before any publication write. Captured bytes never authorize a write.
Every actual destination write passes existing live policy/journal/expected-state
checks; corresponding read consent is rechecked before each destination effect.

Bounds:32 publications; existing capture ceilings256 files/1MiB per file/8MiB total
remain ceilings rather than grants, and lower configured read/mutation limits
still apply. No directories are created. Target paths use the portable manifest
namespace rules, refusing case/file-directory aliases and device/control names.
Targets must not overlap any selected source path, including component ancestors.
Duplicate batch ActionIds are refused. The service retains session admission;
transport and blocking-I/O permits remain the host caller's responsibility until
the synchronous operation has actually ended, even if its transport disappears.

A batch is not a cross-file transaction. Early shape/policy/capture failure returns
an error before a destination write is attempted. Once publication begins, an
interruption returns bounded prior verified receipts, the failed index and the
real underlying error. The failed effect may require mutation-journal recovery;
its failure is never proof that nothing changed. No automatic rollback, retry,
filesystem rescan or grant renewal occurs. Unattempted later actions stay absent.
Per-write ActionIds and expected state remain available for existing reconciliation.

Native tests cover create/exact replacement, stale source versions, denied/foreign
write preflight before missing source I/O, aliases/source overlap/duplicate IDs,
partial progress on stale second destination, read revocation between effects and
redirected destination refusal. No user project or installed runtime is required
for these generated filesystem fixtures. Exact Windows/Linux CI and a verified
native harness reproduction precede main promotion or adapter exposure.
