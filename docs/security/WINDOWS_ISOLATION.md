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

This foundation alone is **not a production process sandbox**. Later C2A wiring can route the proven AppContainer helper from `ProcessManager`, but no workspace ACL/capability grant model has been added and 3C3C1 still denies `Interpreter` / `RepositoryCode` through policy/MCP. No network capability SID is granted by the profile, and public network-containment semantics remain unclaimed until a dedicated network regression gate exists.

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

This proof adds no workspace or network capability and does not alter the fail-closed `Interpreter` / `RepositoryCode` policy.

## Phase 3C3C2C2A runtime launcher wiring

The proven helper is now wired into the internal Windows runtime without changing public authority:

- `ProcessManager` can be constructed with an optional isolation launcher; the launcher path must be absolute, is canonicalized, must be a regular file and is pinned against rewrite/delete/rename for the manager lifetime;
- `FixedTool` keeps the existing direct spawn path, while direct internal `Interpreter` / `RepositoryCode` starts use the helper only when configured; non-Windows remains fail-closed;
- the helper runs inside the same bounded Job Object with a kernel process limit of `logical process_count + 1`, so the internal helper does not consume or widen the authorized workload descendant budget;
- the bounded C1 request is written through piped stdin and runtime stdout/stderr still flow through the existing bounded drains;
- the helper runs under `env_clear`, receiving only the operator allowlist plus the Windows baseline required for AppContainer creation (`SystemRoot`, `LOCALAPPDATA`, `TEMP`, `TMP`);
- helper exit 126 is treated as a failed process result, and launcher/protocol/environment failures map onto existing internal/MCP error contracts without adding a model-controlled isolation selector;
- the app only discovers a sibling helper when present; the current installer/release bundle still does not distribute it;
- native Windows CI proves real runtime routing with logical `process_count = 1`, which requires the Job Object to admit both helper and isolated target.

This is still **not public high-risk execution**: `optic-bridge-policy` continues to reject `Interpreter` / `RepositoryCode`, no workspace ACL/capability grant exists, and no network capability or containment claim is added.

## Phase 3C3C2C2B1 exact-file read grant foundation

Windows now has a deliberately narrow filesystem grant primitive for the AppContainer profile:

- only `create_ephemeral()` profiles, named from fresh 128-bit OS randomness, may mint external ACL grants;
- the trustee is the Package SID already present in the AppContainer token, not a new capability/network SID;
- one exact file is opened handle-first with read + `WRITE_DAC`; final reparse points and null/absent DACLs are refused;
- one non-inheritable `FILE_GENERIC_READ` ACE is added, with explicit revoke and best-effort Drop cleanup;
- the handle remains live for the grant lifetime, and the API exposes no directory/recursive or write grant;
- native CI proves denial before grant, read success only for the granted file, denial of a second file, append denial, explicit revoke, Drop cleanup, and refusal of predictably named profiles.

This is still not public high-risk execution. Abrupt-crash stale ACEs are deliberately non-reusable by later Optic profiles because later profiles receive a fresh Package SID.

## Phase 3C3C2C2B2 exact-file grant lifecycle/wiring

The isolated helper/runtime now carries a deliberately bounded exact-file grant set without adding caller-controlled authority:

- launcher protocol v2 includes the canonical workspace root plus at most 32 exact read-file paths under the existing 64 KiB internal request ceiling;
- `ProcessManager` accepts grants only for isolated `Interpreter` / `RepositoryCode` direct-runtime starts, while `FixedTool` rejects any workspace grant request;
- requested paths resolve to exact regular files beneath the canonical workspace; directory, duplicate, count and escape cases fail closed;
- MCP does not accept grant paths: candidates are selected only when both the active session grant and server-owned task lease carry `FileRead`, and only `WorkspacePrefix(path)` scopes are used (`WorkspaceAll` is ignored);
- the Windows grant primitive opens root and target handles, rejects final reparses, resolves both final paths with `GetFinalPathNameByHandleW`, verifies final target containment beneath the opened root, and modifies the DACL on that same target handle;
- grant guards are retained for the isolated child lifetime and revoke before the ephemeral profile is destroyed.

This wiring still does **not** create public high-risk execution: current operator process leases carry `ProcessRun + ProcessExecutable` only, so they produce zero workspace read grants, while policy/MCP continue to reject `Interpreter` / `RepositoryCode`. No directory/recursive, write or network authority is added.

## Hardened profile

Current direction: add explicit operator-owned process read-grant provisioning and validate representative Rust/Node/Java toolchains before any selected high-risk policy re-admission. Restricted-token and LPAC variants remain comparative compatibility/hardening research rather than implemented authority.

A restricted token reduces privileges but is not equivalent to a VM sandbox.

## Hard isolation

Windows Sandbox/VM may be offered later for genuinely untrusted execution. It is intentionally outside the lightweight default path.

## Session separation

Each session receives independent application-owned process/job records. Shared global handles, current directories and environment mutations are forbidden.

A caller can control only opaque `JobId` values owned by its application session; no arbitrary PID termination API is exposed.

## Remaining security work

Phase 1 Job Objects contain lifecycle, process count and job memory. Phase 3C3C2A proves an AppContainer identity with zero capabilities and denial of one ungranted user-file read; Phase 3C3C2B2 proves explicit captured stdio with handle-list-restricted inheritance; Phase 3C3C2C1 proves a separate bounded internal launcher; Phase 3C3C2C2A wires that helper into the internal Windows `ProcessManager` path with executable pinning and +1 kernel-process accounting; Phase 3C3C2C2B1 adds revocable exact-file read grants; and Phase 3C3C2C2B2 wires bounded server-authority grant selection/lifecycle through the isolated path with handle-based final-path containment. The helper remains non-distributed and policy/MCP still deny high-risk classes; current operator process leases mint no `FileRead` workspace scopes. Remaining work includes explicit operator-owned process read-grant provisioning, broader workspace coverage beyond exact read-only files, representative toolchain compatibility, registry/UI decisions, network-containment proof and policy re-admission. All later hardening must preserve the existing deterministic policy/lease boundary rather than treating OS containment as authorization.
