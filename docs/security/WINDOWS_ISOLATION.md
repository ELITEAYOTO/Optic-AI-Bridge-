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
- at the C1 proof gate the release bundle copied only `optic-bridge.exe`, and `ProcessManager` did not yet select this helper.

This proof adds no workspace or network capability and does not alter the fail-closed `Interpreter` / `RepositoryCode` policy.

## Phase 3C3C2C2A runtime launcher wiring

The proven helper is now wired into the internal Windows runtime without changing public authority:

- `ProcessManager` can be constructed with an optional isolation launcher; the launcher path must be absolute, is canonicalized, must be a regular file and is pinned against rewrite/delete/rename for the manager lifetime;
- `FixedTool` keeps the existing direct spawn path, while direct internal `Interpreter` / `RepositoryCode` starts use the helper only when configured; non-Windows remains fail-closed;
- the helper runs inside the same bounded Job Object with a kernel process limit of `logical process_count + 1`, so the internal helper does not consume or widen the authorized workload descendant budget;
- the bounded C1 request is written through piped stdin and runtime stdout/stderr still flow through the existing bounded drains;
- the helper runs under `env_clear`, receiving only the operator allowlist plus the Windows baseline required for AppContainer creation (`SystemRoot`, `LOCALAPPDATA`, `TEMP`, `TMP`);
- helper exit 126 is treated as a failed process result, and launcher/protocol/environment failures map onto existing internal/MCP error contracts without adding a model-controlled isolation selector;
- at the C2A gate the app only discovered a sibling helper when present; installer/release packaging was intentionally deferred until representative compatibility and eligibility gates;
- native Windows CI proves real runtime routing with logical `process_count = 1`, which requires the Job Object to admit both helper and isolated target.

At the C2A gate this was still **not public high-risk execution**: `optic-bridge-policy` rejected `Interpreter` / `RepositoryCode`, no workspace ACL/capability grant existed, and no network capability or containment claim was added.

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

This B2 wiring by itself did **not** create public high-risk execution: at that gate operator process leases still carried `ProcessRun + ProcessExecutable` only, while policy/MCP continued to reject `Interpreter` / `RepositoryCode`. No directory/recursive, write or network authority was added.

## Phase 3C3C2C2C1 operator-owned exact-file process read provisioning

Windows startup now owns the explicit authority bridge into the proven B2 grant path:

- `--allow-process-read-file <absolute-executable> <workspace-file>` is repeatable and accepted only for an executable already classified/authorized by `--allow-executable`;
- `FixedTool` is rejected because workspace grants are restricted to the AppContainer-isolated `Interpreter` / `RepositoryCode` path;
- each workspace file is validated before lease publication through the existing exact-file resolver, preserving the 32-file ceiling, regular-file requirement, canonical containment and duplicate rejection;
- only the matching process lease gains `FileRead` plus exact `WorkspacePrefix(file)` scopes; no grant configuration leaves the existing process lease unchanged and `WorkspaceAll` is never minted;
- MCP has no process-grant parameter and cannot mint, select or widen these scopes;
- non-Windows startup/direct provisioning fails closed for non-empty process read grants;
- policy still returns `ProcessIsolationRequired` for `Interpreter` / `RepositoryCode`, so this is authority provisioning, not public high-risk re-admission.

PR #78 merged as `70cde40f` from exact green final head `3b871bdf`; CI #347 passed Ubuntu, Windows, native real-binary smoke, installer profiles and dependency policy. During validation, `cmd.exe` / `findstr.exe` did not consume the granted temporary-file ACL like the controlled Rust probe. That result is treated as a toolchain-compatibility boundary; the grant is not broadened merely to make a representative executable pass.

## Phase 3C3C2C3 representative toolchain characterization

PR #82 merged as `8ea01de3` from exact green final head `915b38f4`; CI #357 passed. Node is the first representative interpreter proven end-to-end through the real helper/AppContainer path: startup succeeds, the exact operator-granted workspace file is readable, and an ungranted sibling remains denied. Python and Java currently terminate before workspace access with `STATUS_DLL_NOT_FOUND`. PR #83 merged as `b5a53b08` from exact green final head `25d2eed7`; CI #361 separated rustup proxy behavior from direct binaries, proved direct pinned Cargo starts successfully, observed direct rustc still has a loader dependency, and recorded Python/Java runtime DLL candidates without granting them. No runtime directory, stdlib/JDK/sysroot, write or network authority was added.

## Phase 3C3C2C4 capability-free network proof

