# Implementation Plan

## Phase 0 — freeze contracts

Create Cargo workspace, core traits/types, SessionContext, typed errors, configuration ceilings and ADRs. No broad feature work before these compile.

## Phase 1 — vertical slice

stdio MCP → one session → fs_list/read → policy → process_start/read/stop → Job Object → bounded output.

Gate: native Windows integration test proves child-tree cleanup and memory/output bounds.

## Phase 2 — safe mutation/Git

Transactional patch/write, expected hashes, git status/diff/log, crash journal.

Gate: forced-crash recovery and stale-write tests.

## Phase 3 — multi-session

Independent sessions/projects, per-session jobs/spools/capabilities.

Gate: adversarial cross-session access tests.

## Phase 4 — same-repo parallelism

Git worktrees, deterministic coordinator, integration/conflict gate.

Gate: two concurrent sessions cannot silently overwrite or integrate stale changes.

## Phase 5 — connectivity/install

Local HTTP/tunnel adapter as required, installer/autoconfig/self-test.

## Phase 6 — hardening

Restricted-token compatibility experiments, fuzz/property tests, long soak, signed release pipeline.

Avoid implementing later-phase abstractions early unless a current interface genuinely needs the boundary.
