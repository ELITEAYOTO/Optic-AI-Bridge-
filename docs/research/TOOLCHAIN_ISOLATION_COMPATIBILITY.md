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

## Next decision after characterization

Use the CI evidence to choose the smallest follow-up gate. Candidate follow-ups include a narrowly modeled cwd/traverse authority, managed tool/runtime read profiles, scratch/output write authority, or a tool-specific incompatibility decision. No such authority is approved by this document alone.
