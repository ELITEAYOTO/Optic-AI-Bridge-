# ADR0016 — Operator-selected development environment

Status: ACCEPTED DIRECTION / INCREMENTAL IMPLEMENTATION
Date: 2026-10-16

## Product scope

The next user acceptance target is a genuine Electron application, then a Rust
project, rather than another fixed three-source Node fixture. Installed toolchains
remain external and optional. The operator selects a dedicated project and grants
filesystem, dependencies, execution, network and Git permissions locally. Existing
private repositories, broader roots and multiple projects need separate admission
gates; current public dummy alpha authentication is not a private-project credential.

The existing versioned local JSON configuration is the authority foundation.
Readable boolean presets must compile to explicit capabilities, workspace scopes,
executable/profile identities, network destinations and finite budgets. No model
tool writes policy or self-approves expansion. Full autonomy removes repeated
approval prompts within the operator's scope, not effect ownership, stale-write
checks, cancellation or accounting. A future visual application edits the same
local model; neither YAML nor a mandatory Electron dependency is needed in core.

Live permission changes must fence new admission, advance policy epoch, revoke
affected leases/work and reconcile cleanup before any replacement authorization
is published. File edits are not yet hot reload. Invalid/partially written policy
must not retain silently broader grants or automatically reinstate revoked work.

The first config increment implements optional files/processes/git booleans as
reductions over explicitly selected scopes and tools. An absent section preserves
legacy authority; a present section defaults missing booleans to false. Read
denial removes core capabilities/leases and process file grants, not just tool
visibility. Duplicated/unknown fields fail closed. Config and enabled tool-profile
files loaded through config mode must be outside the writable workspace (opened
file final paths on Windows). These are startup snapshots, not live reload.

## First implementation: typed directory creation

DirectoryCreate is distinct from FileWrite, with its own capability and workspace
lease. Local --allow-directory-create-scope and optional mutation.directory_scopes
in JSON mint that authority. Omission grants nothing. MCP exposes
fs_create_directory only with this authority; expected=absent is mandatory.

One action creates one absent leaf under an existing parent. No recursive mkdir,
overwrite, implicit success for an existing directory, delete, rename or shell
effect is hidden inside it. Depth is bounded to32 components and request/path/
execution limits remain enforced. Windows pins every ancestor without delete
sharing, refuses reparse points and verifies the root's captured native identity
before creation. Authorization admission stays held through the operation.
Other platforms fail closed until the corresponding native boundary is proven.

The effect is irreversible in the policy model: it does not promise file-journal
rollback, durable exactly-once or continued absence/presence after other local
actors run. An interrupted caller must inspect the parent; no automatic retry or
recursive cleanup is permitted. Existing file mutation recovery is unchanged.

Native tests must cover existing file/directory conflicts, missing parents,
prefix boundaries, absent grant/precondition, revoked authority, reparse parents,
root replacement and creating a versioned file inside the new directory.

## Sealed build inputs (data contract, not execution authority)

ProjectInputManifest binds an explicit input list to one session, task lease and
policy epoch. It sorts paths deterministically and caps inputs at256 files,
1MiB per file and8MiB total; paths are portable ASCII, at most240 bytes and16
segments. Windows device names, trailing dots, case-insensitive file collisions,
file/parent collisions and .git/.optic/node_modules/target segments are denied.
Dependency trees must use a separate owned cache contract, not an implicit
recursive source grant. Input selection still requires sensitivity policy.

Sealing consumes complete already-authorized reads, verifies exact size and
content version, and rejects duplicate, missing and extra entries. Snapshot
contents cannot be mutated through the API or printed by Debug. This is an
in-memory manifest of exact file versions, not a globally atomic directory read,
durable recovery record, live lease check, staging write or MCP build tool.
No filesystem is opened by these domain types. A future capture service must
retain admission while reading each native observed target and check the live
binding again before execution. Source edits after capture must never alter the
sealed bytes, and output publication remains a separately authorized mutation.

ProjectInputCaptureService implements this read-side boundary on Windows: one
explicit FileRead ActionEnvelope per sorted manifest entry, full-list policy
preflight before any source open, session admission retained through capture,
live task/session/epoch checks before reads and before returning sealed bytes.
It pins the selected native root, compares its original file identity, and
requires each opened no-reparse file's canonical path to match the manifest.
Existing filesystem read ceilings still apply. Snapshot data must fit each
action's memory budget and the capture's monotonic deadline is capped by both
the action and hard request limit. Other platforms fail closed after policy
preflight. Transport/I/O permits remain the caller's responsibility.

