# Crash before native ACK — 24 October 2026

Research only; no production fix, merge, or V1 GO.

Verified live reference branch: research/317-repeated-native-observation-2026-10-24,
47b6a6310c8bd9d494a0facd2aa661cbced9fc75. Reference CI 40050805563 successful.
Verified live main: 16ca67a75fd49c5681a0672773ffc268c3635f6b.
New isolated worktree/branch: research/317-crash-before-native-ack-2026-10-24.
Existing dirty launcher-crash-transfer worktree was preserved. Four draft files
were copied as a starting point, then reviewed and completed here; no claim that
the uncommitted drafts were validated. The 24 October checkpoint is in #317's
latest comments and dated research documents; no dated 24 October engineering
handoff exists at the reference SHA. ADR-0019 and ADR-0031 remain authoritative.

## Schedules and proof boundary

The guardian owns the genuine non-clonable admission and registered reservation
before starting a separate launcher PROCESS. The reservation is conservatively
prearmed. A fixed suspended current-libtest child is created atomically in the
launcher's own kill-on-close Job. No user repository, Git command, child resume,
arbitrary PID termination, or unsafe runtime code is used.

An independent Python supervisor remains outside its newly created outer Job.
The service host is assigned before an outer gate allows launcher creation.
Its descendants inherit outer containment. The launcher is terminated through
the exact owned CreateProcessW process handle, never via PID lookup. No outer
Job termination occurs before survivor-count and inner-cleanup observations.

Schedules: before-create; after-create; offered values without acceptance;
accepted Job alone; accepted Job and process handle before ACK. No ACK is sent.
DuplicateHandle duplicates from the exact owned source process. A Job accounting
query validates the duplicate; IsProcessInJob validates the accepted child.
Values in an offer alone are explicitly not ownership.

Without a duplicate Job, the guardian has no positive native empty-Job receipt.
It retains its actual admission/reservation, publishes no terminal state and
leaves revoke pending. External survivor accounting may prove no synthetic child
remains, but is NOT used to release this service's quota. The host is then ended
by outer recovery. This is conservative quarantine in a fixture, not durable
production recovery, bounded graceful shutdown or automatic reconciliation.

With a duplicate Job, the child must still survive launcher death (outer count 2:
service host plus one suspended synthetic child) before stop. A deliberately injected query failure retains Running/quota and
blocks synchronous revoke; an independent close request has already returned.
Only actual TerminateJobObject, optional process-handle exit, and a positive
zero-members query on that accepted Job permit Stopped and admission release.
The supervisor checks outer count 1 (the service host alone) BEFORE any outer
cleanup. Job-only acceptance
proves tree emptiness without asserting a separate root-handle exit receipt.

The first-red control is deliberately synthesized: missing acceptance returns
from the guardian, discarding genuine admission/worker ownership. The original
assertion `guardian vanished before retaining uncertain owner` is preserved and
must fail in the child host. This is NOT a discovered production execution path
or evidence that the old successful channel-handoff test would behave identically.
The same missing-receipt schedule with retention must remain explicitly pending.

## Validation

Local environment inventory: Windows 10 build 19045; Rust 1.99.0
(b940084d7, x86_64-pc-windows-msvc). Local Rustfmt check and git diff --check.
Native campaign is restricted to GitHub-hosted Windows, not this operator PC.
Hosted workflow records actual Windows version and rustc -Vv; artifact manifest
records exact source SHA, cases, phase receipts, and outer cleanup confirmation.

Commands: cargo fmt --all -- --check;
cargo clippy --locked -p optic-bridge-runtime --all-targets --all-features -- -D warnings;
cargo test --locked -p optic-bridge-runtime --lib;
cargo test --locked -p optic-bridge-runtime --lib --no-run --message-format=json;
python scripts/research/issue317_crash_transfer.py --test-exe <exact-built-libtest> --output crash-evidence.

### Hosted evidence verified on exact SHAs

