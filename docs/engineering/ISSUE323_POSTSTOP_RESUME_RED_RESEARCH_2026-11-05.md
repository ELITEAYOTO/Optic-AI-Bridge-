# P0 #323 — Windows Job-assigned root resumed after synthetic original stop (5 November 2026)

**STATUS: deliberately RED experimental contract, not a fix or production permission. Windows V1 NO-GO.**

## Exact code lineage

- Research LAB: `E:\Optic-IA-Bridge\optic-p0-native-postrevoke-resume-2026-11-05` on authorized Win10 `DESKTOP-IOSTF61`.
- Base is the **still-unmerged** PR [#511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/511), exact head `292ae8d9fa919f83bb520f5c59800080e89bc555`, not the current `main=89ea451d89cb125d6517949c83660465315ff475`. This base provides the existing private `wrap_child_with_before_resume` hook; it is **not** present in main.
- The research change is one intentionally **ignored** Windows-native acceptance test in `crates/optic-bridge-windows/src/job.rs`; normal production remains unchanged. No change to normal process authorization, session registry, permissions, budgets, MCP routing, network, install, release or CI configuration.

## Precise observed interleaving

1. The test creates a real Windows `System32\cmd.exe` with native `CREATE_SUSPENDED | CREATE_NO_WINDOW`. Its stdout is **the OS handle of a zero-byte temporary file**, created and owned by the test; no shell redirection is used.
2. Independently duplicates the **original process HANDLE** as test-only emergency custody. Confirms real PID and an unsignaled root.
3. Invokes `LimitedJobObject::wrap_child_with_before_resume`, which creates the original kernel Job and calls `AssignProcessToJobObject`. At the exact test hook, confirms original Job `ActiveProcesses=1`, root unsignaled and stdout file still **zero bytes**.
4. Publishes a synthetic original-generation stop as `AtomicBool::store(true, Release)` before the wrapper's genuine `resume_process_threads` → `ResumeThread`. **This is a model stop flag, NOT an actual SessionRegistry/TaskLease revoke**; the live production wrapper has no binding to that stop generation.
5. Actual current wrapper still resumes the root. Test retains its Job/child, waits up to 3s for the exact original PROCESS HANDLE to become signaled and reads `REAL_CHILD_ENTERED` written through the inherited stdout OS handle **after** stop. It prints `ISSUE323_POSTSTOP_RESUME_EXECUTED` with real PID, then intentionally fails the acceptance assertion.
6. Test-only emergency Drop requests termination and waits on the separately pinned original HANDLE (5s cap), then removes its own temporary marker. No unrelated process is killed; no external project or network is used.

## RED integrity

- An initial attempt failed to find its marker because a `cmd.exe` shell-redirection/quoting fixture was unreliable. **Excluded, not counted as a valid RED**. Replaced by preopened stdout file with zero-byte assertion *before ResumeThread*.
- Corrected actual Windows fixture: `running 1 test`, `FAILED; 0 passed; 1 failed`, Rust exit **101**. The terminal record positively shows the original PID, the synchronous original stop at the boundary, and real post-stop stdout `REAL_CHILD_ENTERED`.
- **12/12 independent repeated executions** of this exact Rust fixture on the same Win10 kernel observed the actual red, actual marker, PID and exit 101. All raw logs retained outside Git in LAB `_evidence`. Validated repetition manifest SHA256 `8C02EB5D1E137E04EA694DBB6219AE377E0BC79E90C0DB37DE5D82B15679E74F`.
- Once preserved, test is annotated `#[ignore = "...known RED..."]` so ordinary full workspace/hosted CI **must not be misreported as passing this contract**. To run the red acceptance explicitly: `cargo +1.99.0 test --locked --offline -p optic-bridge-windows --lib research_poststop_suspended_root_must_not_execute_after_resume_boundary -- --ignored --exact --nocapture --test-threads=1`.

## Why this does NOT prove / solve P0

- The callback publishes an **injected model stop**, not the genuine shared session/TaskLease/policy/host revoke. The result proves only the **lack of that binding in this Job wrapper**. It is NOT evidence of real GitRead after session revoke (independently established in draft #355) nor evidence of an already-in-flight `CreateProcessW` race.
- An ordinary check of `AtomicBool` before `ResumeThread` would itself be raceable against stop publication one instruction later. Do **not** "fix" the test with a standalone snapshot or sleep. A legitimate service-owned issuer must synchronize each exact original generation, atomic no-new-Resume admission, recorded in-flight OS calls, prompt stop, original Job/root/stdio custody, quiescent physical receipts, and unknown-state retention. Never hold registry locks over potentially blocking Win32 calls. The specific target test only becomes meaningful as GREEN when the **real service issuer** and full stop contract are used.
- Historical PR #511 first-attempt [Windows CI #41885824035](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41885824035) RED on separate intermittent #185 Node launch. Research does not erase that failure; **do not merge #511 or this branch on the strength of a green unrelated CI**. #185 remains open; #323 remains open; Windows V1 **NO-GO**.

## Next legitimate implementation milestone

Design the single no-new-native-call admission/close boundary shared by **all** direct and lifecycle session/lease/policy/host revokers, with counted in-flight `CreateProcessW` and `ResumeThread` tickets and original immutable custody. Prove **real** stop-before-resume both with no effect and prompt physical Job termination; then prove stop-during-create, stop-during-resume and lost guardian, before connecting MCP `git_status` from the legacy `HardenedCommandRunner`. Stop if any physical or generation evidence remains uncertain.
