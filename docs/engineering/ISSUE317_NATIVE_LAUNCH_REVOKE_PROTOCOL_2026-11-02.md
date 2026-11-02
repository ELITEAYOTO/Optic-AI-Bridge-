# Proposed Windows GitRead native launch and revoke protocol — 2 November 2026

**Status: RESEARCH PROPOSAL, NOT ACCEPTED OR IMPLEMENTED.** This is an engineering contract for the still-unbuilt service-owned launcher, not a V1 approval, execution API, or exception to a security gate. [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) is OPEN / Windows V1 **NO-GO**.

## Baseline and independently reproduced counterexample

At exact `main=f4a2bf5fc6a4d42eacefffd77e24c242e8c44c26`, [independent push/main CI #41486524318](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41486524318) was SUCCESS 4/4, first attempt. That CI **does not** run the ignored research contract in [draft #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355).

The unchanged research test was transplanted **alone** into a disposable Windows 10 local clone of that SHA. `cargo +1.99.0 test --locked --offline -p optic-bridge-mcp --lib issue317_revocation_between_authorization_and_git_spawn_must_prevent_execution -- --ignored --nocapture --test-threads=1` executed **1 test, failed, exit 101** (twice): `ISSUE317_POST_REVOKE_GIT_SPAWN` — actual Git status succeeded after the original session revoke bit was published and before its existing admission drain completed. Log SHA-256: `F1D2454550740D39A190C5F796CA069062E960ED6B8EE5B48F046663D0C201D4`. See the [dated evidence receipt in #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323#issuecomment-6583638706). This is an observed **live legacy runner** defect, not an OS-owner test.

The existing `GitReadService::status_with_max_bytes` still calls `HardenedCommandRunner`. The Windows original Job, four stop signals, joint session+TaskLease observation (#482), and `NativeCallStopTracker` (#479–#481) remain **non-executing** precursors. A green ledger test, even one that seals all tracked calls, cannot turn a snapshot into launch permission.

## Required semantics: do not conflate three events

1. **Stop published:** the original session/TaskLease/host/Job generation has irreversibly marked stop. This must be prompt even when a Win32 call is blocked. It closes new native-call *admission*; it is **not** proof that a previously admitted syscall has returned or that its effect is absent.
2. **Native call ledger sealed and drained:** the **same original owner** refuses future Create/Resume admissions, and all previously counted `CreateProcessW` and `ResumeThread` calls have actually returned. A timed-out or poisoned ledger is **Unknown**, not empty. This proves no unaccounted future call **from that issuer**, but no process-tree termination.
3. **Physical completion / revoke complete:** the original process root has really exited; its original kernel Job has no active children; both *physical* stdout and stderr EOFs were independently observed; stdin, writer threads and retained handles are accounted for; and no issuer may create/resume new work for the revoked original action. Only this phase may release the action's owned resources/capacity. An unknown OS result or a crash without a surviving owner is not completion.

**Linearization caution:** publishing stop and immediately asserting “no Windows process was created after this timestamp” is stronger than can be shown while an earlier admitted `CreateProcessW` may still be executing. Serializing the actual syscall with the revoke lock would prevent prompt stop if that syscall stalls, violating the liveness requirement. The proposed contract therefore distinguishes **no new admission after stop** from **in-flight effects that must be reconciled while the action remains nonterminal**. A process created suspended by an in-flight call must never be *resumed* after its action is stopped; it must remain owned by the original Job and be terminated or left explicitly uncertain. “Revoke complete” must not return until all native calls and physical effects have been accounted for. If the product requires the even stronger no-kernel-create-after-stop-publication guarantee, the architecture must separately provide an OS-level serialization/guard with bounded stop publication or explicitly state that it cannot be met. **No choice is approved by this document.**

## Proposed ownership and authority graph

The eventual internal issuer is **service-owned and non-cloneable**. It must retain immutable originals, not re-query an attacker-controllable opaque ID: session admission/registry/generation/clock, TaskLease witness/registry/generation/clock, exact action and policy epoch, original host-stop source, absolute nonrenewable deadline, canonical repository, pinned Git executable identity, quota/transport permits, original Job, stdin, stdout and stderr pipes, internal stop signal, and its in-flight ledger. The caller must not create a second issuer, substitute stop flags, or supply a new deadline. No launch capability is exposed through MCP inputs.

A **trusted policy check outside locks** validates immutable target/scope and epoch. It does not substitute for a later synchronized launch commitment. The combined session+TaskLease observation under **session lock → TaskLease try-lock** (#482) also does not commit an OS call. A real issuer needs a shared close/admission serialization to which **every original direct and lifecycle revoker** belongs; independent `Arc<AtomicBool>` checks alone do not make that final cross-registry protocol linearizable. Explicitly resolve policy change and host-stop publication in the same design before any live enablement.

The issuer must register and own an OS-call ticket **before kernel entry** and retire it only **after the kernel call returns**, including error/partial-success cases. A Create result is always assigned to the original precreated Job **before any Resume**; prefer at-creation Job-list binding and explicit inherited handle list so no unowned child can escape the custody. Never hold session/lease/policy/JobStore locks across filesystem hashing, `CreateProcessW`, `ResumeThread`, pipe I/O, waits, or callbacks. Lock order and all cross-registry revoke paths require a separately reviewed proof of absence of inversion and unbounded waits.

## Proposed native action states (NOT current implementation)

| State | Allowed effects | Stop/concurrent effect rule |
| --- | --- | --- |
| Prepared | Validate/pin target; reserve original transport/Job/permits; no child | Reject and release **no-child** custody |
| Committing | Under common original-generation close boundary, register exactly one issuer/ticket | A losing revoke forbids new admissions; a winning ticket remains owned, **not** an authorization after the fact |
| CreatingSuspended | One counted in-flight Win32 Create, original Job-list and bounded handle inheritance | Publish stop promptly; hold owner/ticket; classify uncertain until syscall returns |
| SuspendedOwned | Root and primary thread handles pinned to original Job; no code executed yet | Recheck **original** stop/expiry at a synchronized Resume boundary; kill without resuming when stopped |
| Resuming/Running | Count actual Resume syscall and later root/descendant activity | A stop seals future calls, triggers original Job watchdog and awaits real effects; never abandon handles on caller cancellation |
| Stopping/Reconciling | No new action calls; native calls may still be in flight | Wait only within captured absolute bounds; poison/timeout/error preserves **Uncertain** custody |
| ProvenTerminal | Owner-specific sealed ledger + root exit + Job empty + both physical EOFs + stdin/handle disposition | Release capacity only after every required independent fact is observed |
| Uncertain | Ambiguous Win32 ownership, failed kill, blocked pipe or crashed guardian | **NO SUCCESS receipt** and no quota/resource recycling as if terminal; require externally provable recovery |

The “Committing” and “SuspendedOwned → Resuming” rows are **design targets**, not implemented states. A token minted at an earlier check is not sufficient to perform an OS effect later. All admission and cancellation paths must share a documented happens-before boundary. Consider power loss/host crash separately: an in-process watchdog is not itself proof of durable supervision; prove kernel Job kill-on-owner-loss and recoverable custody or enforce an explicit unsupported-host exclusion.

## Acceptance matrix before production migration

The following tests must execute rather than be skipped, preferably with deterministic phase barriers and a bounded independent supervisor (no arbitrary sleep/retry-to-green):

| Controlled interleaving / failure | Mandatory observable result |
| --- | --- |
| Session direct/lifecycle close before native Create admission | No new Create admitted; no executable entered |
| TaskLease direct/session revoke, physical removal and same opaque ID reuse | Old original generation denied; new ID never revives it |
| Policy epoch changes / wrong target / foreign host owner | Fail closed before effect; no stale policy/host commitment |
| Session or lease expiry between joint observation and native admission | Denial using original clock/deadline, no renewal of budget |
| Revoke while actual Create is blocked inside Windows | Stop published without holding a revocation mutex during syscall; no false drain; post-return child owned/stopped, never resumed |
| Revoke before Resume and while actual Resume is in flight | Never accept a new Resume after stop; account for already admitted OS effects and terminate uncertain child |
| Counter overflow, mutex poison, lost owner or native timeout | Permanent seal, explicit Unknown/uncertain custody and original Job stop; no fabricated zero or success |
| Root exits while descendants retain one/both pipe writers | No terminal until original Job empty and **both** physical EOFs |
| Caller aborts, host shutdown, crash before/after native acknowledgement | Original permits/handles remain protected until bounded independently evidenced cleanup or explicit Unknown |
| Real Git through **MCP** after authorized pre-spawn barrier and concurrent revoke | Unweakened #355 scenario must PASS by **not executing Git**, not by removing/disabling the test or faking a result |

Preserve an independently supervised real Git for Windows case, not only `cmd.exe` or an empty Job. Require exact integrated Windows and Ubuntu CI, real source/checkout SHA and physical schema8 evidence where applicable; compare PR-head and squash Git trees; inspect independent first-attempt `push/main` afterward.

## Implementation order and explicit exclusions

1. Review and agree on the exact two-phase stop/complete semantics and original-generation issuer lock graph; identify all direct/lifecycle revokers and the permitted lock order. Make its concurrency tests **RED first** on real behavior when feasible.
2. Implement a narrow service-owned authority and tracked Create/Resume integration in an isolated LAB, without changing MCP routing. Retain all negative tests; on uncertainty stop and do not merge.
3. Once proven under real Windows calls, wire only Git **status** through the native owner, and make #355's real MCP acceptance **GREEN** without reducing coverage. Require complete root/Job/stdio proof and external abandonment/crash supervision.
4. Then independently address sensitive Git metadata/content (#176/#182), first-response behavior (#319), AppContainer/Node (#185), local Windows 10/11 installations and owner-reviewed release gating.

No production GitRead native activation, no credential or policy changes, no runner changes, no release/tag, and no implied Windows V1 GO in this documentation PR. This document records **required questions and tests**, not their answers.
