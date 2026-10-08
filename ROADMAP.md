# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-08
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2D3D, Phase 3A–3D, the selected C5G isolated-Node Desktop profile, Phase 3E1 aggregate declared-memory governance, Phase 3E2A Windows host-memory observation, Phase 3E2B emergency-headroom admission, Phase 3E3 bounded heavy-workload admission, B-02A/B-02B truthful network-denied containment, and A-07A–A-07D exact approval foundations are merged
**Release state:** no production-supported release
**Current gate:** B-01A — immutable application-owned ToolProfile foundation. A-07C/A-07D are merged: standard MCP elicitation is bounded and an accepted decision is revalidated under fresh session admission before an exact ApprovalBroker grant is issued and consumed one-shot; no new public capability was introduced. B-01A now defines a bounded profile over executable/class/workload/exact args/cwd/workspace-read set/env/network/resource ceiling/approval requirement plus an immutable bounded registry and `LeaseScope::ToolProfile`. This gate remains default-off and adds no MCP profile-selection or process authority; later B-01 gates must bind process admission/policy to exact profiles before broader autonomy.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
