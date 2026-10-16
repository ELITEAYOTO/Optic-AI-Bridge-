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
capacity/corruption/reopen tests, abrupt owner exit before allocation, and nine
actual isolated Node lifecycle cases each ending in a closed record. Exact head
3c5611dc34b8f83bfc95d3a43fbfc5e373eca70b passed native workflow38954900323:
ten preparation, eight journal and eight isolated Node cases. The same bounded
cases passed locally from the verified CI artifact (Node4.52seconds). A separate
CI-only test exits the actual owner after native resume and synced Running; the
replacement owner reports Running with no outcome, retains capacity and does not
replay or reopen a PID. It deliberately retains a small quarantine on a disposable
runner. This proves interrupted-owner evidence, not recovered native cleanup.

A native owned-job watchdog is armed while suspended, before resume and before
any post-resume journal I/O. Its deadline is the minimum of the task budget and
the initially observed session/read/process authority TTLs. Renewal never extends
an already armed job. At most four atomic stop signals cover those authorities
and the managed job's operator stop. The watchdog checks at bounded20ms intervals,
holds only a duplicate of the exact owned Job handle, and performs no registry,
filesystem or profile operation. Four admitted stages bound worker count.

Revocation publishes its atomic signal before waiting for admission drainage.
The launch admission fence ends after successful native resume, before syncing
Running. A blocked journal write therefore cannot hold revocation behind disk I/O
or prevent the watchdog requesting owned-tree termination. Cleanup still requires
positive empty-tree confirmation; timer firing is not cleanup acknowledgement.
Running monitors use immutable, already verified authority plus atomic revocation
and the initial TTL, avoiding repeated executable hashing every polling interval.
This observation path cannot authorize a launch or replace executable pinning.

Integrated alpha3b1457ccd9a514520ef86e378861c7f6b3b379ce passed native
workflow38956512499: ten preparation, eight journal, nine Node cases and the
separate opt-in native owner crash. A cfg(test)-only bounded barrier pauses the
owner after resume, before journal acknowledgement. Revocation returns while it
is blocked; request/stage accounting remains occupied until release and confirmed
cleanup. This barrier simulates blocked bookkeeping, not an injected OS fsync
failure. The blocked-owner and admission-signal cases reproduced locally from
verified artifact12070842559 (1.95s/0.01s). No deliberate native crash ran locally.
