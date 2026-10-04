# Roadmap

The canonical roadmap lives in [docs/product/SCOPE_AND_ROADMAP.md](docs/product/SCOPE_AND_ROADMAP.md).

**Last reviewed:** 2026-10-04  
**Current phase:** Phase 2D3 — exact-head Git integration  
**Implementation state:** executable Rust pre-alpha; Phase 1, Phase 2A–2C and Phase 2D1–2D2 are merged  
**Release state:** no production-supported release  
**Next gate:** add separate `GitIntegrate` authority with an exact validated target-head precondition, repository-scoped lease, isolated integration/worktree ownership and deterministic stale-target/conflict/cleanup behavior before exposing any Git mutation MCP tool.

The current `main` is ready for a first manual Windows developer smoke on a disposable repository. That smoke is a machine/integration validation only; it is not a production-readiness claim and does not replace the Phase 2D3 security gate.

Roadmap status is maintained with the repository. Every milestone change must update `STATUS.md`; user-visible changes update `CHANGELOG.md`; architecture/security changes update the owning canonical document and an ADR when a decision changes.

A phase is complete only when its acceptance/security gates pass. Calendar dates are planning aids, not completion criteria.