PR #85 merged as `bb649833` from exact head `6fcc25d1`; PR CI #365 and post-merge `main` CI #366 passed. Native Windows CI first proves a normal host connection to an ephemeral `127.0.0.1` TCP listener, then launches Node through the real zero-capability high-risk AppContainer path with no workspace/env/network grants. The script reaches `NETWORK_ATTEMPT`, never reaches `NETWORK_CONNECTED`, exits through bounded denial/timeout, and the listener accepts no AppContainer connection. This is a mechanically tested property of the tested TCP-loopback path only; it is not generalized to all protocols/address families, systems with external loopback exemptions, or the direct `FixedTool` path.

## Phase 3C3C2C5 exact eligibility and helper packaging

C5A-C5C are merged and post-merge validated through PRs #87-#89. Strong-isolation readiness is represented by a separate exact `ProcessIsolationEligible { executable, class }` lease marker. The marker alone grants no process scope. Policy can admit `Interpreter` / `RepositoryCode` only when the same active lease also contains the matching exact `ProcessExecutable` scope; path/class mismatch remains fail-closed, and network/resource/session/identity checks remain independent. C5B production startup deliberately provisions an empty eligibility set, so no CLI or MCP caller can activate this path yet.

C5D is merged through PR #90 (`5d3f64a7`, exact green head `815a73c8`, PR CI #380, post-merge CI #381). The release bundle includes `optic-bridge-isolation-launcher.exe` beside `optic-bridge.exe`; the installer copies it to the same installed `bin` directory, the doctor verifies the canonical sibling layout, and installer-profile CI compares the installed/reinstalled helper SHA-256 with the supplied helper. Recursive uninstall remains bounded by the existing Optic ownership marker and removes the sibling with the rest of the owned installation root. Helper presence is runtime availability only and never implies eligibility.

C5E is the first production minting path and is intentionally narrower than the eligibility domain permits. On Windows, `--allow-isolated-node=<absolute-node.exe>` may select only an exact canonical `node.exe` already present in operator-owned `Interpreter` authority; startup requires the pinned sibling helper before minting the marker. No MCP request field maps to this configuration, and a caller-supplied spoof field must never create authority. Eligible high-risk leases are capped to `process_count = 1`, matching the proven single-target AppContainer path and leaving descendant execution fail-closed. The real-binary MCP regression proves the unselected control remains `ProcessIsolationRequired`, rejects a multi-process budget, while the selected Node path reads only its separately granted exact file and is denied an ungranted sibling. No Python/Java/Cargo/rustc eligibility is introduced.

## Hardened profile

Current direction: representative characterization is concrete. Node has startup + exact granted-file read + ungranted-file denial + zero-capability TCP-loopback denial evidence through the real helper/AppContainer path; direct Cargo startup also succeeds. Python/Java/direct rustc still have loader/runtime dependencies that are not authorized merely for compatibility. Exact strong-isolation eligibility and policy semantics are merged and C5D packages the helper as authority-neutral runtime availability. C5E now wires only the proven Node operator profile and requires an end-to-end MCP admission/negative-authority smoke; broader high-risk profiles remain disabled. Restricted-token and LPAC variants remain comparative compatibility/hardening research rather than implemented authority.

A restricted token reduces privileges but is not equivalent to a VM sandbox.

## Hard isolation

Windows Sandbox/VM may be offered later for genuinely untrusted execution. It is intentionally outside the lightweight default path.

## Session separation

Each session receives independent application-owned process/job records. Shared global handles, current directories and environment mutations are forbidden.

A caller can control only opaque `JobId` values owned by its application session; no arbitrary PID termination API is exposed.

## Remaining security work
Phase 1 Job Objects contain lifecycle, process count and job memory. Phase 3C3C2A proves a zero-capability AppContainer identity and denial of an ungranted user-file read; later gates add captured stdio, the bounded helper, runtime routing, exact-file grants and explicit operator-owned provisioning. Phase 3C3C2C3 characterizes Node/Python/Java/Rust/Cargo on that real path, Phase 3C3C2C4 adds a native denial proof for the tested capability-free Node TCP-loopback attempt, and C5A-C5C add exact server-owned eligibility semantics without a production minting path. C5D distributes the helper as an authority-neutral installed sibling, and C5E introduces only exact operator-selected Node eligibility with a real MCP admission/denial regression. Remaining work includes installer/UI exposure decisions for that opt-in, unresolved Python/Java/rustc runtime dependencies, broader workspace/write authority, registry/UI decisions, machine-wide resource governance and network claims beyond the tested high-risk loopback path. All later hardening must preserve the existing deterministic policy/lease boundary rather than treating OS containment or helper presence as authorization.
