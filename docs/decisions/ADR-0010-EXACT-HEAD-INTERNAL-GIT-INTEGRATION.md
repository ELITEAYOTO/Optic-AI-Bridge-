# ADR-0010 — Exact-Head Internal Git Integration

Status: ACCEPTED  
Date: 2026-10-04

## Context

Phase 2D2 made bounded Git reads available without giving MCP callers repository paths, raw Git argv or executable authority. Phase 2D3 must add Git integration without turning Git into a generic mutation shell or silently moving a user branch after its observed target head has changed.

Git integration also introduces two separate risks that do not exist in read-only Git: Git may execute hooks/configured helpers, and a worktree/ref mutation can affect repository administrative state even when project files are not directly edited.

## Decision

1. `Effect::GitIntegrate` binds both an exact `source_head` and an exact `expected_target_head`; both are validated full Git object ids.
2. `GitIntegrate` remains a capability distinct from `GitRead` and requires a repository-scoped task lease.
3. The first integration primitive is fast-forward-only. Divergent histories fail closed as `NonFastForward`; merge/rebase/cherry-pick behavior is not smuggled into this foundation.
4. The runtime target is an operator-owned direct ref under `refs/optic/integration/`. Ordinary user branch refs are rejected, symbolic target refs are rejected, and the update uses `git update-ref --no-deref <ref> <new> <expected>` so the exact-head check is also atomic at the Git ref update boundary.
5. Preparatory Git work uses an Optic-owned integration root outside the configured repository checkout. Each operation creates a detached, locked `--no-checkout` worktree keyed by the server-owned `ActionId`.
6. The operation validates that worktree against the exact expected target and removes it before the internal target ref may advance. Cleanup failure therefore fails before the ref mutation.
7. The caller workspace is never checked out, reset, merged or updated by this primitive.
8. Git commands disable prompting/pagers, system/global configuration and replacement objects. `core.hooksPath` is forced to an operator-owned empty directory and that directory is revalidated before mutation-capable Git calls.
9. Windows canonical `\\?\` paths remain authoritative for Optic containment checks but are converted to an equivalent non-verbatim representation only when passed to Git, because Git for Windows does not consistently accept verbatim paths as worktree destinations.
10. No MCP Git mutation tool is exposed by this runtime tranche. Application-owned authority provisioning and the thin MCP adapter are later Phase 2D3 gates.

## Consequences

- A stale target cannot be silently overwritten: Optic checks it before preparation, immediately before the update, and Git repeats the old-value comparison atomically in `update-ref`.
- A symbolic internal ref cannot redirect an Optic integration into a user branch.
- Fast-forward-only integration has deterministic conflict semantics and does not invoke merge drivers or require a populated checkout.
- The current user checkout remains untouched even when the Optic internal integration ref advances.
- Worktree administrative state is temporary and operation-owned, but process-crash recovery of an orphaned locked worktree remains a follow-up gate before public MCP exposure.
- A local actor with write access to the operator-owned integration state can still race external state; the runtime therefore revalidates critical state immediately before effects rather than treating earlier observations as authority.

## Deferred

- application-owned `GitIntegrate` authority provisioning and exact lease resolution;
- bounded recovery/cleanup of worktrees left by process termination;
- MCP `git_integrate` exposure;
- non-fast-forward merge/conflict production semantics;
- multi-session same-repository worktree ownership/coordinator from Phase 4.
