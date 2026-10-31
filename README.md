# Optic AI Bridge

A Windows-first Rust bridge that lets an MCP-capable AI work on local projects
under explicit operator permissions. The model proposes work; deterministic
policy authorizes effects and Windows isolation contains supported processes.

**Pre-alpha. No production-supported release is available.**

## Windows 10/11 V1 — état vérifié au 31 octobre 2026 (après #453)

**Pré-alpha : Windows V1 NO-GO. Aucune release ni installation annoncée prête.**
Le dernier `main` technique vérifié **avant cette PR documentaire** est
`bfcd9dcba0f73bf6d8aa4d148fc5c654c819edf6` : la
[CI indépendante push/main #41195919444](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195919444)
a terminé **SUCCESS 4/4, tentative 1**, sur le SHA intégré exact.
Vérifier à nouveau la branche et les Actions après cette mise à jour documentaire.

Les quatre étapes de stabilisation [#450](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/450)
à [#453](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/453) ont été
**commitées, testées, squash-fusionnées, puis validées indépendamment** :
comptabilité des appels natifs non exécutants, témoin d'arrêt original,
tickets liés à la durée de vie du propriétaire, puis **correction du délai
sur le `HardenedCommandRunner` réel** (le chronomètre commence avant
`Command::spawn` et la préparation des pipes). La dernière CI Windows donne
**424 tests runtime réussis, 0 échec, 11 ignorés**, avec preuves physiques
schema 8 complètes (10 cas Windows / 4 Unix). Les tests spécialisés #317
sont verts sur Windows/Ubuntu (11/11 chacun). Les détails, SHAs et rapports
sont dans la [passation vérifiée](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-31.md).

**Aucune preuve de GO :** le GitRead natif reste non exécutant et non relié
au MCP de production. La synchronisation de lancement/révocation
session/TaskLease/policy/hôte et les preuves réelles root/Job/EOF manquent.
Le runner legacy peut toujours **bloquer sur les `.join()` des pipes hérités** :
la PR #453 corrige la classification du dépassement de temps, **pas** l'arrêt
borné des descendants. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323)
reste ouverte ; [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319),
[#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185),
[#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)
et [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182)
restent des blocages indépendants. Les PR de recherche #327/#353/#355/#358
ne doivent pas être fusionnées en bloc ; #355 garde un test négatif réel.

**Reprendre sans perte de méthode :** consulter le
[handoff technique](docs/engineering/HANDOFF_WINDOWS_V1_2026-10-31.md)
et le [prompt du prochain chat](docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-31.md).
Une seule PR ciblée : GitHub branch/push → tests Windows/Ubuntu sur HEAD exact
→ revue → squash si vert → égalité des arbres Git → **CI `push/main`
indépendante 4/4 sur le squash** → reçus dans PR et #323 → PR suivante.
Pas d'action sur le PC utilisateur, aucun runner auto-hébergé, aucune relaxation
de sécurité ou modification de release/protection sans autorisation précise.

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
