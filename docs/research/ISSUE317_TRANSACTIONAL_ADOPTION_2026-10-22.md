# #317 — Transactional native-owner adoption (22 October 2026)

Status: research branch only; production HardenedCommandRunner unchanged.

## Demonstrated defect

At research source 55b82be, GitOwnerRegistry::adopt accepted T by value.
WrongOwner, Adopted, or Unavailable returned before insertion, implicitly dropping
T. A drop-counter regression failed (1 instead of 0) on Windows 10. This proves
ownership loss on rejection, not an observed escaped native process.

Adoption now borrows &mut Option<T>. Every fallible identity/state/lock check
precedes take(); rejection leaves the caller's payload intact. Success transfers
it exactly once into the registered owner. An empty payload fails without
altering a reservation. The service worker remains responsible for a rejected
payload; this API is never a cleanup or cancellation substitute.

## Native proof and original runner baseline

The independently Job-supervised ignored runtime test uses a real
LimitedJobObject child and real stdout/stderr. Wrong-domain adoption retains
that live child, then legitimate adoption consumes the payload. Dropping a
reply receiver does not remove the record. A genuine SessionRegistry::revoke runs on a scoped thread and signals stop while the real admission is held. New admission is refused; the native owner is stopped and observed before that permit is dropped and revoke drains. Native stop with missing observation
retains capacity. Release requires LimitedJobChild::try_wait (root exit AND Job
empty) plus bounded stdout and stderr EOF reads. A separate outer Job confirms
recovery on success and failure. Child lifetime is independently capped at 8s.
No stdin is supplied by this test; it is not an all-stdio owner proof.

The example issue317_baseline invokes the unchanged public runner with 200ms.
Seven supervised modes cover success, nonzero, output ceiling, retained stdout,
retained stderr, blocked stdin and timeout with retained descendant. On local
Windows 10, four retaining modes remained blocked after 2s; the outer Job saw
runner plus descendant and recovered to zero. Normal/nonzero/ceiling controls
returned. No timeouts or production limits changed. Hosted evidence must be
checked on the exact PR source before acceptance.

Commands (from repository root):

```text
cargo +1.99.0 test --locked -p optic-bridge-runtime --lib git_owner_registry
cargo +1.99.0 test --locked -p optic-bridge-runtime --lib --no-run
cargo +1.99.0 build --locked -p optic-bridge-runtime --example issue317_baseline
python scripts/research/issue317_registry_native.py --test-exe <runtime-test-exe> --output evidence/registry-native.json
python scripts/research/issue317_baseline_supervisor.py --example-exe target/debug/examples/issue317_baseline.exe --output evidence/baseline-native.json
```

Use the compiler-artifact JSON to identify exactly one test executable. The
workflow issue317-registry-native.yml automates this and preserves partial JSON.
The original baseline runner is deliberately expected to exceed its deadline;
a successful reproduction workflow does NOT make production #317 green.

## First failures retained

- Initial local build: auto-selected VS18 lacks msvcrt.lib (LNK1104).
  Selecting the existing complete MSVC14.44 installation in process-local
  environment fixed compilation; no installation or persistent setting.
- Original adoption regression failed before the fix as described above.
- First baseline harness tried deleting its log before Job termination on the
  blocked case (WinError32). The Job still confirmed empty in finally. Cleanup
  now happens after tree recovery and closing the supervisor process handle.

## Remaining acceptance gates

ADR-0035 remains PROPOSED. The native fixture starts/resumes before registry
adoption, uses null stdin and synthetic quotas, and retains a lexical real
session admission in the test. It does not deliver pre-spawn native registration,
per-session authorized quotas, service shutdown/recovery, real revoke/seal/reap
wiring or any GitRead migration. Generic ledger Drop is not proven safe for
active/uncertain native owners. No production integration is justified by this
slice alone. #317 and #323 remain open; #319 and #185 are distinct.
