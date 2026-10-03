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
- existing timeout, explicit-stop and output-overflow process tests remain green.

These are containment tests, not proof that arbitrary untrusted native code is sandboxed.

## Hardened profile

PROPOSED: Restricted Tokens for workloads where toolchain compatibility is acceptable. Validate behavior on real Rust/Node/Java build chains before making this default.

A restricted token reduces privileges but is not equivalent to a VM sandbox.

AppContainer/LPAC remains a research candidate where stronger local isolation is worth the compatibility cost.

## Hard isolation

Windows Sandbox/VM may be offered later for genuinely untrusted execution. It is intentionally outside the lightweight default path.

## Session separation

Each session receives independent application-owned process/job records. Shared global handles, current directories and environment mutations are forbidden.

A caller can control only opaque `JobId` values owned by its application session; no arbitrary PID termination API is exposed.

## Remaining security work

Phase 1 Job Objects contain lifecycle, process count and job memory. They do not independently restrict filesystem access, registry access, user-token privileges or network access. Later hardening must preserve the existing deterministic policy/lease boundary rather than treating OS containment as authorization.
