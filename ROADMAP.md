# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-08
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2D3D, Phase 3A–3D, the selected C5G isolated-Node Desktop profile, Phase 3E1 aggregate declared-memory governance, Phase 3E2A Windows host-memory observation, Phase 3E2B emergency-headroom admission, and Phase 3E3 bounded heavy-workload admission are merged
**Release state:** no production-supported release
**Next gate:** B-02 — broaden OS-level network-containment evidence beyond the selected high-risk Node/TCP-loopback profile and treat the direct `FixedTool` path separately. Phase 3E3 is green through PR #107 (merge `826f8cb`, final head `203d729c`): final-head CI #421 completed successfully after a targeted Windows rerun of one transient AppContainer loopback probe timeout, and post-merge `main` CI #422 passed. Application-owned `WorkloadClass` now bounds process workloads to 2 active heavy jobs bridge-wide and 1 per session without exposing caller-controlled class selection. Optional I/O governance and richer memory-pressure feedback remain separate residual A-02 work rather than the next automatic gate.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
