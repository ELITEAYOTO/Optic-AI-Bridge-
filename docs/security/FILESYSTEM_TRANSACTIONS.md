# Filesystem Safety and Transactions

## Root confinement

All paths resolve relative to a session grant. Reject traversal, unexpected prefixes, alternate/nested repository boundaries and unsafe reparse/symlink escapes.

Canonicalization alone is insufficient against TOCTOU. Sensitive operations revalidate the target as close to use as practical.

## Phase 2A observation boundary

Mutation planning is byte-bounded by `HardLimits::max_fs_mutation_bytes` and uses streaming BLAKE3 `ContentVersion` values. Existing leaf symlinks are rejected. For an absent target, the parent is canonicalized before deriving the effective target path.

`ExpectedState::Absent` means create-only intent. `ExpectedState::Content(version)` means update-only intent against one exact observed content version. A mismatch fails closed instead of silently overwriting.

## Phase 2B Windows commit boundary

Windows durable commit is implemented behind the narrow `optic-bridge-windows` unsafe boundary and is not yet exposed as an MCP mutation tool.

For an existing target:

1. open the file with `FILE_FLAG_OPEN_REPARSE_POINT`;
2. reject a final-component reparse point from handle metadata;
3. obtain the normalized final path from the handle and require it to remain under the canonical workspace root;
4. capture `FILE_ID_INFO` (volume serial + 128-bit file id);
5. hash the opened handle under the mutation byte ceiling;
6. bind the prepared mutation to both the expected content version and exact Windows file identity.

For an absent target, the canonical parent directory is opened and bound to its `FILE_ID_INFO` identity. This detects parent-directory delete/recreate between planning and commit.

At commit time the expected state and identity are revalidated, replacement bytes are written to a create-new temporary file in the same directory and `sync_all` is called, then the expected state and identity are revalidated again immediately before the namespace operation.

Existing targets use `ReplaceFileW`. Absent targets use create-only hard-link semantics, so a target that appears concurrently is not overwritten.

Microsoft references:

- `FILE_FLAG_OPEN_REPARSE_POINT`: https://learn.microsoft.com/windows/win32/api/fileapi/ns-fileapi-createfile2_extended_parameters
- `GetFileInformationByHandleEx` / `FILE_ID_INFO`: https://learn.microsoft.com/windows/win32/api/winbase/ns-winbase-file_id_info
- `GetFinalPathNameByHandleW`: https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew
- `ReplaceFileW`: https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-replacefilew
- `FlushFileBuffers`: https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers

## What Phase 2B proves

Native Windows CI proves that:

- an existing target can be replaced and the resulting content version verified;
- deleting/recreating the same path with the same bytes is rejected because file identity changed;
- absent-target creation is create-only;
- recreating the parent directory invalidates an absent-target plan;
- handle identity and `ReplaceFileW` wrappers behave as expected on the Windows CI platform.

## What Phase 2B does not prove

Phase 2B is **not** a kernel compare-and-swap against arbitrary external writers.

`ReplaceFileW` accepts the final target by path. The last expected-state/file-identity check is deliberately performed immediately before that call, but another local actor could theoretically change the namespace in the remaining instruction window. The project must not describe this as an eliminated race.

A future hardening experiment may evaluate Windows opportunistic locks (`FSCTL_REQUEST_OPLOCK`) or a handle-based rename design. Oplock acquisition/break behavior is subtle and can introduce deadlock/compatibility costs, so it is not added merely to improve a security claim on paper.

References:

- Oplock control: https://learn.microsoft.com/windows/win32/fileio/file-management-control-codes
- Oplock acquisition semantics: https://learn.microsoft.com/windows/win32/fileio/types-of-opportunistic-locks
- Handle-based rename surface: https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle and https://learn.microsoft.com/windows/win32/api/winbase/ns-winbase-file_rename_info

## Post-commit result semantics

