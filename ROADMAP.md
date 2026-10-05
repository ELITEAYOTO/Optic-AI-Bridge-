# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-05
**Current phase:** Phase 2D3 — exact-head Git integration closure
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2C and Phase 2D1–2D3D plus the real-binary closure smoke are merged
**Release state:** no production-supported release
**Next gate:** PR #46 (`c44ba567`, exact green head `f4d04c3`) merged the repository-scoped read-only `git_integration_status` companion and the updated native binary smoke. The current gate adds an explicit, default-off ChatGPT installer/plugin profile for the two integration tools and validates it in Windows CI without touching a real user profile. The final Phase 2D3 closure gate after that is a disposable-repository ChatGPT Desktop smoke.

The first real Windows developer smoke has now passed on a disposable repository through ChatGPT Desktop: MCP stdio initialization, file reads, bounded Git reads, scoped transactional write/patch/delete, stale-version rejection, scope denial and clean recovery retirement were all observed. This validates the current Phase 2D2 machine/integration surface only; it is not a production-readiness claim and does not replace the Phase 2D3 security gate.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
