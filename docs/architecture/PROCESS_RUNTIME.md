# Process Runtime

## Fundamental API

Processes are launched as structured executable + `args[]` + cwd + controlled environment. Arbitrary shell strings are not the fundamental primitive.

`process_start` returns an opaque `JobId`. Output is consumed with cursors through `process_read`. `process_stop` accepts only a JobId owned by the caller application session.

## Authorization boundary

Process authority is application-owned, not MCP-owned.

1. The bridge operator supplies zero or more classified `--allow-executable=<fixed-tool|interpreter|repository-code>:<absolute-path>` values at startup.
2. Each path is canonicalized to one absolute regular file; duplicate canonical paths fail startup closed so the same binary cannot receive competing classes in one lifecycle.
3. The application creates one exact executable task lease carrying both canonical path and operator-owned `ProcessExecutionClass` (`FixedTool`, `Interpreter` or `RepositoryCode`).
4. `ProcessRun` is present on the session only when at least one classified executable was authorized.
5. An MCP `process_start` request canonicalizes its executable and must match one of those operator-created leases.
6. The server resolves the execution class from that active lease; MCP has no class field and cannot select, downgrade or override it.
7. The request becomes a typed `Effect::ProcessRun` carrying the lease-derived class and must pass `PolicyEngine`, which requires exact path + class agreement.
8. Only then may `ProcessManager` spawn the child.

The MCP caller cannot create capabilities, approve itself, mint a task lease or choose an execution class. Phase 3C3B classification is an operator assertion rather than automatic executable inspection: `FixedTool` does not prove a binary cannot load plugins/scripts. Phase 3C3C1 established deny-by-default for `Interpreter` and `RepositoryCode`. The later C5A-C5G gates selectively re-admit only the exact operator-selected Windows Node profile when the same active lease carries both matching `ProcessExecutable` and `ProcessIsolationEligible` authority and the pinned AppContainer helper is available. Other interpreter/repository-code profiles remain fail-closed with `ProcessIsolationRequired` / `optic.process_isolation_unavailable`; MCP still cannot mint eligibility or widen that profile.

Network remains unavailable in the Phase 1 runtime. A request with `network=true` fails closed even though the core policy model already defines the later dual session+lease network contract.

## Ownership

Every job belongs to exactly one application `SessionHandle`. Job control uses an opaque random `JobId`; no V1 tool can terminate an arbitrary PID.

Cross-session JobId access is deliberately returned as unknown rather than exposing ownership information.

On Windows, every process job owns one kernel Job Object. This is per-job containment owned by a session, not one shared Job Object for the entire session.

## Environment and cwd

- The child environment is cleared by default.
- A request may inherit only variable names that were explicitly allowed by the operator with `--allow-env`.
- A requested cwd is project-relative, canonicalized, required to be a directory and required to remain under the canonical workspace root.
- The executable is the intentional absolute-path exception: it must canonicalize to a regular file and match the operator-created executable lease exactly.

## Windows lifecycle and resource enforcement

Phase 1D uses the Optic-owned `optic-bridge-windows::LimitedJobObject` adapter rather than treating a third-party wrapper as the authority for resource limits.

For every Windows process job:

1. `ProcessManager` validates the requested `ResourceBudget` against application hard limits and the active exact-executable task lease.
2. Tokio command creation is wrapped before spawn.
3. The wrapper forces `CREATE_SUSPENDED`.
4. A new Job Object is created.
5. `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` and `JOB_OBJECT_LIMIT_JOB_MEMORY` are configured from the authorized budget.
6. Phase 3C2 additionally configures `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` before assignment. The current application-owned default is 25% per Job Object (`CpuRate = 2500`); failure to establish that hard cap fails the spawn closed rather than silently running uncapped.
7. The suspended root process is assigned to that Job Object.
8. Only after successful assignment are the child threads resumed.
9. stdout/stderr are drained asynchronously while the runtime monitors timeout, stop and output overflow.
10. `try_wait` / `wait` keep the process job non-terminal while any descendant remains active in the Job Object.
11. Stop, timeout, overflow or a process-observation failure requests Job Object tree termination. The runtime then waits at most two seconds for `wait()` to confirm root exit plus zero active Job Object descendants. If that proof is unavailable, the job becomes `TerminationUncertain`; dropping the final live Job Object handle remains the kill-on-close backstop.

This ordering closes the ordinary root-process spawn-before-assignment window for the owned child. Job Objects provide lifecycle/resource containment; they are not a complete security sandbox or a substitute for a restricted token, AppContainer, VM or Windows Sandbox.

## Non-Windows lifecycle

