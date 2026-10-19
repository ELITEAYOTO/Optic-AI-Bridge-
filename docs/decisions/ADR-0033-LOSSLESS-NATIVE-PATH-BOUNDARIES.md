# ADR-0033 — Lossless native paths at Windows scope boundaries

Status: accepted implementation decision, 2026-10-20.

## Context

The native Windows filesystem can expose UTF-16 names that cannot be represented
by the UTF-8 `WorkspacePath` contract. Converting them with `to_string_lossy()`
before a scope comparison collapses distinct names to replacement characters.
A native regression created two temporary sibling roots differing only by an
unpaired surrogate and an intermediate directory alias. Before the fix, reading
through that alias returned the public fixture bytes from the other root.

This is an implementation defect in INV-010, not a new permission or a reason to
accept approximate path identities. The original test failure is retained in
the local validation evidence; the regression remains in the ordinary suite.

## Decision

- Runtime membership comparisons reject an unrepresentable root, prefix or
  suffix instead of replacing characters. The same checked helper protects
  native read/mutation and private-state placement checks.
- Git worktree lexical equality/containment checks also refuse unrepresentable
  names. Git path and hooks arguments retain their exact native units rather
  than constructing a command argument through a lossy display string.
- Operator configuration placement must prove that the observed policy file is
  outside the workspace. Unrepresentable paths therefore produce an error;
  treating a failed comparison as merely "outside" would invert this boundary.
- Ordinary valid Unicode spelling and existing component/case behavior remain
  supported. No unsafe code, new dependency or public MCP tool is introduced.

## Verification and limits

Tests cover the actual native read regression, distinct surrogate names,
invalid suffixes, valid Unicode, Git comparison/argument preservation and both
directions of the operator-policy boundary. All fixtures are newly generated
public temporary data; cleanup removes their exact known names.

This change does not establish complete Windows ordinal Unicode case semantics,
case-sensitive-directory support, prepared enumeration or trusted project
enrollment. Those remain separate evidence/contract work. Display-only formatting
is not automatically an authority boundary; audit callers before changing it.
