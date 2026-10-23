# #317 — Last-check / ResumeThread race: hosted Windows native proof

**23 October 2026 — RESEARCH / NOT PRODUCTION.** This document describes a controlled synthetic Win32 race reproduction and a **test-only** shared launch/close lock. It is not approval for production integration or a claim that real session revocation and native launch are yet linearized.

## Exact-source checkpoint

- Base: `research/317-native-lifecycle-decision-2026-10-23 = 8b72b55ad80f2d73e2948203ad47f8632ae7efee`; original main remained `da34eae7d0efda39d9fb3c6786ec843546926fe9`.
- Research implementation at **`6d12d417bb8316c191ac55b38f61d0793ef02633`**.
- Independent hosted-Windows [CI #39900699107](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39900699107): **success, attempt 1 on that exact source**. Pinned Rust 1.99, workspace Rustfmt, Windows platform test binary compilation, strict Windows Clippy (`-D warnings`), independent external Job-supervised native execution. All **10** synthetic cases passed and were cleaned.
- The initial [red #39900618473](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39900618473) stopped at Rustfmt **before compilation**. The format discrepancies were corrected on a **new SHA**; no test was skipped or lint disabled.
- The historical seven native cases still pass alongside the three new cases. The artifact/log includes an exact `source_sha`, run ID/attempt and one result per case. The Python supervisor gates execution until the exact Rust test host is placed in an independent outer Job, and on all paths recovers the host and child, including the negative control.

## Deterministic Windows reproducer

The `#[cfg(test)]` Rust fixture creates a **fixed synthetic Rust test child** with `CreateProcessW`, `CREATE_SUSPENDED`, `PROC_THREAD_ATTRIBUTE_JOB_LIST`, and a preconfigured inner Job (KILL_ON_JOB_CLOSE, CPU/memory/process limits). The exact child is never derived from a user's project, file, command, network input or credential. When resumed, the fixture writes a marker to a supervisor-provided temporary path. Every case terminates and observes an empty inner Job before reporting success.

| Case | Deterministic ordering | Observed test-only result |
| --- | --- | --- |
| `resume-race` | Launcher checks `Open`, releases the check; a separate thread **completes `Closed`**; launcher then calls `ResumeThread` without rechecking | Marker is written **after closure**; stale check does not enforce authorization. |
| `resume-close-wins` | Close acquires the **same** synthetic lock before launch commit; launcher observes `Closed` | Launcher does not call `ResumeThread`; no marker; suspended child remains Job-owned until terminated. |
| `resume-launch-wins` | Launcher holds the **same** synthetic lock while it validates `Open` and calls `ResumeThread`, recording `Committed`; separate closer later publishes `Closed` | Commit precedes close; the synthetic marker is observed; Job is then terminated and cleaned. |

All three showed zero outer Job members after the exact synthetic host was killed, and zero inner Job members before the host's readiness signal. The newly added fixture does not transfer user secrets or enable real Git execution.

**Critical interpretation:** `resume-race` is a *positive test for a negative property*. CI success means the old check-to-resume gap was deliberately reproduced. The two corrected orderings test **a toy per-launch Mutex**, not the real `SessionRegistry`, `SessionLifecycleManager::revoke`, or `ReadServiceRoot`. The latter are **not** repaired by this branch.

## Relationship with the actual session registry

The separate [registry contract research](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/53f64f9fdc0355dd8461075bac4a76519eb468ec/docs/research/ISSUE317_REVOKE_LAUNCH_CONTRACT_2026-10-23.md) proves the current registry closes admissions and publishes the Windows cancellation atomic **before** draining scoped permits. But the research `ReadServiceRoot::submit` releases its admission permit before the worker's native lifetime, while current production `SessionRegistry::revoke` blocks waiting for live permits.

Therefore simply transferring a permit to a long-running worker risks moving a permanent-observation hang **into revoke**, and merely adding an unrelated mutex around `ResumeThread` **does not coordinate** with actual session closure. These are two separate, still-open architecture gates.

An accepted integration must choose and specify a **single shared linearization point** for:
1. close admission and publish stop intent,
2. commit a launch under native Job ownership,
3. request stop and prove terminality independently of completion of the close request.

For **close wins**, no new launch may commit after close. For **launch wins**, a permitted resume can precede close, but close cannot promise that no instructions execute afterwards; it must urgently request native stop, preserve owners/quotas and refuse seal/reap until terminal evidence. Output publication must also be checked against post-revocation policy.

An implementation must not hold the *global registry mutex* across a potentially blocking Win32 operation without a lock/deadline proof. A session-scoped lock, a durable transactional launch broker, or a different admission protocol must be validated **against actual authority operations** rather than this toy simulation. The intended public two-stage revocation or synchronous compatibility contract remains **PROPOSED, not approved**.

## Outstanding NO-GO gates

- Real registry-vs-native commit/close synchronization using actual session epoch and stop signal, with both deterministic orderings and a supervisor on every native process.
- Persistent observation failure, `Drop` and `try_shutdown` responsibility, and bounded/durable ownership without detached processes.
- Effective deadline, pinned Git executable identity/cwd/environment, task leases and `AuthorizedRead`; avoid spurious `GitRead` / `GitCAS` wiring.
- Windows 10/11 physical acceptance and independent post-merge native CI.
- Unrelated #182 provenance authority, #185 / PR #327 AppContainer CI, and V1 freeze #323 all retain their own gates.

**No production code changed; no PR merged; no branch protections altered; no operator-PC test; #317 and #323 stay open/NO-GO.**

**Recommended next increment:** review a session-scoped close/commit API **as a contract proposal**, with explicit lock ordering and pending native owner semantics; prototype it in separate test-only runtime research tied to the actual registry before migrating even one read-only Git effect.
