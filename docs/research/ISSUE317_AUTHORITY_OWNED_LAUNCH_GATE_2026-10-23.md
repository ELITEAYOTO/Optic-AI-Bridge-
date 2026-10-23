# #317 — Authority-owned launch/close gate research

**Date:** 23 October 2026  
**Status:** RESEARCH ONLY / NO-GO for production integration  
**Branch:** `research/317-authority-owned-gate-2026-10-23` (forked from Codex `279f095915e70d32872a827b39761a3ab883ee38`)  
**Production main baseline:** `da34eae7d0efda39d9fb3c6786ec843546926fe9`

## Objective

The previous Codex work demonstrated a lifecycle cancellation dependency: a real worker-owned admission permit keeps synchronous `SessionRegistry::revoke` waiting; `SessionLifecycleManager::revoke` only calls `ProcessManager::cancel_session` after that wait. A prior **launcher-private** Mutex also did not intercept direct calls to `SessionRegistry::revoke`.

This new small experiment tests a **per-session gate owned by the registry's actual session record**. It is intentionally **not** wired to any real GitRead launcher or native Win32 process. Its purpose is to discriminate a bypassable wrapper lock from a shared security-authority lock and expose remaining contracts.

## What was implemented (test-only instrumentation)

- `crates/optic-bridge-runtime/src/session/registry.rs` gains a per-record `Arc<Mutex<()>>` **behind `#[cfg(test)]` only**; it is absent from the production-compiled library.
- `research_commit_launch(handle, now, closure)`, also **`#[cfg(test)]`**, clones this gate, acquires it, rechecks actual session record ownership/revoked/expiry under the registry mutex, releases the registry mutex, and invokes a **synthetic bounded closure** while retaining the per-session gate.
- `SessionRegistry::revoke` when compiled **with `#[cfg(test)]`** obtains the **same** per-session gate before publishing `revoked` and `revocation_signal`, then drops the gate **before** entering the existing admission Condvar drain. The ordinary production compilation does **not** include this additional locking.
- Codex's test-only `research_close_admission` obtains the same gate before publishing a prompt close, but still **does not** prove native process termination.
- `research_commit_launch` checks that the gate belongs to the current record (`Arc::ptr_eq`) before permitting the effect, to reject an obsolete gate after record replacement.

This is a *hypothesis implemented in a test compilation*, **not an independent proof of unchanged production behavior**. The actual authority code has been conditionally instrumented in tests. A real implementation would change production synchronization and therefore requires an explicit design/security review.

## Deterministic, narrowly scoped assertions

New module: `session/registry_launch_gate_tests.rs`.

| Scenario | Expected test-only contract |
| --- | --- |
| `close_wins_and_no_effect_runs_after_real_record_is_revoked` | Prompt close marks actual record revoked; a later commit is refused without invoking its closure; repeated close and direct revoke are idempotent. |
| `direct_public_revoke_uses_authority_gate_not_private_launcher_wrapper` | A's commit holds its session gate while a separate thread directly calls **`SessionRegistry::revoke`**. Other session B can commit (no global mutex convoy). When A releases the bounded commit closure, direct revoke publishes closure and waits for a genuine A admission permit. New A commits fail after close; dropping the permit completes revoke. |
| `rejected_commit_does_not_execute_on_expiry_or_unknown_handle` | Expired or unknown session cannot invoke the synthetic effect at all. |

The prior Codex two lifecycle/drain tests are rerun on both Linux and Windows. No native child is created by these new tests.

## Independent native production-path negative control

A **separate branch-scoped hosted-Windows** workflow compiles the app integration test so the runtime dependency is **not** built with `#[cfg(test)]`. It reruns the three independent-outer-Job Win32 modes:
- `direct-revoke-race`: real registry revoked first, then a stale-check launcher resumes a suspended Job-owned child, proving that **production behavior still contains the gap**;
- `close-wins` and `launch-wins`: test-wrapper outcomes;
- no operator-PC process or real Git/MCP effects.

A passing test-only contract suite together with a passing *native negative control* is the expected result: this research has **not** repaired the production gap.

## Concurrency/ownership limitations and blockers

