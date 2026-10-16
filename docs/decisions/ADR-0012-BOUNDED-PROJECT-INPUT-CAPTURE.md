# ADR-0012 — Bounded versioned project input capture

Status: IMPLEMENTED CANDIDATE / NATIVE VALIDATION REQUIRED
Date: 2026-10-16

## Context

A build or test may need several project modules. Reopening writable sources
while a process runs can change the code being executed. Process permission
alone must not authorize reading those sources or broaden workspace access.

## Decision

Introduce a transport-independent immutable project input snapshot. A manifest
binds exact paths, byte sizes and content versions to one session, task lease
and policy epoch. It admits 1–256 files, at most 1 MiB each and 8 MiB total.
Portable paths reject case aliases, file/parent collisions, reserved Windows
names and repository/runtime control directories. Sealing checks every declared
file and rejects missing, extra, duplicate, changed or incorrectly sized bytes.
Debug output includes counts and sizes, never source contents.

The Windows runtime capture service requires one existing typed FileRead action
per sorted manifest entry. Every action passes active session, lease, scope,
epoch and resource policy before any source file is opened. Reads retain session
admission, pin the selected root and ancestors, verify the original native root
identity, reject reparse traversal and compare the observed canonical path.
Existing file-read ceilings remain in force. Monotonic deadlines and repeated
authorization checks discard captured data when expiry or revocation occurs.

The synchronous caller must retain transport and blocking-I/O capacity through
completion, including caller cancellation. The snapshot is not a filesystem-wide
atomic transaction: each declared content hash must match, but files may have
been observed at different instants. Non-Windows capture fails closed pending
an equivalent native boundary; pure manifest validation is platform independent.

## Scope and validation

This increment adds no MCP tool, executable grant, package manager, process
launcher, GUI access or workspace mutation. Staging, dependency acquisition,
artifact publication and process containment require independent authorization
and validation. Electron remains optional and is not a core dependency.

Domain tests cover binding, limits, aliases, deterministic snapshots and content
mismatches. Runtime tests cover policy denial before I/O, insufficient budgets,
root replacement, symlinks, stale versions, empty files, read ceilings, deadline
expiry and concurrent session revocation. Native Windows and Linux CI must pass
on this independent branch before integration. Local linking is currently blocked
by the host's missing MSVC msvcrt.lib; formatting can be checked locally.