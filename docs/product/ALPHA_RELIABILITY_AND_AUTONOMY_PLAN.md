# Alpha reliability, project autonomy and integration plan

Status: APPROVED USER DIRECTION / IMPLEMENTATION PLAN, not delivered capabilities.
Date: 2026-10-15. Owner: current Optic development track.

This is the active near-term execution plan. The broader scope roadmap remains
canonical; this document orders delivery gates and explicitly replaces further
ungated fixture proliferation with integration and user-visible increments.

## Current execution — 2026-10-19

The [real-project delivery index](REAL_PROJECT_DELIVERY.md) supersedes older
"next gate" and "not delivered" descriptions below. Main supervision is merged
(PR252), exact Node module metadata/intermediate pinning is merged (PR254), and
alpha exact text replacement plus main alignment passed (PR251/253). The next
alignment candidate retains those features and excludes Cargo/Electron research.

Local native Rust std compilation works. Offline Cargo has a separately measured
NUL-device requirement; its disposable exact-profile CI comparison does not grant
production setup authority. Electron's normal sandbox still has its own startup
blocker. Neither should send the user back through working UI fixtures or make
headless Node/Rust delivery depend on a GUI workaround. Continue through runtime,
write-storage, output-publication and dependency gates before the next FS project.

The old49-test count and earlier candidate/pending wording below are historical;
PR251's editing adapter was validated with76 cases and real packaged HTTP/Rust
acceptance. Preserve exact source identity for every artifact and new proof.

## Latest measured execution — 2026-10-16

The c8a303d Windows CI rerun passed; the earlier TEMP-only Node initialize
failure remains intermittent evidence, not an open deterministic failure.
Packaged permission/lifecycle/crash/resume proofs passed locally. There are
49 alpha regression tests. The prior pending descriptions below are historical.

Versioned multi-file input manifests and native authorized capture now pass
Rust Windows/Linux CI (including revocation, deadlines, native root replacement
and read ceilings). PR224 passed independent CI38836808832 at head72c06c4
and merged into main asbcd26aa. No new build/GUI tool is exposed yet.

Private Node staging passed native CI with relative modules, private output and
external source writes denied. General Electron startup failed both natively
on this PC and in GitHub CI, before the fixed application executes, without FS
or any tunnel. The normal sandboxed path stops with0x80000003. An explicit
outer-AppContainer comparison passed its fixed CI probe at623e003; no automatic
sandbox downgrade is permitted. Exact SID/capability sampling and owned-tree
cleanup are part of the probe. New diagnostic head623e003 retains bounded error
tails rather than aborting on large startup logs. Local execution reached the
renderer but cleanup hit a sharing violation; bounded owned-directory cleanup
retry9e9bbdb is pending native validation. This is not a GUI release.

Next gate remains a real optional Electron runtime with an owned isolated tree,
then bounded production staging/artifact publication, scoped dependency restore
and local GUI launch. Rust follows the same authorized task contract. This is a
native runtime compatibility blocker distinct from FS generation cutoffs. Do
not ask the user to repeat the working agenda/expense fixtures or send longer
prompts as a proposed repair for either failure.
## Execution update — 2026-10-15

### Superseding update — 2026-10-16

The user requests a genuine Electron project next, then Rust, rather than another
agenda-only or fixed Node fixture acceptance loop. Core directory creation and
operator JSON permission reductions passed all nine workflows at0f0c7ae.
The HTTP directory adapter passed46tests and actual Windows nested-source
stop/resume. Bounded diagnostics/previous-stop summaries subsequently passed
48tests and the native crash/recovery proof; final follow-up CI is required.
No new user action is needed for these foundation increments.

Next execution gate: use ephemeral private staging for module inputs and build
outputs instead of recursive write ACLs on real user sources. An explicit native
Node experiment must prove relative module resolution, owned output creation
and denial of external source writes. Success does not itself deliver bounded
production snapshots, dependency egress/install, artifact publication, Rust
toolchain access or Electron GUI containment. Implement those contracts before
advertising a goal-only full-application test. Local Rust linking also needs the
missing MSVC library; CI remains the current Windows/Linux build authority.

