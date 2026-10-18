# ADR0026 — Local permission switches reduce selected authority

Status: IMPLEMENTED CANDIDATE / EXACT VALIDATION REQUIRED
Date: 2026-10-17

Promote the alpha's optional startup JSON permission switches independently of
its HTTP adapter. The operator still selects workspace scopes, executable/profile
identities, mutation state and Git runtime/ref. Booleans can remove these choices;
they cannot invent a scope or override hard ceilings, policy, isolation or effect
ownership. Core gets no required GUI/runtime dependency.

Optional permissions contains files(read, search, write, delete,
create_directories), processes(run) and git(read, merge). Absent permissions keeps
legacy explicit startup behavior. Present permissions defaults every omitted bit
to false. Unknown fields, non-booleans and duplicated fields are rejected. True
means enable only separately supplied existing selections. Git merge is the
existing exact internal-ref GitIntegrate effect, not GitHub merge/commit/push.

Read/search denial removes session capabilities and scoped read authorities.
Disabling read also removes configured process source-read grants. CLI
--deny-file-read overrides such grants regardless of argument order. Disabling
run removes executable/profile/environment/process-file selections. Write,
delete and directory choices independently reduce their mutation leases. These
switches do not authorize network egress, package scripts, general build writes,
GUI launch or a full-access bypass. Existing process isolation rules still apply.

Configuration and enabled profile files supplied through config mode must resolve
outside the writable project. Windows uses the opened file's final path; other
platforms use canonical paths. This is a startup source-boundary check and a
snapshot, not continuous file protection or live reload. Existing private-project
enrollment/sensitivity gates remain. Models cannot edit policy through tools.
Future live changes must fence admission, advance epoch, revoke work and confirm
owned cleanup before publishing replacement rights; file edits do not do that.

Validation includes strict/default-off/no-implicit-scope inputs, protected source
locations, order-independent CLI read denial and a real stdio MCP child. The
actual child must report only allowed capabilities, deny disabled read/list,
hide disabled mutations and execute directory-only authority on Windows, then
preserve legacy reads when the section is absent. Child shutdown and bounded
client output are explicit; no user project is touched. Exact Windows/Linux CI
must pass before main promotion.
