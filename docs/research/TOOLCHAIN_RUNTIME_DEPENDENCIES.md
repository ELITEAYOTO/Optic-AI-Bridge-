# Toolchain runtime dependency characterization

**Status:** Phase 3C3C2C3B diagnostic gate; no new authority and no public high-risk process admission.

## Trigger

The preceding representative-toolchain gate proved that Node.js can already start through the real Optic helper/AppContainer path, read one exact granted workspace file and receive `EPERM` for an ungranted sibling. The same gate showed different failure classes for the other representative tools:

- Python and Java exit before workspace semantics with NTSTATUS `0xC0000135` (`STATUS_DLL_NOT_FOUND`);
- `cargo.exe` and `rustc.exe` found on `PATH` are rustup proxies and fail while trying to use `.rustup` user-profile state.

Those results do **not** justify wider workspace access.

## Gate questions

This gate answers narrower questions without changing ACL or policy behavior:

1. Are the `cargo.exe` / `rustc.exe` files on `PATH` distinct from the actual binaries selected by the active pinned rustup toolchain?
2. Can those direct Rust toolchain binaries start in the existing AppContainer path without the rustup proxy?
3. Which adjacent Python runtime DLL candidates are present next to the hosted interpreter?
4. Are the expected Java launcher/JVM runtime candidates (`jli.dll`, `server/jvm.dll`) present next to the hosted JDK?

The Windows CI probe logs bounded `OPTIC_RUNTIME_DEP` records for those observations.

## Security boundary

This characterization adds no runtime-file grant, directory grant, recursive workspace authority, write authority, network capability, environment grant or public policy admission.

A future tool-runtime profile must remain operator/application-owned and separate from workspace authority. Installation/runtime dependencies must not silently become `WorkspaceAll` or arbitrary filesystem read.

## Decision rule

- If direct Rust binaries work while rustup proxies fail, treat rustup proxy/profile state as a distinct compatibility layer rather than granting `.rustup` wholesale.
- If direct Rust binaries still fail, characterize their loader/runtime dependencies before adding any authority.
- Python/Java adjacent DLL inventory is evidence for the next experiment only; presence of a file is not permission to grant its directory.
- Node remains the positive control proving that the current helper, cwd and exact-file grant are not universally broken.
