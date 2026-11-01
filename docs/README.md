# Documentation map

## Latest GitHub-only Windows V1 checkpoint

- **[Post-#470 / 1 November 2026 current handoff, evidence and safe replay](engineering/HANDOFF_WINDOWS_V1_POST_470_2026-11-01.md)** — verified merged #469/#470, draft #471, #317 RED 3/3, 38 inert native/kernel tests PASS, isolated Cargo build and zero-test false-positive warning. **Windows V1 NO-GO**. Re-query live main and CI after any documentation merge.
- [Prior canonical 1 November 2026 handoff](engineering/HANDOFF_WINDOWS_V1_2026-11-01.md) — historical checkpoint after #464.
- [Next-chat prompt (earlier checkpoint)](engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-11-01.md) — reconcile against the newer handoff before proceeding.

- [Canonical 31 October technical handoff after PR #453](engineering/HANDOFF_WINDOWS_V1_2026-10-31.md)
- [Complete next-chat prompt, updated after PR #453](engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_2026-10-31.md)
- [Current #323 native authority audit](engineering/ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28.md)
- [Live legacy runner liveness/timeout debt](engineering/RELIABILITY_DEBT.md)

Pre-documentation source `main=bfcd9dcba0f73bf6d8aa4d148fc5c654c819edf6`
has independent [push/main CI #41195919444](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41195919444)
SUCCESS 4/4; recheck main after this documentation PR. Windows 10/11
V1 stays **NO-GO**, no release. All #450–#453 are already integrated,
but the native GitRead launch fence, physical terminal evidence and
legacy pipe-join liveness remain unresolved.


## Current project entry points

- [Reusable new-chat prompt](../PROJECT_CONTINUITY_PROMPT.md)
- [Agent instructions](../AGENTS.md)
- [GitHub-only handoff and validation map](engineering/GITHUB_HANDOFF.md)
- [Repository overview](../README.md)
- [Current status and evidence](../STATUS.md)
- [Delivery roadmap](../ROADMAP.md)
- [Product charter](product/PRODUCT_CHARTER.md)
- [Contribution checks](../CONTRIBUTING.md)
- [AI coding invariants](../AI_HANDOFF.md)

## Contracts and implementation

- [System architecture](architecture/SYSTEM_ARCHITECTURE.md)
- [Process runtime](architecture/PROCESS_RUNTIME.md)
- [Recovery journal](architecture/RECOVERY_JOURNAL.md)
- [Security invariants](security/SECURITY_INVARIANTS.md)
- [Threat model](security/THREAT_MODEL.md)
- [Windows isolation](security/WINDOWS_ISOLATION.md)
- [Specifications](specs/)
- [Architectural decisions](decisions/)
- [Test and CI strategy](engineering/TEST_AND_CI_STRATEGY.md)
- [Reliability debt and reviewed boundaries](engineering/RELIABILITY_DEBT.md)
- [Staged code-quality and per-file review](engineering/CODE_QUALITY_REVIEW.md)

## Operations and maintenance

- [Local ChatGPT developer preview](operations/CHATGPT_DESKTOP_QUICK_INSTALL.md)
- [Branch and PR workflow](engineering/REPOSITORY_WORKFLOW.md)
- [Consolidation audit](engineering/REPOSITORY_CONSOLIDATION_2026-10-20.md)
- [Maintenance policy](../MAINTENANCE.md)
- [Changelog](../CHANGELOG.md)
- [Security reporting](../SECURITY.md)

`research/` and dated audits retain evidence and unresolved findings. A research
result is not a supported runtime feature. Older phase documents describe their
own scope/date; use the root status and roadmap for present delivery decisions.
