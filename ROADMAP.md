# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md). The reusable-approval subtrack is recorded in [docs/product/A08_REUSABLE_APPROVAL_TRACK.md](docs/product/A08_REUSABLE_APPROVAL_TRACK.md). The active multi-host, zero-cost connectivity and session-autonomy research track is recorded in [docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md](docs/product/HOST_COMPATIBILITY_AND_AUTONOMY_TRACK.md) so the large canonical roadmap can be realigned surgically without rewriting unrelated history.

**Last reviewed:** 2026-10-10
**Current phase:** Phase 4 — same-repository parallelism, with H-01 multi-host capability characterization active in parallel
**Implementation state:** executable Rust pre-alpha; Phase 1 and Phase 2 are complete; Phase 3 multi-session/resource isolation is substantially closed through Phase 3E3; truthful network-denied containment, exact human approval foundations and ToolProfile authorization are merged; Phase 4 same-repository worktree/merge orchestration is active through Phase 4AP; A-08 reusable session/profile approval is implemented through A-08D2; the separate authority-free H-01 host probe is implemented in draft PR #170
**Release state:** no production-supported release
**Current gate:** H-01 — automated Streamable HTTP host-probe validation is green on the H-01 branch; real FranceStudent conversation/reconnect/header behavior must now be measured on the target PC under issue #171 before the H-03 `HostContext -> SessionResolver` contract is frozen. A-08E remains **blocked by host interaction**, not failed.

The A-08 reusable-approval work remains deliberately narrow and preserved:

- A-08A: **complete** — bounded reusable approval domain model tied to one application session, ToolProfile fingerprint, policy epoch and monotonic expiry;
- A-08B: **complete** — bounded broker lifecycle, active-session binding, owner-scoped revocation and physical cleanup after admitted work drains, without changing existing one-shot approval behavior;
- A-08C: **complete** — profiled process authorization can reuse an active grant only after full per-invocation policy/profile/resource/isolation revalidation; the reusable broker is shared with lifecycle cleanup and a mismatched grant falls back to human approval;
- A-08D: **complete** — Optic hardens inbound JSON-RPC before treating elicitation content as security-significant, exposes a bounded MCP form choice between `once` and `current_session`, defaults malformed/ambiguous accepted content to one-shot, revalidates session/lease/policy after the prompt, and only then issues either the exact one-shot grant or the exact server-resolved session/profile grant;
- A-08E: **blocked by host interaction** — direct/server-side reusable approval behavior is proven, but the tested host UX does not consistently surface the nested approval interaction required by this gate;
- A-08F: **moved behind H-03 evidence** — only a proven adapter-owned conversation/session mapping may justify user-facing wording such as “this conversation”.

This reusable approval never mints capability, file/network authority, process eligibility or broader resource ceilings, and it does not claim to suppress confirmation dialogs imposed independently by ChatGPT or another host.

## Host compatibility and autonomy direction

The new H-track keeps the existing runtime/security core and changes only the boundaries that need new evidence: host adapters, remote transport/authentication, host-context-to-Optic-session resolution and human-selected bounded autonomy.

Product invariants for this track are:

- Optic itself remains free to use and must not require a paid Optic backend, paid relay or API credits;
- free connectivity providers such as Tailscale and Cloudflare are acceptable but remain replaceable adapters rather than mandatory architecture dependencies;
- MCP is one host adapter, not an Optic core requirement;
- FranceStudent AI and ChatGPT are both V1 host targets, subject to the capabilities actually exposed by each host/plan/surface;
- connectivity never grants authority: tunnel URLs, authenticated connections and host-provided conversation ids are correlation/authentication inputs only;
- V1 trust/autonomy is scoped to the current Optic session/chat mapping and remains revocable locally;
- trusted development should support realistic `read -> edit -> build -> test -> inspect -> repeat` loops without repeated approval spam, while every action still passes deterministic policy/resource/isolation checks.

The existing Figma desktop-app concept is aligned with this direction: onboarding, provider-independent connections, FranceStudent, project permissions, sessions/activity, security, diagnostics, revocation and the floating companion already model the required product surfaces. The full app implementation remains later work after these service contracts stabilize.

## Current H-track gates

- **H-00 — complete:** product/architecture freeze recorded: zero-cost operation, multi-host adapters, session-scoped trust, provider replaceability, local-first authority and no production refactor before evidence.
- **H-01 — current:** harmless host capability probe with no filesystem/Git/process authority. Draft PR #170 has green automated unit, Windows-helper and real local Streamable HTTP server/client smokes. Real FranceStudent evidence is tracked in issue #171; ChatGPT is tested only on surfaces/plans that expose a compatible remote MCP path.
- **H-02 — pending, research prepared:** remote transport + authentication proof behind at least one zero-cost provider. Prefer standards-compatible `Authorization` header authentication when the host can carry it; never put access tokens in URI query strings. The exact FranceStudent credential/header behavior remains an H-01 measurement, not an assumption.
- **H-03:** bounded `HostContext -> SessionResolver -> Optic SessionHandle` contract and adversarial lifecycle tests, based on H-01 evidence rather than caller-supplied ids.
- **H-04:** session-scoped autonomy grants (`Review`, `Project Autonomy`, advanced high-authority mode) with independent per-action revalidation and immediate revoke.
- **H-05:** local Optic approval-broker contract independent of host MCP elicitation, suitable for the later desktop app/widget.
- **H-06:** production-bounded Streamable HTTP adapter beside, not instead of, stdio.
- **H-07:** FranceStudent V1 end-to-end path.
- **H-08:** ChatGPT adapter only on surfaces/plans that expose a compatible path; ChatGPT availability is not an Optic product dependency.
- **H-09:** provider adapters/setup automation for Tailscale, Cloudflare, custom HTTPS and future providers.
- **H-10:** full desktop application implementation using the existing Figma concept as the baseline.

A prior isolated FranceStudent test has already demonstrated the basic viability of MCP Streamable HTTP at `/mcp` using stateless JSON responses through a temporary Cloudflare Quick Tunnel. H-01 now upgrades that one-off proof into a reproducible, redacted characterization probe. This still does not claim production authentication, conversation correlation, reconnect semantics, autonomy or local approval until the corresponding gates pass.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end, and the selected isolated-Node Desktop profile has also been validated on a real machine. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have already been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes. `STATUS.md` and the large canonical scope file still contain older top-level focus text and must be surgically synchronized without truncating their historical content before this roadmap branch is merged.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
