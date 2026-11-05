# Issue #323 — synchronous original native-ledger closure under session and TaskLease revokers

**Windows 10 LAB, 5 November 2026 — non-executing native stage only. Windows V1 remains NO-GO.**

## Exact starting point

The LAB was freshly cloned from the official `main` at `c1eefb0d591261c57771b30fe3f2f1fa013efc74`, the squash merge of CI-only [PR #513](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/513). The previous fully supervised real-kernel counterexample remains unchanged at research commit `eb100825d3d620fc3a8c2e828b4f5fa41a7138ab`: a RESUME ticket admitted *before* revoke may still physically reach Win32 ResumeThread *after* the issuer is closed. No code from that research branch has been copied into the live GitRead route.

This change does **not** add any real process-creation entry point. `NativeCallStopTracker` stays inert accounting for an unstarted, service-owned GitRead candidate, retaining the actual original kernel Job watchdog and the original four stop signals.

## The exact defect addressed

Before this tranche, original session/TaskLease watch flags and the native ledger stop flag were distinct atomics. The ledger's `track()` final stop observation could succeed, followed by a direct `SessionRegistry::revoke()`, `SessionLifecycleManager::revoke()` or `TaskLeaseRegistry::revoke()` publishing a separate flag, **without closing the same admission CAS bit first**. A later observer might eventually call `request_stop`, but the interval was not synchronized.

## Changed implementation

- Under `#[cfg(windows)]`, each *original* `SessionRecord` and `LeaseRecord` retains weak links to the **same** `Arc<AtomicBool>` returned by `NativeCallStopTracker::internal_stop_signal()`. Weak references do not prolong a process owner or revive a removed registration.
- At `LeaseObservedNativeGitStatusCustody::arm_original_watchdog_inner`, the existing original watchdog is armed first. The ledger remains owned by that custody, and registration is attempted **before it can be returned to a caller**. Failure seals that same ledger and requests the same original watchdog stop. No second issuer is created.
- Joint registration under `SessionRegistry` first obtains the **original session revoke mutex**, checks the original *counted* `OwnedSessionAdmissionPermit` registry identity, registration generation and admitted horizon, then uses a **nonblocking** `TaskLeaseRegistry::try_lock`. Under the original lease revoke mutex, checks exact `TaskLeaseRevocationWatch` Arc pointer identity, original horizon/session, and installs its weak close subscriber. Under the still-held session mutex, installs the same subscriber on the exact original session record. No filesystem hashing, JobStore, Win32 invocation, waiting, or callback occurs under either lock.
- Both direct and lifecycle session revoke call `SessionRecord::close_original_native_calls()` **before publishing** the session's existing observable stop bit, under its own revoke lock. The original TaskLease `revoke`, `revoke_session`, and `remove_session` paths close under their own lease mutex **before publishing** the lease stop bit or discarding its record. `remove_quiescent_inactive` also closes on physical expiry removal.
- The two source locks now serialize **registration and stop publication for these two specific registry sources**. No observation/poll interval remains between their published stop and the original native ledger's CLOSED bit. The *same* bit remains watched by the independent real kernel Job watchdog. Already-committed in-flight Create/Resume tickets are **not retroactively cancelled**; they must be physically reconciled.
- Subscriber lists are pruned of dead `Weak` references during registration and revoke. An opaque reused TaskLease ID carries a fresh record and cannot retarget a prior original generation.

## Evidence — original registries and real original kernel watchdog

Two new nonignored Windows runtime unit tests use actual registries, original clock-admitted session and transport capacity, real scoped TaskLease/executable pin, registered private process reservation, actual Windows constrained **unstarted** Job, the original watchdog, and the original `NativeCallStopTracker`. They never execute the inert test Git executable.

1. `git_read::native_status_tests::original_armed_native_ledger_closes_in_actual_registry_revocation_paths` validates four independent actual stop paths: session lifecycle's early publication, direct TaskLease revoke, TaskLease revoke_session and TaskLease remove_session followed by **reusing the original opaque ID**. For each source it verifies the *exact original ledger bit* became true, new Create/Resume tickets are refused, the pre-admitted Resume ticket remains counted until dropped, the independent watchdog observed the exact stop and completed its original kernel Job termination request, and the original unstarted Job remains empty.
2. `git_read::native_status_tests::direct_session_revoke_closes_original_native_ledger_before_counted_drain` concurrently invokes the actual blocking `SessionRegistry::revoke` while retaining the counted original session/transport owner and an already-admitted native test ticket. It proves the original stop closes and the real Job watchdog completes **before the direct revoker may return**; revoke only finishes after dropping the original owner. No new native ticket is admitted after close.

The first local targeted executions are GREEN. **24/24 independent, externally 15-second-bounded repeated invocations** (12 per test) produce the expected positive markers; none timed out. Their separate raw stdout/stderr and SHA-256 values are private under `_evidence`, with `ISSUE323_SHARED_NATIVE_CLOSE_24_RUNS_MANIFEST.json` SHA-256 **`CF96EFDFA3FD63B09C05B716AA574660866319905203F3290AC7F28CA9D069E7`**.

## What is still NOT proved

**This is NOT a service-owned real CreateProcessW/ResumeThread issuer.** The source-bound registration closes only the existing *non-executing* original ledger. There is still no live GitRead MCP native launch path or first-attempt nonignored native GitRead acceptance.

- The original `HostStopSource`/Drop, policy-epoch/target identity, original absolute deadline and fatal watchdog signal still participate as separate observed sources, **not yet as jointly serialized close/admission authorities**.
- The original native ledger tracks callbacks, but does not control the real OS `CreateProcessW` or `ResumeThread` entry. A ticket admitted *before* stop can still execute a late native call; the preceding Windows kernel RED establishes this unequivocally.
- Even a finished, sealed ledger and acknowledged Job termination request are **not** terminal receipts. Full evidence requires root PROCESS HANDLE signaled, original Job empty, both *independent* stdout and stderr **physical EOF**, stdin/writers and all inherited handles closed, quotas reconciled, no future admissions, and orphan/crash/poison/timeout treated as **UNKNOWN**.
- An owner cannot bypass a failed joint subscriber registration; the failure triggers original `request_stop()` and watchdog closure. This does not make the stage eligible for a production launch.

## Next engineering step

Extend the single owner-close protocol to the ORIGINAL host-stop publication and pinned policy/target/deadline authority *without holding their locks across Win32 or IO*. Then implement the sole native Create/Resume issuer with pre-admitted-call custody and actual physical Job/process/stdio reconciliation. Only after full nonignored Git-through-MCP Windows acceptance, CI/merge evidence, and explicit V1 freeze gates could the Windows V1 change from NO-GO. Keep [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN and blocked [PR #511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/511) UNMERGED.

## Première campagne complète Windows 10 — réussie

Après les corrections de compilation du prototype privé (dont un import conditionnel Rust et une erreur publique contenant un type privé, corrigés **avant** la campagne complète), **6/6 gates locaux ont réussi dès leur première tentative** sur Rust 1.99 verrouillé/hors ligne : `cargo fmt`, crate Windows (**82 pass / 4 ignorés / aucun échec**), crate runtime, workspace complet, workspace all-features, Clippy strict `-D warnings`. Les tests Windows de ce changement incluent les deux contrôles nonignored de fermeture conjointe et les tests de régression des étapes précédentes ; aucune autorité d'exécution GitRead native n'est créée. Les stdout/stderr bruts et SHA-256 sont conservés uniquement dans le LAB privé sous `_evidence`. Manifest `FIRST_ATTEMPT_GATES_MANIFEST.json` SHA-256 **`EE02D2EDBD9518A01EA7D13FBB079A222F26DB04B74F86CCB83E3D970B0489B8`**. La CI GitHub indépendante et le comportement exact de fusion/main doivent encore être établis ; ne jamais confondre ce résultat local avec une validation complète Windows V1.