This service is not yet wired into MCP or the isolated launcher. Tests cover
scope/capability denial before I/O, revoked leases, immutable captured contents,
version mismatch, empty files, read ceiling preservation, root replacement,
reparse ancestors, and revocation/deadline after copying. No new process writes
or dependency authority are minted by a successful capture.

## Remaining delivery gates

1. Map these primitives into a generated full-project HTTP tool surface with
   bounded directory/source/disk inventory and larger independently reviewed files.
2. Extend contained process writes/modules/children and package-cache ownership;
   prove dependency download and install scripts with explicit egress. An npm
   name allowlist alone does not contain arbitrary lifecycle/build scripts.
3. Build/test an AI-generated Electron project. GUI launch is explicit operator
   authority; the fixed existing viewer does not prove arbitrary main/preload code.
4. Persist bounded project handoff and sanitized owned-task/crash evidence,
   readable as untrusted context. Optic cannot access hidden FS model memory or
   diagnose FS backend crashes without FS-provided evidence.
5. Repeat with cargo check/test/build and the actual installed Rust/native linker.

No new release may advertise all these gates as completed by a directory tool.

## Explicit native Electron compatibility gate

The ignored private_stage_electron_window test is invoked only after selecting
an optional installed Electron runtime. It copies regular non-reparse runtime
entries into an ephemeral AppContainer's private storage, bounded to256 files,
256 directories, eight levels and512MiB total (256MiB per file). The original
installation and user-source ACLs remain untouched. A fixed hidden application
uses a sandboxed no-Node renderer, tests actual renderer evaluation and private
output, and attempts external source read/write which must both be denied.

The native test harness assigns the suspended owned process to a kill-on-close
Job Object before resume (16 processes,1GiB memory,50% CPU); it terminates and
confirms the entire descendant tree empty before reading bounded output or
deleting the known private stage. No PID-based API, model-selected main/preload,
dependency restore or production GUI execution grant is added by this test.
Until the native gate passes, compatibility with Electron's own renderer
sandbox is unproven. Copy limits bound trusted test inputs, not hard runtime disk
quota for arbitrary code. Production staging/artifact limits remain a gate.

The first gate at37a31ac failed before application main execution with native
exit0x80000003 and empty stderr, after all73runtime files were copied. Node
staging, HTTP lifecycle and the fixed operator viewer passed in the same job.
This is a measured compatibility failure, not proof of its cause. Follow-up
enables bounded Electron startup file logging without disabling either sandbox.
The profile storage accessor now queries the fresh profile SID through
[GetAppContainerFolderPath](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-getappcontainerfolderpath)
instead of constructing a path from inherited LOCALAPPDATA. Windows allocation
ownership uses LocalFree/CoTaskMemFree as specified by the native APIs; named
profiles are refused. Native validation is required before using this accessor.

Native Node staging passed with the SID-derived root at03fe060. Electron still
failed identically on CI and on the user's Windows machine, before a startup log
was created. Therefore this is reproducible below FS/tunnel transport; its exact
native startup cause remains unknown. The optional probe artifact records both
head/merge source identity and binary SHA256 and is not an alpha release.

The next probe uses a complete explicit child environment: only SystemRoot and
private profile-local APPDATA/LOCALAPPDATA/TEMP/TMP plus a public test marker.
The Windows primitive validates at most64 ASCII variable names, case-insensitive
uniqueness, no NULs and a32767-unit UTF-16 block; an empty environment is an
explicit double NUL. No process-global environment mutation is used. Node and
Electron programs assert that HOME/USERPROFILE were not inherited. The legacy
launcher retains its existing curated-parent environment until separately wired.
One fixed diagnostic variant tests Electron's documented --no-stdio-init flag;
it does not disable Chromium/AppContainer isolation or admit arbitrary switches.

Explicit private environment and --no-stdio-init did not fix the native startup
failure. The diagnostic probe can create only its own fixed public child under
DEBUG_ONLY_THIS_PROCESS: at most4096 OS events,8192-byte debug strings and32KiB
reported evidence within the existing deadline. It reports debug strings and
exception codes/offsets, handles only the loader's first breakpoint, and passes
later exceptions back to Windows. There is no arbitrary attach/PID or memory
inspection tool. Debugger detach and owned Job Object drain cover error paths.
Debug-mode success would not certify normal launch; the normal gate stays red
until normal-mode containment and renderer checks pass.
