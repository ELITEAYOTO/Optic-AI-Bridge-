# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-07
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2C, Phase 2D1–2D3D, the real-binary closure smoke and the explicit ChatGPT integration packaging gate are merged
**Release state:** no production-supported release
**Next gate:** Phase 3C1 bounded process termination through Phase 3C3C2C2B2 exact-file grant lifecycle/wiring are merged and post-merge validated. B2 merged in PR #76 as `8d127a3b` from exact final head `5aaf3957`; exact PR CI #336 and post-merge `main` CI #337 passed Ubuntu, Windows (including grant-selection/containment regressions, real MCP smoke and installer profiles) and dependency policy. Exact-file read grants can now flow through the isolated helper/runtime only from server-owned `FileRead + WorkspacePrefix` authority, with a 32-file ceiling, exact regular-file resolution and handle-based final-path containment. Existing operator process leases still mint no `FileRead` workspace scopes, so policy/MCP continue to deny `Interpreter` / `RepositoryCode` and no public high-risk read authority is enabled. The next Phase 3C work is explicit operator-owned process read-grant provisioning plus representative toolchain compatibility proof before any selected high-risk policy re-admission; broader workspace coverage, write authority, truthful OS network containment and broader sandboxing remain later gates.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
