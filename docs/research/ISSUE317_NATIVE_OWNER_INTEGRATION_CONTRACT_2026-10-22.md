# Issue #317 — native Git operation ownership: integration contract and acceptance gates

**Status:** proposed engineering contract, **not accepted for production**. Research branch only.
**Date:** 22 October 2026. Intended owner: runtime / session lifecycle / MCP Git.

## Why a local pipe-join patch is insufficient

The production `HardenedCommandRunner::run_inner` launches a direct child, kills/waits for that child on deadline, then performs blocking `join()` calls on stdout, stderr and optional stdin threads. A descendant holding inherited pipe ends can outlive the direct child and indefinitely prevent the joins. Replacing these joins with detached threads, a longer deadline or a timeout around `join()` would allow outstanding resources and effects to outlive the claimed completion.

The Git integration path can perform an externally visible `update-ref` CAS before the runner returns. A timeout therefore has **two independent dimensions**: native execution/cleanup (`TerminationUncertain`) and target mutation outcome (`GitRefOutcomeUncertain`). Neither condition authorizes an automatic retry or a terminal success/failure claim about the other.

## Existing measured evidence (not a new production implementation)

| Scope | Exact source and evidence | Interpretation |
|---|---|---|
| Native inherited-pipe liveness | issue #317 and original supervised Rust fixture investigations | Direct-child exit does not imply that pipe readers have reached EOF |
| Retained-owner fixture, partial failures | 17 Linux / 16 Windows cases on source `06d688b6`, workflows #39688523435 and #39688523364 | A controlled *research* owner can retain `Uncertain`; does not implement bridge lifecycle |
| Actual transport and session tests | CI #39693485443, #39701561425 | Normal `TransportGuard` saturation blocks normal `session_cancel`; synthetic owner can retain admission/capacity |
| Model destruction | CI #39702709032 and #39704081003 | Dropping model `Vec` releases RAII permits, allowing actual lifecycle seal/reap without synthetic native terminal proof |
| Native abnormal-owner termination | commit `59b96d662fa553d58f0d819470b57e317e3ff657`; #39707390217 | Windows synthetic kill-on-close Job empties; Linux ordinary group leaves a live adopted Rust holder until independent recovery |
| Normal CI at that source | #39707390159 | Windows Rust, Linux Rust and dependencies passed; does not validate a production native owner |

## Required resource/authority model

1. **Bridge-owned immutable operation ID.** Mint an opaque ID at admission. Bind it to authenticated principal, project, session and task lease, plus the approved Git executable identity, canonical repository root, operation kind, scopes and effective resource budget. No arbitrary PID-based stop or model-provided authorization. Startup recovery is a distinct `HostStartup` authority; never invent a session grant for it.
2. **Finite pre-spawn reservation.** Reserve process count, memory, CPU and bounded stdout/stderr/stdin bookkeeping before any native creation attempt, including a capacity for records whose termination is uncertain. If no reservation or required OS isolation is available, reject **before** invoking native creation. A moved/cancelled MCP waiter must not drop the owner's reservation.
3. **Atomic adoption or fail-closed startup.** Reserve owner record first. Windows child creation must be suspended; assign it to a properly restricted Job with limits **before resume**. Any partial spawn/assignment failure remains owned until native termination and I/O completion are proven. Linux launch must provide independently demonstrated whole-owned-tree containment, not merely a PGID or a direct-child pidfd.
4. **Explicit I/O ownership.** The owner retains actual handles, readers, writer and join identities, bounded buffers and input/output ceilings. The worker cannot report `Quiescent` until it can demonstrate native tree terminality **and** that all relevant I/O is cancelled/drained and joined. A status string or caller-supplied boolean is not proof.
5. **Out-of-band, authorized stop.** The session lifecycle needs a small, trusted stop control that does not acquire an ordinary, already-saturated `TransportGuard` request slot, does not run as a user-supplied PID action, and is usable after admission is closed. Its identity and resource bounds must be explicit. A public MCP cancellation tool can request it only after normal authentication; no emergency unauthenticated endpoint.
6. **Lifetime and shutdown.** The owner is held by a long-lived bridge-owned service independently of MCP caller futures. A normal `Drop` of an in-memory registry is *not* a satisfactory uncertainty policy. Host shutdown must request owned stops, observe terminal and I/O proofs within fixed deadlines, retain/report uncertainty otherwise, and define OS-backed cleanup behavior if the host process itself dies. Merely returning an error that can be dropped is insufficient.
7. **Session lifecycle gates.** Separate revocation flag/close-new-admissions from waiting for existing admissions to drain. Signal the owners *before* waiting, then join/drain admitted work. `seal_quiescent` and `try_reap` must consult outstanding session-bound Git owner records, and reject sealing/reaping on `RetainedUncertain` regardless of permit lifetime. The startup Git-recovery owner must not be tied to session reap.
8. **Side-effect outcome independently retained.** Preserve the requested expected CAS head, operation ID, and known/uncertain update outcome in a bounded recovery journal. `TerminationUncertain` must not be mapped to `NoMutation`; `CommandTimedOut` after `git update-ref` may conceal a successful mutation. Reconciliation uses exact ref/version evidence before any new mutation.

## Minimal state flow (architecture only)

