# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-07
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2C, Phase 2D1–2D3D, the real-binary closure smoke and the explicit ChatGPT integration packaging gate are merged
**Release state:** no production-supported release
**Next gate:** Phase 3C1 bounded process termination, Phase 3C2 bounded CPU governance, Phase 3C3A pinned executable identity, Phase 3C3B explicit process execution classification, Phase 3C3C1 fail-closed high-risk execution gating, Phase 3C3C2A AppContainer isolation foundation, Phase 3C3C2B1 runtime class propagation, Phase 3C3C2B2 AppContainer explicit-stdio primitive, Phase 3C3C2C1 internal launcher proof and Phase 3C3C2C2A runtime launcher wiring are merged and post-merge validated. C2A merged in PR #72 as `d31086e1` from exact final head `49ffa305`; exact PR CI #318 and post-merge `main` CI #319 passed Ubuntu, Windows (including native runtime-isolation routing, real MCP smoke and installer profiles) and dependency policy. `ProcessManager` can now route direct internal Windows high-risk starts through the pinned sibling helper and accounts for helper + target as two kernel processes while preserving the logical workload `process_count`. The helper remains non-distributed and policy/MCP still deny `Interpreter` / `RepositoryCode`. The next Phase 3C work is explicit workspace grants plus representative toolchain compatibility proof before any selected high-risk policy re-admission; truthful OS network containment and broader sandboxing remain later gates.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
