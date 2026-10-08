# Memory and Output Model

Status: DECIDED principles; numeric values are initial design defaults/TARGETS.

## Invariants

No stdout/stderr history, MCP response, completed-job cache, event queue or audit buffer is unbounded.

Large output belongs on disk, not permanently in bridge RAM or one MCP response.

## Initial design defaults

- active stdout tail: 512 KiB/job maximum in RAM;
- active stderr tail: 512 KiB/job maximum in RAM;
- bridge-owned output RAM budget: 16 MiB;
- default MCP payload budget: 64 KiB;
- hard single response ceiling: 256 KiB;
- completed job tail: 64 KiB;
- completed jobs resident metadata: max 32;
- completed-job TTL: 10 min;
- disk spool: bounded by per-session/global quota + TTL.

These are not measured performance claims and must be tuned from benchmarks.

## Current executable Phase 3A bounds

The current runtime keeps 16 MiB of bridge-wide reserved process-output capacity and adds an 8 MiB per-session ceiling. Active process jobs are capped at 8 bridge-wide / 4 per session; retained process records are capped at 64 bridge-wide / 32 per session. Retained output reservation counts against these limits until its record is retired. When capacity pressure requires retirement, a session may evict only its own terminal record; it never reclaims another session's retained output/result. The application session registry is also hard-bounded to 16 retained sessions. Disk spooling/TTL remains a later design item and is not claimed as implemented.

## Phase 3E1 process-memory reservation

Process working-set/commit memory is separate from bridge-owned stdout/stderr RAM. Phase 3E1 adds admission-time reservation of each job's declared `ResourceBudget.memory_bytes` across active/uncertain process ownership:

- per-job ceiling: 8 GiB (existing budget ceiling, unchanged);
- per-session aggregate declared process-memory ceiling: 8 GiB;
- bridge-wide aggregate declared process-memory ceiling: 16 GiB;
- `Running` and `TerminationUncertain` count against those aggregates;
- proven-terminal jobs release process-memory reservation even when their bounded output/result record remains retained.

On Windows, the individual job ceiling remains independently enforced by the Job Object. The aggregate values are admission reservations, not RSS/commit telemetry and not a guarantee of physical host headroom. Dynamic RAM-pressure/emergency-headroom policy remains future ResourceGovernor work.

## Phase 3E2A host-memory observation

Phase 3E2A is merged and validated through PR #103 (`e2b37be0`, exact head `3c765d9e`, CI #410/#411). It adds a Windows-only, point-in-time host physical-memory observation primitive in `optic-bridge-windows`, reporting validated `total_physical_bytes` and `available_physical_bytes` from `GlobalMemoryStatusEx`; the Win32 `unsafe` call remains confined to the platform crate.

3E2A remains deliberately telemetry-only: `ProcessManager` does not consume the snapshot by itself, and no MCP/session/policy authority was added.

## Phase 3E2B emergency-headroom admission

Phase 3E2B is the current gate. On Windows, `ProcessManager` consumes host-memory observations through a testable provider under the same admission lock used by the 3E1 reservations. The default reserve is `max(1 GiB, 10% of total physical RAM)`. Admission requires current available physical memory to cover that reserve, the full active/uncertain 3E1 declared-memory reservation, and the new job's requested memory budget. This intentionally favors fail-closed headroom over utilization efficiency because Optic does not yet measure each job's actual committed/working-set memory. Provider failure or malformed telemetry fails closed, and host-headroom denial has a stable MCP resource code distinct from the 3E1 aggregate-memory capacity errors.

This remains a point-in-time admission guard, not a guarantee against unrelated processes consuming memory immediately afterward. Heavy-task classes/slots, I/O governance and richer pressure feedback remain separate A-02 work.

## Cursor model

process_read(job_id, stream, cursor, max_bytes) returns bounded data plus next_cursor, EOF/truncation metadata and total byte counters.

## Content references

PROPOSED: large immutable artifacts may be represented by opaque content references backed by bounded disk storage. Deduplication/content hashes can avoid resending identical payloads, but must not become an unbounded content-addressed cache.

## Backpressure

If RAM/disk/global job budgets are exhausted, fail predictably with ResourceLimit rather than accumulating work.
