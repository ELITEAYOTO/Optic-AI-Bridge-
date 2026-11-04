# Issue #185: bounded original-process memory observations (2026-11-04)

**Status: research only. No root cause or release acceptance claim.** This branch extends the already-tested fixed-label AppContainer OS observations, on top of research commit `3e1ddea9de40dfa1f28d62d5e672ac9bfb94eaf4` (whose parent was `main=89ea451d89cb125d6517949c83660465315ff475`). Production `main`, [blocked PR #511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/511), and the Windows V1 NO-GO gate are unchanged.

## Why this evidence is needed

GitHub-hosted Windows CI has naturally exhibited at least two different [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) Node/AppContainer timeout signatures: (1) the original target was created and resumed but did **not** emit `NODE_PROBE_STARTED`; (2) `NODE_PROBE_STARTED` and `NETWORK_ATTEMPT` were emitted but the child still failed to exit within the fixed process deadline. Neither absence of a successful socket connection nor zero CPU proves a valid network-denial result or a stalled loader.

## Narrow change

- Enable the pinned `windows` crate's `Win32_System_ProcessStatus` module **only for existing Windows dependencies**.
- In the existing **opt-in** fixed CPU/IO/handle checkpoint, call Win32 `GetProcessMemoryInfo` on the **same original owned PROCESS handle**, into an exact `PROCESS_MEMORY_COUNTERS` buffer.
- Emit only bounded numeric fields: `process_working_set_bytes`, `process_peak_working_set_bytes`, `process_page_faults`; return the literal `unavailable` on query failure, **not zero**.
- No new PID/TID opening, process enumeration, credentials, path output, file/ACL/network capability, duration grant, host-stop admission, or change to success assertions. No synchronous Wait Chain Traversal is introduced.
- A working set or page-fault delta is only a **hint of OS activity**; it neither attributes the activity to Node JavaScript nor identifies all mapped loader IO. A single snapshot (or zero counter) is nonconclusive.

## RED / GREEN from real Windows kernel on local Windows 10

The real original System32 `cmd.exe` AppContainer test stays suspended and is terminated on the existing 25 ms deadline. Before the code change, its new fixed-field assertions failed **behaviorally** (`running 1 test; 0 passed; 1 failed`, Cargo **exit 101**), with the original CPU/IO/handle labels present and memory labels absent. The first attempt with an incorrect `--exact` filter ran **0 tests**, and is expressly **excluded** from RED evidence. The corrected exact test is `appcontainer::tests::suspended_timeout_reports_kernel_cpu_on_bounded_termination`.

After the change, the same real kernel fixture succeeds **GREEN**, with additional pre-termination assertions that each memory field parses as a real numeric OS value on the original PROCESS handle (a mere `unavailable` label is not accepted by this fixture).

**Final local gate, Rust 1.99.0 `--locked --offline`:** 12/12 independent exact Windows fixture tests; Windows crate **82 passed, 4 ignored**, AppContainer network gate **4 passed**; whole workspace **810 passed, 15 ignored** in both standard and all-features configurations; strict Clippy `-D warnings`, rustfmt `--check`, `git diff --check` PASS. Exact raw logs, per-log SHA-256 and validation summary are retained in the **untracked** `_evidence` directory of the dedicated authorized Windows LAB, never committed.

## Acceptance limitations and next action

The fixture only demonstrates working observation and finite termination in one kernel scenario. No naturally failing Node instance was observed in this change; it does not solve #185 or rehabilitate the **first-attempt RED** Windows CI on PR #511, [run 41885824035](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41885824035).

Run the existing hosted Windows CI once on the **exact research commit** after source publication, preserve that first-attempt result, and correlate any eventual natural Node timeout's CPU, IO, handles, working set and fault counters. Do not rerun-to-green, extend the 10-second bound, weaken the network-denial test or fuse this research into `main` absent issue acceptance. #185 and #323 remain OPEN, Windows V1 **NO-GO**.
