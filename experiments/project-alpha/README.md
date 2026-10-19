# Generated project alpha

## Local experiment Python setup

Setup checks that its virtual environment actually starts, rather than merely
finding python.exe left behind after a Python uninstall/update. A broken
environment stops with an actionable error before dependency installation.
Stop the experiment before rebuilding its environment.
Explicit `Setup-ProjectAlpha.ps1 -RebuildEnvironment -PythonCommand <python.exe>`
validates the selected installed Python, preserves the old environment as
`.venv.previous`, and creates a new `.venv`. A prior backup, redirected path or
invalid interpreter refuses before moving anything. There is no recursive
deletion, automatic backup overwrite, global Python change or project/output
cleanup. Only the local operator runs setup; it is not a model tool.

Delivery target: let FranceStudent develop a small disposable project through
Optic's actual typed Rust tools. Single locally selected session shared by all
chats on this connection (ADR0013). Powerful models use the same policy boundaries.

## Implemented first vertical step

edit_smoke.py drives the actual Rust binary over MCP stdio, creates a generated
HTML mini-project under src, reads it, then replaces it with the exact returned
content version. The operator selects prefix:src write authority; callers cannot
grant authority. It verifies absent/stale overwrite, out-of-scope write, traversal
and post-session-cancel write refusal, and absence of unintended changes.
No process executable, delete or Git integration capability is granted. All
workspace/journal directories are temporary generated fixtures. One60-second
outer test deadline; contents/read/output are small fixed public payloads.

Local Windows proof uses the previously CI-verified bridge0.1.4 fixture binary.
The workflow builds the current source and runs the same proof on Windows/Linux.
Windows verifies the editing journey. Other platforms explicitly verify
optic.mutation_platform_unsupported and unchanged files; no cross-platform
mutation support is claimed. See PROJECT_TARGETS.md for website/application/
Java Minecraft goals and their separate execution gates.
The new project_http/project_gate/project_runner path exposes exactly seven base tools
on a generated retained project: info/list/read/write/patch/exact-text replacement/literal in-file search.
Guard request budgets/headers/body/auth and exact schemas precede SDK dispatch.
Core reads now include content_version only for complete offset0 reads, so edits
use observed versions instead of guessed hashes. This is not a general user-project
server. A manual FS model-generated task-list app was created and the user
confirmed add/check/reload persistence/delete; later enhancement stopped after
HTML-only modification because host-visible full CSS replies were truncated.

Adapter reads return <=1024 bytes per page without duplicate base64 for UTF-8
text. Each page derives a full-file version from one bounded Rust snapshot;
follow next_offset until eof and require the same content_version on all pages.
Metadata precedes text. Files above the locally selected cap (default8192bytes,
opt-in32768bytes) are refused, not partially versioned.
UTC millisecond timestamps supplement monotonic diagnostic durations.

The project path is generated locally; write authority is prefix:src. File text
and resulting patches use the selected UTF-8 byte cap, paths are ASCII segments
with no traversal/Windows reserved names, read pages1024bytes, list pages32entries,
literal searches16hits within one file prefix. Mutation forwarding is serialized
and retains its slot after caller cancellation/timeout until actual completion.
An uncertain write returns project.outcome_unknown; no rollback/retry is claimed.
Local stop drains first, then actually cancels the real Rust session. Project
files survive stop under output/alpha-.../project/src; credentials/grants do not.
Existing user directories, delete/Git/process/network/project-switch tools remain
excluded. The public dummy tokens are suitable only for this generated playground.

Nine local Python tests cover schema/auth/JSON/forwarding/uncertainty and safe
proof logging; dedicated actual HTTP/Rust and owned runner child proofs are wired
into Windows/Linux CI (Windows mutation proof, non-Windows explicit refusal).
The CI-built package includes its source commit and binary SHA256 manifest.
Use HOST_TEST_STEPS.md only with the matching verified artifact.

## Operator-selected larger files

Use `-MaxFileKiB 32` with the matching package; default remains8KiB. This is local
operator configuration, never a tool argument. `optic_project_info.limits` reports
`file_bytes`, `read_page_bytes` and `request_body_bytes`. Pages remain1024bytes,
or4096 with `-ReadPageBytes 4096`; task output pages do not change.

