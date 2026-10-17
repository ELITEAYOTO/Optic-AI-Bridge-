# ADR0029 — Fixed standard-library Rust application proof

Status: TEST-ONLY / EXACT NATIVE VALIDATION REQUIRED
Date: 2026-10-17

Extend the fixed compatibility suite with a real std binary using Vec, integer
parsing, an assertion and println. Compiler and direct Microsoft linker are
selected explicitly, with a complete matching CRT directory and Windows SDK.
No installer or global environment modification is required if those resources
already exist. Incomplete selected resources fail; no post-failure fallback.

The std case copies up to64 rlib/rmeta files from the selected Rust sysroot,
retaining the256-entry inventory ceiling,256MiB file and512MiB total copy ceiling.
It copies only three named CRT libraries, eight named Windows import libraries
and the selected UCRT import; the large SDK directories are never inventoried.
The two relevant Microsoft runtime DLLs are copied alongside the produced app.
Every copy uses the existing no-reparse exact-size/EOF primitive; no installed
ACL/content is changed. These are trusted installed tool selections for fixed
public source, not production runtime/dependency enrollment.

rustc launches its explicit private linker as a real descendant under the same
owned Job. This exact phase allows at most two processes; previous phases and
the produced app retain one. It keeps aggregate1GiB memory,50percent CPU, an
independent15second deadline and128KiB aggregate diagnostics. The only added
child environment entry is LIB pointing at the copied private native libraries.
No inherited PATH/tool flags/credentials or writable project is introduced.

After root and descendants are confirmed empty, the executable must be at most
8MiB. It runs in a separate owned one-process Job, exits0 and emits the exact
fixed pass marker. Fixed copied/generated files and empty directories are removed;
unknown output/temp files remain a cleanup error. The original IR, real error,
read-only output and no_std native cases remain independent exact workflow steps.

This is not a Cargo/crate/build-script/package-install proof, an arbitrary source
or GUI permission, hard runtime disk quota, sensitive-project classification,
crash-journal integration or a production process controller. No new MCP/core
authority or required runtime dependency is added. Exact Windows/Linux/native
CI and a source-bound verified local reproduction precede compatibility claims.


Initial std runs39044959371 and39045141103 successfully compiled/ran the app but
failed cleanup on the unexpected exact output/std-application.pdb. The bounded
name diagnostic preserved failure and did not remove it. The plan now explicitly
records that optional fixed path before compile, rejects preexistence, checks
its regular exclusive no-alias opened handle and16MiB size ceiling, then deletes
only that file. The final STD_PASS marker is printed only after all known files,
empty directories and the fresh profile are closed. No recursive sweep or
reinterpretation of the original failure as success is permitted.