R0: generated-data diagnostics completed locally and through Cloudflare. FS
administrator reproduction was reported; backend cause/fix remains unconfirmed.
R1: native Windows manual/expiry/blocked-output/broken-pipe shutdown cases passed;
the default proof writer no longer uses Python buffered stdout. See the detailed
diagnostic evidence ledger for limits. R2: PR184 merged to main as18a8ec6 and
integrated into alpha; exact head8ecd2bb passed all nine workflows including
CI38761152794 and Read Link38761152706. R3: optional operator4KiB pages remain
experimental with1KiB default. Following source work exposes that cap in the MCP
schema/project_info and adds versioned search with byte offsets/truncation flags.
R4–R6 remain planned; no indefinitely available connection or arbitrary project
build permissions have been delivered by these changes.

The decisions below retain the initial planning evidence; later execution facts
above and linked research reports supersede their pending-state descriptions.

## Decisions and current evidence

- The user reports that Luna eventually completed the attempted corrections.
  This is reported host success, not independent browser/Electron acceptance.
- Real FS editing, versioned writes, Node failures and subsequent passing tasks
  have been measured. The retained alpha can reopen without regenerating sources.
- Several failed host turns end after fast, completed local reads/project_info.
  Observed last-response-to-client-DELETE gaps include51.187,73.906,87.188,
  56.500 and72.922seconds. These are MCP-side intervals, not measurements of the
  FS generation stream's last received byte. They do not identify a timeout owner.
- The user reports the same visible failure with ngrok and Cloudflare. Provider
  independence is plausible but neither tunnel is exonerated by this observation.
- The unofficial FS SDK at d8d054f7c9631756db200369e88569cf1b5a9e2f defines
  60s first-header,90s idle and310s overall stream defaults. This does not establish
  the FS website/backend configuration or prove that any limit caused a failure.
- A separate HTTP400 request_shape rejection was measured before dispatch; this
  is distinct from generation silence, authentication denial and session expiry.
- A Python fatal shutdown reported stdout buffered-lock ownership by daemon
  threads. ProofChannel currently joins its daemon output worker for only0.2s;
  an input daemon also remains possible. Exact offender/reproduction is pending.
- Runtime e37c6a7 passed all nine workflows and exact packaged Windows restart
  proof. It does not close the later manual shutdown defect.
