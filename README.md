# Optic AI Bridge

A Windows-first Rust bridge that lets an MCP-capable AI work on local projects
under explicit operator permissions. The model proposes work; deterministic
policy authorizes effects and Windows isolation contains supported processes.

**Pre-alpha. No production-supported release is available.**

## Windows V1 release status — 31 October 2026

**Windows 10/11 V1: NO-GO; no production release/tag.** At the last verified pre-handoff snapshot, `main=7f1de707c130` passed [independent push/main CI #41188920407](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41188920407) **4/4 on attempt 1**. Technical PRs [#445](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/445)–[#448](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/448) were squash-merged and verified independently. These reinforce startup diagnostics, GitRead response deadlines and **non-executing** Windows native Job/watchdog deadline checks; they **do not** switch production MCP GitRead to a native owner or resolve all V1 blockers.

The critical next task is a synchronized session/TaskLease/policy/Host **native launch and revocation fence** plus terminal Job/root/pipe proof. [V1 gate #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) is open; cold-start [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319), AppContainer [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), sensitive-read [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) / [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) also remain open. [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) is administratively closed, not sufficient evidence of live GitRead readiness.

**Resume the exact PR-by-PR workflow:** [31 Oct verified handoff](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-31.md) and [copy-ready successor prompt](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-31.md). Requery GitHub before acting: this is a dated snapshot, not a pinned current `main`.

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
