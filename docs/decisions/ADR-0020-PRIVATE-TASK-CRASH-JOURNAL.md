# ADR0020 — Bounded private-task crash journal

Status: EXPERIMENTAL / HOST API ONLY
Date: 2026-10-16

An optional operator-owned journal records a fresh profile plan before allocation,
then preparation, start intent, running, confirmed-empty finalization and cleanup.
Plans are generated in memory and cannot be reconstructed from journal/model names.
No PID recovery/kill, automatic effect replay or restored authorization is added.

State is locally selected outside the native workspace. Its entire local-drive
parent chain is pinned; remote/device paths are refused. One exclusive file lock
lives through all worker tickets and binds state to the workspace native identity.
Files are opened exclusively without truncation/reparse traversal; hard-link
aliases are refused. The host must retain transport/blocking-I/O permits.

Bounds:32 records, four unresolved intents,16KiB per record, eight events and2048
bytes per event. Only strictly parsed closed records may be retired for history.
Incomplete intents retain capacity after restart. Unknown files, malformed/partial
lines, workspace mismatch and invalid transitions fail closed. Corruption is an
operator reconciliation gate, never permission to discard state or retry a task.

Records hold input/version and identity fingerprints, a random diagnostic run ID,
fresh internal profile name, epoch and fixed outcome/exit code. No source text,
raw session/task/job handles, command lines, environment or stdout/stderr persists.
Hashes bind observations, not permissions. Records never authorize launches,
cleanup or file writes; no model tool can change this internal bookkeeping.

Each append requires sync_all before the next corresponding native effect.
Running is written after resume; failures reconcile the owned tree. Finalizing
requires known-empty ownership before destructive cleanup. Closed is synced only
after private input/profile cleanup, before releasing the stage slot. Panic or
uncertain termination retains the stage and incomplete record. Process-crash
recovery evidence is not a guarantee for every power-loss/filesystem failure.

Default legacy preparation stays nonjournaled; explicit with_journal enables the
prototype. No public staged MCP launch is enabled. Production enrollment, operator
reconciliation, durable bounded output evidence, writable artifacts/dependencies
and GUI authority remain separate gates. Retained files/diagnostics do not restore
FS model memory or runtime authorization after restart.

Validation requires native lock/nontruncation/hardlink tests, journal transitions,
capacity/corruption/reopen tests, abrupt owner exit before allocation, and five
actual isolated Node lifecycle cases each ending in a closed record. A crash
inside native allocation/execution still needs separate evidence; pre-allocation
crash tests must not be advertised as proof of native cleanup.
