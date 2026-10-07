# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-07
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2C, Phase 2D1–2D3D, the real-binary closure smoke and the explicit ChatGPT integration packaging gate are merged
**Release state:** no production-supported release
**Next gate:** Phase 3C1 bounded process termination through Phase 3C3C2C2A runtime launcher wiring plus Phase 3C3C2C2B1 exact-file workspace read grants are merged and post-merge validated. B1 merged in PR #74 as `11232b90` from exact final head `0c82f0c`; exact PR CI #330 and post-merge `main` CI #331 passed Ubuntu, Windows (including the controlled AppContainer ACL probe, real MCP smoke and installer profiles) and dependency policy. Windows now has a revocable exact-file read-only Package-SID ACL primitive restricted to fresh ephemeral profiles, but the helper/runtime does not yet select grants and policy/MCP still deny `Interpreter` / `RepositoryCode`. The next Phase 3C work is grant lifecycle/wiring into the helper/runtime plus representative toolchain compatibility proof before any selected high-risk policy re-admission; broader workspace coverage, write authority, truthful OS network containment and broader sandboxing remain later gates.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
