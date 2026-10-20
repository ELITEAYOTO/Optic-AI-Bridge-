# Repository workflow

For the current GitHub-only continuation, follow [GITHUB_HANDOFF.md](GITHUB_HANDOFF.md):
no operator-PC execution or self-hosted testing. Canonical handoff documentation
lives on main; read it from main even when changing the alpha adapter. New agent
sessions must inspect live PRs/heads rather than assume a dated inventory persists.

## Reference branches

- `main`: reviewed Rust core, operator integration and canonical current docs.
- `alpha/generated-project-editing`: generated-project FS adapter and its proven
  bundle. Core changes arrive through ancestry-preserving alignment PRs; the
  whole research stack must not be merged backward into main.
- Completed diagnostic scopes #250 (alpha) and #255 (fixed Cargo research) are
  retained in their target histories/PR refs. Their short-lived research branches
  can be retired after exact-head integration; compatibility support remains a
  roadmap gate, not a reason to keep the measurement PR open indefinitely.
- `archive/*` tags: preserved historical snapshots, not release tags or active
  development branches. Only intentional version tags use the release workflow.

## Change lifecycle

1. Start from the actual target branch. One coherent change per branch/PR; do not
   create a long stack of public drafts for every internal development step.
2. State the concrete behavior, validation and remaining limits. Security or
   architecture changes require the owning contract and ADR when applicable.
3. Push a reviewable result. Validate the exact head on applicable platforms and
   inspect review feedback before merging. Do not describe old-head CI as current.
4. Merge completed core work into main. Align alpha explicitly when core history
   changes; preserve ancestry for integration merges.
5. Retire the completed branch after checking its exact head and preservation.
   A bulk repository cleanup requires its own explicit inventory and approval.
6. Update current status, roadmap and changelog without copying the entire
   execution diary into the README. Keep exact detailed evidence in its owner.

Known failing research gates remain visible and separate. A successful alternate
comparison never turns the normal gate green or authorizes a runtime fallback.
An unrelated, documented baseline research failure must be identified explicitly
when integrating a change whose applicable checks passed.

## Avoiding drift

README is an introduction, STATUS owns current measured delivery, ROADMAP owns
next acceptance order, specs/security/ADRs own contracts. Dated research owns
observations. Link to the owner instead of maintaining duplicate phase summaries.
An alpha package retains its exact build source/hash until rebuilt and verified.

No security rule, scope or test may be weakened solely to simplify repository
presentation. Closing a superseded PR is not evidence that it shipped in main.
