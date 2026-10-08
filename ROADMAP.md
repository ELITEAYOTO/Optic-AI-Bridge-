# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-08
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2D3D, Phase 3A–3D, the selected C5G isolated-Node Desktop profile, Phase 3E1 aggregate declared-memory governance, Phase 3E2A Windows host-memory observation, Phase 3E2B emergency-headroom admission, Phase 3E3 bounded heavy-workload admission, B-02A direct-`FixedTool` network characterization, and B-02B truthful network-denied `FixedTool` AppContainer containment are merged
**Release state:** no production-supported release
**Current gate:** A-07A ApprovalBroker foundation — add an opaque application-owned approval id plus a bounded runtime broker that issues and atomically consumes one exact grant bound to `ActionId`, session, normalized effect, exact resource budget, policy epoch and monotonic expiry. Existing `PolicyEngine::evaluate()` remains completely unchanged and fail-closed; A-07B will wire broker consumption only after provenance/lifecycle handling is explicit, and no MCP caller can mint or widen a grant in this gate. A later A-07B gate will wire lifecycle cleanup and the concrete human-approval transport/UX before any approval-required effect is exposed.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
