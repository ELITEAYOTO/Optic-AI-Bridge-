# ADR-0018 — Authorized private project input preparation

Status: IMPLEMENTED CANDIDATE / EXACT-HEAD VALIDATION REQUIRED
Date: 2026-10-16

## Decision

Compose the existing authorized versioned input capture with internal preparation
in a fresh ephemeral AppContainer's SID-derived native storage. The platform
query uses GetAppContainerFolderPath and preserves UTF-16 identity; native string
allocations have matching RAII releases. Named/reused profiles are refused.
There is no inherited workspace write ACL or model-selected destination.

A shared service admits four preparations, including capture in progress and
uncertain setup/cleanup. Capacity is reserved after typed read preflight and
before opening input files. Pure capture failure releases its reservation;
uncertain native allocation or cleanup retains it. Existing manifest ceilings
remain256 files,1MiB per file,8MiB total and16 path segments. These also bound
implied directory bookkeeping; they are not a disk quota for running code.

Session admission spans capture, private copying and final monotonic/live checks.
Sources are not reopened after sealing. The prepared object retains exact
session/task/epoch expectations, while in-memory input bytes are released. A
caller must retain transport/blocking-I/O permits and own one service for its
bridge, rather than constructing a new budget per call.

Preparation pins native directories and checks its root identity on cleanup.
Only recorded no-reparse files and directories are removed; unknown contents
cause a cleanup error instead of recursive deletion. Explicit discard reports
errors, Drop is best effort, and uncertain cleanup never releases accounting.

## Deliberate boundaries

The profile is private to the prepared object. This API cannot start a process,
change policy, install dependencies, publish artifacts or grant GUI/network
access. The host-only input-root accessor is not execution authorization. Future
execution must revalidate the owner/epoch and independent process authority,
retain the preparation until the owned tree is known empty, and bound output
writes. No MCP tool or mandatory Node/Python/Electron dependency is added.

This increment has no durable restart journal or automatic reclamation of old
profiles after a host crash. It must not be wired into a public staged-process
workflow until owned-task recovery and artifact/output contracts are implemented.
Existing unresolved sensitive-source classification is unchanged.

## Validation

Tests cover copy integrity after source edits, ownership, successful cleanup,
stale input rejection, capacity before missing-file I/O, and retained accounting
when an untracked file prevents cleanup. A focused native workflow publishes a
hashed exact test harness for local reproduction. Windows/Linux/dependency CI
must pass on this main-based branch. Local Rust linking currently lacks the
host's msvcrt.lib; formatting remains locally checkable.
## Bounded owner worker and cancellation (2026-10-16)

The optional async preparation API reserves one of the same four stage slots
before waiting for shared blocking-I/O capacity. The creating native worker owns
the profile throughout copying and cleanup; no unsafe Send/Sync assertion is
added for SID pointers. Transport and I/O permits remain held until preparation
has actually ended even when the awaiting caller is cancelled. The returned
host lease contains a bounded one-command control channel and a cleanup receipt.
Explicit discard waits for cleanup; dropping a delivered or undelivered lease
wakes its owner for cleanup. Idle preparations retain stage capacity, not request
capacity. Cleanup reacquires the same I/O governor; unavailable cleanup retains
accounting instead of silently releasing it. One service belongs to one bridge.

This main increment still cannot start any process. The worker is not a durable
restart journal and must not be presented as crash recovery or task replay.
Native tests cover acknowledged discard, an abandoned delivered lease and an
abandoned in-flight request retaining its effect permits until actual completion.

Optic itself removes recorded inputs without recursively traversing unexpected
contents. Windows may independently reclaim its fresh profile storage during
profile deletion; retained accounting is not a backup guarantee for unexpected
data inside that owned storage. External user source paths receive no grants.
