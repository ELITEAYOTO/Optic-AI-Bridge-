# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md). The active reusable-approval subtrack is additionally recorded in [docs/product/A08_REUSABLE_APPROVAL_TRACK.md](docs/product/A08_REUSABLE_APPROVAL_TRACK.md) until the large canonical roadmap can be surgically realigned without rewriting unrelated history.

**Last reviewed:** 2026-10-09
**Current phase:** Phase 4 — same-repository parallelism, with the approval/autonomy track continuing in parallel
**Implementation state:** executable Rust pre-alpha; Phase 1 and Phase 2 are complete; Phase 3 multi-session/resource isolation is substantially closed through Phase 3E3; truthful network-denied containment, exact human approval foundations and ToolProfile authorization are merged; Phase 4 same-repository worktree/merge orchestration is active through Phase 4AP; A-08 reusable session/profile approval is implemented through A-08D2
**Release state:** no production-supported release
**Current gate:** A-08E — real ChatGPT Desktop proof that one explicit `current_session` approval covers multiple distinct invocations of the same allowed ToolProfile while a different profile, changed policy epoch, revoked session and out-of-profile invocation still require approval or fail closed. A-08A merged as `06fcfc94`; A-08B1/B2/B3 as `d441ca20`, `1f1cde2a` and `b669ca76`; A-08C1/C2 as `e3d3ff53` and `0034d54d`; A-08D0/D1/D2 as `7a0c5cb6`, `7f4b592a` and `d8c8a545`. Exact D2 head `30d4d82c` passed Ubuntu, Windows, dependency policy, AppContainer, isolated Node, profiled approval, Git MCP, installer-profile and ChatGPT Desktop artifact round-trip gates before merge.

The A-08 reusable-approval track is deliberately narrow:

- A-08A: **complete** — bounded reusable approval domain model tied to one application session, ToolProfile fingerprint, policy epoch and monotonic expiry;
- A-08B: **complete** — bounded broker lifecycle, active-session binding, owner-scoped revocation and physical cleanup after admitted work drains, without changing existing one-shot approval behavior;
- A-08C: **complete** — profiled process authorization can reuse an active grant only after full per-invocation policy/profile/resource/isolation revalidation; the reusable broker is shared with lifecycle cleanup and a mismatched grant falls back to human approval;
- A-08D: **complete** — Optic hardens inbound JSON-RPC before treating elicitation content as security-significant, exposes a bounded MCP form choice between `once` and `current_session`, defaults malformed/ambiguous accepted content to one-shot, revalidates session/lease/policy after the prompt, and only then issues either the exact one-shot grant or the exact server-resolved session/profile grant;
- A-08E: **current** — real ChatGPT Desktop proof with multiple allowed invocations plus negative profile/revoke/policy-change/out-of-profile cases;
- A-08F: only after measuring the real host lifecycle may user-facing Desktop wording say “this conversation”; until then it must say “current Optic session”.

This reusable approval never mints capability, file/network authority, process eligibility or broader resource ceilings, and it does not claim to suppress confirmation dialogs imposed independently by ChatGPT or another MCP host.

The FranceStudent/remote-MCP target stays in Phase 5. The transport direction remains Streamable HTTP over a replaceable user-owned tunnel such as Tailscale or Cloudflare, but the tunnel is connectivity only. Remote principal/session mapping must reuse Optic-owned authorization and cannot trust a tunnel URL or caller-supplied conversation id as authority.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end, and the selected isolated-Node Desktop profile has also been validated on a real machine. These are integration/security validations, not production-readiness claims. A-08E requires a new real Desktop proof specifically for reusable approval semantics and is not closed by those earlier smokes.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have already been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes. `STATUS.md` and the large canonical scope file still contain older top-level focus text and must be surgically synchronized without truncating their historical content.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
