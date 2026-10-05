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

## Cursor model

process_read(job_id, stream, cursor, max_bytes) returns bounded data plus next_cursor, EOF/truncation metadata and total byte counters.

## Content references

PROPOSED: large immutable artifacts may be represented by opaque content references backed by bounded disk storage. Deduplication/content hashes can avoid resending identical payloads, but must not become an unbounded content-addressed cache.

## Backpressure

If RAM/disk/global job budgets are exhausted, fail predictably with ResourceLimit rather than accumulating work.
