# ADR-0036 — Git patch content requires explicit local operator trust (PROPOSED)

Date: 2026-11-01
Status: PROPOSED — requires owner review; no Windows V1 signoff.

## Problem
GitRead status and log expose bounded metadata, but `git_diff` returns repository content.
A Git index/blob preserves staged data even after the file or its original alias is
unlinked. On Windows 10, a synthetic ignored `.env` hard-linked as
`innocent-note.txt` was denied by `fs_read` while a staged Git diff exposed
the synthetic marker. The leak persisted after unlinking the alias. Filenames,
current file metadata, and separately executed `git diff --name-only` cannot
prove the provenance of historical or indexed Git object bytes.

## Proposed local-stdio contract
- GitRead with fixed executable/repository is enough for bounded `git_status`
  and `git_log`, subject to existing classified-name restrictions.
- `git_diff` remains registered but refuses patch bytes with
  `optic.policy_denied` by default, including ordinary-looking changes.
- At trusted host startup only, an explicit `--allow-git-diff-content` flag,
  paired with `--git-executable`, opts in to treating all Git diff bytes as
  high-trust content for the configured repository. The MCP client cannot
  activate that switch in a tool request; duplicate/unsupported flags fail.
- This switch grants **no** `FileRead` or `ReadSensitiveScope` authority and
  cannot make `fs_read` or `fs_list` reveal sensitive aliases. Existing
  filename-classified sensitive Git patches STILL require independent
  FileRead + ReadContent + ReadSensitive authorization.
- Opt-in and session GitRead authorization are checked before launching the
  diff command and checked again before publishing bytes, under the existing
  bounded request admission. This does **not** fix #317 revoke-to-spawn race.

## Tradeoff / compatibility
Without the opt-in, existing integrations that expect ordinary `git_diff`
patches will now receive a policy refusal. This is a deliberate fail-closed
security contract, **not** backwards compatible. The installer does not add
the flag automatically; operator-controlled installation profiles must
explicitly decide whether exposing Git patch bytes is acceptable.

An opt-in means patch bytes may contain secrets in apparently harmless paths;
it is not proof that Git or the repository history is safe. A future finer-grain
trust/provenance mechanism could replace this all-or-nothing local gate.

## Test evidence required
Windows and Ubuntu exact-head CI, real Windows 10 synthetic MCP cases with
live and unlinked-after-staging aliases, default refusal and explicit opt-in,
no FileRead expansion, independent filename-sensitive refusal, ordinary
diff, status/log and transport limits. Original RED must remain preserved.

## Explicit non-goals
No remote real-project exposure, no Git native process-owner changes, no
alteration of deadline or Git process hardening, no V1 release approval.
#182 retains unresolved metadata/provenance considerations and #317/#323 stay
open until their separate acceptance contracts are met.
