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
