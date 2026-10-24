# Codex interruption recovery protocol

**Standing engineering practice — Optic AI Bridge.** This protocol preserves authorized engineering work when an assistant session, tool connection, or response ends unexpectedly. It does **not** disable, evade, diagnose internally, or predict any platform cybersecurity control. If an operation is blocked, do not disguise or retry the blocked operation to circumvent that decision.

**Scope:** exclusively `ELITEAYOTO/Optic-AI-Bridge-`. Optic Desktop and REL are separate repositories, with independent agents and worktrees. This procedure never grants permission to modify a user's machine, security settings, or unrelated processes. Follow [AGENTS.md](../../AGENTS.md), [REPOSITORY_WORKFLOW.md](REPOSITORY_WORKFLOW.md), and the repository's security invariants first if any instruction conflicts. The runtime [recovery journal proposal](../architecture/RECOVERY_JOURNAL.md) concerns application transactions and remains separate from this agent workflow.

## Session entry: discover the checkpoint without chat history

At the start of **every** session, before mutations, perform this bounded read-only discovery:

1. Verify the actual worktree root and official origin. Read `AGENTS.md` and the current canonical handoff. Treat a recovery location supplied by the operator as the first candidate; otherwise check the recovery path explicitly recorded in that handoff and `<parent-of-worktree>/codex-recovery`. On the known operator workspace, this resolves to `E:\Optic-IA-Bridge\codex-recovery`. Do not recursively scan other projects or create a missing directory during discovery.
2. Verify each candidate is outside **all** registered worktrees. Read its four entry documents in the order below. If `SNAPSHOT/CURRENT.json` exists, require its named snapshot to have `VERIFIED.json` and matching HEAD metadata; otherwise use the completed snapshot explicitly referenced by `RECOVERY.md`. Reject path traversal and symlink escapes. Do not choose the highest folder number or newest timestamp as proof of validity.
3. Recompute every declared checksum and independently compare the manifest's repository, official remote, branch, HEAD and worktree with live Git. Local paths may have moved: reconcile them explicitly, never reset to a stored HEAD. If CURRENT and the notes disagree, or multiple valid candidates identify different work, preserve them and report the ambiguity before mutation.
4. If a known interrupted task has no usable checkpoint, report the missing evidence instead of reconstructing success from chat. For a new task with no recovery directory, record that no checkpoint was found and establish an external recovery location before long or risky work. Continue only within the authorization already given.

This is a standing instruction followed when an agent reads `AGENTS.md`; it does not install a background service, guarantee that a session will run, or automatically detect a service's internal security decisions. Until this documentation is reviewed and merged, other branches do not acquire it automatically. After integration, sessions must read the current canonical instructions even when working on an older research branch.

## 1. Durable work, not durable conversations

An interrupted session may never emit its final reply. Save independently readable state **during work**, after meaningful milestones and before long or fragile operations—not only in the final answer. Prefer small reviewed commits on a dedicated research branch. A successful commit is not proof that code was tested or published remotely.

Use a persistent recovery directory **outside every Git worktree**, for example the operator's known directory:

`E:\Optic-IA-Bridge\codex-recovery`

This is an **example**, not an instruction to create it on another computer. Discover and verify its real location; never replace a directory merely because it has the expected name. A directory on the same `E:` drive protects against chat interruption **but not disk failure**. Arrange an independently stored backup only with operator authorization, secret review, and suitable access controls.

The local recovery root should contain these four stable entry points (names retained for compatibility with the existing incident):

| File | Purpose |
| --- | --- |
| `RECOVERY.md` | Start-here procedure, last verified snapshot ID, what is safe to resume, unresolved hazards |
| `STATE.md` | Timestamp/timezone, repository remote, branch, HEAD, worktree, dirty/untracked files, known remote status |
| `TESTS.md` | Executed commands and exact results, OS/version, targeted test selection, logs, first failures, skipped tests |
| `ACTIONS.md` | Ordered completed/incomplete actions, decisions, next safe step, explicit non-actions |

