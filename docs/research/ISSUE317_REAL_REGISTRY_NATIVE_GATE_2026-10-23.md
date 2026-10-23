# #317 — Real SessionRegistry versus native Win32 resume (research)

**Date:** 2026-10-23. **Status:** EXPERIMENTAL, test-only, unapproved for production.  
**Production baseline:** `main=da34eae7d0efda39d9fb3c6786ec843546926fe9`.  
**Test branch:** `research/317-real-registry-native-gate-2026-10-23`.

## Purpose

Bridge two already-proven but separate findings, without modifying the public runtime:
1. the actual `SessionRegistry` publishes revoked state and cancels future admission before a synchronous revoke drains existing admission permits;
2. a Win32 suspended child may execute **after** the last pre-resume check has become stale unless a shared close/commit gate orders the two operations.

This slice exercises the **real production registry code and real Windows `CreateProcessW` / `ResumeThread`**, via a new **ignored** integration test in `optic-bridge-app`. Its launch gate is **test-only** and does not claim that real production call sites have been updated.

## Isolation and review surface

- `crates/optic-bridge-app/tests/issue317_registry_native_gate_windows.rs`: ignored, Windows-only integration fixture, using a synthetic self-test executable. No real Git, user workspace, executable allowlist bypass, network, credentials or assistant input.
- `scripts/research/issue317_real_registry_native.py`: pre-arms an independent external `KILL_ON_JOB_CLOSE` Job **before releasing** the test-host's startup gate, terminates only its own exact host and confirms the external Job empties even on assertion failure.
- `.github/workflows/issue317-real-registry-native.yml`: branch-scoped hosted-Windows CI with pinned Rustfmt/Clippy, locked Cargo compilation, exact-source evidence and retained original red runs.
- Windows-only dev-dependency on the **already workspace-locked** Win32 bindings in the app crate. No new public production functions, no `SessionRegistry` behavioral changes.

The native fixture constructs an inner Job with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, active-process and memory caps **before** `CreateProcessW`, then uses `PROC_THREAD_ATTRIBUTE_JOB_LIST` with `CREATE_SUSPENDED`. The native child is the exact synthetic test binary, with an environment-provided private marker file. No PID search, arbitrary process kill, inherited stdio or fallback spawn-then-assign. `OwnedHandle` retains the real returned process/thread handles and the test checks an empty inner Job **before** releasing its session admission permit. The external Job is the independent crash-recovery backstop.

## Deterministic native/authority cases

| Case | Actual SessionRegistry interaction | Native action and expected evidence |
| --- | --- | --- |
| `direct-revoke-race` (negative control) | A real admission is active; caller performs an early `get_active` check. Another thread invokes actual `SessionRegistry::revoke` and the test waits until `get_active` reports **Revoked**. That synchronous revoke is still pending on the held admission. | The stale-check caller **then** resumes the already-Job-associated child, which writes an execution marker after actual closure. Test passes only if this undesirable sequence is positively reproduced. |
| `close-wins` | A **test-only session-scoped Mutex** is set Closed by the closer, which then calls the real registry's revoke; test waits until the actual registry is revoked. | The launch side inspects the shared gate and denies resume, retains the suspended child, terminates its Job, checks marker absent/Job empty and only then releases admission. |
| `launch-wins` | The launch side observes the real grant and performs real native resume while holding the same **test-only** gate, before a closer acquires it and calls actual registry revoke. | The child may execute before or after the actual close; test verifies it executed, then the owner stops/observes the Job and releases admission before revoke completes. **No zero-instructions-after-close claim.** |

## Architectural interpretation and hard limitations

