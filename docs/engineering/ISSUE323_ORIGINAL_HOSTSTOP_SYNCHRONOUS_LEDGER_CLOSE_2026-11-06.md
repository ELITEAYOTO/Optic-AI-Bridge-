# #323 — Close the original native ledger before HostStop publication (Windows)

**6 November 2026 · Windows 10 LAB · preparatory non-executing stage only.**
**Windows V1 remains NO-GO.** This work does not add a native GitRead MCP launch path.

## Grounded baseline and exact defect

- Baseline: `main=203309227612ccf2be2438b4a808c3c82c78d56a` after PR #514.
- PR #514 closed the **same original** `NativeCallStopTracker` ledger bit under the SessionRegistry and TaskLeaseRegistry revoker mutexes, but HostStop was still an independent atomic observation.
- The original host could publish `HostStop=true` without first closing the exact watchdog-backed native ledger's internal admission bit. The ledger's next observation eventually saw the host bit, but the two close publications were not jointly serialized.
- This defect is narrower than the still-open **late already-admitted Win32 Create/Resume** counterexample and does not cure that counterexample.

## Test-first RED — actual original host, ledger, and kernel Job watchdog

The existing, nonignored Windows runtime test `git_read::native_status_tests::native_call_tracking_reuses_original_session_lease_job_host_stops_after_watchdog_arm` was strengthened to require **the exact original ledger close bit** to be set when actual `HostStopSource::request_stop` returns. The fixture retains original session and TaskLease, counted transport, reserved process capacity, **actual unstarted Windows kernel Job**, watchdog, and borrowed native call ticket.

On the uncorrected baseline, the test compiled and ran: `running 1 test`, `FAILED (0 passed; 1 failed)`, **exit 101**. Panic marker: `RED #323: real original HostStop publication must seal the same native ledger bit...`. Separate raw stdout/stderr evidence is under the private LAB `_evidence/RED_HOSTSTOP_ORIGINAL_LEDGER_*.txt`. This is an actual host stop with a kernel watchdog; **no Git child is launched**.

## Focused correction

- Windows-only, `HostStopSource` and the original `HostStopWatch` retain a shared `Arc<Mutex<Vec<Weak<AtomicBool>>>>` of the exact original native-ledger internal close bits. A weak reference cannot prolong a stopped native owner or redirect an original generation.
- `request_stop` and owner `Drop` close all reachable subscribed ledger bits with release stores **under that same short mutex and before the observable HostStop bit is published**. The mutex encloses only weak-pointer work and atomics, **never any Win32 call, filesystem IO, Job operation, pipe read, or callback**.
- Registration uses `try_lock`, checks the original stop and ledger bits under the mutex, and **fails closed** (seals its own ledger) if stopped, contended, or poisoned. A newer host has a different set of original subscribers.
- `LeaseObservedNativeGitStatusCustody::arm_original_watchdog_inner` registers the exact already-watchdog-backed ledger with HostStop **after** the existing session/TaskLease joint binding and before returning from arm. A registration failure seals this same original ledger and rejects arm; it creates no substitute owner.
- Original host stop does **not** erase already admitted Create/Resume tickets; they remain counted until dropped.

## GREEN and repeatability

The same test now passes without being ignored or weakened. Two new nonignored Windows host tests separately cover explicit source stop, owner Drop, rejected late binding, source-generation separation, and 64 coordinated stop/registration races per invocation.

A private, externally 15-second-bounded Python verifier ran **36/36 tests PASS**: 12 exact invocations of the integrated original Job/watchdog test, 12 explicit/Drop host tests, and 12 concurrency tests. Each invocation enforced `running 1 test`, `1 passed`, `0 failed`, `0 ignored`, exit 0; raw stdout/stderr and SHA-256 are under the LAB `_evidence/`. The manifest `HOSTSTOP_36_RUNS_MANIFEST.json` SHA-256 is `6A9E6BA551A8DCF9E082D5CBD91B6C5F93001F5FE6DE0315405F4BBB22129F5C`.

The **first local six-gate campaign was 5/6 PASS**: fmt, Windows crate, runtime crate, workspace and all-features passed, while Clippy strict failed on an unnecessary `return` in the new host code. This failure is preserved in `FIRST_ATTEMPT_GATES_MANIFEST.json`, SHA-256 `F7A6CE7615EC80FB723AC508C27936B1C009C170B48E003BC269A903F3AE992A`. The only subsequent Rust change removed that redundant `return`; it did not weaken an assertion, timeout or security check.

The **final corrected source tree passes 6/6 independent offline Windows local gates** including `cargo clippy -- -D warnings`, documented under separate `FINAL_GATE_*` raw logs and `FINAL_TREE_GATES_MANIFEST.json` SHA-256 `334A51EC997798FA91A50228E1486D3A5861CAEFAFFF5F71BC0565066A1C5353`. This does not rewrite the first local attempt. Host-side CI, any independent code review, and post-merge CI must be separately assessed; even green hosted CI does not confer production acceptance.

