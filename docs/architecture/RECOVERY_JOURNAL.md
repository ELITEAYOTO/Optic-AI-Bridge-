# Crash Recovery and Journal

Status: PROPOSED, based on OpticCode transaction experience.

Use a small append-only or transaction-manifest journal for operations whose interruption can leave durable state: file mutation transactions, worktree creation/integration, session leases and cleanup state.

## Requirements

- journal entries are bounded/rotated;
- no secrets or full source contents by default;
- state transitions are explicit and idempotent;
- startup recovery detects incomplete operations;
- recovery never assumes a half-applied write is safe;
- rollback/reconcile is hash-validated;
- completed ephemeral job output is not treated as durable audit history.

## Recovery states

Prepared → Applying → Applied → Finalizing → Committed

Failure path:
RollbackStarted → RolledBack / RollbackFailed

Exact states may be shared with or adapted from OpticCode after reuse review.
