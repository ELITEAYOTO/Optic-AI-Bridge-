# Optic AI Bridge

A Windows-first Rust bridge that lets an MCP-capable AI work on local projects
under explicit operator permissions. The model proposes work; deterministic
policy authorizes effects and Windows isolation contains supported processes.

**Pre-alpha. No production-supported release is available.**

## Choose the right track

| Track | Location | What it provides |
| --- | --- | --- |
| Validated core | `main` | Scoped/versioned file operations, directory creation, Git primitives, owned process lifecycle, permissions and local session supervision |
| FS generated-project alpha | [`alpha/generated-project-editing`](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/tree/alpha/generated-project-editing) | Remote editing, isolated Node check/test, explicit same-run resume and bounded diagnostic evidence on generated projects |
| Compatibility evidence | [Electron #250](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/250), [Cargo #255](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/255) | Completed diagnostic scopes; remaining GUI/build/dependency gates are tracked in the roadmap |

The core requires neither an embedded LLM nor Node, Python, Docker or Electron.
An optional execution profile uses a separately installed runtime selected by
the operator. Python belongs to the experimental HTTP adapter, not the Rust core.
The current FS alpha uses public dummy credentials and a shared generated-project
context; it is not production authentication or per-conversation isolation.

## Start here

- [Resume in another AI chat](PROJECT_CONTINUITY_PROMPT.md) — complete GitHub-only prompt, no operator-PC testing.
- [Agent entry point](AGENTS.md) and [current handoff](docs/engineering/GITHUB_HANDOFF.md) — live-state bootstrap, CI map and open limits.
- [Current status](STATUS.md) — merged work, exact evidence and remaining gates.
- [Delivery order](ROADMAP.md) — the next real-project acceptance steps.
- [Documentation map](docs/README.md) — architecture, security, tools and operations.
- [Contributor guide](CONTRIBUTING.md) — checks and development workflow.
- [AI handoff](AI_HANDOFF.md) — mandatory invariants for coding agents.
- [Repository workflow](docs/engineering/REPOSITORY_WORKFLOW.md) — branch and PR lifecycle.

For the existing local ChatGPT Desktop developer preview, follow
[the installation guide](docs/operations/CHATGPT_DESKTOP_QUICK_INSTALL.md).
The default profile does not grant arbitrary process execution. Optional Node
and Git-integration profiles require explicit operator configuration.

## Build and verify

For the current GitHub-only continuation, run checks in the agent's own isolated
environment or on GitHub-hosted runners, not on the operator's PC. The commands
below describe contributor checks; they are not a request for local execution.

Use the Rust version pinned in `rust-toolchain.toml`. Native Windows builds also
need the matching Windows SDK and MSVC toolchain; other systems cannot validate
Windows containment merely by compiling portable code.

```text
cargo build --locked --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

See [test strategy](docs/engineering/TEST_AND_CI_STRATEGY.md) for native, integration
and explicit research gates. A successful research comparison does not replace a
failed normal compatibility gate.

## Repository layout

| Directory | Owner |
| --- | --- |
| `crates/` | Rust application, domain services and native Windows containment |
| `experiments/` | Bounded research harnesses and generated-project adapters |
| `packaging/`, `scripts/` | Operator installation, bundles and validation |
| `docs/specs/`, `docs/security/`, `docs/architecture/` | Contracts and invariants |
| `docs/decisions/` | Architectural decisions |
| `docs/research/` | Measured evidence and unresolved compatibility questions |

General writable builds, dependency installation and arbitrary GUI applications
still need their own authorization, storage, recovery and native acceptance.
See [the roadmap](ROADMAP.md); do not infer these permissions from an installed
runtime or an AI model's coding ability.

Historical implementation notes remain in Git history. The repository's
[consolidation record](docs/engineering/REPOSITORY_CONSOLIDATION_2026-10-20.md)
links the exact earlier snapshots and preserved PR heads.
