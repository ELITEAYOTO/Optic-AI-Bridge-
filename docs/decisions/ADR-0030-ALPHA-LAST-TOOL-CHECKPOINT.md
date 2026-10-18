# ADR0030 — Bounded previous-tool observation in the generated alpha

Status: EXPERIMENTAL ALPHA / NATIVE VALIDATION REQUIRED
Date: 2026-10-18

The supervised alpha keeps one safe tool observation outside model-writable src.
On explicit local resume, optic_diagnostics exposes that prior-process record
alongside the independent prior-stop summary. Existing in-memory32-event history
still reports persisted=false. No new tool, effect, capability, runtime grant or
model-supplied filesystem/log path is introduced. This is generated public-dummy
alpha metadata, not private-project enrollment or production credential storage.

Under the existing exclusive operator-run lock, one last-tool.json and one fixed
.last-tool.pending are each limited to1024bytes. The strict versioned record has
a fresh non-authorizing bridge marker, bounded request sequence, tool name,
admitted/completed phase, UTC timestamps, boolean success and allowlisted error
code/uncertainty flag. It excludes arguments, source/path data, handles, job IDs,
credentials and native exception text. Duplicate/unknown fields, oversized,
reparse/non-regular or hardlinked records fail closed. Context is explicitly
untrusted_for_authorization; editing it cannot change grants or start a process.

An admitted record is flushed before forwarding. It does NOT prove policy
approval or effect execution: a disconnect can prevent forwarding. A completion
records the bridge result, including outcome_unknown. Task-start success is not
proof of job success, and a clean stop report is not a tool receipt. A missing or
incomplete record is never proof that an earlier effect did not happen. No
rollback, replay, PID adoption or restored job ownership is inferred. After
restart inspect current sources/results and use freshly authorized operations.
FS backend/model generation state and the cause of an FS cutoff remain invisible.

Writes use exclusive sidecar creation, file flush/fsync and atomic replacement.
A valid interrupted sidecar is promoted as metadata on local resume so another
immediate crash does not discard the newer observation. A malformed/unsafe file
is retained for local inspection; new non-diagnostic calls fail before forwarding.
The diagnostic tool remains available. No recursive cleanup or history sweep.
This covers process interruption, not a promise of power-loss durability or
transactional exactly-once effects; directory metadata durability is not claimed.

Disk work runs off the server event loop with one retained in-flight operation
and no waiting queue. Caller cancellation does not release its slot before the
worker finishes. Concurrent tool completions cannot overwrite a newer admission.
A busy checkpoint denies before forwarding with dispatch_attempted=false. A
write failure latches unhealthy persistence; later non-diagnostic calls refuse.
A completion-persistence failure preserves the actual effect result and adds
checkpoint_saved=false, rather than misreporting a successful mutation as failed.
The active caller must not retry that confirmed effect. After crash the remaining
admission is conservatively uncertain. Filesystem syscall latency is not given a
hard cancellation guarantee by this offload; it is not a new I/O timeout boundary.

Unit/HTTP tests cover abrupt child exit before completion and between fsync and
replace, repeat recovery, strict byte/schema/object checks, older concurrent
completion, cancelled caller slot retention, before-dispatch disk failure and
confirmed-result preservation on completion failure. The existing native owned
runner crash proof checks the exact prior observation after stop and actual
supervisor death with sources unchanged, core exit and no automatic job replay.
