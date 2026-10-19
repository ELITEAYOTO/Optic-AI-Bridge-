# Real project delivery — current boundary and next acceptance

Reviewed:2026-10-19. Source status, not a production release announcement.
This is the current delivery index; older phase/candidate entries retain history.

## What is available and where

| Layer | Validated behavior | Remaining boundary |
| --- | --- | --- |
| Main core | Scoped/versioned files and directory creation, operator permission reductions, owned process lifecycle, local session supervision, exact nested Node source reads | No general writable build or dependency installation profile |
| Generated FS alpha | File/module edits, exact text replacement, bounded reads/results, selected read-only Node check/test, supervised reconnect/explicit same-run resume and bounded prior-tool evidence | Generated project only; shared connection authority; public dummy credentials are not production credentials |
| Private runtime services | Authorized input capture/staging, owned Node execution, crash evidence and separately authorized same-workspace artifact publication | Host APIs remain unwired to a general public build tool; capture/collection limits are not runtime disk quotas |
| Native compatibility research | Direct rustc compiles, links and executes fixed std applications; Cargo comparison and Electron diagnostics | Test-only evidence, not arbitrary Cargo/dependency/GUI permission |

Main PR252/b33b1f9 delivered the finite-window local supervisor; PR254/e1b8288
promoted exact Node module metadata and intermediate path pinning. Alpha PR251
and PR253 passed exact CI and packaged/native editing acceptance. The current
alpha/main alignment requires its own exact CI before merging. A previous package
must keep its original source identity; this document does not relabel it.

The alpha's opt-in supervised mode retains bounded renewable authorization while
the local supervisor remains active. Default timed mode is retained. Stop/revoke,
individual task limits and uncertainty handling still apply; this is not an
unlimited authorization token or restored authority after a crash.

## Delivery order

1. Keep validated main and alpha aligned with reviewed PRs. Read-only Node module
   resolution must not accidentally grant directory listing or neighboring data.
2. Close the installed-tool runtime requirements using actual native evidence.
   Headless Node/Rust work can advance independently of Electron's GUI blocker.
3. Admit writable build storage only with an explicit growth/cleanup/crash
   contract and truthful limits for every writable location. Do not describe a
   periodic usage scan, post-exit collection ceiling or fixed input-copy limit as
   a hard runtime quota. Select an enforcement design before claiming one.
4. Connect owned build/test execution and output collection to authorized artifact
   publication. Source versions, output ownership and live write consent remain
   separate. Unknown completion never becomes permission to repeat a mutation.
5. Add dependency restore as independent operator-approved egress, cache writes
   and executable code. Start with offline/locked inputs; package names alone do
   not authorize build scripts or arbitrary subprocesses. Keep ambient credentials
   and the operator's installed runtime files outside writable project scope.
6. Deliver a goal-only FS test in a selected disposable project: create the
   structure/modules, test, build, diagnose a real failure, inspect the result and
   resume the same project after disconnect. Then separately admit existing user
   projects, arbitrary Electron main code and other toolchains.

These are acceptance gates, not a calendar estimate or a requirement to create
another agenda/expense fixture. Operator-selected language/tool profiles replace
per-function prompts and fixed sample filenames as the real environment matures.

## Current native blockers

The normal Electron experiment remains blocked during Windows/Chromium sandbox
startup. The explicit outer-only comparison does not authorize an automatic
sandbox downgrade. Its research stays outside main.

The offline Cargo experiment reproduces a separate NUL-device access failure
before rustc startup. A temporary exact-profile NUL permission comparison is
restricted to disposable hosted CI and requires its own cleanup/revoke proof.
It is not an installed setup feature, a global AllPackages grant or a change to
the user's PC. See research PR255; keep normal and prepared results distinct.

No existing installed runtime is bundled into core or granted recursively merely
because a tool fails to start. Optional future host preparation needs a concrete
operator review, explicit scope and recovery/uninstall behavior before use.

## Recovery and diagnostics promises

Optic can retain project files, observed versions, bounded task/transport evidence
and an untrusted project handoff. It cannot restore FS's hidden model state or
read FS backend logs it has not received. Server completion is not proof that a
host delivered the complete result to the model. Preserve unknown outcomes and
refresh observed state before a new authorized change.

The next user-facing test must identify the exact source/package, selected
workspace, tool profile, artifact path and stop/resume procedure. There is no
manual user action required while native gates and integration are unfinished.