1. **No real Win32 resume under the new registry-owned gate.** The guarded closure is synthetic. The previously proven native `ResumeThread` race still exists in normal builds.
2. **Native call boundedness:** holding a per-session gate across a kernel call that can hang delays session close. The experiment uses a finite channel deadline but cannot prove a bounded `ResumeThread` or `CreateProcessW`.
3. **Synchronous drain dependency unchanged:** direct `revoke` still waits for admission permits after publishing close; Codex's observed process-cancellation dependency therefore remains in production, as does potentially unbounded observation in the experimental service.
4. **Record replacement and lock ordering:** commit checks the actual gate's identity; a full ABA/re-registration and concurrent physical reap model must be tested before any design is shipped. Registry lookup briefly holds the global mutex but drops it before waiting on the per-session lock. All close, expiry, renewal, reap and lifecycle call paths must be audited for an unavoidable common gate.
5. **No final-authority output policy:** a job that committed before close can execute instructions afterward; output/error publication and data provenance must be revalidated at the appropriate policy epoch.
6. **No durable native pending owner:** Job membership, descendant-held pipes, uncertainty in `Drop`, resource quotas and terminal proof remain separate safety conditions.
7. **Unapproved report API:** `SessionRevokeReport` cannot silently represent prompt closure as completed native quiescence; its changed counters were demonstrated in the Codex research.

**Do not merge this branch wholesale** into `main` or mark ADR-0035 accepted; do not close #317 or #323. PR #327 and provenance issue #182 remain separate.

## Next proof / Codex opportunity

Use the restored local Codex quota **only if the next authorized slice requires real Windows native call/Job lifecycle behavior beyond hosted runners**. A useful bounded task is to audit and instrument a *single authority-owned commit/close gate with the actual native suspended launch path*, produce negative tests for a close that arrives during `ResumeThread`, and demonstrate a shutdown owner retaining its Job across persistent observation errors. Do not change production APIs or runner until direct-close bypass, boundedness, uncertain-owner and report compatibility gates are decided.

The next implementation discussion should compare an authority-owned per-session gate versus a dedicated launch-authority broker, with explicit lock-order proof and a separate prompt-close / deferred-quiescence API.


## Poisoned gate / exception-safety continuation

A Rust panic while the guarded synthetic effect executes poisons its session-scoped mutex. A security-critical close path may **not** simply propagate `PoisonError` and leave an active grant in place. The research-only `revoke` and test prompt-close therefore recover the poisoned mutex guard specifically for **publishing cancellation**, while `research_commit_launch` refuses another effect on a poisoned gate. The new `panicked_commit_poison_never_blocks_direct_revoke` regression uses `catch_unwind` to force this state and verifies closure is still published on the *actual* record. This is a test compilation only, not a production panic-handling claim.

**Code-source validation:** [Linux/Windows contract CI #39907661548](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39907661548) on exact test code SHA `a36b9f865e8b84ccd7f34f1ae56c84fd3a342a61`, successful, attempt 1, Rustfmt, runtime `clippy --all-targets --all-features -- -D warnings`, **4 launch-gate tests and 2 inherited Codex close-contract tests on each OS**. Prior Rustfmt-only failures [#39907133492](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39907133492) and [#39907217043](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39907217043) are retained on earlier SHAs and were corrected without weakening strict checks.

The separate hosted-Windows native production-path workflow is deliberately built without the runtime's `#[cfg(test)]` authority gate, so its `direct-revoke-race` scenario **should still succeed as a negative control**. A native control success therefore means the actual production issue remains open; it does not prove the synthetic gate is deployed or closes #317. Do not unify these two evidence classes.

**Important remaining reviewer concerns:** Session record replacement/ABA between acquiring a gate and committing a close still requires exhaustive test-driven design; `research_commit_launch` checks `Arc::ptr_eq` before effect, but production authority has no new such gate. A mutex held during a potentially stuck native `ResumeThread` can delay closure indefinitely. Any robust design must specify explicit bounded native call behavior or a dedicated authority broker; admission drainage/termination uncertainty and `SessionRevokeReport` are still unresolved.