The HTTP body cap remains65536bytes. JSON escaping can reach that cap before32KiB
of text does; use small versioned patches when needed. Search still scans only
the first8192bytes and reports incomplete search. Supervised inventory remains
128files (at most4MiB source at32KiB each), not a hard runtime disk quota. Paths,
grants, job limits and runtimes do not expand. Resume must select32again for files
above8KiB; refusal never truncates source. The fixed viewer records the cap in a
strict v2 snapshot; legacy v1 retains8KiB. No model Electron or general build grant.

## Prior-tool observation after restart

Supervised mode keeps one <=1024-byte last-tool observation and one fixed sidecar
outside model-writable src. After local resume, `optic_diagnostics` returns
`previous_tool_checkpoint` with the tool/sequence, admitted or completed phase,
safe success/error and UTC times. It excludes source, arguments, paths, handles,
job IDs and credentials. This is untrusted context, never authority or a claim
that the previous effect did or did not execute. It does not reveal FS internals
or restore hidden chat memory. Existing32-event in-memory history is separate.

Before-dispatch persistence failure returns `project.checkpoint_unavailable`;
concurrent disk work can return `project.checkpoint_busy`. Both disclose
`dispatch_attempted=false` and point to the diagnostic tool. A metadata failure
after an actual success preserves success and adds `checkpoint_saved=false`.
Do not retry a confirmed mutation because its diagnostic could not be saved.
Restarted incomplete/unknown effects require inspection, never automatic replay.
See ADR0030 for fixed storage, process-crash and I/O ownership limits.

## Next delivery checkpoints

1. Verify and deliver the parameterized adapter/package, then measure FS actually
   creates and edits the task-list mini-site with correlated results. Automated
   editing remains distinct from a real model-driven host run.
2. Add one locally chosen isolated test runtime/profile, then process start/status/
   output/termination through existing policy/approval/lease boundaries. Verify
   negative network/process cleanup cases before an FS execution test.
3. Review and consolidate the dependency stack for alpha. Merge verified product
   changes deliberately; research-only negative fixtures need not become runtime.

First exercise: a static HTML task-list app with CSS/JavaScript, followed by local
tests once isolated execution is admitted. No package download, arbitrary shell,
user home/project selection, credentials or Git publishing in the first package.
Broader real-project enrollment follows a successful disposable-project run.
Storage research is parked as a separate track; it does not silently become the
alpha authentication implementation. Existing auth/enrollment gates remain.

### Operator-selected read page size

`Start-ProjectAlpha.ps1 -ReadPageBytes 4096` selects a larger file-read page
ceiling for this run; the default remains 1024. The tool schema defaults to the selected ceiling; smaller explicit requests
remain smaller. `optic_project_info.limits` reports both the page and file limits. UTF-8 characters are
not split, full-file versions still govern edits, and the 8192-byte file limit
and all write permissions are unchanged. This is an experimental throughput
option, not a guarantee that every FS model receives every response untruncated.
Escaped JSON and metadata also contribute to actual response size. Return to
1024 if the host truncates results. This does not expand task output pages.

Search results now include UTF-8 byte offsets for each matching line and the
whole-file `content_version` when `eof=true`. `text_truncated` marks shortened
line previews. `more_matches` means additional hits exist in the searched prefix;
when `eof=false`, the remainder is unknown and `content_version=null`. Use these
offsets for focused reads before a version-conditional edit. Search does not
increase file access authority or read outside its existing bounded prefix.

Local stop remains available after the16-command diagnostic/control budget is
exhausted. Closing stdin or sending a malformed/oversized console line requests
the same fail-closed stop path. This does not remove the current session timer;
the separately planned supervised connection is not enabled.


### Opt-in supervised connection

`Start-ProjectAlpha.ps1 -Supervised` selects availability until local stop instead
of the finite SessionMinutes mode. The package manifest must identify the matching
supervision feature. The Rust core renews the same bounded leases; the adapter
has no renewal/re-provisioning tool. Stop, stdin EOF and local revoke close the
service; lost/expired core authority is not resurrected. The default timed mode
is unchanged. See SUPERVISED_HOST_TEST.md for the complete retained-run flow.