- **The experiment uses the actual `SessionRegistry` and native Win32 operations together**, unlike the earlier model-only native gate, but the close/commit Mutex is **not owned by the registry**; direct production calls to `SessionRegistry::revoke` bypass it. The negative control explicitly demonstrates why this matters.
- A long-running worker's permit must **not** simply be held until a permanently failing native observation finishes: today's `revoke` drains admission synchronously and could hang indefinitely. The test deliberately scopes the permit to a finite native launch/stop proof, then drops it only after Job terminal accounting.
- A session close and process quiescence are different facts. A successful admission close does not itself prove stream EOF, descendant cleanup, stop/resume exclusivity, the worker's result publication policy or a safe `Drop` under observation failure.
- This proof does **not** authorize production `AuthorizedRead`, GitRead integration, a new public two-stage revoke contract, an Electron/Rust GUI release, or changing GitHub protections. Existing #182 provenance, #185/PR #327 AppContainer and #323 V1 freeze remain independent.
- This is Windows-hosted automation, not device acceptance on Windows 10/11. No operator PC is used.

## Next review gate

Only after exact-source hosted-Windows evidence: propose a **session-authority-owned** launch/close linearization API, inventory every direct `revoke` caller and approval/lease/process cleanup ordering, define pending native owner states and bounded `try_shutdown`, then verify actual gate/win32 interleavings and output publication across revocation. **No production migration or merge of research branches without an explicit separate review.**


## Evidence checkpoint — exact Windows SHA 402cc390 (23 October 2026)

**Completed successfully:** [hosted Windows native CI #39902263833](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39902263833), **first attempt on exact source `402cc390fde812cb3afb8a5526fc4357677b7f09`**, including whole-workspace Rustfmt, locked `optic-bridge-app` Windows test compilation, strict Clippy with `-D warnings`, and an independent external Job supervising all three fixed native cases. The test runner recorded `real_session_registry=true`, `real_native_resume=true`, `outer_job_cleaned=true`, `complete=true`, `production_wired=false`.

Actual native outputs (all had external Job **1 → 0** after exact test-host death and inner Job **0** before host readiness):

| Mode | Real registry and native evidence | Result |
| --- | --- | --- |
| `direct-revoke-race` | A permit remains outstanding. The real `SessionRegistry::revoke` marks the session Revoked before `ResumeThread`. The stale-check caller then resumes its owned child; the execution marker is present. | **Gap reproduced**, intentionally passing negative control |
| `close-wins` | Test-scoped gate closes, actual registry is revoked, native child is not resumed, execution marker is absent, owner terminates the suspended Job before dropping permit. | **Close-first exclusion demonstrated** |
| `launch-wins` | Test-scoped gate allows native resume before real close, marker present; real revoke then closes the session and remains pending until the Job empties and the permit is dropped. | **Commit-first ordering demonstrated**, not an instruction-level post-close guarantee |

All three tests performed bounded checks, used a fixed self-test process and terminated native Job ownership under independent supervision. The negative test proves that a **real registry revocation** can precede a stale `ResumeThread` when no launch/close gate is shared. The positive cases demonstrate a **wrapper protocol** invoking real authority operations, not a change to the authority implementation itself.

**Preserved first reds and targeted corrections:** [#39901591076](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39901591076) failed Rustfmt, [#39901717465](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39901717465) exposed a `Cargo.lock` entry needed for an existing Windows dev-dependency, and [#39902088233](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39902088233) confirmed a Win32 accounting-type import typo (the initial JSON compiler format masked diagnostics, so the workflow was improved to display errors). [#39902133826](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39902133826) preserved the later Rustfmt import-order red. Corrections were made on **new commits**, with all strict checks retained.

**NO-GO remains:** No production `SessionRegistry` or `SessionLifecycleManager` contract is modified. The test-scoped per-session gate can be bypassed by existing direct callers of `revoke`; therefore a safe production fix requires an authority-owned or otherwise non-bypassable close/commit linearization point, with a verified finite native-call/lock-order policy and a separate durable pending-owner shutdown model. The real service still may release its permit at submit and its `Drop` may block on permanent observation failure. No real GitRead activation, real Windows 10/11 client acceptance or #323 V1 release is validated.
