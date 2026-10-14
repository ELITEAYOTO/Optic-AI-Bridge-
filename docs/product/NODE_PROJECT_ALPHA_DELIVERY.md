# Node project alpha delivery

User selected a web application with Node.js as the next end-to-end exercise
on2026-10-14. Java/Minecraft remains a later execution gate, not a dropped goal.
This follows Product Charter and ADR0013: one locally selected project/session
per FS connection; model power does not grant authority.

## Intended operator journey

1. Locally select a dedicated disposable development directory or a worktree
   for an existing repository. Display the actual canonical project and scope.
2. Locally select installed Node and named check/test/build/server profiles.
   Display arguments, read/write areas, time/output/memory/process limits and
   network permissions. Reusable permission belongs to the connection/session.
3. Enroll a non-public credential before any personal project is accepted.
   Existing public dummy tokens cannot protect arbitrary user workspaces.
4. AI creates directories/files, inspects/searches/patches them, then invokes
   configured tasks and reads structured status/output. No arbitrary shell/PID.
5. Operator stop/revoke terminates owned jobs and preserves project output.
   Unknown mutation/start results are observed, never automatically retried.

The next user test should demonstrate actual app creation, a failing test,
source correction, passing test and build artifact/preview where authorized.
Do not call syntax checking alone an end-to-end application build. Dependencies
and localhost preview need separate verified network/containment scopes.

## Implemented preparation

execution_gate.py forwards named frozen operator-owned profiles into existing
Rust process_start/read/result/stop requests. Remote arguments cannot replace
the executable/args/cwd/environment/network/budgets or grant authority. At most
8 profiles and16 lifetime starts, one retained forwarding slot, bounded output,
session/principal/epoch job binding, cancellation-safe accounting and redacted
uncertainty errors. Stop acknowledges a request only, not proven process exit.
Rust remains the authority for typed effects, lease/profile approval and
isolation. No SDK elicitation is auto-approved by this adapter.

Seven synthetic gate tests and one real-SDK HTTP test verify these boundaries;
the combined suite now has18 passing tests. Optional explicit composition
registers four task tools beside six editing tools, with shared retained
admission and stop suppression. The standard editing startup still supplies no
execution gate and exposes exactly six tools. No FS Node execution is claimed.

Local native Windows proof used the CI767 binary/helper pair, artifact11910869922
(SHA256 aba0bfc63d93cc2729e44cc07b62a67d6ec91091a776ce82222a7499029bf8a5),
sourcee6f2336, with the installed Node22.15.1. Existing isolated MCP negatives
passed, including sibling-file denial. New HTTP proof: a locally frozen driver
reads exactly granted project sources, reaches a failing assertion, receives a
version-preconditioned source fix and passes syntax/test. Stop revokes Rust
session. vm is a test driver only; AppContainer owns containment.

Direct unflagged Node --check <file> failed locally on ancestor lstat EPERM
despite exact-file grants. With locally frozen --preserve-symlinks-main and
--preserve-symlinks options, both syntax checking and direct test-file entry
passed without adding ancestor ACL rights. The test-file entry also proves an
ungranted sibling remains unreadable. The unflagged result is host-dependent
characterization; unrelated failure/timeout is still rejected. No general npm/
module-tree/build output/network permission follows from this small proof.
Dependency installation, build output writes and preview remain gated.
Relative require('./math.js') also failed locally although direct fs reading of
the explicitly granted math.js bytes succeeded. Native characterization
distinguishes direct read from module lookup, with a dedicated exit44 marker;
unrelated process failure/timeouts cannot pass it. Directory metadata lookup
must be solved for real multi-module Node projects without granting outside
workspace content. A fixed driver is not the final application experience.
Workflow now builds sibling helpers and runs this native characterization on
Windows. Current-source CI confirmation remains pending.

Operator-configured compact_results removes per-call probe/auth metadata from
AI replies while retaining local TOOL proof logs and useful status/version/output.
The existing diagnostic editing runner remains verbose by default. No token
benchmark claim is made. project_info now reports connection.remaining_seconds
and expires_soon, and the runner warns locally120seconds before normal expiry.
It neither renews core grants nor silently adopts a stopped project.

## Remaining integration gates

- Package the shared admission/lifecycle and measured isolated exact-file Node
  check/test path, then validate normal module trees without expanding grants
  implicitly. Actual network denial remains an independent required check.
- Explicit workspace creation/read/write scope and safe directory tools;
  runtime output-write/descendant needs cannot be inferred from FileWrite.
- Local enrollment/reusable grants suitable for selected personal projects.
- Bounded useful text/search/output without proof metadata dominating results;
  show remaining session time and warn before expiry instead of late surprises.
- Dependency/build/preview permissions only after their actual isolation proof.

Current blocking evidence for native execution: CI769 Windows test
capability_free_appcontainer_denies_node_loopback_connection timed out before
Node emitted NETWORK_ATTEMPT (stdout/stderr empty, no connection accepted).
This is not proof of a successful network denial. Failed job was rerun without
weakening assertions; rerun was cancelled when a newer head started. Atf6827cb,
all nine workflows succeeded, including CI772's actual Windows network denial,
isolated Node and reusable approval checks, and real-link24's native Node HTTP
proof. This resolves the earlier failed check for that tested head; later UX
changes still require their own CI confirmation.

## Host evidence already collected

FS generated three task-list files, paged reads reached eof with stable versions
(3003/6700/7342bytes). User confirmed add/check/delete, filter/counter, rename,
Enter/Escape, empty-name refusal and localStorage persistence. UTC timestamps
present. Last requested counter extension did not appear on disk; supplied logs
show no completed corresponding mutation before25-minute local stop. Keep this
as an incomplete modification, not a passed edit or causal FS crash diagnosis.
