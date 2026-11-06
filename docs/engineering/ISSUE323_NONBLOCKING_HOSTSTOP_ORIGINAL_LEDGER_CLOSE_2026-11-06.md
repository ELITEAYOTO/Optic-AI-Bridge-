# #323 — Original HostStop publishes without waiting on native subscriber registration

**Windows 10 isolated LAB · 6 November 2026 · non-executing native GitRead stage only. Windows V1 stays NO-GO.**

## Exact provenance and scope

- Baseline is [draft PR #515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/515), original source HEAD `e3436299d2f263fdcf265d003694e8cd298f6498`, with `main=203309227612ccf2be2438b4a808c3c82c78d56a`. This tranche must be reviewed as a **stacked** change, not auto-merged into either branch.
- #515 previously ensured that the original host publishes its public stop bit **only after synchronously closing the same already-watchdog-backed native ledger bits**. Its `HostStopSource::request_stop()` took the native subscriber `Mutex` using a **blocking** `lock()`.
- Real original TaskLease/session/Job custody from #514/#515 is unchanged. This tranche changes only the Windows HostStop subscription storage, its test coverage, Cargo manifests and this dated note. It adds **no** Win32 Create/Resume issuer or live GitRead MCP route.

## Genuine RED on the untouched baseline

A new nonignored `host_stop::tests::original_host_stop_publishes_while_registration_mutex_is_held` bound the exact original ledger, then deliberately held the original subscription mutex while invoking the real `HostStopSource::request_stop()` from an independent thread. It observed stop and the internal ledger bit after a **bounded 200 ms window**, released the lock and joined the worker before asserting. On the untouched source, **1 test FAILED, exit 101**, marker `ISSUE323_HOSTSTOP_BLOCKING_REGISTRATION_LOCK`, without a Git child or leaked thread.

Private raw RED stdout SHA256 `F08B0196EA096FFEDE394CB334C86566F3CA2155185FC7D7688BF26FA3F28B5E`, stderr SHA256 `04CE6C683B1489A6D3D19D31F8B96DB1ADE2B183F43B1A60B203067F0FA012EB`.

## Focused nonblocking publication prototype

- Introduces a **pinned Windows-only** `arc-swap = 1.9.1` dependency; the lockfile was minimally changed (addition only). No `unsafe` code is added in `optic-bridge-runtime` (which forbids unsafe code).
- The original HostStop source retains a new private `close_started: AtomicBool`, original public `stopped` signal, a **registration-only** `Mutex<()>`, and immutable **atomically published** `Arc<Vec<Weak<AtomicBool>>>` snapshots. A weak subscriber still cannot keep a removed native owner alive or retarget its original generation.
- A registrar `try_lock()`s the writer mutex; contention or poison seals its own original ledger and rejects registration. It checks `close_started` and public stop before writing, atomically publishes a pruned subscriber snapshot, then performs a final **SeqCst** check. If stop won during registration, the registrar seals that exact ledger and refuses to return it as armed authority.
- `request_stop()` and source `Drop` first set `close_started`, then load a stable snapshot **without taking the registration lock**, seal every reachable original ledger with SeqCst stores, and **only then publish** the public host-stop bit. No filesystem, Win32, Job/pipe operation, callback or registration mutex wait occurs in that path.
- Proof boundary: registration's atomic snapshot publication precedes its final SeqCst stop check; stop's private SeqCst close-start publication precedes its snapshot acquisition. Therefore a concurrently committed ledger is either present in the stop snapshot or observes the close-start fence at its final post-publish check and closes itself. A registration still in progress cannot confer native call authority until it returns. Pre-admitted native calls remain counted, not cancelled or falsely reported terminal.
- **Liveness limitation:** No waiting on the subscriber writer lock; not a hard wall-clock upper bound for arbitrary OS scheduling, CPU stalls, large subscriber snapshots or unexpected allocator contention. Stop-time work scales with subscriber count, which is pruned on registration.

## Windows verification receipts

- After the change, the original RED test passed using **`--locked --offline`**, 1 actual test / 1 pass / 0 ignored. Raw GREEN stdout SHA256 `440D21AFE657174C8143DDB0E17E9DECFB1D7A03E83F99FE4857FA2B1DF16F90`, stderr SHA256 `430B43B2F2F4608327B3C95144456A2071AAAF84113004DECCBCFA60A15AD4BA`.
- The held-lock test was then strengthened with a worker-start handshake and a 500 ms bounded publication check; the positive test is not a zero-test or ignored result.
- **60/60** independent, externally 15-second-bounded Windows executions passed: 12 each of held-writer publication, original source stop/Drop, 64-race stop vs registration, poisoned/contended registration, and the real **original unstarted Windows kernel Job + watchdog + counted session/TaskLease/host** integrated test. Each receipt required `running 1 test`, `1 passed`, `0 failed`, `0 ignored` and exit 0. Private manifest `HOSTSTOP_ATOMIC_60_MANIFEST.json` SHA256 `23EB80559FD5341D80382C1EB02D310017CAE64955D9110AFFB40BD3474419DF`.
- The **full six-gate first-attempt local Windows campaign** on the final source tree finished **6/6 PASS** on pinned Rust 1.99 `--locked --offline`: fmt, Windows crate, runtime crate, complete workspace, workspace all features, and Clippy strict `-D warnings`. Independent raw stdout/stderr and their SHA-256 values are private in `_evidence/HOSTSTOP_FIRST_*`; `HOSTSTOP_FIRST_GATES_MANIFEST.json` SHA256 `CFE1536AB910E4D83345613774BB5ED177EF3A64E67930000B4DEF9B7482D761`. No failure was retried or replaced.
- Independent hosted PR CI, dependency-policy checks and code review **must still be recorded separately**. They do not retroactively change the original RED or supervised 60-run outcomes.

## Explicit nonclaims / stop gates

**Still no service-owned native Win32 Create/Resume issuer.** Original policy/target/deadline close-and-commit, already admitted real Win32 effects after stop publication, root HANDLE signaled, original Job empty, both independent physical stdout/stderr EOFs, inherited handles and quota reconciliation, and crash/poison/timeout UNKNOWN custody remain unsolved. The separate intermittent Node/AppContainer #185 main failure also remains open. Keep [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) OPEN, **Windows V1 NO-GO**, [PR #515](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/515) DRAFT, and [PR #511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/511) OPEN/unmerged. No installer/release/bridge-daily/Desktop/REL changes.
