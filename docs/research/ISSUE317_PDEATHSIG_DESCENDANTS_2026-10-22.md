# Issue #317 — PR_SET_PDEATHSIG descendant-boundary experiment

**Research-only proposal (2026-10-22). Not merged; no production owner.**

## Falsifiable native Linux hypothesis

Linux `PR_SET_PDEATHSIG` can kill a direct child when the **creating parent thread** terminates, but this setting is cleared for a child created by `fork()`. Therefore even a direct child correctly armed with `SIGKILL` on parent death must not be treated as proof of descendant-tree termination. This is separate from the prior negative observations for ordinary PGIDs/setsid, uncertain admission-model Drop and absent cgroup v2 delegation on hosted Ubuntu.

Canonical source: [PR_SET_PDEATHSIG](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html), especially fork clearing, thread caveat and effects of subreaper death.

## Bounded independently supervised test

`scripts/research/issue317_pdeathsig_descendants.py` creates exactly three synthetic processes beneath a private **external subreaper**:

- A known parent fixture.
- An immediate guarded child which calls `prctl(PR_SET_PDEATHSIG, SIGKILL)` and confirms the original parent still exists before creating another child.
- An ordinary Python grandchild started with `start_new_session=True`; it does **not** enable its own parent-death signal.

The parent is terminated with `SIGKILL` through its `Popen` handle after the guarded child and grandchild both report readiness. The external supervisor requires the guarded child to be reaped with signal `SIGKILL`, while the grandchild is **observed alive after both ancestor deaths** in a separate session. It then uses a previously opened pidfd to kill that exact known fixture grandchild and `waitpid` to reap it. The entire experiment is deadline-bound; every fixture self-expires after nine seconds, and `finally` cleans up known fixture processes. No arbitrary PID kill, cgroup mutation, kernel privilege, real Git command or operator-PC execution occurs. The artifact publishes only booleans and exact GitHub provenance—not PIDs or filesystem paths.

**Safety/interpretation boundary:** this tests *ordinary spawned descendant semantics*. It does not prove all hostile or arbitrary native descendants, a production process-tree registry, durable shutdown after an outer supervisor crash, or suitability of a global subreaper in the Bridge. A passing CI is a **negative proof that parent-death signaling on only the direct child is insufficient**, not a working Linux isolation solution.

## Criteria / next gate

A dedicated hosted Ubuntu workflow must validate Python syntax and pass this scenario on the **exact changed SHA**, with direct-child signal cause and surviving grandchild independently observed, and final external cleanup confirmed. Any failure stays visible; a red/cancelled first attempt is not rewritten as green.

Combined with [the earlier Linux delegation preflight](ISSUE317_CGROUP_V2_DELEGATION_PREFLIGHT_2026-10-22.md), this prevents selecting PGID-only, direct pidfd-only or PDEATHSIG-only cleanup as sufficient. An acceptable implementation still needs proven membership, whole-tree cleanup, budget/accounting, session stop under saturation and a fail-closed unsupported-platform policy. Normal Windows/Linux CI remains independent and #318 remains untouched.

**CI status at source creation:** pending; no claims of tests passing at this SHA.
