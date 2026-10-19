# Fixed offline Cargo compatibility — 2026-10-19

Status: RESEARCH; LOCAL BLOCKER REPRODUCED; NOT PRODUCTION CARGO SUPPORT.

The existing standard Rust proof compiles/links/runs a fixed Vec/parse/assert/
println app using copied installed compiler/CRT/SDK files in a fresh AppContainer.
The new opt-in fourth probe attempts a small real Cargo library with no external
dependencies. Cargo is selected from the exact same installed direct toolchain,
not the rustup proxy. No installation files or global environment are changed.

## Intended acceptance, not yet reached

Run Cargo check, break an actual unit assertion, run the failing test, correct it,
run the passing test, then require offline refusal for a deliberately unavailable
dependency. Restore the manifest, repeat check and confirm source/lock bytes.
All commands use offline/frozen, one Cargo job, the copied direct rustc/linker,
private CARGO_HOME and cleared environment. The Cargo phase alone allows at most
three processes (Cargo/rustc/linker); prior compiler/application phases retain
their one/two-process budgets. Existing CPU/memory/deadline/output bounds remain.

After confirmed owned-tree quiescence only, a test-only collector preflights a
known-name inventory:256 entries, depth8,16MiB/file,64MiB total. Pins and exclusive
regular-file handles reject reparse/hardlink/writer aliases. Unknown names refuse
before deletion. Deletion rechecks native identity through its exact handle.
This is neither arbitrary artifact enrollment nor a hard runtime disk quota.

## Actual local observation

Two attempts reached Cargo but returned101 before rustc -vV was executed:
`failed to open NUL device for child stdin: Access is denied (os error5)`.
The separate fixed child uses the same owned AppContainer with no capabilities:
both NUL and the explicit device spelling refuse read and write with error5.
The parent-token read of the explicit device succeeds. No device ACL was edited.
The later Cargo assertions and cleanup acceptance are therefore unproved; there
is no overall PRIVATE_CARGO_OFFLINE_PASS result. Existing direct compilation
success is not reclassified as Cargo success.

Rust1.99's [Windows stdio implementation](https://github.com/rust-lang/rust/blob/1.99.0/library/std/src/sys/process/windows.rs)
opens the fixed NUL device when a child uses null stdin. Cargo's pinned
[process builder](https://github.com/rust-lang/cargo/blob/rust-1.99.0/crates/cargo-util/src/process_builder.rs)
uses that null-stdin path for captured commands. This explains the reported
failure stage; it does not establish every subsequent Cargo requirement.

The same restriction is discussed in Microsoft's
[AppContainer issue73](https://github.com/microsoft/win32-app-isolation/issues/73).
A participant reports an elevated ACL workaround there; this is not a verified
current Microsoft support contract or authority for Optic to apply it. A global
ALL APPLICATION PACKAGES grant differs from Optic's exact ephemeral-profile file
grants. No local device/desktop/installation ACL modification, elevated child,
toolchain patch, arbitrary injection or production sandbox fallback has run.

## Next decision

First compare the unchanged probe on disposable Windows CI. If confirmed, review
an explicit operator-owned host-preparation contract (exact resource/rights,
existing-state verification, preview, crash/reboot behavior and uninstall) or a
separately selected optional execution backend. Do not promise stock Cargo works
inside the current unprepared AppContainer or block direct Rust work on Electron.
Device compatibility alone would not close writable storage, dependency restore,
secrets/egress or artifact-publication gates. No FS/tunnel request is involved.

The standalone private_null_device_comparison_without_toolchain also passed
locally: parent read succeeds, both isolated read/write spellings return5, the
owned profile closes. It requires no selected compiler or Cargo installation.
The known-output collector regression passed unknown-name, hardlink, held writer,
optional symlink and size-bound refusals before known-file cleanup. Ordinary
workspace tests and all-target Clippy passed; Cargo acceptance remains failed.

## Explicit disposable-host comparison (candidate, not locally executed)

A separate cfg(test)-only comparison may temporarily grant NUL read/write to
only the fresh profile SID on Microsoft-hosted CI. It requires the exact explicit
mode, GITHUB_ACTIONS=true, RUNNER_ENVIRONMENT=github-hosted and a fresh profile.
It has no caller-selected target, Everyone/AllPackages entry, owner/SACL change,
new capability or installed core API. The fixed target must be a character device.
It reuses the existing handle-bound DACL merge and SID-specific revoke primitive.
Explicit revoke plus a fresh child observation must restore the original access
result before the overall pass marker; RAII is best effort on unwind. A host crash
could leave that disposable profile's SID entry until the VM is destroyed; this
is not a production host-preparation/rollback design and must not run on this PC.

The workflow always runs normal and prepared cases as separate fresh profiles,
reports both exits, and keeps a normal failure red even if the comparison passes.
It does not automatically continue the same failed task with different authority.
No broader host changes or assumed general Cargo success follow from either case.
