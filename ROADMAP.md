# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-04  
**Current phase:** Phase 2D3 — exact-head Git integration  
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2C, Phase 2D1–2D3A and Phase 2D3B1 are merged
**Release state:** no production-supported release  
**Next gate:** Phase 2D3A is merged in PR #40 (`22b34149`) and 2D3B1 authorization is merged in PR #41 (`f3d0898f`). The current 2D3B2 branch adds bounded fail-closed recovery for orphaned Optic worktrees; explicit startup/operator provisioning remains the next sub-gate. Only after recovery + startup authority pass does 2D3C add a thin conditional MCP adapter. No Git mutation MCP tool is exposed yet.

The first real Windows developer smoke has now passed on a disposable repository through ChatGPT Desktop: MCP stdio initialization, file reads, bounded Git reads, scoped transactional write/patch/delete, stale-version rejection, scope denial and clean recovery retirement were all observed. This validates the current Phase 2D2 machine/integration surface only; it is not a production-readiness claim and does not replace the Phase 2D3 security gate.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
