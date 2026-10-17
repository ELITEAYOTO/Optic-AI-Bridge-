# ADR0028 — Fixed native Rust compile/link/run compatibility proof

Status: TEST-ONLY / EXACT NATIVE VALIDATION REQUIRED
Date: 2026-10-17

After ADR0025/27, prove a complete fixed native application path without relying
on installed MSVC CRT libraries: compile a public no_std library to one COFF
object, invoke an explicitly selected installed Microsoft link.exe or rust-lld with
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

## Observed linker boundary

Initial head1f925b9 compiled the COFF object but rust-lld returned exit1 with
permission denied before linking (native run39043251239). Installed linker
version reports LLVM23.1.1, Rust LLVM commit1b9c0d5ff9bbe7634aead059efe6b11a7eeba145.
Pinned InitLLVM.cpp calls GetCommandLineArguments, whose GetExecutableName calls
GetLongPathNameW and propagates failure. This is a source-supported hypothesis,
not measured proof of the failing native call. No user-directory ACL widening,
sandbox downgrade or successful LLD claim is permitted on this evidence.

The CI's explicit native proof profile now selects installed Microsoft link.exe
and at most eight named adjacent runtime DLLs; no automatic fallback occurs.
Both linker flavors retain identical owned Jobs, private paths and budgets.
A successful Microsoft roundtrip does not resolve the recorded LLD limitation.
The eight names and copies are fixed, their inventory/copy ceilings unchanged.
The generated no_std application needs no CRT libraries; the linker process
itself can use its copied Microsoft runtime DLLs.


Microsoft run39044081753 reached the linker but failed1104 creating
LOCALAPPDATA/Packages/<fresh-profile>/AC/Temp/lnk{GUID}.tmp beneath the explicit
private LOCALAPPDATA. This was visible only when both bounded streams were
reported (Microsoft wrote the diagnostic to stdout). The fixture now creates
that exact known private directory chain from its own generated profile name.
It deletes these directories only when empty after confirmed execution. Unknown
leftover temp files remain a cleanup failure; no recursive sweep is introduced.
