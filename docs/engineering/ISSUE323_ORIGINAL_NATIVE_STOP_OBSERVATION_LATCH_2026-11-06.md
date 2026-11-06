# #323 — Latch observed original authority stop / absolute expiry into the exact native ledger bit

**6 November 2026 · Windows 10 isolated LAB · non-executing original native GitRead stage ONLY. Windows V1: NO-GO.**

## Baseline and exact narrow defect

- Base is [stacked draft #516](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/516), immutable `90fbe2a4378ed57e59a6f3d46626a5b2aa3ae5cb`, itself above [draft #515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/515). `main` remains `203309227612ccf2be2438b4a808c3c82c78d56a`; nothing is merged.
- `NativeCallStopTracker::Shared::is_stop_required()` previously returned **true** upon observing an original session/TaskLease/Job/Host signal or its original `Instant` deadline without *also setting* the `shared.stop_requested` bit already watched by the same original kernel Job watchdog.
- This was a **read-side latching gap**, not a claim that a new Win32 call was admitted. The admission checks already returned a denial. It also was not a fix for the source's own synchronous stop publication.

## Test-first genuine RED

A new nonignored Rust unit test `native_inflight_calls::tests::observed_original_sources_and_expiry_latch_exact_native_watchdog_stop` used the original four independent authority signals, verified the *exact* original `internal_stop_signal()` bit was initially false, then published each original stop source or supplied an already-expired absolute deadline. The contract requires the observable native stop bit to have become irreversibly true after `stop_requested()` returned true. On unmodified #516 source, the test ran **1 FAILED / 0 ignored, exit 101**, marker `RED #323: original source 0 observation must latch the exact native watchdog bit`.

Private raw RED stdout SHA-256 `91AC118E3500F745A22E79D8E64CED67961E264ABCC1C9AB1A0FA3C13C185A31`; stderr SHA-256 `4E87904BD2DDBF8B54230DA54DE88E5B133896A33C516A5126ECF4EB3CE0D13C`. The test uses no child, real kernel call or synthetic authority minting.

## Focused correction / no new authority

`Shared::is_stop_required()` now first checks existing internal closure, then the already fail-closed poisoned ledger. When it observes **any** retained original stop or `Instant::now() >= original.deadline`, it **stores true with Release ordering into that same original internal Arc<AtomicBool>** before reporting stop. It never clears, retargets, clones an issuer or resets the original deadline.

A concurrent native call already counted before this observation stays counted until the borrowed ticket returns. This is only a **read-side irreversible latch**: it does not guarantee that the original source publication synchronously sets the internal bit without an observer, it does **not** prevent late OS effects by a previously admitted syscall, and it does not let the source bypass the separate native issuance requirements.

No new dependency, unsafe code, Win32 call, watcher thread or production native GitRead launch path is added.

## GREEN and verification

- Same original RED test becomes actual **1 PASS / 0 ignored**.
- The real original Windows **unstarted kernel Job, original watchdog and original session/lease/host stop integration test** is strengthened to require the exact internal ledger bit after observing the **original Job stop**. HostStop's stronger *pre-publication* closure from #515/#516 is preserved.
- Existing original deadline tests now explicitly require the exact internal bit after precommit expiration and after an already admitted native call retained through deadline expiration.
- Separately supervised **60/60 PASS** (five exact nonignored tests, 12 repeats each, each externally 15 s bounded): sources + already expired deadline; precommit expiry; original generation/counted ticket; original deadline with retained ticket; real original Windows kernel Job/watchdog. Raw stdout and stderr, exit codes, timeouts, SHA-256 values and every selection count retained under private `_evidence`. Manifest SHA-256 `A1D31980ECC63FBF046F5836E6FAFF2AD1FDC73D00AEBFA0929D35411AB94F39`.
- The **first-attempt full Windows 10 local campaign finished 6/6 PASS** on the tested source tree, with pinned Rust 1.99 and `--locked --offline`: cargo fmt, Windows crate, runtime crate, workspace, workspace all-features, and strict Clippy `-D warnings`. Raw stdout/stderr and SHA-256 are separate from repeated tests under private `_evidence/LATCH_FIRST_*`. First-attempt `LATCH_FIRST_GATES_MANIFEST.json` SHA-256 `3F2A67987AC0728CA08B5032640CF089E2B58EADA308A645F5550222F97D3360`. No retry-to-green or hidden validation failure.
- Independent hosted PR CI must be assessed separately; **this does not turn a unit test into MCP native acceptance**.

## Remaining hard blockers

Still no service-owned real `CreateProcessW` / `ResumeThread` issuer, no joint policy epoch/target/deadline launch fence, no proof of already-admitted late native syscall effects, no original root HANDLE signaled + Job empty + both independent physical stdout/stderr EOF + stdin/inherited writer closure, no UNKNOWN custody for crash/stall/error or complete nonignored Git-through-MCP acceptance. Existing #185 intermittent Node/AppContainer first-attempt main failure is not solved. Keep [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) open, Windows V1 **NO-GO**, and parent PRs #515/#516 open and draft/unmerged. No installer/release or changes to daily Bridge/Desktop/REL.
