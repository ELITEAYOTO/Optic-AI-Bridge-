# Windows V1 — evidence-led freeze readiness (proposed)

**Status: RESEARCH / NOT APPROVED / NO RELEASE.**
**Reviewed:** 2026-10-22. **Product candidate:** a *local, headless Rust/MCP Windows 10/11* Bridge V1, **not** Optic Desktop/Electron and **not** real-project remote/autonomous access. This is a proposed scope requiring an explicit operator freeze decision, not an announcement of a V1.
**Verified at this checkpoint:** `main=82ad1234f184591e0f2c80c86296a65a883ecc20`, `main.protected=false`, 0 open PRs; research `bootstrap/issue-317-windows-rust-owner=1544ab9f338bac03562ac786e9b88f7061254154`, exact-head normal CI [#39738348645](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39738348645) completed/success on the research head. The native owner fixture has **13 research-only Windows cases** at unchanged native source `6d2261c7`, [#39735459098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39735459098); real `SessionRegistry` tests have passed normal CI on the newer head. These facts **do not** prove a production process-owner fix or Windows 10/11 certification.

This document makes acceptance binary *per published evidence*. It does **not** claim exhaustive testing, immunity from vulnerabilities, or `100% bug free`. A known release-blocking fault remains an unconditional **NO-GO**, even if all unrelated CI jobs are green. Preserve first failed runs and distinguish a correction from a green retry.

## Release scope: include, exclude, or explicitly decide

**Proposed local V1:** existing headless Rust core/MCP tools, typed effects, local authenticated operator-approved grants as currently supported, supported Windows 10/11 and specified PowerShell host combinations only after real acceptance. Pin the exact executable/runtime versions and supported installer paths. No expansion of privileges, sandbox, network, files or Git capabilities to satisfy tests.

**Explicit exclusions unless separately authorized and proven:** Electron/GUI, Linux functional confinement, real-project remote execution/Streamable HTTP ([#177](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/177)), external-host identity/session resolution and autonomous remote approvals ([#171–#175](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/171)). Exclusion must be enforced by unavailable/rejected runtime paths, not just documentation. Existing code still has to handle unsupported operations safely.

**Conditional feature decision:** Access to *real project data* (even when local) cannot be called safe merely because a path has been selected; [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) and [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) need explicit local capability/sensitivity boundaries and tests. Any real-project remote/autonomous mode remains a separate prohibited milestone requiring H-01–H-05. If the V1 scope cannot provide a proven file/Git read boundary, disable those paths in the candidate rather than imply a blanket safe read contract.

## Freeze/no-go matrix — evidence owner and exit condition

| Gate | State at this review | Required acceptance before freeze |
| --- | --- | --- |
| W1 — bounded Git command ownership [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317) | **NO-GO / open.** Production `HardenedCommandRunner` may block on stdio `join()` beyond its nominal deadline. | An independently supervised real Windows Git owner retains pre-spawn reservations, limits and all native Job/pipe/stdin handles; stop progresses on revoke/caller Drop, including racing launch; uncertain cases cannot free capacity or permit `seal_quiescent`/`try_reap`. One read-only Git path migrated and verified *before* any mutating CAS path. Separate CAS-outcome recovery must be proven if GitIntegrate ships. |
| W2 — sensitive filesystem and GitRead [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176), [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) | **NO-GO for unsupported real-project exposure**, not proof of an actual secret leak. Lexical/hardlink/porcelain safeguards are partial. | Approved deterministic local read contract; test ordinary-name secrets, hard links/aliases, snapshots and branch metadata or explicitly disable unproven sensitive paths. Remote/autonomous access remains excluded until separate host/auth approval gates. |
| W3 — initial MCP latency [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) | **OPEN / intermittent**; unrelated PowerShell BOM defect #321 was fixed. | Locate failure phase on an actual recurrence and validate bounded correction, or explicitly exclude affected host/version with a reproducible startup acceptance matrix. Do not increase timeout to conceal the failure. |
| W4 — AppContainer/Node [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) | **OPEN / intermittent**, cause unproven. | Preserve actual network-denial and no-connection assertions; demonstrate stable supported execution or fail-closed exclusion of the unreliable profile. No isolation bypass or weaker smoke accepted. |
| W5 — sessions, permissions and native cleanup | **PARTIAL**. True Git owner/service not wired; H-02–H-05 remote guarantees not a local claim. | Cross-session isolation, monotonic lease expiry, independent stop, startup `HostStartup` identity, uncertain cleanup, bounded resource accounting, fail-closed native errors, and interrupted session revoke/reap tested on applicable native Windows targets. |
| W6 — Windows version/install matrix | **NO CERTIFICATION**. GitHub windows-latest runners are not Windows 10/11 client acceptance. | Define supported Windows 10/11 build/architecture, PowerShell 5.1 and pwsh combinations, packaged install/upgrade/uninstall, stdout MCP framing, restart, no ambient secret/network and actual cleanup; capture exact OS/build/installer SHA. Device-local testing requires fresh operator consent. |
| W7 — reproducibility and endurance | **PARTIAL**. Fast CI passes are not long soak/restart proof. | Full exact-source Windows Rust/MCP/security and Ubuntu regression plus dependency CI, independent post-merge CI, bounded concurrency/soak/fault injection relevant to real operations, and no unexplained security-sensitive intermittent failures. Record first red, retry and SHA separately. |
| W8 — release controls and provenance | **PARTIAL**. Tag provenance #320 is present; `main.protected=false`, release publication token remains broadly scoped during build. | Review branch protection/review requirements with owner approval; ensure tag ancestry/exact-main CI and immutable source/artifact evidence, minimal release credentials, no unauthorized tag/mutable source, and a separately approved operator freeze. The existing prerelease workflow is *not* itself V1 authorization. |
| W9 — continuity and risk audit | **PARTIAL**. `STATUS.md` and handoff contain dated checkpoints; high-risk review is not exhaustive. | Selectively reconcile source SHA/CI and module invariants in canonical main docs; record boundaries, outstanding risk, precise supported/excluded scope, known limitations, rollback and incident instructions. Do not mass-reformat files or erase history. |

**Status policy:** only change a gate from NO-GO/partial to PASS with reproducible evidence on the **actual integrated `main` candidate**, not a test-only research branch. A waiver may exclude a *feature* from V1 only when exclusion is enforced and independently verified; an unsafe reachable code path cannot be waived by relabeling it a feature. No forced false “all clear”.

## #317 minimum coherent engineering slices (one PR at a time)

1. **Service-backed reservation and authority contract.** Define private, service-minted opaque owner ID; Session vs HostStartup identity, immutable grant/task/executable/root; finite pre-spawn process, CPU, RAM, output, stdin and retained-uncertain capacity. Reject before effect if exhausted. Authentic trusted stop must bypass normal saturated transport admission but not identity checks. Add real session signal/admission/revoke race tests.
2. **Windows native ownership.** Encapsulate Job and suspended child into service-held owner, not a caller `Drop` temporary; restricted Job assigned before resume, fail-closed startup/assignment errors and retained shutdown uncertainty. Prove real stdout/stderr EOF and joined/cancelled stdin before release. Independent Windows watchdog and resource/handle leakage measurements.
3. **Session lifecycle integration.** Use pre-drain `revocation_signal` and owner registry for stop while admission is still held; avoid deadlock. Gate `seal_quiescent` and `try_reap` on native owners with missing proof; unrelated Session/HostStartup unaffected. Bounded shutdown and unresolved-state diagnostics.
4. **Read-only Git migration, then mutation separately.** Move one GitRead path with exact structured authority/budget while maintaining restricted environment. Test abandoned requester, inherited pipe holder, process-tree termination, cross-session revocation and output limit. Git update-ref CAS may commit before timeout; mutation outcome journaling and reconciliation require a distinct proof and decision.
5. **Rebuild evidence on merge source.** Review a narrow branch cut from current main rather than bulk merging `bootstrap/issue-317-windows-rust-owner`. Verify exact PR head, squash source diff, then independent `main` CI plus Windows-specific tests. Preserve any first red.

The earlier thirteen native cases and later real-registry unit tests are **research building blocks**, not a production owner service. See [ADR-0035 proposal](../decisions/ADR-0035-WINDOWS-GIT-OWNER-STAGING.md), [real-session race contract](ISSUE317_REAL_SESSION_SIGNAL_ACCEPTANCE_2026-10-22.md), [security invariants](../security/SECURITY_INVARIANTS.md), and [release source gate](../engineering/RELEASE_SOURCE_GATE.md).

## V1 freeze decision record (leave pending)

- Freeze candidate commit/tag: **UNSET**.
- Exact Windows 10/11 install/compatibility evidence: **MISSING**.
- #317 fix merged and independently proven: **NO**.
- Scope of safe local project access and remote exclusions: **PENDING**.
- Known security-sensitive flake disposition: **PENDING**.
- Release credentials/protection decision: **PENDING OPERATOR**.
- All required freeze gates passed: **NO**.
- Operator approval to lift feature freeze and cut V1: **NOT REQUESTED / NOT GIVEN**.

**Next action:** prioritize the real Windows service-owned Git process interface and integration with session quiescence, not another unconnected fixture or GUI feature. Reassess this checklist after each *actual* merged corrective PR. Windows V1 is a decision after evidence, not a promise triggered by a green CI badge.