Snapshots are append-only, numbered and immutable once marked verified (for example `0010-final-verified`). Construct a **new** snapshot in a staging directory, collect a manifest and SHA-256 checksums, verify its content, then publish the completed snapshot by an atomic rename where supported. Update `RECOVERY.md` *last* so it always points to a completed snapshot. If an atomic rename is unavailable, use a completion marker created only after verification. Never silently overwrite an earlier verified snapshot.

A snapshot may include: branch/HEAD metadata, the four notes, `git diff --binary` (working and staged changes distinguished), commit identifiers, sanitized test logs, and **explicitly allowlisted** untracked files required for recovery. Git diff does **not** back up untracked files. Do not automatically copy entire worktrees, build outputs, credentials, `.env` files, tokens, private repositories, machine profiles or arbitrary directories. Verify symlinks and file sizes before copying.

Pause the session's own edits while copying. Recheck HEAD, branch, status and source bytes after copying; any concurrent drift invalidates the new checkpoint and leaves the previous pointer intact. For unpublished commits, preserve a binary branch delta against a recorded, available base or another reviewed portable Git backup; commit IDs alone do not preserve missing objects. Record deletions and keep staged changes distinct from the full working delta so restoration does not apply them twice. Set explicit bounds (for example 20 snapshots, 20 MiB and 200 journal lines); at capacity stop adding snapshots and request reviewed consolidation, never delete another session's work automatically.

At each meaningful checkpoint, record the **current facts**. Suggested read-only collection commands (adapt to the actual shell):

```sh
git rev-parse --show-toplevel
git remote -v
git branch --show-current
git rev-parse HEAD
git worktree list --porcelain
git status --porcelain=v2 --branch --untracked-files=all
git log -5 --format='%H %aI %s'
git diff --binary --
git diff --cached --binary --
```

These are evidence-gathering commands, **not** a guarantee that every generated output is safe to share. Scrub paths, secrets, usernames, token values, and private material before uploading. Record exit codes, time budgets and tested SHA separately; never infer that a test ran merely because its command exited successfully. Check the selected test count (a `0 tests` result proves nothing about the intended test).

Use `GIT_OPTIONAL_LOCKS=0` for read-only inventory. If Git flags ownership, verify the path and origin first and use only an approved command-local `safe.directory` exception, never a blanket/global trust change. Each validation receipt records full command, cwd, start/end time and timezone, OS/toolchain, exit code, executed/passed/failed/ignored counts, log path and SHA-256, plus HEAD/status before and after. A dirty source needs its saved delta/hash; source drift invalidates attribution. Keep planned, running, interrupted and completed operations distinct in ACTIONS and update the notes after each coherent unit.

## 2. Reopen after any interruption — read-only first

A new Codex session must **not rely on the previous chat's text**. Do the following before edits, checkout, reset, branch switching or test execution:

1. Identify the **actual** repository path and verify its remote resolves to `ELITEAYOTO/Optic-AI-Bridge-` using read-only commands. Do not search or mutate Optic Desktop/REL.
2. Read `RECOVERY.md` → `STATE.md` → `TESTS.md` → `ACTIONS.md` and locate the referenced completed snapshot. If missing, stop and report rather than guessing.
3. Compare the snapshot's SHA-256 manifest against its bytes. Check branch, HEAD, worktree list, dirty files, staged files and untracked files **independently** against Git. Treat stored SHAs as historical references, never reset targets.
4. Inventory any in-flight action (partially written file, test process, interrupted commit, unpushed commits, open PR). Do **not** kill an unrelated PID, delete a worktree, run `git reset --hard`, `git clean`, or overwrite unknown local changes.
5. If conflict, missing bytes, or mismatch is found, record the exact discrepancy and preserve both sides. Restore into a **new isolated directory** or worktree first and verify file hashes and patch applicability there. Promote nothing onto an existing worktree without an explicit comparison and safe review.
6. Only then continue authorized, non-blocked tasks. Rerun the smallest relevant deterministic checks, preserving failed outputs. Verify any newly published GitHub commit and CI on the **exact SHA**, when GitHub access actually works.

