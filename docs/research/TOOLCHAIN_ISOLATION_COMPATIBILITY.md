# Representative toolchain isolation compatibility

**Status:** Phase 3C characterization gate; no public high-risk process re-admission.

## Goal

Measure representative Windows developer toolchains through the real Optic isolation helper/AppContainer path before changing policy or widening filesystem authority.

This gate is diagnostic first. A tool failing to start or failing to consume an explicitly granted file is a compatibility result, not permission to broaden the workspace automatically.

## Matrix

The Windows CI probe covers:

- Node.js (`Interpreter`): startup, one explicitly granted file read, one ungranted sibling read;
- Python (`Interpreter`): startup, one explicitly granted file read, one ungranted sibling read;
- Java (`Interpreter`): startup plus Java source-file mode with exact source/target grants;
- Cargo (`RepositoryCode`): startup only in this characterization gate;
- rustc (`RepositoryCode`): startup only in this characterization gate.

The test also records the exact executable path used by the hosted Windows runner and bounded stdout/stderr diagnostics.

## CI #356 observed results

Exact characterization head: `40589afec0700aa06abf1b6586067e0d1b9dc6f4`.

| Tool | Hosted executable | Startup | Exact granted file | Ungranted sibling | Current interpretation |
|---|---|---|---|---|---|
| Node.js 22.23.3 | `C:\Program Files\nodejs\node.exe` | **PASS**, exit `0` | **PASS**, sentinel read | **DENIED**, exit `1`, `EPERM` | First representative interpreter proven to start through the real helper/AppContainer path, consume the existing exact-file grant, and remain unable to read an ungranted sibling. |
| Python 3.12.10 | `C:\hostedtoolcache\windows\Python\3.12.10\x64\python.exe` | exit `-1073741515` | same | same | `-1073741515` is NTSTATUS `0xC0000135` / `STATUS_DLL_NOT_FOUND`; failure occurs before workspace file semantics are exercised, so investigate runtime DLL visibility rather than widening workspace authority. |
| Java Temurin 17.0.20 | `C:\hostedtoolcache\windows\Java_Temurin-Hotspot_jdk\17.0.20-101\x64\bin\java.exe` | exit `-1073741515` | same | same | Same `STATUS_DLL_NOT_FOUND` startup failure; investigate JDK/JVM runtime dependencies separately from workspace authority. |
| Cargo | `C:\Users\runneradmin\.cargo\bin\cargo.exe` | exit `1` | not exercised in this gate | not exercised | rustup proxy reports it cannot create/access its `.rustup` home state. This is a tool-runtime/profile dependency, not evidence that Cargo needs broad workspace read. |
| rustc | `C:\Users\runneradmin\.cargo\bin\rustc.exe` | exit `1` | not exercised in this gate | not exercised | Same rustup-proxy `.rustup` home-state failure. Follow-up should compare the proxy with the actual pinned toolchain binary before adding any authority. |

All existing Windows tests, the real MCP Git integration smoke, installer-profile validation, Ubuntu CI and dependency policy remained green on CI #356. No ungranted sentinel read succeeded.

## Security rule

The characterization test may tolerate incompatibility, but it must fail if an ungranted sentinel file is successfully read. Exact-file authority is not widened to make a tool pass.

Current public policy remains unchanged: `Interpreter` and `RepositoryCode` continue to fail with `ProcessIsolationRequired` / `optic.process_isolation_unavailable`.

## Why startup and file-read are separate

A real tool may fail for reasons unrelated to the exact target file:

- its executable or runtime libraries are not AppContainer-readable;
- its current working directory is not accessible with the current authority;
- it needs a standard library, JDK modules, Cargo/Rust sysroot, registry keys or other installation resources;
- it enumerates a parent directory before opening an exact path;
- build/test/package-manager modes require output writes or execute repository-controlled code.

Those dependencies must be measured and modeled explicitly. A successful `--version` does not prove project compatibility, and a failed source/build command does not justify recursive workspace read/write by default.

The Node result also disproves a blanket assumption that the current workspace cwd or exact-file ACL is universally unusable: Node succeeds without additional cwd authority. Directory traversal/read grants therefore remain unimplemented until a specific tool demonstrates that requirement.

## Next decision after characterization

Use the CI evidence to choose the smallest follow-up gate. The next gate should characterize **tool-runtime dependencies**, not workspace breadth:

1. compare rustup proxy `cargo.exe` / `rustc.exe` with the actual pinned toolchain binaries;
2. identify Python runtime DLL requirements that cause `STATUS_DLL_NOT_FOUND`;
3. identify Java/JVM runtime DLL requirements that cause the same startup status;
4. keep Node as the positive control for exact-file read and sibling denial;
5. do not add network, recursive workspace read, workspace write or public policy admission in that investigation.

Candidate later authority models include managed tool/runtime read profiles or tightly scoped scratch/output authority, but no such authority is approved by this document alone.
