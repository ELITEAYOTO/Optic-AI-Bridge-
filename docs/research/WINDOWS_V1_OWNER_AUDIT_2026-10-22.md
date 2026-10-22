# Windows V1 owner and security audit — 22 October 2026

Baseline reverified: main c94461446a49ee61dc527a74f4e198afc8e55b4d,
PR318 merged, no open PR. Independent main CI39716573989 and native doctor
39716574009 succeeded first attempt. No operator-PC project command or write.
The user-supplied attachment was read only. No security settings changed.

## #317: production finding versus proposed native slice

hardened_command.rs runs a direct child, starts output threads and optionally an
input writer. The direct-child polling loop observes output overflow/deadline,
but child.wait and all three joins can outlive that loop. Earlier native fixtures
already demonstrated retained-pipe overruns; this baseline file is unchanged.
Do not spend another matrix reproducing an unchanged finding without a hypothesis.

Additional source concern: ? on try_wait/wait/read joins can return before sibling
I/O handles are joined. Fixing only their ordering cannot solve inherited-pipe
liveness, so no isolated join reorder is presented as an ownership correction.

The experimental native Rust slice reuses actual production OwnedJob and
CapturedOutputPipe, inside a cfg(test) child module. It is not a Python production
replacement and does not alter HardenedCommandRunner. The Python component only
supervises the Rust test process under an outer kill-on-close Job assigned before
a file gate permits fixture creation. Known synthetic sleepers stop at 8 seconds;
outer 40-second watchdog and 2-second termination confirmation apply.

Eight cases: normal/nonzero, descendant retaining stdout+stderr, process deadline,
explicit cancellation, shared captured-output ceiling, unassigned suspended root
recovery, injected missing-proof retention followed by real native confirmation.
The same native owner persists during missing proof. Every successful collected
case requires root exit, Job ActiveProcesses=0 and both output EOF. No output
reader thread exists; no join is omitted. Capture remains at most 64 KiB; polling
uses at most 4 KiB per stream per iteration. The deadline case uses 200 ms only
as a synthetic budget, with separate startup readiness and cleanup observation.
It is NOT a production end-to-end deadline contract.

Validation is pending until exact-source workflow results are observed. Native
proof archive records SHA/run/attempt and partial results on failure; no raw log,
PID, environment or private paths. Standard CI remains independent.
Do not call the Rust probe a production fix. Stdin, injected OS failures, native
crash proof, resource-reservation and session cleanup integration remain gates.
See proposed ADR-0035 for the least-risk migration sequence.

## V1 security matrix (priorities are engineering, not CVSS)

| Priority / boundary | Present guarantee and evidence | Missing proof / minimal next gate | Local versus remote |
|---|---|---|---|
| P0 #317 Git process ownership | Absolute structured args, env_clear, bounded combined capture, direct-child timeout; prior native pipe-overrun reproduction | Session/HostStartup owner, pre-spawn limits/reservations, tree+I/O proof, trusted revoke/stop, CAS outcome | Blocks robust local Git operation completion; remote adds authority/lifetime impact |
| P0 #176 FileRead/FileSearch | Policy B shared canonical-name floor; dual ordinary+sensitive scopes; canonical list/page gate; opened-handle hard-link guard (#308/#312/#314) and native CI | Ordinary names can contain arbitrary secrets; native ancestor/alias races and project content contract | Local operator-selected scope must remain explicit; real-project remote exposure blocked |
| P0 #182 GitRead composition | Verified porcelain-v2 NUL paths, same-command raw+patch parsing, sensitive typed FileRead checks (#309/#310/#318), no ext-diff/textconv | Concurrent index/worktree provenance, aliases, branch/ref metadata contract | GitRead is not blanket sensitive authority; remote real-project guarantee absent |
| P1 #319 MCP startup | Exact natural 30s failure retained; main doctor now has real-pipe bounded error diagnostics | In-flight fixed-label evidence on natural recurrence; no root cause established | Startup reliability remains open, successes do not eliminate intermittence |
| P1 #185 AppContainer/Node | Actual NETWORK_ATTEMPT + denied result required; no accepted connection and terminal state; native launcher markers | First failed launch/probe phase and cause, delivery failures and incomplete stdout are not success | Security test remains blocking; no timeout/ACL/network relaxation |
| P1 interruption/persistence | Version/CAS preconditions, bounded journal/recovery, prior corruption/open/retirement fixes and CI | Power-loss durability, multi-file partial cleanup, namespace/concurrent mutation proofs | Local data-integrity gate; do not claim ACID from short CI |
| P1 H02-H05 remote | Typed application sessions/leases, deterministic local policy/admission/expiry | Production remote principal/auth, broker approval provenance and cross-host binding | Remote stays unavailable; generated alpha dummy auth is not production |
| P2 platform/endurance | Hosted Windows native + existing Ubuntu regressions; bounded synthetic scenarios | Windows10/11 target/PS5.1 combinations, bounded soak and realistic lifecycle/fault acceptance | Hosted Windows Server does not certify all user PCs |

