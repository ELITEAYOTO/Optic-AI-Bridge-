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

At33b0327 the owned debugger confirms the same fatal breakpoint at Electron
image offset0x332579e, with no debug strings. Read-only inspection of the selected
44.7.2 executable shows a no-sandbox branch immediately before a failing broker
virtual call. Chromium's public BrokerServices layout suggests alternate-desktop
creation; this is a supported inference, not a symbolized causal proof. No
installed executable or ACL was edited. A fixed outer-appcontainer comparison
adds --no-sandbox only inside the verified fresh AppContainer/owned Job. It is
explicitly labelled preliminary compatibility with the inner Chromium sandbox
disabled; it is not an automatic fallback, normal-gate pass, descendant token/
network proof or production GUI authorization. Native layered-isolation and
all-child containment must be proven before adopting an execution mode.

The outer-only comparison at8365e61 progressed past that first failure and
produced a specific startup FATAL at Electron node_bindings.cc:710: access to
NUL needed for Node initialization was refused, recommending --no-stdio-init.
The explicit outer-only comparison therefore also selects that documented
initialization flag. The normal layered gate remains unchanged. This measured
second failure is distinct from the earlier broker breakpoint and FS errors.

At63a08fc core CI38835636525 passed Windows/Linux checks. The combined outer-only
comparison progressed again but exited1 before the fixed application marker.
Follow-up selects fixed NODE_OPTIONS preserve-symlinks/preserve-symlinks-main
(the same resolution behavior already needed by private Node staging), with
trace-exit diagnostics. No parent metadata ACLs or runtime installation ACLs are
widened. Before a GUI comparison can pass, the test samples only members of its
owned kernel job, rechecks handle membership against PID reuse, and requires
exact profile SID, TokenIsAppContainer and zero capability SIDs on every live
sample. At least two simultaneous processes must be observed for a normal GUI
success. Snapshots are capped at16 members; sampling is not a universal proof
about every possible short-lived child. Network and production staging/lifecycle
still require their own gates. Debug-only success is labelled separately.

## Successful explicit comparison and next containment gate

At623e003 the outer-AppContainer-only fixed Electron probe passed native CI;
normal Chromium sandbox startup still fails0x80000003. After bounded cleanup
retry9e9bbdb, the exact verified probe also passed locally with installed
Electron44.7.2, including renderer42, source read/write denial, exact sampled
AppContainer SID/zero capabilities and confirmed empty job before cleanup.
Integration heada087cb9 repeated the comparison successfully in native CI
run38837282705; that workflow remains red because the normal mode still fails.

A follow-up fixed probe now targets an owned live127.0.0.1 listener from both
Electron main's Node socket and the renderer fetch path. Main must observe
EACCES/EPERM; the native listener must observe no accepted connection. This
separates a denied network path from a closed remote port. Its native validation
is pending. It is a loopback regression, not a proof of every Windows broker,
network protocol or GUI capability. No production fallback or new tool is added.
Loopback characterization8dea8e0: Electron main reports ETIMEDOUT rather than
EACCES/EPERM. This is not by itself an access-denied proof. The next probe adds
a successful parent TCP control connection, then checks the same live listener
observes no child connection. Only EACCES/EPERM/ETIMEDOUT are recognized main
outcomes, reported explicitly; an application-side deadline or arbitrary socket
error fails. Renderer failure plus no accepted TCP remains the independent
endpoint observation. Claims stay limited to this controlled loopback path.
## Native comparison validation — 2026-10-16

Exact head119cca047e7f0db4b2ad31abce54750833b99c8f passed the explicit outer-only
Electron comparison in CI38837928895/job116771116738 and locally with installed
Electron44.7.2. Positive parent TCP control succeeded; no main/renderer connection
was observed at that controlled listener. Accepted main errors are explicitly
EACCES/EPERM/ETIMEDOUT; this is an endpoint observation, not universal network
containment. Renderer42, source read/write denial, exact sampled token identity,
zero capabilities, owned-tree drain and private cleanup also passed. Normal
inner-sandbox startup still fails0x80000003, so the workflow remains red by design.
No automatic fallback, public GUI grant or broad runtime environment was shipped.

Main PR224 and post-merge main CI38837205821 passed. Next implement the owned
bounded production stage lifecycle and separately authorized artifact publication;
then explicit dependency/network restore and local GUI permission, followed by
an end-to-end goal-only FS test. Current private probe has no model authority.
## Authorized private input preparation (unwired runtime increment)

PrivateProjectStageService composes existing versioned capture with internal
copying into a freshly created ephemeral AppContainer's inputs directory. The
outer session admission spans capture, materialization and final live checks;
source files are never reopened after sealing. Exact manifest session/task/epoch
binding remains attached while the temporary in-memory source bytes are released.
No process profile/SID is exposed, and this API cannot start a process.