Before repeating an interrupted external mutation (commit, push, PR creation or dispatch), inspect its actual effects first; a missing response does not prove failure. Recovery does not authorize local project execution where the canonical GitHub-only rules forbid it. Use the permitted isolated environment or hosted CI for those checks.

**Recovery verification tiers:** (a) file exists; (b) checksum matches; (c) isolated restoration matches; (d) separate invocation rereads it; (e) wholly new Codex session restores it without old conversation; (f) independently backed-up/published source and remote CI succeed. Never report an untested tier as achieved.

## 3. Interruption and safety-control incidents

When a security control interrupts work, respect it. Never change the prompt or execution technique solely to get around that control, disable endpoint protection, edit policy, or silently resume the blocked operation. Preserving already authorized work is separate from reattempting a restricted action.

If permitted, capture **afterwards** an operator-readable incident report locally (not automatically in GitHub):

```text
Timestamp and timezone:
Product/model (if known):
Observed exact user-visible message (redacted as needed):
Request/correlation ID (if surfaced; otherwise "not available"):
Authorized task and scope:
Last VERIFIED checkpoint/snapshot:
Git remote, branch, HEAD, worktree status after interruption:
Tests previously completed and their SHA:
Pending or ambiguous operations:
Recovery actions taken and bytes/checksums verified:
What remained unavailable or untested:
Approved next step / operator decision:
```

Do not claim the incident report reveals internal security decision reasons; a local agent ordinarily cannot see them. For legitimate recurrent false positives, use official support channels with redacted factual evidence and do not supply secrets. Do not build automatic policy-detector or bypass mechanisms.

## 4. Publication and clean separation of evidence

- **Locally recovered** ≠ **independently reproduced** ≠ **pushed** ≠ **CI passed** ≠ **safe for production**.
- Remote authentication unavailable? Record that state; **do not** pretend to have published. Use only supported authorization flows (for example the user's GitHub browser/CLI login), never request tokens in chat.
- Before any push, inspect diff and commit tree, run secret checks and repository-required validation, and verify the intended remote and branch. Push only to a dedicated branch; use a draft PR when review is required. Never auto-merge a research branch or auto-close #317/#323.
- Retain original red CI attempts and the exact failing SHA. A later green run does not erase a previous failure, nor prove a bug's cause.
- Keep runtime safety and admission/quota/Job ownership semantics fail-closed. `Pending` / `TerminationUncertain`, simulated states, and short native samples are not successful termination receipts.
- Keep the canonical production status in `STATUS.md` / `ROADMAP.md`; this page is an **operational procedure**, not a competing status or handoff document.

## 5. Observed local recovery case, 24 October 2026 (historical, not live status)

The operator supplied a report of an interrupted Codex session on issue #317. According to that report, local recovery was consolidated under `E:\Optic-IA-Bridge\codex-recovery`; snapshot `0010-final-verified` was verified in a subsequent invocation; four files restored into an isolated directory had matching SHA-256 checksums. A local branch `codex/317-native-wait-phases` reportedly held **four commits**, with local HEAD `c3c5e6556cf6f2e5ab6d3ecfc8902a205bc259c6`.

The report says **six diagnostic tests + two existing runtime tests** passed, as did Rustfmt and strict Clippy. A deliberately rejected native observation retained **Win32 24 / HRESULT 0x80070018**. Root-wait pending, nonempty Job and root error 6 were **simulated** scenarios, not proofs of a full native guardian. No production guarantee follows from these results.

**Unverified at the time of that report:** reread from a wholly new Codex session, full system/disk failure recovery, Ubuntu tests, remote CI, GitHub publication, and whether the original interrupt can recur. In that incident GitHub was unavailable and no PR or merge was claimed. The historical branch/HEAD must not be taken as current. Continue only after verifying the real repository and recovery files. Windows V1 remains NO-GO until normal release gates are satisfied.

## 6. Agent quick decision rule

If a session or command disappears, **protect evidence before continuing**: inspect persistent recovery files and Git independently; verify snapshot checksums; preserve dirty work; separate observed/tested/simulated; resume only permitted tasks; publish a reviewed, validated branch when possible. If it cannot be checked, label it **unknown**, not successful.
