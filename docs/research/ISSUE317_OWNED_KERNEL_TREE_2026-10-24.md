# Issue #317 — Counted session admission through native kernel-tree termination (research)

**Status:** test-only, no production runner integration, no V1 approval, no client-facing capability. Date: 2026-10-24.

**Ancestry:** branch `research/317-owned-native-kernel-tree-2026-10-24` is intentionally based on previously validated research `research/317-owned-native-retention-2026-10-23=555d32ff9a20e23a4ffb10efca4daf7dff2c589b`, itself based on production `main=100c43b53c795be5678c8b4f2c0af4353ca5b196`. That earlier test-only branch already proves the short generation-bound commit under `SessionRegistry.sessions` and real `JobStore` accounting. This research adds actual Windows kernel containment **without merging either research branch into main**.

## Targeted invariant

An independently owned `OwnedSessionAdmissionPermit` and an existing genuine `ProcessManager::reserve_native_private` record travel to the same worker. The worker prepares a **real kernel Job** with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, an active-process cap and memory cap before any Win32 process creation. It commits its reservation once against the original admission generation, then calls `CreateProcessW` with `PROC_THREAD_ATTRIBUTE_JOB_LIST` and `CREATE_SUSPENDED`, for the **fixed current libtest executable only**, no inherited handles and a minimal explicit environment. There is **no** post-spawn Job assignment or child resume. A stop check is performed before creation; concurrent stop arriving after this check must be handled after creation, not mistaken for guaranteed prevention.

When a genuine direct registry revoke publishes the original stop bit, the worker observes it while still owning the original permit, reservation and live suspended child; direct revoke **must not return** during this pending native state. On an external finite test release gate, the worker terminates the exact Job, obtains a positive *root exit* receipt and *Job empty* accounting result, marks the JobStore record stopped, then releases the owned admission; only then may the synchronous registry drain return. Unrelated session remains usable; early quiescent removal fails.

This is a **deterministic, live synthetic ordering**, not a production stop controller. The externally controlled gate simulates native stop delay, and the test does not actually exhaust Win32 observation attempts.

## Safety boundaries

- Uses a `windows.workspace` dependency only in `cfg(windows)` development dependencies of the Rust runtime. The extra code is under `#[cfg(all(test, windows))]`; production library executable and tool routes are unchanged.
- Kernel Job handle, suspended process and thread are all strongly owned; the Job's last-handle close terminates any held child if the test panics. No arbitrary PID access, shell command, user project, network, token, secrets, workstation, Git executable, policy mutation or result publication is involved.
- This proof intentionally reimplements a **small fixed Win32 test fixture** instead of exposing the unsafe test launcher as a general-purpose public runtime API or importing the entire Codex branch. It does not authenticate typed GitRead, a task lease, executable pin or policy provenance (#182).
- It does not show surviving creator-host crashes while holding a real session and JobStore simultaneously. The previous [independently supervised kernel Job host-death research](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6375807549) independently proves the actual creation/host-death property on a fixed Windows child; this new test addresses the **in-process authority/drain/ownership composition**, not another host-crash proof.
- A production worker must never drop a counted permit on an arbitrary deadline, guessed termination, a failed Job query, stream EOF alone, or a failed stop request. A persistent observation error can still leave graceful revoke blocked. The lifecycle's native out-of-band stop, durable owner root, pipe/descendant EOF and uncertainty retention are **not implemented** by these synthetic tests.
- This does not fix the production `LimitedJobObject::wrap_child` post-spawn association gap. PR #327 is separate and issue #323 remains NO-GO.

## Required evidence

Run pinned Rustfmt, strict all-targets/all-features Clippy, full runtime library tests on Windows and Ubuntu, and one explicit named Windows integration regression on the **exact source SHA**. Preserve initial failures. Do not label the fixture successful before actual CI logs show the test executed and native Job containment proved; do not merge the research tree wholesale. If later selected for production, design a sealed root-held owner/ticket and additional hosted native failure probes before using any real GitRead path.
