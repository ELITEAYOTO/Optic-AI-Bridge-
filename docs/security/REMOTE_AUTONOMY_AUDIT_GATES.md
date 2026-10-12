# Remote/autonomy security audit gates

**Status:** active pre-production security gate  
**Last reviewed:** 2026-10-12

This document consolidates the security findings that must be resolved before Optic exposes a real developer workspace through a remote host or claims bounded `Project Autonomy` / advanced high-authority session behavior.

It does not replace the owning implementation issues. It records the cross-cutting security contract so future transport/session/UI work cannot accidentally bypass a gate by treating it as a narrow filesystem or Git detail.

## Principle

Remote transport does not create new authority semantics. Every real-project remote effect must remain inside the same application-owned session, capability, lease, policy, resource and isolation model as local stdio.

A remote adapter must not widen scope merely because a host can reach it.

## Gate A — sensitive content and metadata classification — issue #176

The current authority model contains `ReadSensitiveScope`, but a scope type is not by itself a sensitive-path classifier.

Before remote real-project reads are supported, Optic needs an explicit, testable classification contract for at least:

- secret/environment files;
- credential/token/key material;
- operator-selected additional protected paths;
- content visibility;
- directory/listing metadata visibility;
- mutation semantics for protected content;
- canonical-path evaluation so aliases cannot bypass classification.

A policy such as “all workspace reads are non-sensitive unless explicitly marked otherwise” is not acceptable as proof that secrets are protected.

`fs_list` must be treated as a metadata disclosure surface, not only a convenience API.

## Gate B — canonical list authorization and pagination

Current read path authorization benefits from canonical target observation before policy evaluation. Directory listing needs the same discipline when scoped authority matters.

Before prefix-scoped remote search/list is relied upon:

- resolve the directory target canonically before the authorization decision;
- bind the effect to that canonical logical workspace path;
- reject aliases that resolve outside allowed scope;
- ensure response-size fitting cannot return a non-progressing cursor;
- preserve deterministic bounded scan/page limits.

A caller must not be able to exploit a lexical alias or response truncation artifact to escape scope or create an infinite page loop.

## Gate C — repository-control metadata is not generic project content — issue #180

Broad `FileWrite` / `FileDelete` authority must not silently imply repository-control authority.

The generic mutation namespace should deny protected Git control state such as:

- `.git/HEAD`;
- refs;
- repository config;
- hooks/control metadata;
- worktree `.git` indirection files;
- other Git-owned state where generic file mutation would alter repository control semantics.

Ordinary project files such as `.gitignore` remain normal workspace content.

The preferred rule is:

> Generic file mutation cannot modify repository-control state by default; repository-control effects use the dedicated Git capability/runtime.

The denial must occur after canonical target observation so symlink/alias tricks cannot bypass it.

## Gate D — native path -> WorkspacePath must round-trip exactly — issue #181

On Unix, `\` is a legal literal filename character. `WorkspacePath` accepts Windows-style backslashes as separators and normalizes them.

Therefore any conversion from a native Unix path segment into an authority-bearing `WorkspacePath` must prove lossless round-trip representation.

Required rule:

- validate each native segment before joining/parsing;
- reject any segment whose logical representation changes its identity;
- reject rather than normalize an unrepresentable native name;
- centralize the conversion helper where practical;
- test prepared reads, listing and canonical mutation observation.

This is a Linux/macOS real-project gate. The current Windows-first synthetic FranceStudent experiment is unaffected.

## Gate E — GitRead must not bypass sensitive-content policy — issue #182

Git output is a separate authority surface from the generic filesystem service.

Current examples:

- `git_status --untracked-files=all` can reveal protected path names;
- `git_diff` can reveal contents of tracked/staged protected files;
- `git_log` is primarily bounded commit metadata and has a different sensitivity profile.

Before combining `GitRead` with a policy that denies sensitive project reads, choose one explicit model:

1. `GitRead` is documented as a higher-trust capability that includes repository diff/status visibility; or
2. Git tools consume the same sensitive-path classifier; or
3. Git metadata and Git content are split into narrower capabilities.

Whatever model is chosen must be visible in profiles/UI and regression-tested against tracked, staged and untracked protected files.

## Gate F — session revoke must wait for admitted read/list/Git effects — issue #183

The lifecycle manager already has an admission/quiescence concept for sensitive effects. `fs_read`, `fs_list` and Git-read operations currently use active-session snapshots without consistently holding a `SessionAdmissionPermit` through the entire effect.

For local revoke to truthfully mean “this remote session is stopped”, project effects must:

- enter session admission before authority evaluation;
- hold the permit through canonical observation, policy/lease authorization and the bounded effect;
- release the permit only when the effect has reached a known result;
- let `revoke()` wait for admitted work to drain;
- guarantee permit cleanup on timeout/error paths.

This is especially important for H-03/H-05 because the user must be able to revoke a remote session locally even while the host remains connected.

## Gate G — session identity is never derived directly from transport

MCP `2026-07-28` is stateless at protocol level. No current FranceStudent signal has been proven to be a trustworthy conversation identifier.

Therefore:

- no `Mcp-Session-Id` / request id / Cloudflare header / tunnel URL is authority;
- host metadata may only be bounded correlation input;
- H-03 must support an Optic-minted explicit application handle when no reliable host conversation signal exists;
- the real authority remains the independent Optic-owned `SessionHandle` and its application-owned leases/grants.

## Gate H — remote cancellation and `session_cancel` are control-plane operations

The current stdio server binds one application session at startup. `session_cancel` is therefore a powerful local session-control operation.

A shared remote adapter must not expose it as a generic ordinary tool to arbitrary host calls.

H-03/H-05 must define explicit ownership and local-control semantics for remote revoke/cancel before any multi-session remote surface includes equivalent behavior.

The synthetic PR #178 adapter deliberately excludes `session_cancel`.

## Gate I — authentication is separate from project/session authority

H-02 authentication must prove:

- missing credential rejected;
- wrong credential rejected;
- valid credential accepted only as a connection principal;
- revoked credential fails immediately;
- rotation does not resurrect old credentials;
- credentials never appear in URLs, normal logs or tool outputs;
- provider/tunnel account identity alone is insufficient.

Authentication does not mint capabilities, leases, project selection or autonomy grants.

## Gate J — autonomy never disables per-effect safety

Future `Review`, `Project Autonomy` or advanced high-authority modes may suppress repeated human prompts, but they do not bypass:

- session lifecycle;
- task-lease ownership/scope;
- policy epoch;
- ToolProfile/executable identity;
- filesystem/Git canonicalization;
- expected-state mutation preconditions;
- resource ceilings;
- network rules;
- Windows isolation;
- audit/revocation.

The AI cannot mint, widen or extend its own autonomy grant.

## Relationship to current experiments

The real Optic MCP link in PR #178 is intentionally safe relative to these open gates because it:

- creates a disposable synthetic workspace itself;
- accepts no caller-selected real workspace root;
- exposes only session info, bounded list and bounded read;
- provisions zero process executables;
- provisions zero mutation scopes;
- disables Git;
- excludes `session_cancel`;
- uses a random per-call adapter invocation proof.

Real project exposure remains blocked until the relevant gates above are resolved and tested.

## Definition of done

This audit gate is considered satisfied for remote real-project use only when the owning issues are either fixed with regression tests or closed by an explicit documented architecture decision that preserves equivalent containment.

A green transport/tunnel demo is never sufficient by itself.