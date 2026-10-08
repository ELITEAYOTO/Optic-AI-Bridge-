# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-08
**Current phase:** Phase 3 — multi-session runtime
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2C, Phase 2D1–2D3D, the real-binary closure smoke and the explicit ChatGPT integration packaging gate are merged
**Release state:** no production-supported release
**Next gate:** Phase 3C isolation characterization is merged through the capability-free AppContainer network proof. PR #82 (`8ea01de3`, exact green final head `915b38f4`, CI #357) characterized representative toolchains through the real helper/AppContainer path: Node starts, reads one exact operator-granted file and is denied an ungranted sibling; Python/Java fail earlier at runtime loading; rustup proxy behavior was separated from direct toolchain binaries. PR #83 (`b5a53b08`, exact green final head `25d2eed7`, CI #361) proved direct pinned Cargo starts successfully, direct rustc still has a loader dependency, and identified Python/Java runtime candidates without granting them. PR #85 (`bb649833`, exact head `6fcc25d1`, PR CI #365 and post-merge `main` CI #366) added a native zero-capability AppContainer proof that the tested Node TCP loopback connection is denied and not accepted by the host listener. Public `Interpreter` / `RepositoryCode` execution remains fail-closed. The next gate is an application/server-owned strong-isolation eligibility marker and selected policy re-admission; compatibility evidence must not imply broad workspace, runtime, write or network authority.

The first real Windows developer smoke passed on 2026-10-04 against the Phase 2D2 file/Git-read surface. The later 2026-10-05 disposable-repository ChatGPT Desktop smoke completed the Phase 2D3 exact-head integration gate end-to-end. These are integration/security validations, not production-readiness claims.

The user-scoped Windows quick installer, MCP doctor, uninstaller and tag-driven prerelease bundle workflow have also been pulled forward as developer-preview Phase 5 groundwork. No public release has been tagged yet, so the repository remains pre-alpha with release state `none`.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
