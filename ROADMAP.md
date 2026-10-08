# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-08
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2D3D, Phase 3A–3D, the selected C5G isolated-Node Desktop profile, Phase 3E1 aggregate declared-memory governance, Phase 3E2A Windows host-memory observation, Phase 3E2B emergency-headroom admission, Phase 3E3 bounded heavy-workload admission, B-02A direct-`FixedTool` network characterization, B-02B truthful network-denied `FixedTool` AppContainer containment, A-07A bounded exact ApprovalBroker foundation, and A-07B lifecycle-owned approval cleanup are merged
**Release state:** no production-supported release
**Current gate:** A-07C trusted approval elicitation transport — enable rmcp's standard form elicitation explicitly, require the connected client to advertise form-elicitation support, bound each request with a timeout, and preserve `accept`, `decline`, `cancel` and unsupported-client outcomes distinctly. The transport itself mints no ApprovalGrant and existing `PolicyEngine::evaluate()` / public MCP authority remain unchanged and fail-closed. The next A-07 gate will bind accepted elicitation to exact broker issuance/consumption under session admission before any `RequireApproval` effect can execute.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
