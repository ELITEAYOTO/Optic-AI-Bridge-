# #182 — Git patch provenance acceptance matrix (synthetic only)

Status: **RESEARCH TEST CONTRACT, NOT AN IMPLEMENTED FIX**.
Date: 2026-10-22.
Target baseline: `main=3fe3de0c258b2f8fc4328d6c405c8bf05cb22e3d`.
The current repo has an independent ongoing PR #327 for #317; this document
must not be represented as a PR merging a GitRead security-policy change.

## Independent hosted Windows observation (2026-10-22)

Test source `b1e002b3bcc594c8a81c173c42909bfe34271929`, independent
[hosted Windows CI #39864401186](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39864401186)
completed **success, attempt 1** on that exact SHA. The specifically filtered,
opt-in Windows test
`git_tools::tests::research_git_diff_hardlink_alias_disclosure_is_reproducible`
reported **1 passed, 0 failed, 54 filtered out** and emitted:

```text
OPTIC182_RESEARCH {"synthetic_only":true,"file_read_hardlink_denied":true,"git_worktree_alias_exposed":true,"git_index_alias_exposed":true,"git_read_without_file_read_exposed":true,"production_security_fixed":false}
```

The test exercised actual Rust MCP tool handlers and the real Git executable
within a synthetic repository, not a full stdio JSON-RPC transport. Its fixture
used a fake marker only. It **asserts the currently known exposure**; it is
not a confidentiality regression test that can be promoted unchanged.
`git_tools.rs` only registers it as `#[cfg(windows)]`, `#[ignore]` and
the research workflow opts in explicitly. No production grant/policy has been
changed. This hosted Windows environment is not Windows 10/11 client
certification.

First negative CI attempts preserved rather than retried to green:
- [#39863953660](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39863953660)
  source `e782c991`, failed Rustfmt (four exact hunks, before executing test).
- [#39864133466](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39864133466)
  source `2ae196af`, failed compilation because three `Result::expect_err`
  invocations required `Debug` on `rmcp::Json`. Corrected using
  `.err().expect()` without changing test expectations.

The accepted runner passed Rustfmt, Clippy and the named test. This
**strengthens the evidence for the trust-contract gap** without fixing it,
authorizing the proposed restriction, or resolving the separate #317/#185
release gates.

## Known issue reproduction and evidence

Source: synthetic Windows 10 MCP test documented in
[issue #182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182#issuecomment-6350690757).
Codex attached a ZIP with sanitized records of nine passing assertions and
external supervisor Job cleanup. These are user-machine-reported tests, not
independently rerun by this drafting pass.

Fixture concept:

1. Disposable Git repository under test-specific temporary root.
2. Commit ordinary `settings.txt`; write untracked `.env` with one
   conspicuous **fake** marker.
3. Replace ordinary path with a hardlink to `.env` and verify file identity
   and link count from OS-opened handles.
4. Session without `ReadSensitive`: `fs_read(.env)` and
   `fs_read(settings.txt)` denied; synthetic `git_diff` presently exposes
   marker under ordinary name.
5. Stage alias; compare indexed `git_diff` after alias was removed/replaced.
   Grant `GitRead` but explicitly deny FileRead as a separate case.

Neither the fake marker nor the test fixture must be a real secret, private
repository, external path or online service. No user-PC action from an agent
without a new and explicit authorization; prefer GitHub-hosted Windows runner.

## Release-blocking response-level expectations (after policy approved)

| Case | Negative/positive expected contract |
| --- | --- |
| Worktree patch for ordinary alias of sensitive hardlinked data | Restricted V1 must deny complete patch unless there is *verified* source authority; deny cannot leak marker or names from unapproved sources. |
| Staged alias pointing to formerly sensitive data, then alias removed/replaced | Deny when provenance/authority cannot be proved; current physical link count is not evidence. |
| Whole repository patch mixing ordinary and protected worktree files | No partial redaction or partial patch from a rejected request; deny full response. |
| Staged new/deleted file, cached index vs HEAD predecessor | Both old and new content sources require coverage; reject unverifiable one. |
| Renamed/copied path, native-unrepresentable name, unknown raw status | Reject unverifiable framing/identity; never skip unknown record. |
| Binary Git changes, filters/attributes, local Git configuration | Establish explicit vetted behavior; do not infer safety from disabled external diff/textconv alone. |
| Source file with ordinary name but synthetic sensitive content | Name-only classification cannot certify content; deny if policy requires source proof. |
| Explicit FileRead denial but GitRead allowance | Restricted-profile patch cannot silently bypass the FileRead restriction. |
| Expiring/revoked scope during Git call | Keep actual admission/authorization through response release; no partial output after denied/revoked admission. |
| Scope-limited ReadContent/ReadSensitive on different path | Must not grant access to unrelated patch sources; no implicit elevation. |
| Ordinary committed source diff | If restricted mode deliberately denies it, test should state and document the compatibility break; if allowed, prove the object/authority basis first. |
| Status names and branch metadata; log IDs/time | Specify independently what metadata GitRead discloses; do not silently assume content-denial also sanitizes status/log. |

## Test execution contract

- Use synthetic disposable Git repo; actual installed pinned Git executable
  and current real MCP server end-to-end, not just unit mocks.
- Use native Windows CI for hardlinks, reparse points and Job cleanup; Ubuntu
  as regression. A supervising Job/timeout must recover synthetic descendants.
- Pin exact SHA, machine/build, Git/toolchain versions, permissions, grant
  profile and all request/response statuses; preserve first failing runs.
- Use fake marker bytes and no real project/secret; upload only sanitized
  outputs with checksums and raw log redaction.
- Require CI for formatting, Clippy, Rust tests, security assertions, MCP
  and Node smoke; no weakening of existing network or process denial checks.
- Review production diff for all tool paths, including staged/unscoped Git,
  native aliases and lost task-lease authority. Conduct independent main CI
  after any reviewed merge.

## Open engineering questions for the follow-up

1. What explicit product profile is the *restricted local V1* and where is
   that scope currently enforced? Does it actually include `git_diff`?
2. Which exact source object(s) does each Git patch hunk represent? How are
   worktree bytes, index blobs and HEAD blobs securely related to active grants?
3. Is a bounded negative-only Git-diff availability gate sufficient for V1,
   or is a new operator-approved authority model required to keep positive
   ordinary GitRead functionality?
4. Can repository-owned attributes/configuration influence Git patch output
   even when external diff and textconv flags are disabled?
5. How are sensitivity leaks in stderr, status branch headers and returned
   error diagnostics separately prevented?
6. Can the implementation prove denial on cancellation after subprocess
   completion but before response serialization?

**Hard stop:** No automatic policy acceptance, GitRead default grant changes,
new network access, bulk research merge, PR #327 alteration, or V1 tag.
