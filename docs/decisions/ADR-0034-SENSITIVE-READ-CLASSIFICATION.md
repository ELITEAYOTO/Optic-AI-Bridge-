# ADR-0034: Shared Sensitive-Path Classification for Read Authorities

Status: **Accepted security direction (operator, 2026-10-21); partial implementation in the first corrective increment**

## Decision

Choose **Policy B** rather than treating GitRead as inherently high trust:
filesystem content, listing metadata, and Git status/diff must consume a
shared deterministic sensitivity policy. Outputs whose paths or coverage
cannot be verified must **fail closed**, never silently broaden access.
Ordinary FileRead and metadata FileSearch do not imply sensitive-content or
sensitive-name access. Existing `ReadSensitiveScope` is additional authority,
not a substitute for `ReadContentScope`.

Classification applies to **canonical workspace paths**, comparing complete
components and ASCII case-insensitively. A first conservative lexical floor
covers .env variations, credential/secret/token names, common key formats,
private key identities, sensitive config directories and Git metadata.
No arbitrary application-config or source file is guaranteed free of secrets
by its name. Treat this classifier as a *minimum safeguard*, **not** complete
content inspection, provenance assessment, or permission to expose a real
project remotely.

## First bounded implementation

- Shared pure classifier lives in `optic-bridge-core::requires_sensitive_authority`,
  independent of OS, transport, LLM or Git subprocesses.
- `PolicyEngine` requires **both** `ReadContentScope` and matching
  `ReadSensitiveScope` for classified `Effect::FileRead` canonical paths.
  The default local `workspace_all_non_sensitive()` has no sensitive scope.
- `fs_list` checks canonical requested root and each returned entry before
  publishing a page. Protected metadata requires active additional
  `ReadSensitiveScope` and FileRead capability alongside ordinary search
  authority. The **whole page is denied**, not silently filtered/paginated
  past entries. This avoids incorrect cursor semantics but can deny otherwise
  ordinary pages containing a protected name. Error itself may reveal that
  the listing was blocked, not the name.
- Positive and negative unit and MCP tests must run on hosted Windows and
  Linux before accepting this increment.

## Not solved by this first increment

- GitRead still provides independent raw `git_status` and `git_diff`.
  It does **not** yet consume the new classifier. #182 stays OPEN and any
  sensitive-content-safe / real-project remote claim remains BLOCKED.
  A later **separate tested correction** must validate complete Git outputs:
  tracked/staged/untracked/renamed path records; whole-repo and path-scoped
  diffs; UTF-8/native path representation; unknown/replaced paths and
  concurrency. Unparseable or unverifiable outputs must be denied, not
  partially redacted with unreliable offset/pagination.
- A name rule can never prove that an innocently named source/config file
  does not contain a secret. Establish operator-selected protected paths,
  stronger provenance/content inspection and explicit conservative behavior
  for unknown files before remote real-workspace use (#176).
- Ancestor namespace races, directory handle pinning, alternate names and
  filesystem power-loss durability are separate unresolved risks.
- No new capability, transport, GUI, identity scheme, Git command, override
  channel, network grant or default sensitive permission is authorized.

## Second scoped increment — Git status porcelain verification (candidate)

The Git status path stream is a separate source from canonical filesystem
objects. Under Policy B it must be parsed from the actual `--porcelain=v2 -z`
result **before releasing any raw bytes**. Verify each changed, untracked,
unmerged, ignored or renamed/copy name (including *both* rename names)
using `WorkspacePath::from_native_relative` and the shared classifier.
Unknown records, absent terminators, unrepresentable native paths and
unexpected header types must deny the response, not be skipped.

A classified Git status name requires the same additional active
`ReadSensitiveScope` as `fs_list`, alongside explicit `FileRead` and
`GitRead` session authorization. No default grant is created. Admission
remains held across command execution and classification so revocation cannot
complete while verification is in flight. Ordinary status remains available
with only GitRead when no protected pathname appears.

**This is a candidate, not a merged guarantee.** Exact-head native Windows
and Linux CI, malformed-record tests and staged/untracked fixtures must pass.
`git_diff` is **still unrestricted by sensitive paths** and remains the
next independent security gate. Branch-name metadata, harmless-looking files
containing secrets, alternate filesystem identities and concurrent Git
repository changes require separate review. Real-workspace remote exposure
continues to be blocked.

## Regressions / future acceptance

Exercise sensitive and ordinary canonical paths, directory ancestry,
case variants, prefix-mismatched leases, metadata-only sessions and explicit
sensitive scopes. Next Git-specific change must preserve subprocess hardening,
limits, admission/revocation and normal ordinary-source diffs when safely
verifiable. Never enable real-project remote access based on the lexical
classifier alone. Do not silently elevate `GitRead` into full sensitive trust.