The original RED stdout/stderr SHA-256 values are `7C7DDF0330B684EFCCA4727704FE2D5F552ADBA2598675F1445B8853FFE3EC2D` / `34A98AF82E378A23978D0B82F59625DDB083DD81E04BBEC4F6435D8828C41389`; targeted GREEN stdout/stderr are `A1F7D8A9A29D15AEC6A5152C90A89DB8017A1A474B1AA85099BF076E0DC25100` / `7584DB3866F3CE4E79223FD80B0CC5680170BACBCF25F09FAA350E5E836D209B`.

**Existing baseline problem:** independent post-merge CI for the preceding PR #514 on `main=2033092...`, first attempt [#42118032514](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/42118032514), failed on the known Node/AppContainer startup case #185. Do not rerun-to-green, conceal it or interpret this host-ledger patch as a Node startup fix.

## Additional adversarial review of PR #515 (6 November, separate evidence)

After the original PR head `58ea7f7a04848e7a165a7744526be1fa299b3379`, a further code review expanded **tests only**; the actual production host-close mechanism was not changed.

1. The existing **real original kernel Job/watchdog + original counted owner** integration test now covers `host-drop` as well as direct `host` stop. The *original* `HostStopSource` is physically dropped while the earlier `HostStopWatch`, original ledger, already admitted native ticket, Job, session and TaskLease are still retained. A replacement foreign source cannot revive that original. The same original ledger bit is closed and no new tickets are accepted, while already admitted tickets remain counted through retirement.
2. A new nonignored Windows unit test proves that a **contended** original host registration mutex denies registration and seals the would-be ledger; an intentionally **poisoned** mutex also denies a new ledger, and the real source still publishes stop after recovering the poisoned guard. The poison panic is test-only and deliberately caught, not an uncaught test failure.

The initial ad-hoc short-name command with `--exact` selected **zero** tests; it was **not** counted as proof. A corrected fully qualified invocation executed **1 test / 1 PASS / 0 ignored**.

A *new separate* private 15-second-per-invocation supervisor executed **48/48 nonignored Windows tests PASS**, 12 per each of four exact cases (original Job plus host Drop, explicit host stop, 64-collision host bind race, contended/poisoned bind). Every successful record requires actual `running 1 test`, `1 passed`, `0 failed`, `0 ignored` and exit 0. New manifest SHA-256: `D86AE0240AAE68384FF7A1B44BC4ADF147956579280E5E638E4CEAFCA62032F1`. The previous 36-run manifest remains unchanged.

For the **new post-review source tree**, a separate second full Windows offline validation (fmt, Windows crate, runtime crate, workspace, workspace all-features, strict Clippy) finished **6/6 PASS**. Its independent `REVIEW_TREE_GATES_MANIFEST.json` SHA-256 is `77E631B3ED5CDEDA31C1C47EEC286A7703C16B9EAEF758F40626EE3697B19388`. Earlier first-attempt 5/6 and corrected 6/6 receipts were not overwritten.

This is an *additional self-audit and test reinforcement*, **not an external/human code review**. The host stop critical-section liveness under a permanently stalled process/thread, all late already-admitted real Win32 effects and the complete terminal OS/stdio proof remain unproven; maintain draft status pending review. Keep #323 OPEN and Windows V1 NO-GO.

## Remaining P0 gates / explicit nonclaims

- HostStop joining the non-executing ledger's close bit is **not** authority to enter `CreateProcessW` or `ResumeThread`. A **ticket admitted before revoke can still physically enter Win32 after stop**; it must be retained, reconciled, and physically terminated.
- Policy epoch, original target/executable identity, real deadline expiry, and fatal watchdog source still need complete common close semantics. Absolute deadline is observed, not a synchronous publication lock.
- No accepted live GitRead-native path, no full root original signaled + original Job empty + **independent physical stdout and stderr EOF** + stdin/inherited handle lifecycle, no crash/poison/abandonment proof, and no actual Git MCP no-post-revoke acceptance.
- **PR #511 remains open and blocked** by its first general Windows CI failure (#185). This host-stop research does not override it. Keep issue #323 open, Windows V1 NO-GO, no public release or native GitRead enablement.

## Reproduction

In a **separate** authorized Windows 10 LAB on Rust 1.99 with offline lockfile:

```powershell
cargo +1.99.0 test --locked --offline -p optic-bridge-runtime --lib native_call_tracking_reuses_original_session_lease_job_host_stops_after_watchdog_arm -- --nocapture --test-threads=1
cargo +1.99.0 test --locked --offline -p optic-bridge-runtime --lib host_stop::tests::concurrent_host_stop_and_registration_never_leave_open_original_ledger -- --exact --test-threads=1
```

Source and evidence remain isolated from the everyday installed Bridge. Do not publish private LAB logs or machine paths in an external issue; provide only sanitized receipts and hashes.
