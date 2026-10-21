# ADR-0034: Shared Sensitive-Path Classification for Read Authorities

Status: **Accepted policy B; path-name floor verified through #312; full sensitive-content guarantee unresolved**.

## Native hard-link alias safeguard (bounded FileRead defense)

The canonical-name floor cannot distinguish two hard-link names for one physical
file. For prepared `fs_read`, a multi-linked opened file is conservatively
treated as **potentially sensitive**, even when the selected path looks ordinary.
The caller must hold the normal ReadContent lease plus explicit matching
ReadSensitive authority for the observed path. The link count comes from the
same opened native file, not an independently re-resolved filename. This is
intentional conservative denial for ordinary multi-linked files and does not
change FileSearch or GitRead contracts.

This safeguard cannot detect secrets copied into ordinary files, unrecognized
paths, staged Git blobs or all concurrent namespace/identity races. It does
not justify real-project remote access; #176 and #182 remain open. Native
Windows and Linux regressions are required for acceptance.

## Subsequent bounded filename-floor extension — #312 (2026-10-21)

The shared classifier now recognizes direnv `.envrc` and `.envrc.*`, including ASCII case variations. [PR #312](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/312) was squash-merged as `30295539fc6911f221cee6ac3c59785398c7a050`: accepted head `4e61a8f0aa0b65872d76ea9a2584b4ab79b89f33` passed [source CI #39571281997](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39571281997), and independent [main CI #39571875338](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39571875338) passed first attempt on exact squash SHA (Windows native, Ubuntu and dependencies). This only improves an existing deterministic lexical floor; it neither detects secrets inside innocently named files nor resolves #176/#182. No default permission or Git/process/runtime policy widened.

## Historical verified integration checkpoint through #310 — 2026-10-21

This section supersedes the pre-merge *candidate* descriptions below; those
remain as dated design history, not as present code status.

- [#308](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/308):
  shared core path classifier, FileRead dual-scope check and canonical fs_list
  page metadata gate.
- [#309](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/309):
  structured Git porcelain-v2 `-z` status paths, unknown/unrepresentable
  output refused, extra metadata-sensitive scope when names are classified.
- [#310](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/310):
  one hardened `--raw -z --patch` Git invocation; parse the raw file paths,
  require Git's verified extra NUL separator before patch text, refuse unknown
  or malformed records, and require typed FileRead with matching ReadContent
  and ReadSensitive lease for classified protected patch paths while session
  admission is held. Ordinary source diffs remain supported.
- Exact #310 squash commit `bd2915c3a9833c1da5a8fc5005445d1884260d4f`.
  Accepted source SHA `154d7ea6ab6b10aa967ab0a2f2034f2f06400fe9`
  passed [CI #39551993433](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993433)
  and [native preparation #39551993151](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993151)
  on first attempts. Independent [main CI #39552523257](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39552523257)
  passed first attempt on the exact squash commit (hosted native Windows,
  Linux, dependency policy). Earlier red CI attempts remain preserved.

**Remaining policy B limits:** this is a filename-based **floor**, not
content DLP or proof that arbitrary ordinary-looking code/config lacks
credentials. Git worktree/index snapshots are not atomic against concurrent
modification; branch-name metadata, alternate path identities, ancestor races
and true canonical target binding require further investigation. The real
workspace remote exposure and new-feature freeze are unchanged. Security
issues #176 and #182 stay open. #176 was inadvertently auto-completed by the
GitHub PR-body keyword parser at #310 merge, then explicitly reopened;
future PR prose must avoid auto-closing keywords while discussing open issues.



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

The conservative path-name floor also covers direnv `.envrc` and `.envrc.*` files,
which may export environment variables or load private credentials. This is a
deterministic name check, not proof of secret-free content in other files.


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

## Third bounded candidate — verify Git diffs without a second Git command

For #182, `git_diff` must not expose protected patch text under GitRead
alone. A separate `git diff --name-only` preflight is **not** an authority
boundary: working files/index can change between two Git calls. The scoped
implementation instead requests `--raw -z --patch` in the existing **single**
hardened Git invocation, retaining no-ext-diff/no-textconv/no-renames and
submodule exclusion. Before returning a patch, the runtime parses the raw
NUL-delimited changed-path prelude and classifies **each** path with the
shared conservative classifier. Unexpected headers, malformed native paths,
unknown statuses and unverifiable patch framing fail closed. The exposed
response remains the bounded Git patch, not the private raw prelude. Since
the raw prefix consumes the existing output-byte budget, some close-to-limit
diffs may now be rejected rather than truncated or exposing unverified data.

For **sensitive path patches**, MCP must additionally authorize a
`FileRead` typed effect against a matching, active `ReadContentScope`
**and** `ReadSensitiveScope` while still holding the session admission
guard; a GitRead capability alone never substitutes for file-content
authority. Directory-pathspec and repository-wide diffs are checked against
each emitted changed path, not merely the user-supplied filter.
Ordinary path-only diffs should remain usable with just GitRead.
Renames are disabled by the command; surprising rename/copy records, if
present, must validate both names rather than authorize only one.

This is a **candidate** pending the source SHA's Windows-native/Linux
Git-fixture CI and independent main CI. Git's own diff queue is trusted
to generate the raw-path metadata and patch in the *same invocation*;
it is not a claim of atomic worktree snapshots in the presence of
concurrent edits. Filename classification remains a minimum lexical
guard and does **not** prove normal-looking file contents contain no
secrets, nor does it resolve ancestor identity races, unsafe aliases
or remote real-project exposure (#176/#182). Do not close those issues
based on this candidate.

## Regressions / future acceptance

Exercise sensitive and ordinary canonical paths, directory ancestry,
case variants, prefix-mismatched leases, metadata-only sessions and explicit
sensitive scopes. Next Git-specific change must preserve subprocess hardening,
limits, admission/revocation and normal ordinary-source diffs when safely
verifiable. Never enable real-project remote access based on the lexical
classifier alone. Do not silently elevate `GitRead` into full sensitive trust.
