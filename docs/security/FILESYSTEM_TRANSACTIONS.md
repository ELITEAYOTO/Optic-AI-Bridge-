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

A future hardening experiment may evaluate Windows opportunistic locks (`FSCTL_REQUEST_OPLOCK`) or a handle-based rename design. Microsoft explicitly documents oplocks as coordination mechanisms whose acquisition/break behavior can be subtle and can deadlock if file operations are performed in the wrong sequence, so they are not added merely to improve a security claim on paper.

References:

- Oplock control: https://learn.microsoft.com/windows/win32/fileio/file-management-control-codes
- Oplock acquisition semantics: https://learn.microsoft.com/windows/win32/fileio/types-of-opportunistic-locks
- Handle-based rename surface: https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle and https://learn.microsoft.com/windows/win32/api/winbase/ns-winbase-file_rename_info

## Post-commit result semantics

After the namespace commit, verification is best-effort but explicit. If the new content is observed exactly, return `CommitVerification::Verified(version)`. If the namespace operation may already have committed but verification cannot prove the final state, return `CommittedButUnverified` rather than an ordinary pre-commit failure. Callers must treat this state as requiring reconciliation, not as permission to retry blindly.

## Transactions and recovery

Phase 2C adds the durable journal/recovery layer. A mutation journal must record enough state to distinguish at least prepared, committing, committed/verified and ambiguous/incomplete work, remain bounded, and be reconciled on startup. Forced-crash tests are mandatory before public mutation tools are enabled.

Prefer patch/transaction APIs over blind whole-file replacement. A mutation records expected prior state, intended operation and resulting state. Recovery must remain project-scoped.

## Sensitive files

Credential stores, `.env`/secrets, keys, Git internals and security configuration receive explicit policy treatment rather than relying on filename blocklists alone.
