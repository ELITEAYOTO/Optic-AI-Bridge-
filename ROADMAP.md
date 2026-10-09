# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-09
**Current phase:** Phase 4 — same-repository parallelism, with the approval/autonomy track continuing in parallel
**Implementation state:** executable Rust pre-alpha; Phase 1 and Phase 2 are complete; Phase 3 multi-session/resource isolation is substantially closed through Phase 3E3; truthful network-denied containment, exact human approval foundations and ToolProfile authorization are merged; Phase 4 same-repository worktree/merge orchestration is active through Phase 4AP; A-08 reusable-approval foundations are merged through A-08B3
**Release state:** no production-supported release
**Current gate:** A-08C — profiled process authorization may reuse an active session/profile approval only after the normal invocation, ToolProfile, policy, resource and isolation checks are revalidated for that invocation. A-08A merged as `06fcfc94`; A-08B1/B2/B3 merged as `d441ca20`, `1f1cde2a` and `b669ca76`. Existing one-shot approval semantics remain available and unchanged.

The A-08 reusable-approval track is deliberately narrow:

- A-08A: **complete** — bounded reusable approval domain model tied to one application session, ToolProfile fingerprint, policy epoch and monotonic expiry;
- A-08B: **complete** — bounded broker lifecycle, active-session binding, owner-scoped revocation and physical cleanup after admitted work drains, without changing existing one-shot approval behavior;
- A-08C: **current** — profiled process authorization can reuse an active grant only after full per-invocation policy/profile/resource/isolation revalidation;
- A-08D: MCP elicitation can offer one-shot vs session/profile approval when the host supports it, while unsupported hosts remain one-shot/fail-closed;
- A-08E: real ChatGPT Desktop proof with multiple allowed invocations plus negative profile/revoke/policy-change cases;
- A-08F: only after measuring the real host lifecycle may user-facing Desktop wording say “this conversation”; until then it must say “current Optic session”.

This reusable approval never mints capability, file/network authority, process eligibility or broader resource ceilings, and it does not claim to suppress confirmation dialogs imposed independently by ChatGPT or another MCP host.

The FranceStudent/remote-MCP target stays in Phase 5. The transport direction remains Streamable HTTP over a replaceable user-owned tunnel such as Tailscale or Cloudflare, but the tunnel is connectivity only. Remote principal/session mapping must reuse Optic-owned authorization and cannot trust a tunnel URL or caller-supplied conversation id as authority.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end, and the selected isolated-Node Desktop profile has also been validated on a real machine. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have already been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.