- Initial prototype `c1ace797`: [CI #40052119243](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40052119243) **failed**. Preserve this original red checkpoint; it is not a successful research result.
- Corrected Win32 fixture `ce11fbe8`: [CI #40052249699](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40052249699) **passed**, including six externally supervised Windows schedules, rustfmt, strict Clippy and runtime library tests on the hosted matrix.
- Workspace-check addition `ab812b26`: [CI #40052329408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40052329408) **passed**, including the added Windows `cargo test --locked --workspace` step.
- Latest source-changing checkpoint `3df5ebfd2b955b00d4041731038721c7b017f2d0`: [CI #40053510714](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40053510714) **passed on attempt 1**. Ubuntu runtime library: **270 passed, 0 failed**. Windows runtime library: **355 passed, 0 failed, 13 ignored** (two fixed external-host entrypoints intentionally ignored by regular libtest). Windows full workspace validation also passed. Rustfmt and strict Clippy succeeded on both operating systems.
- Subsequent report-only checkpoint `9d6dedf431215a41718d1611effff5b4c3f8409b`: [CI #40053843692](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40053843692) **passed on attempt 1**, with both Windows and Ubuntu jobs successful. Windows again passed the six externally supervised schedules and full workspace suite. No production behavior changed in this report-only commit. These checks validate the latest previously published branch tip at the time of this audit, not the present documentation clarification commit.

The latest Windows job's printed supervisor manifest contains `complete: true` and identifies the runner as `Windows-2025Server-10.0.26100-SP0`, **not a local Windows 10/11 acceptance test**. Evidence artifact `issue317-crash-evidence-3df5ebfd2b955b00d4041731038721c7b017f2d0` (artifact ID `12544656231`) was uploaded by the workflow. The six observed outcomes are:

| Supervised case | Native Job duplicate accepted? | Total processes in outer Job before cleanup (includes service host) | Result |
| --- | --- | ---: | --- |
| Negative control: `after-create` with intentionally lost guardian | No | 1 | `FirstRedGuardianLost` (expected test counterexample, **not** a discovered production incident) |
| `before-create` | No | 1 | `PendingNoNativeReceipt` |
| `after-create` | No | 1 | `PendingNoNativeReceipt` |
| `offered` without acceptance | No | 1 | `PendingNoNativeReceipt` |
| `job-only` | Yes | 2 | `GuardianJobEmptyProven` |
| `accepted-before-ack` | Yes | 2 | `GuardianJobEmptyProven` |

The counts are the **outer supervisor's** independent process accounting before it performs emergency outer cleanup: **1 = service host only; 2 = service host plus one synthetic child**. They are *not* counts of descendants alone. In the no-native-receipt cases, the in-process guardian retains the original admission and quota and **does not** publish a false terminal result; the supervisor must eventually end the entire synthetic service host. This is quarantine, **not** a service liveness fix. In the accepted-native-Job cases, a real positive inner-Job observation authorizes native terminal publication and resource release; the outer Job does not manufacture this proof.

The earlier first-red failure, the absence of a receipt, and the positive accepted-Job proofs are distinct evidence categories. A green CI means the **expected scenario assertions** passed, not that all crash paths are production-safe. No broad research merge or V1 GO is authorized.

## Residual risk and architecture

Prefer a service owner that already holds the admission, registration and native
Job before delegating child creation. Cross-process handoff must define durable
acceptance, duplicate ownership, acknowledgement loss, partial handle delivery,
and reconciliation for inaccessible source handles. The scheduled crash points
bracket ownership transitions, but do not exhaust every native-call interruption
or failure path. Crash-before-create with
conservative prearm illustrates retained capacity without a recoverable native
receipt. An offer or timeout cannot resolve that ambiguity. A service process
crash additionally needs external durable quarantine/accounting; this fixture's
in-process guardian does not provide it. Descendant stdio, EOF, persistent real
Win32 failures, host power loss, and production task/lease provenance remain open.


## Separate service-preowned Job extension — 24 October 2026

Research-only extension on `research/317-service-preowned-native-job-2026-10-24`.
The *original* six-case matrix above remains historical evidence for
`3df5ebfd` / `9d6dedf` / `ef264fc`; it has not been retroactively rewritten.

**Exact code SHA:** `49c39d0339f6ac53cadc3ee924f359f329555f4d`.
[CI #40055108927](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055108927)
completed successfully, attempt 1 on that exact SHA. Hosted Windows
runtime library: **355 passed / 0 failed / 13 ignored**; Ubuntu:
**270 passed / 0 failed**; Rustfmt, strict Clippy, and full Windows
workspace tests passed. The Windows hosted runner reported
`Windows-2025Server-10.0.26100-SP0`, not an operator PC Windows 10/11
acceptance. The exact Windows job emitted a supervisor manifest with
`complete: true`; the evidence artifact is available from
[the run](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055108927/artifacts/12544229304).
The **first source attempt** [#40055072684](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055072684)
failed strict Rustfmt before tests. Only the exact formatting difference
was corrected, without weakening assertions or gates.

The new service-like guardian creates and retains the REAL native Job with
memory and process limits and owns the counted admission and JobStore
reservation **before** a separate Windows launcher process is permitted
to act. It duplicates a Job HANDLE into that fixed test launcher, but
retains the original native handle: neither later receipt nor child-handle
transfer is required to *obtain* native Job ownership. The delegated
launcher creates only the fixed current-libtest child, suspended with
creation-time Job membership; a separate outer supervisor observes
survivors **before** emergency cleanup.

| Added schedule | Before-crash outer count (host+launcher+optional child) | After-launcher-death outer count | Guardian proof / terminal status |
| --- | ---: | ---: | --- |
| `preowned-before-create` | 2 | 1 (host only) | Positive original Job-empty proof; `Failed` |
| `preowned-after-create` | 3 | 2 (host and suspended child) | Original Job remains owned; exact Job terminated and positively empty; `Stopped` |

In both schedules **no ACK** is delivered, no root process handle is
accepted by the guardian, and a synthetic unavailable native observation
cannot justify releasing the real admission, quota or ledger entry.
The registry's early-stop request is separate from synchronous revoke
drain; unrelated session admission remains usable. The guardian finally
publishes a terminal status *only after a positive native Job-empty query*.
The outer Job cleans up only after these observations; outer cleanup does
not count as inner proof. The previous negative and receipt-less schedules
continue to pass with their original expected pending/red results.

**Important exclusions:** the guardian remains in the service host, and
a crash of the guardian / entire host is not recovered. The delegated
Job handle is duplicated with SAME_ACCESS for this fixed trusted
fixture, **not** a production minimum-rights design. The mailbox is
a synthetic test handshake, not a task-lease or authenticated runtime
protocol. Failures of the native duplication call, interruption inside
`CreateProcessW`, permanent kernel observation errors, held
stdout/stderr descendants, stdin/EOF, bounded shutdown, Windows 10/11
acceptance, task-lease/executable/repository provenance and restart
reconciliation remain unresolved. This does NOT implement a production
owner and does NOT authorize migration of real GitRead.

Integration decision: **research only, NO-GO**; no broad experimental
merge; #317/#323 remain open. Any minimal production extraction must
be based on current `main`, with independently verified exact-commit
Windows and Ubuntu CI and review of #176/#182.


## Limited launcher Job access — 24 October 2026 (separate research)

The original historical checkpoints and outcome matrices above remain unchanged.
Branch: `research/317-limited-launcher-job-rights-2026-10-24`.
**Exact successful code SHA:** `17ec42ccdb5e1ddc9268c4cea63c9665278ae765`.
[Windows/Ubuntu CI #40055810146](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055810146):
**SUCCESS on attempt 1 of that SHA**, strict Rustfmt/Clippy both operating systems,
Windows runtime library **355 passed / 0 failed / 13 ignored**, Ubuntu
runtime library **270 passed / 0 failed**, all eight externally supervised
synthetic crash schedules, and `cargo test --locked --workspace` on Windows.
Supervisor manifest printed `complete:true` on Windows Server 2025 build
26100. [Evidence artifact](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055810146/artifacts/12545173622).

**Change and native permission proof:** the service/guardian retains the
original full-rights, already limited, kill-on-close native Job. The exact
fixed synthetic launcher receives a *distinct duplicated Job handle*
with **only** `JOB_OBJECT_ASSIGN_PROCESS (0x0001)` and
`JOB_OBJECT_QUERY (0x0004)`; `DUPLICATE_SAME_ACCESS` is not used
for this service-to-launcher offer. On the launcher side, a real
`TerminateJobObject` attempt **before any child is created** is
required to fail with Windows `ERROR_ACCESS_DENIED` (HRESULT
`0x80070005`). If it unexpectedly succeeds or produces another
error, the test fails. The limited handle still supports a real
`QueryInformationJobObject` and creation of the fixed suspended
libtest child with `PROC_THREAD_ATTRIBUTE_JOB_LIST`. The independent
outer supervisor verifies the reported access-denial proof and the
survivor count **before** its emergency cleanup.

| Hosted schedule | Launcher lacks TerminateJobObject authority? | Outer survivors after launcher death | Guardian's final native proof and outcome |
| --- | --- | ---: | --- |
| `preowned-before-create` | Yes, Win32 denied before create | 1 (host only) | Original Job positively empty; `Failed` |
| `preowned-after-create` | Yes, Win32 denied before create; child creation succeeded | 2 (host + synthetic child) | Guardian terminates original Job; positively empty; `Stopped` |

Both cases report `launcher_terminate_denied=true`,
`native_job_receipt=true`, `root_handle_receipt=false`,
`original_admission_retained=true`, `revoke_pending_before_proof=true`,
`terminal_published=true` and `quota_released=true` **only after**
the positive inner-Job proof. The six older scenarios retain their
distinct expected outcomes (including the intentional negative control).
This is controlled kernel evidence, not a production permission policy.

**First failures remain visible, not reclassified as green:**
- [#40055602734](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055602734) stopped at a strict Rustfmt import-order difference.
- [#40055647652](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055647652) reached Windows compilation, which revealed that this pinned `windows` crate does not export those two access-mask names and expects a typed `DUPLICATE_HANDLE_OPTIONS`; corrected using the documented numeric masks and explicit options type.
- [#40055754894](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40055754894) failed strict Rustfmt line wrapping before tests.
No assertion, branch rule, scope, policy, or CI gate was relaxed.

**Remaining NO-GO boundaries:** service-to-launcher mailbox remains
a fixed trusted synthetic fixture; no authenticated real task-lease,
root identity/provenance check, least-privilege process handle, or
AppContainer GitRead integration has been proved. No test demonstrates
a full service-host crash, permanent native observation failure,
independent guardian restart/reconciliation, descendant-held
stdout/stderr, stdin EOF or bounded service shutdown. The child is never
resumed; the native Job is **not a sandbox**. The proof is from hosted
Windows Server 2025 rather than operator Windows 10/11 acceptance.
`main` remains untouched, PR #327 independent, issues #317/#323 open;
Windows V1 remains **NO-GO**.


## Rejected native destination delegation — research-only checkpoint

Branch: `research/317-failed-job-delegation-2026-10-24`.
**Exact tested code SHA:** `9122615dd40856cf57b6f1d61ebbf66e1ece632b`.
[Hosted CI #40057184478](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057184478)
**SUCCESS on attempt 1 of this SHA**: Windows runtime library
**355 passed, 0 failed, 13 ignored**, Ubuntu **270 passed, 0 failed**,
Rustfmt and strict Clippy on both platforms, **nine** externally
supervised Windows-native scenarios, and the full Windows
`cargo test --locked --workspace` suite. The fixture ran on a
hosted Windows Server runner, not an authorized operator Windows 10/11
machine. [Manifest and synthetic host logs](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057184478/artifacts/12546266069)
were uploaded.

**New negative scenario, `preowned-offer-failed`:** with the original,
limited, kill-on-close Job held by the guardian and a genuine counted
session admission/JobStore reservation, the fixed synthetic launcher is
started under separate outer supervision. The guardian calls the same
private Win32 `DuplicateHandle` path used for a successful limited
`JOB_OBJECT_ASSIGN_PROCESS | JOB_OBJECT_QUERY` delegation, but supplies
an intentionally **invalid target-process handle**. Windows rejects the
operation with `ERROR_INVALID_PARAMETER` (HRESULT `0x80070057`).
No remote Job handle or ACK is published and no child is created. The
separate launcher is then terminated by its exact owned process handle.

The outer supervisor observed **two** processes (service host + launcher)
before termination and **one** (service host alone) after launcher
termination, before emergency outer cleanup. The guardian still holds
its original Job; the manifest confirms `native_duplication_failed=true`,
`native_job_receipt=true`, `original_admission_retained=true`,
`quota_retained_before_proof=1`,
`revoke_pending_before_proof=true`, `status_before_proof=Running`,
`other_session_usable=true`, and `root_handle_receipt=false`.
Under an injected native accounting fault the admission and reservation
remain retained and no terminal state is published; only a **real positive
zero-members native Job observation** allows the guardian to publish
`Failed`, release the active quota, and complete the waiting revoke.
The outer supervisor's cleanup is **not credited** as the native proof.

**First failures, preserved exactly:**
- [#40056936313](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40056936313) stopped at strict Rustfmt; the source formatting was corrected without changing assertions.
- [#40056998380](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40056998380) passed Rustfmt/Clippy and Rust tests but failed the ninth native schedule. The first-host log in [artifact #12545488548](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40056998380/artifacts/12545488548) showed that the invalid destination is rejected with **`0x80070057`**, rather than the initially expected **`0x80070006`**. The assertion was corrected to require precisely the *observed* Windows error; accepting any error or retrying the failed CI was avoided. This initial panic is a test expectation fault, not evidence of a real production resource leak.

**Strict limitations:** this is a deterministic invalid-target failure
with an existing service-owned native Job, **not** a proof covering
inaccessible source handles, partial native creation, all
`DuplicateHandle` failure modes, a service-host crash, persistent real
native accounting failures, durable restart/reconciliation, descendant
stdio ownership, stdin/EOF, bounded shutdown, or real GitRead/task-lease
provenance. A green research CI does not authorize promotion of the
experimental branch. Production `main` and security policies are
unchanged; #317/#323 remain open and Windows V1 remains **NO-GO**.


## Failed native creation with a service-preowned Job — 24 October 2026

Branch `research/317-failed-native-create-2026-10-24`.
**Validated code SHA:** `b5a91deea1b306c56a2c03de7bb5a86b699d1d6f`.
[Hosted Windows/Ubuntu CI #40057931887](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887)
**SUCCESS on the first attempt at that exact SHA**: Rustfmt and strict Clippy
on both platforms; Windows runtime library **355 passed, 0 failed, 13
ignored**; Ubuntu runtime library **270 passed, 0 failed**; full
Windows `cargo test --locked --workspace` passed. All **ten** synthetic
externally supervised Windows schedules completed with the expected
outcomes. The emitted manifest has `complete:true`, records
`Windows-2025Server-10.0.26100-SP0`, and is preserved in
[the uploaded artifact](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057931887/artifacts/12546595512).

**New schedule `preowned-create-failed`:** the guardian creates and
retains the real, resource-limited native Job, original generation-bound
counted admission, and `JobStore` quota/identity before delegation. The
independent fixed research launcher receives only
`JOB_OBJECT_ASSIGN_PROCESS | JOB_OBJECT_QUERY` and proves that
`TerminateJobObject` is denied on that restricted handle. It invokes
the **real** `CreateProcessW` path for the exact current libtest binary,
but with a fixture-owned working directory guaranteed absent. The
Win32 syscall fails: the fixture accepts only a Windows
`ERROR_DIRECTORY` or `ERROR_PATH_NOT_FOUND` HRESULT (the exact one
returned was **not** recorded in the manifest), and verifies the
guardian-owned Job has **zero** active processes. It does not accept a
generic injected error as a native failure. An unexpected successful
child creation would trigger exact-Job cleanup and fail the test.

The supervisor observed **2** processes before launcher termination
(host + launcher), **1** afterwards (host only), *before* emergency
outer-Job cleanup. The mode delivered **no** child handle or ACK and
published `native_creation_failed:true`, `native_duplication_failed:false`,
`launcher_terminate_denied:true`, and `job_preowned:true`. The
genuine guardian admission was retained, the reservation remained
`Running`, capacity remained **1** and normal `revoke` remained
blocked under an injected native-accounting observation failure.
The guardian subsequently obtained a **real positive empty-Job query**
before publishing terminal `Failed`, releasing the quota and admission,
and joining the responsible owner. The outer supervisor did not provide
or receive credit for the inner proof. An unrelated session remained
usable.

**First-red evidence preserved:**
[CI #40057892808](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40057892808)
failed strict Rustfmt for one multiline statement **before functional
execution**. Commit `b5a91dee` applied the exact formatter change
without changing assertions or disabling any CI/security gate.
The success applies to that *new SHA's first attempt*, not a rerun
of a failed SHA.

**Limitations / production decision:** this tests a deterministically
missing working directory on an authorized fixed executable, not every
`CreateProcessW` partial-success/error condition or all possible
Windows error codes. It is a research-only fixture with a separate
synthetic launcher and a living service host; it does not prove
service-host crash/restart reconciliation, permanently failed native
observation, descendant-held stdout/stderr pipes, stdin/EOF,
AppContainer/provenance/task leases or physical Windows 10/11
acceptance. Research `#![forbid(unsafe_code)]` remains intact in the
runtime; Win32 `unsafe` is confined to the Windows crate. No
production GitRead migration, no broad research merge, no change to
`main`, and no V1 GO. PR #327 remains independent; #317/#323 remain
open.
