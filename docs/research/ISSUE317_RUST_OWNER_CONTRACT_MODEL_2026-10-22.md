# Issue #317 — Rust owner/admission contract model (research only)

**Date:** 22 October 2026. **Branch:** `bootstrap/issue-317-owned-cleanup-study`; not merged into `main`.

## Purpose and evidence boundary

This checkpoint concerns an **isolated Rust test model**, `crates/optic-bridge-runtime/tests/issue317_owner_lease_contract.rs`, using the actual `SessionRegistry`, `SessionAdmissionPermit`, `TransportGuard` and `RequestPermit` types. It is **not** the production `HardenedCommandRunner`, a native process owner, or an implemented lifecycle hook.

Prior validated model commit: `85ffc042c00696ea90b4f212a406d2c613f34002` — [normal CI #39700522911](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39700522911), **completed/success on attempt 1**, Windows native, Ubuntu and dependency policy; both original model tests observed `ok` on both OS. Its predecessor `87b4f7f1550e4348c3bed35211159453e9caa021` exposed a rustfmt failure in Ubuntu before being superseded/cancelled by the formatting correction. Preserve the original first-attempt result.

## Model assertions and incremental test scope

- A session-owned reservation is rejected unless its **actual** `SessionAdmissionPermit::grant().handle` matches the declared session. HostStartup has no fake session admission and does not impersonate transport.
- Finite owner capacity is reserved **before** a native creation attempt. The model explicitly separates `Reserved`, `NativeCreationInFlight`, `SuspendedOwned`, and `RunningOwned`.
- A reservation may be dropped as a pre-creation failure only **before** native creation begins. Once creation has been attempted, even failed setup before resume requires cleanup evidence.
- Stop request is modeled as an owner-scoped control operation that does **not** consume the already-saturated ordinary transport slot or block on session-admission draining.
- Injected uncertain cleanup retains the owner record, transport request capacity and session admission. A separate session revocation waiter cannot complete until the admitted owner is released.
- Resource release in this **synthetic state machine** requires the ordered `NativeTreeProvenTerminal` and `PipeIoCancelledAndJoined` states. Sealing must be denied while an owner remains.
- Cross-session stop denial and a separate `HostStartup` owner are checked; normal and uncertain cleanup states must not free another authority's resources.

**Extended state-model validation:** [`9d126cdd307c86fed60375f921f29e3e6be5686d`](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/commit/9d126cdd307c86fed60375f921f29e3e6be5686d), [normal CI #39701561425](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39701561425), **completed/success (attempt 1)** on native Windows, Ubuntu and dependency policy. All three contract-model tests passed on both operating systems. The earlier `73cf8512c43f32d66fde971fdcd60e8a275f4647` attempt failed Ubuntu rustfmt before supersession; it is not retroactively green.

## Explicitly unproved and unsafe to assume

1. There are **no actual native child processes, Job Objects, process groups, pipe readers, writer threads, child handles, CPU reservations, network containment checks, or crash cleanup** in this model. The functions named `prove_native_terminal` and `prove_io_joined` are **test-injected assertions**, not OS attestations; production must never expose these as untrusted booleans or caller-controlled claims.
2. The `Authority` enum in the model is **not authenticated**. It demonstrates a separation requirement, not a production cancellation capability. Host/session principal binding, transport authentication, revocation authorization and fairness remain design work.
3. The model's `may_seal` is **not wired** into `SessionLifecycleManager::seal_quiescent`/`try_reap`, and its in-memory reservation alone cannot prove fail-safe behavior on global shutdown or native process escape. Dropping a test registry releases RAII permits; a production uncertain owner cannot be silently discarded.
4. A pre-spawn intent transition does not itself establish ownership of a native process; Windows assignment-before-resume and Linux escaping descendants/reaping still require **separate, native, supervised evidence**.
5. `GitRefOutcomeUncertain` (CAS may have committed despite timeout) is independent from `TerminationUncertain` (native cleanup not proven). This model proves neither actual Git outcome nor retry safety.
6. Existing #317 native fixtures (17 Ubuntu / 16 Windows Astra cases) and actual `git update-ref` timeout ambiguity are prior experimental evidence, not an integrated production fix. #318 remains unmerged with its own Windows MCP-initialize CI failure.

## Next integration gates

Before proposing production changes: design a bounded and authenticated stop authority available under transport saturation; provide an OS-native owner retaining real handles, pipe I/O and reservations under partial startup/cleanup failure; link retained owners into session seal/reap **and host shutdown**; prove Windows and Linux negative cases under independent supervisors and exact-head CI; keep `Session` and `HostStartup` ownership separate. Avoid rerunning prior evidence without a new falsifiable hypothesis. No new PR while #318 is blocked, no direct `main` change, no user-PC execution, and no security/time-budget relaxation.

## Destruction-time negative control — newly added, acceptance pending

An additional test, `research_registry_drop_recycles_capacity_despite_uncertain_cleanup`, deliberately puts the **test-only** owner in `RetainedUncertain`, then drops the entire `ContractRegistry` instead of calling its guarded `release`. Because the model stores real RAII admission and transport permits in an ordinary `Vec`, destruction **releases the permits without proof of native cleanup**. This is a **negative characterization of the experimental model**, not proof of such a defect in a shipped native Git owner (none exists). The test asserts the actual permit counters and `SessionRegistry::revoke` behavior after Drop so that this limitation is recorded as executable evidence.

**Hypothesis:** in this synthetic model, all protection is confined to explicit transition methods. The Rust destructor does not enforce the `RetainedUncertain` policy. If the new native CI confirms this characterization, the next design must make owner lifecycle and shutdown a hard acceptance gate, not merely add more transition states.

**Required production decision before ownership integration:** the host-owned service must keep authority and reservation of uncertain native resources throughout normal shutdown; fail closed and report incomplete teardown without advertising session quiescence, and establish OS-managed cleanup/recovery for an actual process crash or host termination. A test mock that simply `Drop`s an RAII field or suppresses Drop cannot prove containment; a model-only `try_shutdown` that returns an error is also insufficient if its error is itself dropped. Windows Job lifetime/kill-on-close and Linux escaped-descendant rules require separate OS-native proofs. Do not confuse intentional Bridge process exit with arbitrary runner disconnection.

**Status:** the new negative test and this paragraph are research-only; their exact new commit and CI must be checked before claiming validation. Do not modify `main`, merge #318, or alter production `HardenedCommandRunner` based solely on this model.
