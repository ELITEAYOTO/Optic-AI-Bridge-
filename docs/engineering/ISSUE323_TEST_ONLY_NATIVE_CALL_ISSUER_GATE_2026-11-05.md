# #323 — original native-call issuer gate prototype (Windows, 5 Nov 2026)

**RESEARCH-ONLY: a narrow gated Windows GREEN, not the real service-owned native issuer. Windows V1 remains NO-GO.**

## Provenance and scope

- Baseline exact research parent `bf92528c48e19196df67bb313a1e41f594c1cfab`, tree `27c028762544e96d5ba1b880733062a4202cc0a2`. Parent's full in-LAB lifecycle/lease/host revocation fixture is still intentionally RED and normally ignored.
- At research start: `main=89ea451d89cb125d6517949c83660465315ff475`; [PR #511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/511) still open/unmerged, its original Windows PR CI #41885824035 failed. Green on a bootstrap branch does **not** authorize merging #511.
- Entire experiment runs in a disposable, **separate authorized Windows 10 LAB**, never the daily installed Bridge. No live GitRead MCP, policy, installer, profiles, tags, release, production activation or `main` changed.
- **Production-facing refactor:** `LimitedJobObject::wrap_child_with_before_resume` still passes exactly the same real `resume_process_threads` callback to a new private helper `wrap_child_with_before_resume_and_resume`. That helper makes an alternate native resume callback injectable for tests; the existing production wrapper path is unchanged in behavior. The intentionally RED legacy test exercises this production path, not the prototype.
- **Prototype is gated out of production builds:** `#[cfg(all(windows, test))] mod native_issuer_prototype`. It has no exported public service API or runtime/MCP integration. Its simple `NativeIssuerPrototype` owner is non-`Clone`, though an `Arc` may share **the same** original owner for concurrency tests.

## Prototype state and linearization

The private test-only issuer packs two native call counters and one irreversible CLOSED bit in a single `AtomicU64`. It uses a compare/exchange loop to register the original `CreateSuspended` or `Resume` callback before executing that callback. `close()` atomically publishes the same CLOSED bit, **without a mutex or waiting for Win32**.

- **Close wins before admission CAS:** the new callback is refused with `PermissionDenied` and is never invoked. This invariant is linearizable between the issuer's CAS and close RMW.
- **Admission CAS wins before close:** the callback remains counted until it really returns, even when the external revoker publishes close in the meantime. The callback **may physically execute after close**; it is not retroactively cancelled and requires separate OS Job/PROCESS/pipe supervision. No claim of absolute "no ResumeThread after stop" can be made for this ordering.
- `seal_and_drain_until` seals admission and polls an absolute deadline without taking a lock on an OS callback. An empty gate means **no recorded callbacks**, not that any process exited, the Job is empty, both pipes reached EOF, stdio was released, or all quotas are safe to return. Failure to drain in time remains `TimedOut`, not a success receipt.
- A counter overflow irreversibly closes the owner. The proof does **not** integrate current runtime `NativeCallStopTracker` or prove durable recovery from crashed/detached OS calls. Do not create two detached accounting authorities in the future service.

## Three tests, each repeated 12 times

### Actual real Win32 GREEN (nonignored)

`job::tests::research_prototype_issuer_seal_after_real_lifecycle_and_host_stop_denies_kernel_resume`

- Creates actual System32 `cmd.exe` `CREATE_SUSPENDED`, noninteractive, with zero-length physical stdout marker; retains independent duplicated original PROCESS HANDLE. Assigns it to the actual constrained Job Object, verifies Job active=1 and original root unsignaled.
- Holds a genuine original counted `OwnedSessionAdmissionPermit` across the native resume decision. A different thread calls actual `SessionLifecycleManager::revoke`, publishing original session **and registered TaskLease stop** while revoke completion remains blocked on the counted admission. The original real `HostStopSource::request_stop` is published before the boundary too.
- **Controlled adapter:** the test explicitly calls `issuer.close()` *after* it observes session/lease/host stop. This is NOT an atomic integration to those revokers and NOT a production issuer. It is the specific variable changed from the prior RED path.
- The private test injection then executes `issuer.issue(Resume, || resume_process_threads(process))`. Because close won before admission, the Win32 callback is **never invoked**. The wrapper's existing resume-error cleanup terminates the real assigned Job/root and verifies the original physical process signal. Original stdout remains **zero bytes**; the genuine lifecycle revoker only completes after the admission is dropped. This is **not** a full physical stdio EOF/terminal receipt.
- Distinct markers `ISSUE323_PROTO_ORIGINAL_STOPS_PUBLISHED` and `ISSUE323_PROTO_WIN32_RESUME_DENIED`. **12/12 GREEN (exit 0)**.

### Existing real Win32 RED unchanged (explicitly ignored)

`job::tests::research_lifecycle_and_host_stop_with_counted_owner_pre_resume_must_deny_code`

With exactly the same real lifecycle/lease/host stop published pre-Resume, the **legacy ungated wrapper** still executes the original suspended child and physically writes `REAL_CHILD_ENTERED`. This test intentionally panics rather than claiming success. **12/12 behaviorally RED (exit 101)**, with `ISSUE323_LIFECYCLE_HOST_POSTSTOP_EXECUTED`. Preserving this result guards against falsely claiming that the prototype modified the live path.

### Atomic owner/concurrent drain positive controls (nonignored)

`native_issuer_prototype::tests` contains two tests: close refuses 512 fresh `CreateSuspended`/Resume callbacks while never reopening; and a genuinely concurrent already-admitted callback stays counted, immediately observes close, makes bounded drain time out while still in flight, then drains **only after** the callback is released. **12/12 group PASS**, each group run executes both unit tests.

All 36 supervised executions used external independent 15-second per-process bounds, distinct stdout/stderr files and SHA-256 for each. **No timeouts**. Full private manifest `_evidence/ISSUER_FIRST_36_BEHAVIORAL_RUNS_MANIFEST.json`, SHA-256 **`08BA83D301B85513679918EB12D5F1A3A0DB826853F4D9F1E8A7C826AC19499F`**. Preserve raw logs privately.

## Explicit blockers before any production issuer or V1 release

This prototype DOES NOT bind its own `close()` into all *direct and lifecycle* session revokers, TaskLease direct/session/remove+opaque-ID reuse, original policy epoch, executable identity, host peer Drop/shutdown, original deadline or fatal watchdog. In particular, observing independent stop flags then closing this gate still admits a cross-registry race **between publication and close** if any other native thread is attempting admission; the test forces a favorable deterministic order and labels it as such.

The next architecture must deliver a single service-owned, nonforgeable, noncloneable **original-generation native-call issuer** which makes every real Create/Resume admission and every relevant stop publication participate in one close protocol. A revoker must publish stop promptly while an already admitted Win32 syscall may be blocked; never hold registry, policy, JobStore or watchdog locks across Win32, hash, wait or IO. Retain every in-flight CreateProcess/Resume ticket and the original Job/PROCESS/stdin/stdout/stderr custody until verified completion; original root signaled + original Job empty + both physical EOFs + all handle/quotas reconciled required. Any timeout, poison, failed stop or crash without proof remains UNKNOWN, never a positive terminal receipt.

Only then develop *nonignored real Git-through-MCP* acceptance against original-generation revoke/host stop, target/policy changes, in-flight crashes and post-stop process trees (including #355 context). No rerun-to-green of PR #511, tag, merge, release, or Windows V1 GO.

## First-attempt local Windows regression gates

After the test-only Rust module was brought into a compiling state, all six **first attempts of the full local gate campaign** returned exit 0: pinned Rust 1.99 formatting; Windows crate unit tests (**87 passed / 8 intentionally ignored / 0 failed**); runtime crate tests; full workspace tests; workspace all-features; and strict Clippy all targets/features `-D warnings`. Locked/offline constraints were retained for the applicable cargo commands. Private first-attempt raw stdout/stderr and per-file SHA-256 remain in `_evidence`. Manifest SHA-256 **`6398B40898C9972B540E9AADB8B106800A9A3D8D8E8EFF91897A079565ACC6EA`**. The initial prototype compile had a corrected Rust `Result` type-alias mismatch *before* the full gate campaign; do not confuse compilation iteration with a successful first build. Passing ordinary CI still skips explicitly ignored legacy RED security acceptance.