Each service instance has four occupied preparation slots, including uncertain
setup/cleanup failures. Each stage inherits the256-file/8MiB source bounds and
at most4096 implied directory records. The caller must own one shared service,
retain transport/I/O permits and preserve live checks when execution is added.
Native root identity and pinned parent chains fence preparation and cleanup.
Cleanup deletes only recorded regular files and directories, without recursive
walking of a mutable tree. Unknown cleanup retains accounting rather than freeing
a slot. Explicit discard reports failure; Drop is best effort. This is not a
hard disk quota for running code, durable crash recovery or a process grant.

Native regression tests cover exact copies after original source edits, bound
ownership, successful explicit/drop cleanup, finite capacity and stale input
rejection. This new head requires native Windows/Linux CI; local linking still
lacks msvcrt.lib. No public MCP tool or stable ZIP changes yet. Execution handoff,
owned-task recovery, artifacts and dependency authority remain separate gates.
## Bounded preparation owner worker

An opt-in background preparation API retains native profiles on their creating
worker rather than adding unsafe Send/Sync promises for SID pointers. Four stage
reservations bound preparing/idle owner threads, and each control channel has
capacity one. A Sendable host lease carries immutable owner binding and the
prepared input path; it exposes no native profile, process command or policy.

The worker owns the caller's transport and blocking-I/O permits through the
synchronous preparation even when its async caller is cancelled. It checks the
original transport deadline before and after copying. Idle prepared state holds
only its stage slot. Explicit discard waits for the worker's cleanup result;
lease Drop closes the control channel and triggers owned cleanup. Cleanup
reacquires the shared I/O governor; unavailable ownership retains capacity.

Regression tests cover acknowledged cleanup, abandoned ready leases, and a
cancelled caller while its worker is deliberately blocked: transport/I/O/stage
capacity must remain occupied until the worker finishes. No native profile is
moved between threads and no process starts. Native exact-head validation is
required. Durable recovery and future execution handoff remain separate gates.
## Owned native execution building blocks

ContainedAppContainerProcess consumes an owned suspended AppContainer child and
binds it to configured kill-on-close Job Object limits before its only resume.
A resumed root is refused; a second resume is refused. Polling completion requires
both observed root exit and zero owned descendants. Explicit termination attempts
are followed by bounded kernel confirmation (1..5000ms); a request alone is not
proof. The API accepts no PID, grants no policy and is not itself a full sandbox.

CapturedOutputPipe owns its sole reader, exposes only its writer for explicit
child stdio inheritance, and polls at most64KiB per chunk. Idle is distinct from
EOF; broken-pipe means confirmed EOF after buffered bytes are drained. No output
spool file is created. The future runtime controller must enforce aggregate
output/job/memory/CPU admission and retain stage ownership on uncertain termination.
These primitives add no MCP execution mode. Windows native tests are required.
### Experimental managed private Node controller (2026-10-16)

A host-only, unwired controller now consumes one prepared input lease to run an
explicitly authorized installed Node executable. It accepts an entry in the
sealed manifest, fixes argv/environment, denies Node writes/child processes and
uses a zero-capability AppContainer assigned to a limited Job before resume.
The existing ProcessManager owns admission, owner-scoped job IDs and bounded
stdout/stderr; the private worker retains the native tree independently of the
start response. Stop, deadline, output overflow and revoked authorization require
confirmed tree termination. Uncertain termination retains stage accounting.
Terminal job state follows owned cleanup; no MCP exposure or restart recovery is
claimed. Dependency installation, builds, artifacts and Electron remain separate.

Windows profile deletion is explicitly acknowledged before freeing a known-clean
stage slot. Optic removes recorded inputs without recursively walking unknown
contents; Windows may reclaim storage belonging to its fresh AppContainer profile.
This is not a backup/preservation promise for unexpected data inside that profile.
No user source directory receives recursive write grants.

Exact head5406a4f passed CI38906210858 and native38906210466: two-module
execution/write denial, owned stop, output overflow, session revocation and task
deadline. Earlier exact c7436b4 also passed three Node and six preparation cases
locally with installed Node22.15.1. Core has no mandatory Node dependency.

A retained private job record now correlates its original ActionId and session.
The host can recover its JobId after a lost start reply; admission rejects a
second start using that same retained action. This shares the existing bounded
job history, introduces no extra unbounded registry, and exposes nothing to MCP.
It is not durable exactly-once behavior: terminal history can be evicted and host
restart loses these in-memory records. Never automatically replay an unknown
start after restart. Exact candidate d64cf2e validation is pending.