After the namespace commit, verification is best-effort but explicit. If the new content is observed exactly, return `CommitVerification::Verified(version)`. If the namespace operation may already have committed but verification cannot prove the final state, return `CommittedButUnverified` rather than an ordinary pre-commit failure. Callers must treat this state as requiring reconciliation, not as permission to retry blindly.

## Phase 2C1 recovery-journal boundary

PR #15 introduces the recovery state machine independently from the Phase 2B commit path so the persistence/reconciliation contract can be tested before it controls real mutations.

Recovery state is stored under an operator-provided state directory that must canonicalize outside the project workspace. The journal directory is therefore not addressable through a normal workspace-relative MCP path.

Each operation has one per-ActionId append-only journal with schema version, canonical workspace path, exact previous state and intended resulting content version. The complete state sequence is:

`prepared → committing → verified|ambiguous`

The ordering contract is strict:

1. create the `prepared` record in a new temporary journal file;
2. `sync_all` the file;
3. publish it by a same-directory rename to the stable journal name;
4. append and `sync_all` a complete `committing` record **before** a namespace commit is allowed to be attempted;
5. after the namespace effect, append and `sync_all` either `verified` or `ambiguous`;
6. retire the journal only after terminal handling.

The first PR proves the journal/recovery logic but does not yet connect step 4 to the actual Phase 2B namespace operation. That wiring and forced-crash proof belong to Phase 2C2.

### Bounded recovery

Two independent hard ceilings are enforced:

- `HardLimits::max_mutation_journal_file_bytes` — 64 KiB by default per operation;
- `HardLimits::max_mutation_recovery_records` — 256 entries by default per startup scan.

Zero is never interpreted as unlimited.

Recovery reads are bounded. The directory scan is bounded and deterministic. Unexpected files, symlinks or invalid ActionId names fail closed instead of being silently ignored, except a well-formed unpublished `*.prepared.tmp` record which is safe to discard under the protocol because `committing` was never durably published.

A torn trailing state line is ignored and the last complete line is used. If no complete initial `prepared` record exists, or if immutable fields/state ordering disagree, the journal is corrupt and recovery fails closed.

### Reconciliation semantics

For a complete journal:

- `prepared` only → `PreparedNotCommitted`; the protocol has not allowed a namespace commit yet;
- `verified` → `VerifiedTerminal`; do not reinterpret a later third-party change as evidence the earlier commit did not happen;
- `committing` or `ambiguous` → observe the real target through the existing bounded/canonical mutation-observation path:
  - exact intended version → `ObservedCommitted`;
  - exact previous state → `ObservedNotCommitted`;
  - anything else → unresolved recovery conflict, retain the journal and fail closed.

Recovery never blindly retries an ambiguous operation.

## Durability claim boundary

`sync_all` / Windows file flushing improves file-data persistence, but Phase 2C1 does **not** claim full power-loss ACID durability for the journal or workspace namespace. In particular, filesystem/directory metadata ordering across sudden power failure is not proven by the current tests.

The currently supported claim is narrower: the state machine is designed for deterministic process-crash recovery, and Phase 2C2 must prove that claim with real child-process termination around the journal-wrapped Phase 2B commit. Power-loss guarantees, if ever claimed, require a separate documented persistence design and test strategy.

## Phase 2C2 required crash gates

Before public mutation tools are enabled, a child process must be terminated at least at these boundaries and startup recovery must produce the expected classification without blind retry:

- after durable `prepared`;
- after durable `committing`, before namespace commit;
- immediately after namespace commit, before terminal journal state;
- after terminal state, before journal retirement.

The journal ActionId should also identify the actual staging artifact so recovery can reason about and clean only artifacts owned by that operation.

## Transactional services

Prefer patch/transaction APIs over blind whole-file replacement. File write/patch/delete runtime services are Phase 2C3 and must reuse the journal-wrapped commit boundary rather than bypass it. Public MCP mutation remains disabled until the recovery and policy gates pass.

## Sensitive files

Credential stores, `.env`/secrets, keys, Git internals and security configuration receive explicit policy treatment rather than relying on filename blocklists alone.
