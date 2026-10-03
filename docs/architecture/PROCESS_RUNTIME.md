# Process Runtime

## Fundamental API

Processes are launched as structured executable + `args[]` + cwd + controlled environment. Arbitrary shell strings are not the fundamental primitive.

`process_start` returns an opaque `JobId`. Output is consumed with cursors through `process_read`. `process_stop` accepts only a JobId owned by the caller application session.

## Authorization boundary

Phase 1C process authority is application-owned, not MCP-owned.

1. The bridge operator supplies zero or more `--allow-executable` values at startup.
2. Each value is canonicalized to an absolute regular-file path.
3. The application creates one exact executable task lease for that session lifecycle.
4. `ProcessRun` is present on the session only when at least one executable was authorized.
5. An MCP `process_start` request canonicalizes its executable and must match one of those operator-created leases.
6. The request becomes a typed `Effect::ProcessRun` and must pass `PolicyEngine` with the active session plus the exact task lease.
7. Only then may `ProcessManager` spawn the child.

The MCP caller cannot create capabilities, approve itself or mint a task lease.

Network remains unavailable in the Phase 1C runtime. A request with `network=true` fails closed even though the core policy model already defines the later dual session+lease network contract.

## Ownership

Every job belongs to exactly one application `SessionHandle`. Job control uses an opaque random `JobId`; no V1 tool can terminate an arbitrary PID.

Cross-session JobId access is deliberately returned as unknown rather than exposing ownership information.

## Environment and cwd

- The child environment is cleared by default.
- A request may inherit only variable names that were explicitly allowed by the operator with `--allow-env`.
- A requested cwd is project-relative, canonicalized, required to be a directory and required to remain under the canonical workspace root.
- The executable is the intentional absolute-path exception: it must canonicalize to a regular file and match the operator-created executable lease exactly.

## Current platform lifecycle

### Windows Phase 1C

The runtime uses `process-wrap` with its Tokio Job Object wrapper plus kill-on-drop. That wrapper creates the child suspended, assigns it to the Job Object and resumes it, closing the ordinary spawn-then-assign escape window for the owned root process.

The runtime then asynchronously drains stdout/stderr and monitors timeout, explicit cancellation and output overflow. Owned process-tree termination is requested through the wrapped child lifecycle.

This is lifecycle containment, not a complete security sandbox.

### Non-Windows Phase 1C

Development/CI uses a process-group wrapper plus kill-on-drop. This preserves lifecycle semantics for cross-platform tests but does not prove Windows containment; native Windows CI remains mandatory for Windows claims.

## Output and registry bounds

- stdout and stderr share one authorized output budget per job;
- only bytes inside that budget are retained in memory;
- overflow sets a truncation flag and triggers process-tree termination;
- if a child exits naturally before the monitor observes overflow, the terminal status is corrected after the drain tasks complete so overflow cannot be misclassified as a normal exit;
- `process_read` is cursor-based and independently capped per call;
- active jobs and retained terminal records are both bounded;
- retained terminal records may be evicted deterministically to admit later work without allowing an unbounded history;
- total reserved output RAM is checked before admitting a new job.

## Time and cancellation

Timeout is a hard runtime deadline for the job. Explicit stop and `session_cancel` use the same owned-tree termination path.

Cancellation propagates session → task lease/job → process tree. Timeout, explicit stop and output-overflow paths are covered by native Windows and Linux tests.

## Phase 1C versus Phase 1D limits

Phase 1C validates the requested `ResourceBudget` against application hard limits and the exact task lease before spawning. It actively enforces timeout, output and registry bounds.

`memory_bytes` and `process_count` are **not yet claimed as kernel-enforced Windows limits**. They are currently authorized/bounded request values. Phase 1D must add a narrow audited Windows adapter that configures the corresponding Job Object resource limits before the child resumes and must prove them with native Windows tests.

Phase 1D must preserve:

1. suspended child creation;
2. Job Object creation/configuration;
3. kill-on-close;
4. active-process and memory ceilings derived from the authorized budget;
5. assignment before resume;
6. deterministic normal/cancel/timeout/shutdown handle cleanup;
7. native child-tree plus memory/process/output/time tests.

Job Objects provide lifecycle/resource containment; they are not a complete security sandbox.
