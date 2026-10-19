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

## Native CI observation at bcb459f

Run39287609437/job118179187087 reproduced normal Cargo exit101 and isolated
NUL error5. The separate exact-profile SID comparison successfully changed
only that profile's observed NUL opens to read/write success. It then passed
check, failing/corrected unit-test assertions, offline missing-dependency refusal,
restored check and source/lock comparisons, reaching the collector. Collection
correctly refused the previously unrecorded exact `.cargo-artifact-lock` name.
The final cleanup/revoke/overall pass marker was not reached, so that run is not
a completed Cargo acceptance or an acknowledged explicit revoke proof.

The fixture now records only that observed additional lock name; all unknown
products still refuse before any deletion. Bounded diagnostics list at most eight
unknown relative names without reading file contents or traversing unknown
subdirectories. Entry count includes unknown names. The native collector
regression passes with the new lock while retaining all negative cases. Exact
updated CI remains required. Main PR254/e1b8288 is merged into this research
history; no Node/runtime production change is carried as research-only code.

At28059e7, native CI39288416438/job118182413707 again passed the prepared Cargo
execution assertions and stopped before deletion on `.cargo-build-lock` in the
host/target debug directories. That second exact Cargo-owned lock name is now
recorded too. Unknown names remain a refusal, with no recursive cleanup fallback;
normal Cargo failure remains separate and red. Updated cleanup/revoke proof is
still required. The lock split is also discussed in Cargo issue16853, but native
observations, rather than an assumed version layout, determine this fixture set.

At855be31, native CI39289488345/job118186101602 collected and removed21 known
target files totaling3,733,832bytes, then refused the previously unrecorded
Cargo-home registry directory. The fixture now records that exact directory and
its standard CACHEDIR.TAG name. A separate read-only post-exit home diagnostic
lists at most16 relative names/depth4 with no contents; it does not authorize
cleanup. Collector traversal still refuses other unknown directories/products.
Local home/target boundary fixtures and standalone NUL diagnostics pass. Overall
prepared cleanup/revoke acceptance remains pending the updated CI run.

## Completed disposable comparison at21ac09b

Native CI39291321987/job118192403768 completed the prepared case with exit0.
The actual offline check/test, deliberately failing assertion and corrected test,
missing dependency refusal, source/lock verification, owned-tree quiescence,
known output cleanup and explicit NUL revoke all passed. A fresh child observed
the original profile access denial again after revoke. The collector removed21
target files (3,733,831bytes) and4 Cargo-home files (57,521bytes).

The independent normal case still returned101 for the same NUL denial. The
workflow therefore remains red by design; this is a completed prepared research
comparison, not a normal Windows Cargo acceptance or production setup feature.
No device permissions were changed on the operator's PC.

The operator approved developing an optional Windows preparation path on
2026-10-19. That direction does not authorize applying a host change: the precise
resource, profile identity, rights, lifetime and recovery mechanism must be
reviewable separately before any local privileged application. The AI, project
processes and ordinary core must remain unelevated. The test-only CI comparison
must not be exposed as an installer or callable MCP operation.

## Completed research scope and dedicated gates — 2026-10-20

The integration targets the fixed evidence harness, not production Cargo support.
Standard private-Rust CI retains its direct compiler/linker/application assertions
without appending an unsupported Cargo requirement. Offline Cargo now has a
dedicated research workflow with independent normal and prepared jobs on separate
disposable hosted Windows workers. The normal acceptance still fails on NUL and
remains red; no continue-on-error, fallback or successful alternate result can
replace it. The workflow is opt-in outside its own source/contract PR paths.

The prepared probe also creates an unrelated capability-free profile while the
exact owned SID's grant is active, requires both NUL spellings to remain denied
for read/write there, and closes that profile. Existing execution, bounded
collection, explicit revoke and restored-denial assertions remain required.
The concrete Windows preparation broker is still design-only; no local ACL
mutation, administrator service or model-callable policy change is introduced.

At code head282524ce5d22b45e5da973ca4e6ca439e727e8de, dedicated workflow
39339416828 completed the prepared job118356950903 successfully, including the
new unrelated-profile denial/close,21 target outputs (3,733,831bytes),4 home
outputs (57,521bytes), explicit revoke and restored denial. Independent normal
job118356951094 failed on the same NUL error5. Integration documentation keeps
that failed normal acceptance visible; it is not a production fallback.
