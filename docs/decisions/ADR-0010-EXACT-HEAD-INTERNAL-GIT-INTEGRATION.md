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
10. Recovery never uses broad `git worktree prune`. Git registration is read through bounded machine-readable `worktree list --porcelain -z`, user worktrees outside the canonical Optic integration root are ignored, and Optic-owned candidates are separately capped by the concurrent-request ceiling. An owned candidate must be a direct canonically encoded ActionId-named, registered, locked, non-symlink, canonically contained worktree.
11. Recovery validates the complete bounded snapshot before cleanup, revalidates each filesystem path immediately before removal, and keeps the ownership lock until Git removes the worktree using double `--force`; ambiguous state fails closed.
12. Git read and integration executable authority are explicitly separate: `--git-executable` provisions the read runtime, while `--git-integration-executable` belongs only to the integration/recovery runtime. Neither option silently grants the other's capability.
13. Integration startup requires a complete executable/root/internal-ref tuple. Recovery runs before MCP serve whenever that tuple is present. `--allow-git-integrate` is a separate explicit opt-in that alone permits minting the repository-scoped `GitIntegrate` lease/session capability; recovery-only startup grants no mutation authority.
14. The MCP server requires integration runtime and application-owned authority to agree when authority is retained.
15. The thin MCP adapter exposes only exact `source_head` + `expected_target_head`. Repository/ref/path/executable/raw-argv/lease/ActionId authority remains application-owned; unknown public fields are rejected. The adapter generates ActionId, resolves the existing repository lease and calls only `AuthorizedGitIntegrationService`.
16. Once an atomic ref update succeeds, inability to complete post-update verification is an explicit uncertain-outcome class. Public adapters must not map that condition to a pre-effect failure or safe retry.
17. A client must be able to obtain a fresh exact target precondition without learning the internal ref. `Effect::GitIntegrationObserve` is therefore read-only but requires the same `GitIntegrate` capability, exact application-owned task lease and repository scope. `git_integration_status` returns only the current `target_head`; it exposes no ref/repository/path/executable/argv/lease authority and does not weaken the later exact-head compare-and-swap.

## Consequences

- A stale target cannot be silently overwritten: Optic checks it before preparation, immediately before the update, and Git repeats the old-value comparison atomically in `update-ref`.
- A symbolic internal ref cannot redirect an Optic integration into a user branch.
- Fast-forward-only integration has deterministic conflict semantics and does not invoke merge drivers or require a populated checkout.
- The current user checkout remains untouched even when the Optic internal integration ref advances.
- Worktree administrative state is temporary and operation-owned. Because cleanup occurs before target-ref mutation, a process-termination orphan is a pre-ref-effect artifact; bounded recovery can remove only a proven owned orphan without inferring whether integration committed.
- Recovery does not mutate unrelated worktree administration through prune and refuses partial cleanup when its initial bounded snapshot is ambiguous.
- A local actor with write access to the operator-owned integration state can still race external state; the runtime therefore revalidates each cleanup path immediately before removal and revalidates critical target state immediately before ref effects rather than treating earlier observations as authority.
- An operator can run integration recovery without granting either `GitIntegrate` or `GitRead`; capability creation remains a distinct explicit startup decision.
- Public integration input cannot redirect the internal target or select another Git executable/repository/lease.
- If the atomic target update may already have occurred, the response contract preserves uncertainty rather than encouraging blind retry.
- Reconnected clients can refresh only the opaque target commit needed for optimistic concurrency without learning or selecting the internal integration ref.

## Deferred

- CI/merge validation of the 2D3D target-observation companion and final ChatGPT Desktop integration smoke;
- non-fast-forward merge/conflict production semantics;
- multi-session same-repository worktree ownership/coordinator from Phase 4.
