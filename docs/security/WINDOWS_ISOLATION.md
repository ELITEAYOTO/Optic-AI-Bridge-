# Windows Isolation

## Phase 1 baseline

Implemented process containment is deliberately narrow:

- every structured Windows process job owns one Job Object;
- every process job belongs to exactly one application session;
- root child creation is forced suspended, the Job Object is configured and assigned, then the child is resumed;
- kill-on-close is enabled as the final descendant-cleanup backstop;
- the authorized process-count budget maps to `JOB_OBJECT_LIMIT_ACTIVE_PROCESS`;
- the authorized memory budget maps to `JOB_OBJECT_LIMIT_JOB_MEMORY`;
- timeout, explicit stop and output overflow terminate the owned Job Object tree;
- removing/unwrapping the containment wrapper is fail-closed: terminate the Job Object, request root-child kill as a fallback, then close the owned handle rather than detach or leak containment;
- workspace/cwd, executable lease, environment inheritance and resource authorization remain application policy concerns outside the Win32 adapter;
- no privilege elevation is provided;
- process network access remains unavailable in Phase 1.

The Job Object is **per process job**, not one Job Object shared by the whole session. Session ownership is enforced by the application `JobId` registry and policy boundary.

## Native security gate

Windows CI must prove Windows-specific claims; Linux process-group tests are not evidence for Job Object enforcement.

The Phase 1D native gate currently verifies:

- active-process limit blocks descendant creation when the authorized process count is one;
- job-memory limit prevents a fixture from reaching an allocation target above its authorized ceiling;
- timeout kills a descendant tree before a delayed child survival marker can be written;
- closing/dropping the live kill-on-close Job Object prevents the root child from surviving;
- explicitly unwrapping the containment wrapper also prevents the child from surviving;
- existing timeout, explicit-stop and output-overflow process tests remain green.

These are containment tests, not proof that arbitrary untrusted native code is sandboxed.

## Phase 3C3C2A AppContainer foundation

The Windows-only unsafe boundary now has a proven AppContainer identity primitive:

- every proof profile is created fresh for the current user and carries zero capability SIDs;
- `AppContainerSecurityCapabilities<'a>` keeps the raw AppContainer SID pointer lifetime-bound to its owning profile in safe Rust;
- the native proof child is created suspended with `STARTUPINFOEXW` and `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`;
- the child token is queried for `TokenIsAppContainer` before resume;
- a control `findstr.exe` process can read a user-owned sentinel, while the same executable in the no-capability AppContainer cannot read the ungranted file;
- the proof process has a bounded wait and is terminated fail-closed on timeout.

This is **not yet the production process sandbox**. The primitive is not wired into `ProcessManager`, no workspace ACL/capability grant model has been added, and 3C3C1 still denies `Interpreter` / `RepositoryCode`. No network capability SID is granted by the profile, but public network-containment semantics remain unclaimed until the production path and a dedicated network regression gate exist.

## Phase 3C3C2B1/B2B2 production-path foundations

B1 carries the server-resolved execution class into `ProcessStartSpec` and adds an independent runtime `IsolationUnavailable` guard, so direct internal calls cannot bypass the policy-level high-risk denial.

B2B2 adds a reusable Windows-only suspended AppContainer spawn primitive with captured-stdio support while preserving the same fail-closed admission state:

- stdin/stdout/stderr are duplicated as dedicated inheritable handles; the caller's original handle flags are not mutated;
- only those duplicates are listed in `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`;
- the same `STARTUPINFOEXW` launch carries `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` with zero capability SIDs;
- the caller can verify `TokenIsAppContainer` before resume;
- unfinished children are terminated in `Drop`, and bounded wait failure also terminates the owned child;
- native Windows CI proves both the existing ungranted-file denial and observable explicit stdout capture through the production primitive.

This still does **not** re-admit `Interpreter` or `RepositoryCode`.

## Phase 3C3C2C1 internal launcher proof

A separate `optic-bridge-isolation-launcher` now proves the next internal boundary without making it part of the production process path:

- the helper is a distinct binary, not a hidden mode of `optic-bridge.exe`;
- its stdin protocol is application-internal JSON bounded to 64 KiB before parsing, with unknown fields rejected;
- executable and cwd must be absolute and are recanonicalized inside the helper; argument count, conservative Windows command-line size and timeout are bounded;
- every request creates a fresh zero-capability AppContainer; the target receives `NUL` stdin and only helper stdout/stderr are forwarded through the explicit handle list;
- `TokenIsAppContainer` is checked before resume and the existing bounded/fail-closed child wait is retained;
- native CI executes the real helper and proves stdout/stderr forwarding, ungranted-file denial and oversized-request rejection;
- the release bundle still copies only `optic-bridge.exe`, and `ProcessManager` does not select this helper.

This proof adds no workspace or network capability and does not alter the fail-closed `Interpreter` / `RepositoryCode` policy. When the helper is wired under the Job Object, kernel `process_count` must reserve one extra internal process for it without expanding the workload's logical descendant budget.

## Hardened profile

Current direction: wire the proven internal launcher + AppContainer explicit-stdio path into `ProcessManager` with explicit workspace grants and correct +1 helper process accounting, then validate representative Rust/Node/Java toolchains before any selected high-risk re-admission. Restricted-token and LPAC variants remain comparative compatibility/hardening research rather than implemented authority.

A restricted token reduces privileges but is not equivalent to a VM sandbox.

## Hard isolation

Windows Sandbox/VM may be offered later for genuinely untrusted execution. It is intentionally outside the lightweight default path.

## Session separation

Each session receives independent application-owned process/job records. Shared global handles, current directories and environment mutations are forbidden.

A caller can control only opaque `JobId` values owned by its application session; no arbitrary PID termination API is exposed.

## Remaining security work

Phase 1 Job Objects contain lifecycle, process count and job memory. Phase 3C3C2A proves an AppContainer identity with zero capabilities and denial of one ungranted user-file read; Phase 3C3C2B2 proves explicit captured stdio with handle-list-restricted inheritance; Phase 3C3C2C1 proves a separate bounded internal launcher driving that primitive. The helper is still non-distributed and not wired into the production `ProcessManager` path. Remaining work includes runtime selection/Job Object integration with +1 helper process accounting, explicit workspace grants, representative toolchain compatibility, registry/UI decisions, network-containment proof and policy re-admission. All later hardening must preserve the existing deterministic policy/lease boundary rather than treating OS containment as authorization.
