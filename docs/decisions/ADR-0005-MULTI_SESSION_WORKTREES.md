# ADR-0005: Isolated Same-Repo Work via Git Worktrees

Status: Accepted in principle

Parallel AI sessions must not write concurrently to the same physical checkout. Each same-repository task receives an isolated worktree/branch and integrates through a conflict-aware gate.

Detailed integration algorithm remains to be finalized after prototype tests.
