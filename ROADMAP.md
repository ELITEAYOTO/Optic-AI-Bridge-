# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-06
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2C, Phase 2D1–2D3D, the real-binary closure smoke and the explicit ChatGPT integration packaging gate are merged
**Release state:** no production-supported release
**Next gate:** Phase 3A, 3B1 and 3B2 are merged. Phase 3B3 is implemented in PR #53: task leases are hard-bounded globally/per-session, startup session provisioning goes through `SessionLifecycleManager`, and an internal bounded expiry supervisor reuses the same admission/revoke/reap path. Exact code/test head `5d39f5b` passed Ubuntu, Windows and dependency-policy CI. Final documentation-head CI, merge, and post-merge `main` CI remain required before Phase 3C begins. Public session creation and renewal remain unimplemented.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
