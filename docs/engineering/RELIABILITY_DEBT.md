# Reliability and implementation debt

Reviewed: 2026-10-20. This is a bounded source review and delivery register, not
a claim of a complete security audit. STATUS owns delivered evidence; this page
owns concrete unfinished implementation/verification work.

Continuation mode: [GitHub-only handoff](GITHUB_HANDOFF.md), with no operator-PC
execution. Hosted native proof remains required where the boundary is Windows.

## Findings addressed in the current hardening change

Native path comparison used replacement-character conversion in runtime scope
checks. A Windows fixture reproduced an actual read from a distinct sibling root.
The fix refuses unrepresentable native paths, audits analogous Git and operator
policy placement checks, and preserves Git's native argument units.
See [ADR-0033](../decisions/ADR-0033-LOSSLESS-NATIVE-PATH-BOUNDARIES.md).
The regression exercises the boundary, not a string implementation detail.

## Remaining work, ordered by the boundary it unlocks

| Work | Current evidence or debt | Completion criterion |
| --- | --- | --- |
| Native path identity | Lossy comparison is addressed; Unicode case folding and Windows case-sensitive directories are not fully characterized | Native fixtures and a documented canonical comparison/identity contract; no guessed equivalence |
| Cargo #255 | Fixed offline prepared fixture passed only on disposable CI; ordinary profile still cannot open NUL | Finish optional broker/profile provenance, preview, exact SID grant, interrupted recovery and revoke before any local privileged setup |
| Electron #250 | Diagnostic scope is merged; normal sandboxed GUI still fails | Prove normal startup compatibility; keep failure visible and never silently select the outer-only comparison |
| Private writable builds | Core has input capture, staging, owned tasks and publication services, but no general MCP build profile | Proven writable-storage/cleanup contract, authorized host composition and real native goal-driven acceptance |
| Dependencies | Installed runtime presence does not authorize downloads, package scripts or egress | Operator-selected restore scope, package/script policy, credentials and network boundaries |
| Private-task journal retention | Finite record/unresolved ceilings intentionally refuse admission when full; no host retention/retirement workflow is delivered | Host-only version-pinned retirement of closed evidence introduced; explicit operator integration/retention UX and operational proof still required |
| Filesystem enumeration and sensitive metadata | Older read-boundary review deferred prepared enumeration and classification | Bind authorization to the observed enumeration target and settle content/listing/Git-control policy with native alias tests |
| Windows MCP TEMP-only smoke intermittency | Main CI 39356540735 failed a 30 s first-response wait with the bridge alive; a Windows failed-job-only rerun on the *same SHA* succeeded. No reproducible cause established | Retain failure and rerun as separate evidence; instrument bounded MCP startup/stdio path and add targeted deterministic regression if a defect can be reproduced; never silence the check |
| Research integration follow-up | Bounded measurements from #250 and #255 are integrated on their target tracks; normal compatibility remains unavailable | Implement the support/recovery contracts rather than retaining completed measurement PRs; do not import the whole alpha into main |

## Source review coverage

The original bounded review inspected the then-open #250/#255 deltas (now merged),
native path helpers, project input manifests and
capture, artifact publication, private task launch and journal validation.
Finite byte/entry/event bounds, exact snapshot hashes, staged publication progress
and explicit unknown outcomes already exist in those services; they should be
retained rather than rewritten merely to reduce file count.

Do not equate absence of TODO markers with completion. Internal service APIs are
not a delivered end-to-end project workflow, and test-only compatibility helpers
are not operator installation features. Prefer regression-backed corrections and
one coherent PR over another chain of small unmerged drafts.
