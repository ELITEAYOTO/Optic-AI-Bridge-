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

## CI #360 observed results

Exact characterized head before this documentation update: `7bf026834bea4e508fcf67d16f5e254ee072c377`.

| Runtime | Observation | Result | Interpretation |
|---|---|---|---|
| Cargo | PATH proxy `C:\Users\runneradmin\.cargo\bin\cargo.exe` vs direct `C:\Users\runneradmin\.rustup\toolchains\1.99.0-x86_64-pc-windows-msvc\bin\cargo.exe` | distinct files; direct Cargo exits `0` and reports Cargo 1.99.0 | The rustup proxy/profile layer caused the earlier Cargo failure. The actual pinned Cargo binary already starts through the existing AppContainer path with no new authority. Do not grant `.rustup` wholesale merely to support the proxy. |
| rustc | PATH proxy vs direct pinned toolchain `rustc.exe` | distinct files; direct rustc still exits `-1073741515` (`0xC0000135`, `STATUS_DLL_NOT_FOUND`) | Removing the rustup proxy is not enough for rustc. Its loader/runtime dependency must be characterized separately before adding any authority. |
| Python 3.12.10 | DLL inventory beside hosted `python.exe` | `python3.dll`, `python312.dll` | Concrete adjacent loader/runtime candidates exist. Their presence is diagnostic evidence only and does not authorize the Python directory. |
| Java Temurin 17.0.20 | runtime inventory beside hosted JDK | `bin\jli.dll`, `bin\server\jvm.dll` | Concrete launcher/JVM candidates exist. Again, no directory grant follows from this observation. |
| Node.js | positive control from the retained characterization test | startup and exact granted read pass; sibling remains `EPERM` | Confirms the helper/AppContainer/cwd/exact-file path remains healthy while this gate runs. |

CI #360 passed Ubuntu format/Clippy/tests, Windows Clippy/tests, both native characterization probes, the real MCP Git integration smoke, installer-profile validation and dependency policy. No filesystem, network, environment or policy authority was changed.

## Security boundary

This characterization adds no runtime-file grant, directory grant, recursive workspace authority, write authority, network capability, environment grant or public policy admission.

A future tool-runtime profile must remain operator/application-owned and separate from workspace authority. Installation/runtime dependencies must not silently become `WorkspaceAll` or arbitrary filesystem read.

## Decision rule

- Direct Cargo is compatible enough at startup that future Rust/Cargo work should use the canonical pinned toolchain binary rather than silently depending on the rustup proxy.
- Direct rustc still needs loader/runtime characterization; next inspect exact neighboring runtime DLL dependencies rather than granting `.rustup` or the entire toolchain directory.
- Python/Java adjacent DLL inventory is evidence for a later minimal-runtime experiment only; presence of a file is not permission to grant its directory.
- Node remains the positive control proving that the current helper, cwd and exact-file grant are not universally broken.

## Follow-up ordering

Before introducing a general tool-runtime authority model, two narrow proofs are valuable:

1. prove capability-free AppContainer network denial with the already-compatible Node runtime, because that directly addresses a major outstanding audit concern for the high-risk path;
2. then characterize the exact runtime-file needs of direct `rustc`, Python and Java, preferably as file-identity-bound, operator-owned runtime profiles rather than recursive directory access.

Neither follow-up permits public `Interpreter` / `RepositoryCode` policy admission by itself.
