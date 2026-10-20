# Reliability and implementation debt

## Linux published-journal directory entry synchronization (candidate)

After [merged PR #286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/286), Linux GNU uses an opened directory and `renameat2(RENAME_NOREPLACE)` but did not call `sync_all` on that directory. Only the temporary journal file was fsynced. Proposed small change synchronizes that same directory after publication, and returns an error *while retaining the stable journal* if the directory sync fails. A Linux injected-failure test verifies the existing journal can still be inspected and reconciled. This addresses one concrete persistence barrier, **not** full power-loss guarantees or hardware cache behavior; Windows directory persistence, ancestor races and journal retirement persistence are still separate unknowns. Verify exact branch PR/CI before calling it delivered.

## Journal publication no-clobber invariant — integrated by #286

[PR #286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/286) was squash-merged as `67c65a5d78a2921cee826184a2ad8bc573e4fb88` (exact head `7007ad3d202e42f642cece5f8df131035806ff35`). Hosted Windows/Linux Rust, dependency policy and the three native checks passed on that head; independent [post-merge CI #39385303625](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39385303625) passed Windows/Linux/dependencies on first attempt. Preexisting evidence and concurrent publisher regressions passed; Linux uses `renameat2` no-replace, Windows uses no-replace `MoveFileExW`. Other platforms fail closed. Parent directory identity, directory synchronization and sudden-power-loss guarantees were explicitly outside #286.

## Setup-time state-root creation before denial — scoped correction under review

Pre-increment `MutationRecoveryJournal::open` creates the requested state-root directory tree before canonical workspace-confinement rejection. An absent nested path inside the workspace therefore causes an unrequested workspace mutation even though journal creation is denied. Branch `fix/recovery-state-root-preflight` preflights the canonical nearest existing ancestor before creating anything, retains the final post-create check and adds a negative no-directory-creation test and a valid outside-root control. Verify the PR and exact-head CI before calling this delivered. The preflight is intentionally conservative and does not pin ancestors or make a concurrent pathname-swap guarantee.


## Windows recovery staging evidence — focused #284 correction (status via PR)

Pre-fix `JournaledMutationService::cleanup_recovered_staging` validated only pathname regularity and matching file bytes before closing the reader and unlinking a separately resolved path. A hard-linked staging alias with content equal to the durable intent could be accepted and removed without detecting the external alias. [PR #284](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) narrows the operation to one exclusive native single-link, no-reparse delete handle: recheck opened size, bounded-hash it, delete with that handle, retain durable journal on any rejection. The new Windows regression asserts the external link and staging link both remain, with original bytes and durable journal intact. See PR for the latest exact-head CI/merge result; **do not count source proposals as merged**. Parent directory identity/reparse behavior, power-loss durability, and other concurrent namespace races remain unproven. Node/AppContainer loopback and MCP first-response timing defects are separate unresolved items.


## Journal append opened-file identity — addressed by #282 (2026-10-20)

`MutationRecoveryJournal::append` on pre-fix `main` checked only pathname `symlink_metadata` before a distinct append open; an otherwise valid journal hard-linked outside its state directory could receive new bytes. [PR #282](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/282) merged as `bf79b320b4e7aa04a02c87fe448d589296bc8bb2` and rejects a hard-link alias before append: on Windows via the existing exclusive no-reparse, single-link checked state handle, and on Unix through opened regular-file device/inode/single-link comparison with pre-open metadata. The byte ceiling uses the opened file length; a negative regression verifies both links retain identical original bytes. Exact final head `c837cead` passed hosted [CI #39375905332](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905332) and [native preparation #39375905331](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905331). This is delivered **scoped behavior**, not a general journal audit. Unix concurrent namespace/size races, Windows directory pinning, staging cleanup, journal publication and full sudden-power-loss durability remain unproven.

## Windows initialization timing — integrated diagnostic, root cause open

The Windows TEMP-only doctor has intermittently failed on its first MCP response after 30 seconds while the bridge remained alive ([CI 39356540735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39356540735), [CI 39370890039](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39370890039), both successful after exact-head failed-job-only reruns). In a distinct native proof, the AppContainer Node loopback test intermittently returned `TimedOut` at 10 seconds with no child output ([CI 39363540900](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39363540900), [CI 39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540)). Neither is proven related to the journal hardening.

Merged PR #280 (`main` `8b6ccdc`, exact final head `ce15d03`, hosted CI 39373512931 passed) adds an opt-in monotonic stderr-only static phase timeline to the Rust server and corresponding host spawn/initialization timestamps to the existing CI TEMP-only doctor. The objective is to narrow where the **next occurrence** stalls. The hosted Windows TEMP-only diagnostic smoke passed and measured 7ms process start, 30ms initialize send, and 4823ms initialize response on one successful run. This is not an established causal fix. Separate post-merge main CI 39373992973 was in progress at this checkpoint. No timeout increase or relaxed gate is authorized. Other surfaces remain uninstrumented.


## Journal evidence lifecycle — targeted corrections, not a full audit

- **Read #277 (merged)**: `main` `c2282ff5eba85d2a7480dd353db638a0623bd06d`.
  Hard-linked external recovery bytes previously passed pathname-only metadata
  checks. Windows now validates an exclusive no-reparse single-link opened
  handle, while Unix checks descriptor identity and single-link count. Exact
  head `825bcdb` passed all four hosted CI checks. No general power-loss
  recovery or adversarial directory replacement proof follows.
- **Retire #278 (merged)**: the original `remove_regular_file` accepted
  a hard link as regular and then removed its directory entry, losing evidence
  of a foreign alias. Merged `main` `b15fe7636dcb31033d71ecdfea9ed51bdc786a2b`
  rejects linked objects, deletes by validated exclusive handle on Windows
  and has a cross-platform negative preservation regression. Exact head
  `9778b49` passed four hosted checks in CI 39369331802 and 39369331816.
  This does not prove Unix concurrent pathname removal safety.
- **Still open**: journal append uses a pathname metadata check followed by
  a separate append opener; Windows staging cleanup validates, reads, closes
  and removes via separate pathname operations. Both need isolated assessment,
  not bundled into #278. Unix deletion remains path-based after descriptor
  validation, so the concurrent namespace replacement race is not closed.
  Explicit state-root/directory pinning and full sudden-power-loss persistence
  guarantees remain unproven.


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
| Windows native AppContainer Node loopback test intermittency | On PR #274 exact head `283c9e0`, CI 39363540900 attempt 1 failed `capability_free_appcontainer_denies_node_loopback_connection`: process `TimedOut` at 10s with no output, rather than normal probe exit; three serial diagnostics immediately passed, then failed-Windows-job-only rerun on the identical SHA passed. A second, independent recurrence happened on #278 post-merge `main` `b15fe76`: [CI 39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540) attempt 1 repeated `TimedOut` at 10s/no output on the same test; all three targeted serial diagnostics passed, while Linux/dependency checks were green. Failed-Windows-job-only attempt 2 on the identical SHA passed, including the original test and subsequent Node/MCP smokes. Neither first-attempt failure proves a journal regression or identifies the underlying cold-start root cause | Preserve both original failures, all three diagnostic passes, exact-head reruns and logs. Investigate cold startup/isolation launch and scheduling with bounded evidence; do not weaken the isolation proof, bypass the native gate or automatically extend the timeout |
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