The long-lived adapter keeps fixed60-second ingress/start windows,128conservative
source-file slots (8192bytes each),16retained job handles, bounded process/output
history and the existing one retained effect slot. Terminal results are released
explicitly with optic_task_forget; active/uncertain work is refused. optic_task_list
recovers only handles already received by this bridge for the current credential
epoch. It cannot establish that an unknown upstream start never executed. Source
inventory is checked locally before remote admission; its metadata is not a new
runtime file-access grant. Source creation by the bridge stays within that quota;
out-of-band operator changes are outside the generated-source accounting model.

Native runner proof includes real idle beyond a short lease, new transports,
Node assertion failure/versioned correction/check/test, stop after command-budget
exhaustion, same-run resume, EOF/revoke, pinned-owned runner death/core exit and
crash recovery. This is public generated alpha evidence, not private enrollment,
per-chat isolation, arbitrary dependencies or an unrestricted build environment.

# Optional directory-creation increment

An additional local AllowDirectories switch (Supervised required) enables
optic_fs_mkdir for one absent src directory at a time. Its parent must exist;
expected_kind=absent is mandatory and existing entries conflict. The matching
package manifest must include directory_creation=typed-absent-v1. This is an
explicit separate core capability, not an implication of file write. Default
launches retain their previous tool surface. Generated directory slots are128
including startup inventory; failures/uncertainty conservatively retain a slot.
Native runner proof exercises a new nested source and same-run restart.
This increment does not grant package installation, process writes or an
AI-controlled Electron main process. See ADR0016 for the remaining gates.

## Model-readable diagnostics

Supervised mode adds optic_diagnostics: last8 safe tool outcomes from an in-memory
32-event ring for the current credential epoch, received/finished UTC milliseconds,
monotonic duration and explicit uncertain outcomes/pending work. No argument,
source, credential or raw exception text is retained. task_read remains the route
for actual program output. Diagnostic polls do not evict useful tool events.
Reconnect retains this ring, but restart/rotation does not. On local resume the
operator stop report is captured under the run lock, filtered and invalidated
before editing. Missing prior evidence is unconfirmed shutdown, not proven crash.
Neither feature sees FS model-internal logs or restores hidden conversation state.

### Core integration and package validation — 2026-10-18

The integration candidate brings main through PR246 into this experimental alpha,
retaining local supervised availability and the source-read permission reduction.
Local combined Rust tests,49 Python cases and actual Node HTTP/supervised resume
checks passed. Exact CI still determines the tested package identity.

The package contract is generated-project Node check/test plus a fixed operator
Electron viewer. Its workflow keeps the native viewer/renderer/network/export
checks. The unrelated general private-Electron startup test remains unchanged
in private-application-staging.yml, where normal outer-AppContainer startup still
fails0x80000003 and explicit outer-only comparison is separate. Removing the
extra duplicate invocation from package generation does not resolve that failure,
add a model Electron profile or allow an automatic sandbox downgrade. Node profile
selection rejects electron.exe; the experimental GUI gate remains visibly red.


## Exact localized text replacement

`optic_fs_replace(path, expected_version, old_text, new_text)` replaces one
literal passage in an existing UTF-8 file. Inspect the relevant context and use
the observed opaque core content_version; no UTF-8 offset arithmetic or complete
file rewrite is needed. Empty old_text, no match, multiple matches (including
overlapping occurrences), invalid UTF-8, stale versions and selected-size overflow
are refused. No regex, fuzzy match, replace-all or inferred new version is used.
Whitespace and CRLF count literally; new_text may be empty for a localized removal.

This is adapter composition of existing separately authorized Rust fs_read and
fs_apply_patch actions, not a new core effect or permission. One retained gate
slot covers both calls. The Rust mutation still receives the original expected
version and rechecks live authorization/target state, so an external edit between
read and patch is not overwritten. No direct Python source-file write is used.
The complete bounded preparatory read stays internal; its text is not duplicated
in the model response. File8/32KiB, body64KiB and all source scopes remain unchanged.
Both independent read and write permissions are required; an unavailable upstream
write tool can retain the existing conservative outcome_unknown error. Inspect
state rather than blindly retrying any uncertain mutation.

Supervised recovery classifies this method as a mutation and stores only the
existing safe observation, never the matched/replacement text. The package marker
is text_replacement=single-exact-v1. This does not add build/dependency/GUI authority.