`Reserved` -> `NativeCreationInFlight` -> `SuspendedOwned` -> `RunningOwned` -> `StopRequested` -> `NativeTreeProvenTerminal` -> `PipeIoCancelledAndJoined` -> `Released`.

At any transition after native creation starts, missing proof yields `RetainedUncertain`. It holds the owner record, resources, native handles and session quiescence gate. Timeouts and worker panic cannot transition directly to `Released`. A verified pre-spawn failure may release reservations only when no process could have been created.

## Cross-platform feasibility gate — not solved

| Windows | Linux |
|---|---|
| A suspended, restricted Job Object with kill-on-last-handle-close was demonstrated in a synthetic crash. Still prove limits, nested-job behavior, no breakaway, handle inheritance, cancellation and I/O joins in the **real** Rust integration. | Process group/session ID, direct pidfd, `PDEATHSIG`, or a Python test subreaper alone do not guarantee termination/reaping of escaping descendants on Bridge crash. Determine whether a suitable dedicated per-job supervisor or delegated cgroup v2 boundary is actually available in the supported environment. |
| Verify native tree quiescence, then join bounded I/O; do not assume `TerminateJobObject` synchronously proves both. | Require native membership/reaping proofs, restart/crash recovery tests and independent supervision without a global Bridge-wide subreaper or arbitrary PID actions. |

**Fail-closed decision gate:** if a supported operating-system environment cannot establish the required containment and reclaimability, the corresponding Git executable operation must remain unavailable or explicitly classified as unsupported; do not silently fall back to unbounded best-effort child kill. The precise Linux mechanism is **undecided** pending proof; this document does not declare cgroup delegation universally available.

## Sequence and deadlock review

Under a held Git admission with the only normal transport slot occupied, today `session_cancel` can return `optic.server_busy` and `SessionRegistry::revoke` waits for the admission before `ProcessManager::cancel_session`. A future stop invoked *only after* that wait cannot resolve a Git command whose pipes are still held. Required ordering: authenticate and close new admissions -> request owner stops using reserved trusted control -> confirm native/I/O cleanup or mark explicit uncertain -> drain admissions -> revoke subordinate leases -> gate session seal/reap. Audit races between stop, launch, owner insertion, expiry and host shutdown. Stop-request acknowledgements must not be mistaken for completion.

## Acceptance matrix before a production patch

| Proof | Expected negative/control | Acceptance |
|---|---|---|
| Windows native spawn containment | Fail assignment / Job limit setup before resume | No fixture code executes without assigned limits; tracked partial failure is recoverable |
| Linux native descendants | `setsid` escape and parent crash with pipe inheritance | No unowned live tree or unjustified quiescence, or deny launch before side effect |
| I/O | stdout/stderr/stdin held open or failing to start/read/write | Return bound or explicit retained uncertainty; never detached unaccounted reader/writer |
| Revocation under one-slot saturation | Git worker holds admission, normal MCP cannot acquire slot | Trusted stop progresses without granting client bypass; seal/reap stay blocked until proof |
| Caller cancellation and Drop | Caller future/owner registry dropped | Resources remain accounted; no false seal or premature reservation reuse |
| Host shutdown/crash | Stop during spawn, escaped child, crash between CAS and observation | Native containment/recovery plus truthful uncertain outcomes on restart |
| Git CAS | `update-ref` succeeds but child remains blocked past deadline | Preserve potentially committed mutation, verify ref before operator-authorized retry |
| Security and resource budgets | Cross-session cancel, oversized output, CPU/memory/process-slot exhaustion | Denials before side effects; no arbitrary PID or ambient credentials |

All applicable checks must be bounded, independently supervised and run on hosted native Windows and Linux for the exact source SHA, plus a full normal CI pass and review of first failed attempts. They are **not** satisfied by the existing test models.

## Implementation staging recommendation (not authorization to merge)

1. Write a dedicated reviewed ADR and narrow owner API only after selecting a **proven** containment design for each supported OS. Keep it separated from transport and Git mutation semantics.
2. Add native owner construction/stop/cleanup tests under independent watchers, with fail-closed unsupported-platform behavior. No production Git call-site rewiring yet.
3. Wire lifecycle revoke/seal/reap and host-startup owner separately; add saturation/failure-injection regression; prove held capacities and removal/restart policy.
4. Migrate one Git read-only code path first on an isolated branch cut from contemporary `main`, review API and resource accounting; move CAS mutation paths only after separate outcome-journal proof.
5. Do not open another PR while #318 is the sole unresolved candidate; never merge the entire long-lived research branch.

## Repository/CI hygiene

The old research workflow `.github/workflows/issue317-owner-study.yml` previously matched all `scripts/research/issue317_*.py`; consequently adding an *independent* experiment launched the full earlier 17/16-case study again (normal workflow #39707390158). Narrow that path filter to its three actual scripts, its fixture, workflow, and original research doc. This avoids repeating expensive native research cases for unrelated new #317 scripts. The dedicated abrupt-exit workflow retains its own path trigger. The filter change should be validated by inspection and standard GitHub Actions execution; **no claim** that unmodified future workflows have been empirically skipped until such an event occurs.

**Evidence scope:** proposed design and CI-trigger correction only. `main`/production unchanged. #318 remains independent and blocked on its own Windows MCP initialization failure. Issues #185 and #319 are distinct.
