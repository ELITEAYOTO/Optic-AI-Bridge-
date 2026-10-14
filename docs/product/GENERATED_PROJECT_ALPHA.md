# Generated-project editing alpha

Date:2026-10-14. Implements the first practical single-session editing exercise
under ADR0013; not production enrollment or all-tool release.

The Windows package exposes exactly six bounded tools over HTTP to a retained
generated project. Rust owns FileRead/FileSearch/FileWrite effects, policy and
exact mutation preconditions; the Python adapter cannot mint session authority.
The local CLI chooses only its generated workspace and prefix:src write scope.
All host chats on this connection share the same session/project and grants.

The remote boundary has a fixed tool allowlist, strict duplicate/nonfinite JSON
refusal, typed bounds, relative ASCII path segments, UTF-8 contents, exact absent/
content versions and no caller project/session/authority selectors. Output payload
ceiling65536bytes (including worst-case escaped UTF-8 text); core upstream structured ceiling16384bytes. Full offset0 core
reads expose a BLAKE3 ContentVersion of observed bytes, not a credential or a
promise that a later mutation will succeed. Partial/tail reads return null.

One mutation/read forward slot, no queue; caller timeout/disconnect does not free
it until the underlying call ends. Authentication rechecks before dispatch and
before results, including after patch's preparatory bounded read. A dispatched
write may already commit when reply/auth changes: outcome_unknown suppresses
the result without claiming rollback. Subsequent recovery must observe the file
and choose an explicit expected state; no automatic effect retry is implemented.
Local stop first blocks admission/drains, then calls actual Rust session_cancel.
Failed drain/revoke never produces a success acknowledgement.

Request ceilings/lifetime inherit the proven probe guard:64KiB body,16KiB headers,
8 active requests,512 lifetime requests,25minutes. Files/patch results <=8192bytes;
32list entries/page,16literal search hits/file, output proof queue128/budget4096.
Diagnostics contain fixed tool names/markers/status and public match bits, never
contents, caller paths or credential values. The local console shows only the
generated relative run name. Workspaces/journals are saved after stop, not reset
or reused on restart. No credential persistence is introduced.

Public dummy auth remains a fixture control, not real-project confidentiality.
The package takes no existing project. Initial exercise is a generated static
task-list app; Java/application sources may be edited but no toolchain or process
is granted. Subdirectory creation is not yet a remote tool; a few fixed source
folders are prepared. Isolated runtime/build/dependency/server scopes (especially
Java/Minecraft) remain separate delivery gates. Browser preview has no Optic
containment guarantee. No production Store or research journal format selected.

Acceptance before manual host exercise: exact current-source Windows/Linux CI,
actual HTTP->Rust create/read/patch/stale denial, owned runner startup/edit/stop
with retained output, strict schema/auth negatives, Rust partial-version tests,
matching artifact manifest and target-machine package smoke. FS host results are
recorded separately after the user runs HOST_TEST_STEPS.md.
