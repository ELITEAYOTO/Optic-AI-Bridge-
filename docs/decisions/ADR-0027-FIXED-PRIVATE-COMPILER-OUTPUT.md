# ADR0027 — Fixed private compiler file output proof

Status: TEST-ONLY / EXACT NATIVE VALIDATION REQUIRED
Date: 2026-10-17

Extend ADR0025's opt-in fixed public no_std Rust fixture with one compiler-written
LLVM file, `output/library.ll`, inside the fresh AppContainer's private storage.
Keep the original stdout compilation, real E0308 error and corrected recompile.
No source workspace is made writable, and no public/core/MCP permission is added.

The fixed file is collected only after the invocation returned from confirmed
owned-tree quiescence. The collector also rejects retained native uncertainty or
a failed compile result, pins its known root/output directory, verifies root
identity and opened final path, rejects reparse/hard-link aliases and concurrent
writers through the existing exclusive native file primitive, and reads at most
128KiB plus one detection byte. Missing, empty or oversized outputs fail closed.
Only the fixed recorded output/fixture files are deleted. Unknown extra files
still make empty-directory cleanup fail rather than triggering recursive deletion.

Native regression cases exercise the actual compiler file, oversized file,
concurrent writer, hard link, reparse redirect, missing file and uncertain owner.
The workflow runs the existing exact ignored compiler test in disposable Windows
CI and retains the source-bound hashed harness for optional local reproduction.

This remains a compatibility experiment with trusted fixed fixture paths and
compiler. The completion argument is internal test state, not a production
transferable output receipt. No arbitrary private-root enrollment, untrusted
manifest, artifact publication, disk spool, content classification, Cargo,
linker, dependencies or native application execution is established.

The collection byte bound is not a hard disk quota while the compiler executes.
Private writable build admission still needs a separate disk-growth/cleanup and
crash-recovery design. Do not expose this helper as a general build tool or relax
read-only Node policy on the strength of this proof.
