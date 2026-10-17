# ADR0028 — Fixed native Rust compile/link/run compatibility proof

Status: TEST-ONLY / EXACT NATIVE VALIDATION REQUIRED
Date: 2026-10-17

After ADR0025/27, prove a complete fixed native application path without relying
on installed MSVC CRT libraries: compile a public no_std library to one COFF
object, invoke the selected toolchain's direct rust-lld in link flavor with
explicit entry/subsystem/no-default-libraries/machine/output/object and one
explicitly selected Windows SDK x64 kernel32 import library, then run the
generated executable and require the real exit code42 from ExitProcess.

The original IR/stdout, E0308, corrected compile and private file denial proof
remain a separate exact opt-in test. The new exact test reuses them in its fresh
profile before the native phases. It copies linker and import library under the
existing512MiB total/256MiB file runtime ceiling; installed toolchain ACL/content
are untouched. CI SDK version discovery is bounded32 directories.

Compiler, linker and generated application each run sequentially in a fresh
one-process owned Job inside the same zero-capability AppContainer. Each retains
1GiB memory,50percent CPU, independent15second deadline, bounded128KiB stdout+
stderr, explicit private environment and confirmed-empty-tree cleanup. They
never receive a writable project path or inherited PATH/credentials. Generated
object/executable sizes must fit128KiB after quiescence. Delete only recorded
object/executable/import/linker files and existing fixed fixtures; unexpected
files prevent empty-directory removal, not recursive deletion.

This proves one public console fixture and its exact entry/exit behavior. It
does not establish std applications, Cargo, crates, build scripts, dependency
installation, arbitrary native program authority, GUI/Electron, network-denial
telemetry or runtime disk quotas. Copy/collection ceilings are not disk-growth
containment during compilation. The helper is cfg(windows,test), not a process
service or MCP tool; production build admission/enrollment remains separate.

The hashed Windows harness must pass exact CI and a source-bound verified local
reproduction before claiming this installed toolchain can complete the roundtrip.