Development/CI uses a process-group wrapper plus kill-on-drop. This preserves lifecycle semantics for cross-platform tests but does not prove Windows containment; native Windows CI is mandatory for Windows claims.

## Output and registry bounds

- stdout and stderr share one authorized output budget per job;
- only bytes inside that budget are retained in memory;
- overflow sets a truncation flag and triggers process-tree termination;
- if a child exits naturally before the monitor observes overflow, the terminal status is corrected after the drain tasks complete so overflow cannot be misclassified as a normal exit;
- `process_read` is cursor-based and independently capped per call (64 KiB by the current default HardLimits);
- active jobs and retained terminal records are both bounded;
- retained proven-terminal records may be evicted deterministically to admit later work without allowing an unbounded history; `TerminationUncertain` is deliberately not terminal for quota/eviction/reap purposes;
- total reserved output RAM is checked before admitting a new job;
- Phase 3C2 also reserves a fixed application-owned CPU share per active/uncertain job before spawn. Defaults are 25% per job, 75% aggregate across Optic-owned process jobs and 50% per session. `TerminationUncertain` keeps that reservation; retained proven-terminal history does not.
- Phase 3E1 additionally reserves each requested process `memory_bytes` budget across active/uncertain jobs before spawn. Current defaults are 16 GiB aggregate across Optic-owned process jobs and 8 GiB per session, while the existing 8 GiB per-job ceiling remains unchanged. `TerminationUncertain` keeps the memory reservation; a proven-terminal job releases it even if its bounded result/output record is still retained.

The CPU and process-memory aggregate figures are Optic admission ceilings, not measurements of whole-machine utilization. Phase 3E2B (PR #105, merge `aaaf4e17`, exact head `9cb0516f`, CI #414/#415) adds a separate Windows admission guard using the 3E2A physical-memory snapshot: the effective emergency reserve is the larger of 1 GiB or 10% of total physical RAM, and a new job is admitted only when current available physical memory can cover that reserve plus all active/uncertain 3E1 declared process-memory reservations plus the new job's requested memory budget. Counting the full declared reservations is deliberately conservative and can double-count memory already physically consumed; without per-job usage telemetry this is the fail-closed way to preserve growth headroom for already-authorized Optic jobs. Unrelated applications can still consume RAM immediately after the point-in-time observation, so this is an admission-time safety guard rather than control of the whole host. Heavy-task concurrency remains the separate Phase 3E3 admission gate.

## Time and cancellation

Timeout is a hard runtime deadline for normal execution, followed by a separately bounded two-second termination-confirmation window. Explicit stop, `session_cancel`, output overflow and process-observation failure use the same owned-tree termination/confirmation path.

If termination cannot be proven in that window, `process_result` reports `termination_uncertain`, output is marked truncated, drain tasks are aborted/closed, and the record continues to hold its active-job/output reservation and session ownership. It cannot be evicted or physically reaped as a terminal record. This is a quarantine state: it proves neither that the process tree is alive nor that it is dead.

Cancellation propagates session → task lease/job → process tree. Normal timeout, explicit stop and output-overflow paths plus bounded confirmation/uncertain ownership are covered by CI tests.

## Kernel-enforced Windows budgets

The authorized `ResourceBudget` now has distinct enforcement layers:

- `timeout_ms`: runtime monotonic deadline, followed by Job Object tree termination on Windows;
- `output_bytes`: Optic-owned bounded stdout/stderr retention and overflow termination;
- `memory_bytes`: Windows `JOB_OBJECT_LIMIT_JOB_MEMORY` on native Windows, plus Phase 3E1 global/per-session declared-budget admission before spawn;
- `process_count`: Windows `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` on native Windows;
- CPU: Phase 3C2 uses an application-owned fixed per-job percentage rather than a caller field; Windows enforces it with `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`, while `ProcessManager` separately bounds aggregate and per-session reservations before spawn.

The native Windows CI gates prove these limits using dedicated fixtures/tests:

- with `process_count = 1`, descendant creation is blocked;
- with a 128 MiB job-memory ceiling, the fixture cannot reach a 384 MiB allocation target;
- a parent that spawns a descendant is timed out and the descendant is killed before it can write a delayed survival marker;
- dropping a live limited Job Object prevents the child from surviving long enough to write its own delayed marker;
- Phase 3C2 queries `JobObjectCpuRateControlInformation` and proves the 25% hard cap is stored as `CpuRate = 2500`; runtime tests prove global/per-session admission and uncertain-state reservation behavior.

These tests complement, rather than replace, policy/lease ceilings. Authorization decides what execution is allowed; the Windows Job Object contains each process tree within its process-count, memory and CPU ceilings, while Optic's runtime admission bounds aggregate/session CPU and declared process-memory reservations.
