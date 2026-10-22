# ADR-0036 — Git patch provenance and sensitive-read authority for Windows V1 (PROPOSAL)

Status: **PROPOSED — product/security decision NOT approved; no runtime change**.
Date of source review: 2026-10-22.
Reviewed main: `3fe3de0c258b2f8fc4328d6c405c8bf05cb22e3d`.
Related: [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182),
[#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176),
[#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317),
[Windows V1 freeze #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323),
[accepted ADR-0034](ADR-0034-SENSITIVE-READ-CLASSIFICATION.md).

This proposal refines **accepted Policy B**. It does not silently convert
`GitRead` into sensitive-read permission or override ADR-0034. No rollout
decision is authorized by the existence of this document.

### Stronger index-versus-worktree proof — accepted exact SHA b4868b39

[Independent hosted Windows CI #39865183807](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39865183807)
completed **success, attempt 1** on tested source
`b4868b39fcee80834d5e3490154a252dc6f8962d`.
The same opt-in synthetic test reported **1 passed, 0 failed** and its
evidence includes
`index_exposed_after_alias_replaced=true` and
`worktree_diff_exposed_old_index_after_alias_replaced=true`.

The test **stages** a fake marker through an ordinary-named hardlink to a
sensitive file, then **removes the hardlink and writes a harmless,
single-linked replacement** at the ordinary path. The true MCP `fs_read`
handler can then read the replacement under ordinary FileRead scopes,
while **both staged GitDiff and unstaged GitDiff still return the fake
marker from Git's retained index object**. This establishes that even
perfect observation of the CURRENT worktree file identity does not
authorize the bytes emitted from a Git index snapshot. An `lstat`,
open-file link-count probe or pathname classifier before Git is thus
insufficient as a general Git patch provenance guard.

First extension run
[#39865005308](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39865005308)
failed one Rustfmt layout hunk on source `a1501dff`, before
the test. The exact correction was committed at a new SHA `b4868b39`.
The earlier [successful basic test #39864401186](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39864401186)
remains separate valid evidence. **Research pass means reproduction succeeded,
NOT that confidentiality was fixed.** An approved future correction must
reverse these expectations to require denial in restricted scope.
ADR status remains PROPOSED.

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

## Confirmed source behavior

- `ReadonlyMcpServer::git_diff` first evaluates typed `Effect::GitRead`,
  then checks `Effect::FileRead` only for paths listed in
  `GitDiffSnapshot::sensitive_paths`.
- `GitReadService::diff` uses one hardened `git diff --raw -z --patch`
  invocation and parses the raw changed-path prelude before exposing its
  bounded patch. The parser checks **reported Git names**, not all physical
  or historical sources of the patch.
- `BoundedFileSystem::prepare_read` holds an opened native file handle and
  observes its physical link count. A multi-linked file requires additional
  sensitive-read authority, even under an ordinary alias. This property
  cannot be transferred to Git blobs by a separate pathname check.
- Independent Windows 10 synthetic MCP reproduction in
  [#182 evidence](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182#issuecomment-6350690757)
  showed a plain `settings.txt` hardlink to `.env` denied by `fs_read`
  but exposed in worktree and staged `git_diff` through the ordinary path.
  The test used a fake marker, not credentials. The demonstration does not
  prove an external filesystem escape or a real-secret incident.
- Git patches can contain *different source objects*: worktree, index,
  HEAD/older objects and transformations. The current physical file's name,
  metadata or link count cannot attest to all emitted bytes, particularly
  after staging or concurrent replacement.

## Proposed V1 security decision — owner approval required

**Prefer conservative fail-closed availability of Git patch content** for the
*restricted, local V1 profile*. Until each patch's relevant source objects,
authority and snapshot/provenance can be justified, deny `git_diff`
content rather than release unaudited bytes. Treat this as a deliberate
product-scope restriction, not a disguised network/sandbox permission change.

This is the safest short-term direction but **has a real compatibility cost**:
ordinary GitRead-only users currently receive ordinary source diffs under
ADR-0034. Disabling those responses for a restricted profile changes observable
behavior; do not make this change in production without explicit operator
approval, a reviewed scope definition and a proper runtime guard.

No new `ReadSensitive` grant may be inserted automatically. Requiring
`FileRead + ReadContent + ReadSensitive` for an arbitrary diff would be a
conservative *authorization* decision but still not proof of provenance or
absence of copied secrets; positive permissions must remain deliberate and
scoped to evidence. A read-only model must not mint its own privileges.

### Alternatives considered

| Option | Benefit | Problem / condition |
| --- | --- | --- |
| A — Treat GitRead as explicit full-repo diff visibility | Existing behavior can remain available with informed operator choice | Broadens the meaning of trust if silently inferred; FileRead denials would not cover Git. Requires a distinct high-trust operator grant, accurate UI/profile labeling and security decision; **not approved**. |
| B — Restricted-profile fail-closed patch response | Prevents current synthetic bypass at response release without guessing provenance | May make `git_diff` unavailable for some/all local V1 grants. Requires explicit approved restriction and tests that prove the rejection is enforced in the actual runtime and all routing paths. |
| C — Object-bound read/provenance authority | Could preserve ordinary diffs with more precise checks | Needs verified HEAD/index/working tree sources, object identity and transaction consistency, native aliases, filters and concurrency. It still cannot be general content-DLP. Substantial separate architecture work. |

**Recommendation:** B first for an explicitly defined restricted V1 profile,
then reconsider C if safe ordinary Git diff access is a real product
requirement. Do not implement A by default.

## Non-negotiable design constraints for an approved follow-up

1. The decision is made **inside the admitted operation** before returning
   any patch bytes, without a client/model-configured bypass. A denied result
   returns no partial base64 patch, including on whole-tree/staged paths.
2. Validate actual profiles, capabilities, leases, canonical root and the
   exact Git execution context. No implicit FileRead or ReadSensitive grant.
3. A pathname-only `lstat`, hardlink-count check before/after Git, more
   filename patterns, or a second `git diff --name-only` preflight **cannot**
   justify patch provenance. These fail under races, index blobs and history.
4. Preserve one-invocation raw framing validation, no ext diff/textconv,
   disabled submodule traversal, hardened child environment, output limits,
   action deadlines and actual Job/process ownership gates from #317.
5. Scope `git_status` and `git_log` independently. Status includes
   file names and branch.head/upstream metadata, which are **not filesystem
   paths**; commit IDs and parent IDs have a different disclosure model.
   This proposal does not label them secret-safe by default.
6. No external/remote real-project use, broad Git authority, V1 tag,
   release, or ADR acceptance while these gates are unresolved.

## Evidence required before approving/promoting a corrective PR

See the [test matrix](../research/ISSUE182_GIT_PATCH_PROVENANCE_ACCEPTANCE_2026-10-22.md).
Exact-source Windows-native and Ubuntu CI + dependency-policy checks, an
independent run on integrated main after reviewed merge, and negative MCP
exposure tests are mandatory. The present PR #327 is an independent #317
research effort with a red #185 CI; do not combine them or force a rerun to
green. Keep one open PR at a time and preserve the initial #185 failure.

## Decision record

- Proposed preference: restricted-profile fail-closed Git patch handling.
- Operator product/security approval: **PENDING**.
- Runtime implementation: **NONE in this proposal**.
- Verified positive ordinary-diff provenance: **NOT ESTABLISHED**.
- #182 and #176: **OPEN**. Windows V1 #323: **NO-GO**.