- GitHub audit today found42 open PRs (#169–#223, nonconsecutive). #223 is Draft
  and targets experiment/public-root-identity-check, not main. Green stacked PRs
  are not equivalent to reviewed changes delivered on main.

## Delivery order and acceptance gates

| Order | Deliverable | Acceptance gate |
|---|---|---|
| R0 | Reproducible connection diagnosis | Correlated bounded report separates observed boundaries; unknowns remain explicit |
| R1 | Reliable local stop/expiry/crash recovery | Native Windows stop/expiry/output-stall cases exit without Python fatal shutdown and preserve sources |
| R2 | First reviewed integration batches | Fresh main-based candidate, exact-head CI and boundary regressions; merged behavior and changelog recorded |
| R3 | Efficient file/module/error tools | Real multi-file app without forced minification; measured reduction in calls/tokens with complete results |
| R4 | Operator-controlled persistent project connection | Disconnect/reconnect and supervised renewal retain project without widening authority; local stop reliably revokes |
| R5 | Operator-selected project/task environment | Model chooses implementation within authorized directory and task/dependency boundaries; small goal-only prompt succeeds |
| R6 | Real Node/Electron and Rust projects | Isolated build/test/artifacts plus separately approved native GUI launch, reproducible on user Windows |

R0/R1 and integration inventory can proceed together. Do not use tunnel/model
outages as a reason to postpone all local work or all merges. Security fixes
needed by the next increment precede increased external authority.

## R0 — Diagnostic experiment

Build a generated-data-only harness, separate from the user's active application.
No account cookies, real project contents or auth values in reports.

1. Use a scripted MCP client with fixed inputs against the exact same harmless
   server build: direct loopback, Cloudflare, ngrok. Run3 repetitions per cell,
   initially payloads128B/1KiB/4KiB/8KiB and response delays0/1/3/10s. Keep the
   admitted execution/transport budgets explicit; a deliberate exceeded budget
   should produce the expected classified failure, not an accidental live hang.
2. Separately vary client idle gaps15/45/75/100s and reconnect. In the current
   stateless JSON transport, a completed response is not an open response stream;
   idle keep-alive closure can be normal if a subsequent request reconnects.
3. On FS run short tools-only, read-then-short-answer, read-then-one-edit, and a
   bounded deliberate slow-tool case. Same synthetic files/prompt/model, record
   configured model label and3 repeats. Test a second model only after holding
   other variables constant. Do not replay a mutation after unknown completion.
4. Correlate run/request/invocation ids, UTC milliseconds, monotonic admission,
   tool begin/end, response start/end, client response received/body parsed,
   response byte count, process exit and local stop reason. Existing COMPLETE
   proves local ASGI send completion, not remote receipt or model consumption.
5. Keep reports bounded (target<=10MiB per run, at most3 retained report files;
   reject/refuse or explicitly count dropped diagnostics when capacity is reached).
   Include HTTP status AND tool ok/error code; HTTP200 alone is not tool success.
6. Request admin-side correlation only when needed: stream start/last-byte/end,
   event type (including response.failed/error), upstream status/finish reason and
   applied proxy/provider/idle/overall deadlines. No raw credential-bearing HAR or
   message contents are required. FS stream data is unavailable from Optic alone.

Interpretation: direct failure implicates a local/client path; direct success and
repeatable tunnel-client failure localize a provider path. Success through both
tunnels with scripted client but FS failure localizes the remaining host/model/
configuration path, without proving the precise responsible component. A model
code/assertion error is a separate category from connection loss.

Do not claim that a prompt preventing long thinking or arbitrary MCP pings fixes
the issue. Heartbeats are appropriate only on an identified live streaming link,
where supported by its protocol and client; Optic cannot heartbeat the separate
FS-to-model stream. Preserve the stateless JSON baseline for the first experiment.

## R1 — Shutdown and recovery

Give stop one owner and a defined sequence: close admission, drain or cancel owned
work, revoke core session, prepare viewer only on confirmed success, terminate
and account for console/diagnostic workers, then return from interpreter.

Reproduce both manual stop and expiry while output is queued, redirected, slow,
broken or full; include stop during an admitted request, stdin EOF and repeated
stop. Use owned subprocesses with bounded watchdogs. A thread join timeout alone
cannot make a worker holding Python's buffered stdout safe at interpreter exit;
do not replace the fatal error with an unbounded shutdown hang. Choose an output
ownership/nonblocking or isolated sink design based on this reproduction.

Write a bounded local stop report outside model-editable source. Record confirmed
revocation separately from unknown cleanup. Abrupt exit must leave sources and
journal intact; no viewer-ready claim on uncertain stop. Resume uses fresh jobs
and authorization and never replays an uncertain mutation automatically.

## R2 — Main branch and PR consolidation

Inventory exact head/base, unique diff, review status, known gaps and CI for each
of the42 PRs. Create a dependency graph and disposition list, not42 automatic
merges. Prioritize: production boundary corrections (e.g. #184), documentation
alignment, reusable diagnostics, then a cohesive explicitly experimental alpha.
Disposition each historical fixture as merge-as-test-only, superseded/close after
retaining evidence, or still gated. Negative characterization tests must retain
their non-claims; a passing test may demonstrate an unresolved vulnerability.

Build each integration batch from fresh main without importing the entire stack
unreviewed. Use squash for a cohesive noisy development unit, or dependency-order
merge where preserving ancestry prevents duplicate/conflicting changes. Rebase/
retarget only after checking downstream PR effects. Exact candidate CI must be
green on Windows/Linux and include tests relevant to the affected boundary.
Publish concise behavior/validation/remainder notes. Keep unfinished features
marked experimental; merging them does not declare production readiness.

No merge was performed by preparation of this plan. User authorization to merge
exists; final review/CI determines readiness, rather than another generic approval.

## R3 — File access, schemas and model efficiency

1024-byte responses were a compatibility workaround for observed host result
truncation. Finite response bounds are required;1024 is not intrinsically secure.
Current alpha also reads up to8193bytes upstream for each page, so repeated pages
can reread/hash the same small file. Measure this rather than advertise savings.

Replace the fixed page choice with bounded host-compatible profiles (initial
candidate1/4/8KiB), truthful EOF/next offset/version, UTF-8-safe offsets, line-range
reads, targeted search and bounded multi-file batches with aggregate budgets.
Test exact metadata placement and last-byte receipt on FS before increasing the
default. Full-file rereads are not necessary for every small edit: exact patch
context plus an observed whole-file version and subsequent checks can suffice.

Lift the8KiB alpha source ceiling through independently reviewed file/body/output
budgets and proper multi-file modules. Do not force minification or weaken version
preconditions. Read-before-edit only relevant context; optional versioned bounded
snapshots need their own cache/memory/expiry design. Retain binary handling and
explicit truncation indicators; no invisible cutoffs.

Make tool schemas concise and errors actionable: bounded safe expected-field/type
diagnostics, distinction between invalid arguments, policy denial, tool failure
and transport failure. Ordinary invalid tool input should not unnecessarily turn
into a host-fatal connection experience; preserve strict ingress/auth validation.

Benchmark same tasks/build on fixed fixtures: calls, request/result UTF-8 bytes,
latency, reread volume, mutations, retries and task outcomes. Report host token
usage only when actually available; byte counts are not token counts. Compare
with equally bounded shell output, not a deliberately verbose shell baseline.

## R4 — Stop-controlled connection and resumable work

Adapter increment implemented (exact-head CI pending): opt-in Supervised mode,
fixed-window ingress/start budgets, bounded generated-source inventory and slots,
explicit terminal-result release and current-epoch task inventory. Native Windows
proof passed renew/reconnect, Node workflows, local stop/EOF/revoke, actual parent
crash/core exit and same-run recovery. Default timed mode stays unchanged. See
ADR0013 amendment and SUPERVISED_HOST_TEST.md. Private-project admission, broader
Node modules/builds and host-specific FS acceptance remain separate gates.


Core foundation implemented: opt-in local CLI supervision renews only existing
bounded leases through SessionLifecycleManager, with no new MCP renewal tool.
Native Windows proofs confirm default expiry, unattended renewal with unchanged
authority, terminal revoke, expiry across process suspension and clean EOF exit.
See experiments/project-alpha/SUPERVISION_CORE.md. HTTP adapter/windowed ingress,
long-lived job retention and final remote acceptance remain gated; the delivered
alpha timer is not removed by the core foundation alone.


Accepted user experience: Optic stays available until the operator stops it;
ordinary host disconnect, one chat ending or elapsed50minutes must not discard
the project or require restarting the whole setup. This is a proposed new local
supervised mode, not available in the delivered50-minute package.

Separate persistent project enrollment, transport connection, renewable bounded
authorization leases, and individual task time/resource budgets. Trusted local
supervision may refresh leases only within the operator's existing exact scope,
while active and unrevoked; model requests, tunnel traffic and heartbeat activity
cannot grant or extend authority. Policy changes/revoke stop renewals and actions.
Design revocation races, sleep/resume, supervisor death and crash behavior before
implementation; update ADR0013 and the charter's time-bound wording accordingly.

Remove artificial lifetime512-request exhaustion in supervised mode in favor of
bounded per-window admission/rate limits and bounded job/history retention. Keep
per-task timeout, output/memory/process limits, disk/log bounds and immediate stop.
Persistent credentials/enrollment require the applicable storage/security gates;
public dummy alpha credentials must not be reused for a private-project service.

FS chat history is host-owned: Optic can retain files, observed versions, bounded
task evidence and a user-visible project handoff, not restore hidden model memory.
An optional model-readable handoff is untrusted context, never authorization.
Until FS provides trustworthy chat binding, disclose that chats using one
connection share the locally selected project; do not promise per-chat isolation.

## R5/R6 — Real environment and toolchain autonomy

Operator selects a dedicated project directory and capability/task preset once.
The model can plan files/modules/tests and choose among permitted tools without
being instructed one function per message. Preserve conflict-aware edits, path
identity, scoped create/rename/delete, task ownership and explicit irreversible
actions. Start with generated workspaces; private existing-project admission
requires remaining sensitive-content/Git-control/enrollment gates.

Detect installed runtimes and expose structured doctor results. Add fixed or
operator-approved build/test/dev profiles, authorized dependency restore with
separate egress/write/subprocess budgets, bounded artifact inspection and owned
dev-server lifecycle. npm/cargo/build scripts execute code; approving a command
name alone does not contain it. Writable build outputs, package-manager caches,
native toolchain children and allowed network hosts need explicit design/tests.

Node first, then real Electron project main/preload/renderer build and packaging,
then Rust cargo check/test/build, and later Java/Maven/Gradle. Use existing local
tools; do not bundle toolchains or add an Electron requirement to Optic core.
The current fixed viewer is not proof that arbitrary AI-generated Electron main
code is contained. Interactive GUI launch is separately explicit and locally
visible, with its own authority/stop contract; do not advertise AppContainer GUI
support until actually proven.

Acceptance uses a short goal-only brief in a selected workspace: model creates
the app structure, tests, builds, diagnoses a failing case and produces an artifact;
operator validates the UI and locally stops/reopens without lost sources. No
mandatory imposed HTML/model/tests filenames or8KiB global design constraint.

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