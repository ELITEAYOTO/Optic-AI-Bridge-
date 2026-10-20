# Read boundary hardening — 2026-10-12

Historical snapshot: the candidate/branch status below describes 2026-10-12,
not the current repository. See [current status](../../STATUS.md),
[the debt register](../engineering/RELIABILITY_DEBT.md) and preservation history
before classifying any finding as still open or already integrated. This note
does not assert that all findings in this historical audit are closed.

Status: implementation candidate on `security/read-boundary-hardening`; unmerged.
Base: main `156653a406f01383e2fc14cada513963453b04f3`.
Owners: #183, #181 and the pagination subfinding of #176.

## Implemented candidate

File read, directory list and Git read share a bounded blocking-worker boundary.
The worker acquires session admission before authority evaluation and holds it
through observation, authorization and the actual effect, using its admission
grant rather than a detached active-session snapshot.

The worker also owns transport and blocking-I/O capacity. Cancelling/timing out
its async waiter cannot release those guards while the effect continues. Revoke
marks the session inactive, denies new work and waits for admitted work to drain.
A timeout is not a promise that an OS read or Git process stopped. Existing Git
runtime deadlines and filesystem byte/scan ceilings remain. Queued workers check
the transport deadline and session state before project work. This restores the
existing quiescence contract without new capability, lease or remote authority;
`session_info` remains a metadata-only snapshot.

`WorkspacePath::from_native_relative` separates observed OS paths from caller
input. Each normal UTF-8 native segment must retain its logical identity, within
the 4096-byte path bound. Unix literal backslashes cannot become separators.
Canonical prepared reads, existing/absent mutation observations, list entry names
and resolved list directories use that conversion. Caller-input parsing still
accepts Windows-style separators. This is representation safety, not a new
filesystem race-containment guarantee.

When response fitting removes every entry of a nonempty listing page, `fs_list`
fails with `optic.response_too_large` instead of repeating the input cursor.
Fitting includes the final cursor's size. A successful truncated page retains at
least one entry and advances; actually empty directories/pages remain valid.

## Verification

- Local native Windows: full `cargo test --locked --workspace --all-features`,
  full-workspace/all-targets/all-features Clippy with warnings denied, and rustfmt.
- Deterministic suspended read/list workers cover revoke after caller cancel,
  timeout, success and error; capacity remains reserved until completion and
  subsequent work fails inactive. A suspended actual Git runtime operation proves
  revoke still waits after its caller is cancelled.
- Listing tests cover zero-progress rejection, empty-page validity and final
  cursor size. Native conversion tests cover normal paths and length limits.
- Unix-only tests cover literal-backslash/non-UTF-8 segments and alias-to-literal
  read/list/existing-and-absent mutation targets. These require exact-head Ubuntu
  CI; a Windows run does not prove Linux behavior. Record CI in the PR.

## Deferred register — no guessed implementation

| Point | Missing contract / next step | Owner |
| --- | --- | --- |
| Canonical list authorization | Prepared enumeration must bind authorization to the same target across aliases/reparse/replacement races. Settle a handle/iterator-backed cross-platform boundary and test prefix aliases. Current lexical search authorization remains open. | #176 |
| Sensitive content/metadata | Decide protected-path rules, operator overrides, listing visibility and mutation semantics before enforcement. | #176 |
| Git visibility | Choose higher-trust GitRead, shared classification or split capability semantics; test tracked/staged/untracked data. | #182 |
| Git-control mutation namespace | Complete canonical .git directory/indirection/worktree/case/alias rules, with transactional Windows fixtures. A lexical ban is insufficient; ordinary .gitignore stays ordinary. | #180 |
| Host authentication/correlation | Dummy bearer/header wire proof and fresh h01-v2 new-chat/reconnect/overlap captures are missing. | #171/#172 |
| Session/autonomy/local approvals | Principal-bound mapping, transactional subordinate leases, idle/absolute/renewal and retry/resume contracts remain pending. Preserve explicit Optic-handle fallback. | #173–#175 |
| Real Link error/overlap proof | Experiment implementation is stacked on #178; failed-call schema and exact error mapping require separate experiment review. Keep synthetic-only. | #177/#178 |

No draft is merged by this tranche. Applicable unresolved gates continue to block
real-project remote exposure even if these corrections pass CI.
