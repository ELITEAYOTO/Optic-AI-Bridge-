# ADR0025 — Opt-in public Rust compiler compatibility proof

Status: TEST-ONLY / EXACT NATIVE VALIDATION REQUIRED
Date: 2026-10-17

Before proposing broad writable Cargo/build authority, prove that an explicitly
selected installed direct Rust compiler can compile a fixed public no_std library
inside a fresh zero-capability AppContainer. Emit LLVM IR only to bounded stdout;
then compile a deliberate type error and require the real E0308 diagnostic.
No linker, Cargo, dependency install, arbitrary source execution, package script,
private-project access, public MCP route or production permission is introduced.

The cfg(test) ignored probe selects rustc.exe directly, not a rustup proxy. It
copies the compiler, at most eight adjacent DLLs and exactly one core and
compiler_builtins rlib each into its fresh native private storage. Directory
discovery is bounded256 entries per selected directory; copies are capped at
256MiB/file and512MiB total. No installed runtime file ACL/content is changed.
This is a fixed trusted-fixture copy, not production runtime enrollment or a
versioned authorized sysroot/dependency capture service.

Structured fixed argv controls crate type, target, sysroot, no-debug/panic-abort
and stdout emission. Only SystemRoot and private TEMP/TMP are forwarded. A
configured Job owns the suspended child before resume, with one process,
1GiB memory,50percent CPU and an independent15second deadline. Polling output is
bounded128KiB accepted aggregate, without disk spools. Root exit is insufficient:
confirm the owned tree empty before profile deletion. Native uncertainty retains
only this fresh profile rather than deleting storage on a kill request.

This test's runtime-copy ceiling is not a hard disk quota on an executing
compiler or its private storage. It establishes neither arbitrary Rust/Cargo
containment nor writable-build/dependency authority. It does not run the emitted
code, prove network denial or build a native application. Those gates remain.
Core retains no mandatory Rust/Node/Electron/Python runtime dependency.

Exact Windows CI builds an archived hashed harness and runs only this opt-in
case with installed Rust1.99.0. A verified local reproduction is required before
claiming compiler compatibility. Local missing MSVC CRT libraries are irrelevant
to this IR-only path and are not changed or installed by the probe.