Evidence owners: STATUS, RELIABILITY_DEBT, SECURITY_INVARIANTS, ADR0034,
GitHub issues176/182/185/317/319 and linked exact-head/post-merge CI.
Issue bodies predate integrated patches; use latest canonical contracts/comments.
No claim that all source files were reviewed or all vulnerabilities eliminated.

## Repository security

Live main.protected=false; rulesets=[] at this audit. Detailed protection read
requires administration permission the connector lacks. No configuration changed.
Proposed owner-approved rule: prevent deletion and force-push; require PRs and
current Rust / windows-latest, Rust / ubuntu-latest, Dependency policy checks;
require branch up-to-date if operationally practical; require resolution of review
threads; require independent review where another trusted maintainer exists.
Do not force sole-maintainer self-approval or silently configure bypasses.
Decide the bot/app bypass policy explicitly before activation.

Inspected ci.yml and native workflows use hosted labels and contents:read;
checkout/upload Actions are full-SHA pinned. Native proof uses no secrets or
external project/network fixtures. An opt-in archive collector in the previous
doctor workflow has actions:read only to sample run state, not to promote results.
Checks protect only what they actually test, not artifact truth by themselves.
Audit of all workflows, dependency supply chain and secret access is incomplete.
Keep Cargo.lock and pinned compiler/OS proof, review action updates; never treat
a name classifier or successful dependency policy job as whole-repository security.

## Maintainability and decision

Core/policy/MCP/runtime boundaries remain useful; unsafe Win32 stays in the Windows
crate. The immediate debt is semantic ownership across Git/session/HostStartup,
not directory aesthetics. Public Copy HardenedCommandRunner cannot retain owners.
Shared native primitives should be reused instead of duplicating Job/pipe code.
Research scripts and long historical status checkpoints need clear scope labels;
no file, branch or evidence was deleted. Do not reorganize source globally.

No PR should claim #317 fixed from this native slice. Proposed next PR after the
native prerequisites: bounded service-owned Windows Git operation registry and
session stop/seal/reap gates, with a read-only migration only after proof. Keep
CAS journal migration separate. No new feature, Linux study or production fallback.

## Reprise / acceptance checklist

Reverify main, research head, PRs and Actions; read ADR0035 and this report.
Check exact native and standard CI attempts, not a preceding green SHA. Preserve
first red formatting/compile/native failures. Use archive VALIDATION.json and fixed
case lines; never raw logs as publication proof. No self-hosted/operator PC tests.
If this slice passes, record precisely which eight native cases passed and retain
all untested gates above. If it fails, diagnose only the first actual signature.
Do not re-run merely for a badge. Keep #317 and sensitive/security incidents open.

## First validation failures preserved

Source6bb52322365a10aae171518c879224141190c863: native workflow39718489770
stopped at formatting, so no native success is claimed. Normal CI39718489667
also found an unfulfilled clippy expectation on a synthetic deliberately Job-owned
child; remove that unnecessary expectation rather than suppress the lint. The
formatted source is produced only as diagnostic evidence and committed before
any acceptance run. Native normal-case assertions additionally verify both actual
synthetic output streams, not merely nonempty captured counts.
