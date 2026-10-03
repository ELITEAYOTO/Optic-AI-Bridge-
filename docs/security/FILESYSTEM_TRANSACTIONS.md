# Filesystem Safety and Transactions

## Root confinement

All paths resolve relative to a session grant. Reject traversal, unexpected prefixes, alternate/nested repository boundaries and unsafe reparse/symlink escapes.

Canonicalization alone is insufficient against TOCTOU. Sensitive operations revalidate the target as close to use as practical.

## Writes

Prefer patch/transaction APIs over blind whole-file replacement.

A mutation records expected prior hash, intended operation and resulting hash. Writes use temp-file + flush/atomic replace where platform semantics permit.

## Conflict detection

If expected hash/version differs, return StaleBase/Conflict. Never silently overwrite.

## Transactions

Reuse/adapt OpticCode's manifest/journal/rollback approach after dependency extraction review. Transactions must remain project-scoped and recoverable after crash.

## Sensitive files

Credential stores, .env/secrets, keys, Git internals and security configuration receive explicit policy treatment rather than relying on filename blocklists alone.
