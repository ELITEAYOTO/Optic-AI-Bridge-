# Windows Isolation

## V1 baseline

- process trees owned by per-session Job Objects;
- kill-on-close cleanup;
- time/process/resource limits;
- workspace/path policy;
- controlled environment inheritance;
- no privilege elevation.

## Hardened profile

PROPOSED: Restricted Tokens for workloads where toolchain compatibility is acceptable. Validate behavior on real Rust/Node/Java build chains before making this default.

A restricted token reduces privileges but is not equivalent to a VM sandbox.

## Hard isolation

Windows Sandbox/VM may be offered later for genuinely untrusted execution. It is intentionally outside the lightweight default path.

## Session separation

Each session receives independent process ownership. Shared global handles/current directories/environment mutations are forbidden.

## Security tests

Native Windows CI must verify child/grandchild cleanup, timeout/cancel, handle closure, access denial, and path/reparse edge cases.
