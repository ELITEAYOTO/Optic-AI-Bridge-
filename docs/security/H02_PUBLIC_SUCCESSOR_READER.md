# H-02 bounded public successor/seal reader

Date:2026-10-14. Public fixture only; no auth state or journal format chosen.

The one-slot election has one winner but does not define when a reader may use
the result. This candidate adds a bounded observer and a separate fixed seal
file. The seal is a deterministic SHA-256 digest of the fixed public baseline
and operation marker. It is not keyed, secret, a signature or an authenticity
boundary. Copy/replay by another same-user program is not prevented.

## Observer results

| Files observed | Result |
| --- | --- |
| No slot and no seal, valid baseline | empty, operation absent |
| Bounded regular single-link slot, no seal | reserved, operation absent |
| Exact known record and corresponding exact seal | validated, fixed operation |
| Orphan, partial, unknown or mismatched seal | fixed failure, no repair/fallback |
| Oversized, linked/reparse, changed-size file or invalid baseline | fixed failure |

All results explicitly carry fixture_only=true and authority=none. Even a full
unsealed record remains reserved; no operation is inferred. Slot/seal/baseline
reads have fixed ceilings of64/96/64 bytes (one additional byte is read on slot
and seal to detect growth). There are two compiled-in record/seal pairs, one
slot and no directory enumeration, migration, reset, deletion or reconstruction.
These multi-file path reads are not an atomic snapshot or native containment.
Concurrent publication may cause conservative failure/reserved observation.

## Writer and uncertainty

The sealed candidate requires an empty observation, then completes its own
exclusive claim's write/flush/readback/close before attempting exclusive seal
creation. A pre-existing orphan seal refuses before slot creation. It never
adopts an unsealed claim or overwrites an existing seal. Expected input is checked
before observer I/O; every candidate remains one-attempt/lifetime-slot bounded.

A valid seal may be visible before the writer finishes flush/close/reply. The
observer's validated result is therefore exact public-pair validation, not a
durability or acknowledged-commit verdict. Injected seal-flush failure leaves
a validated pair while the writer returns failure. Actual os._exit before seal
leaves reserved; exit after seal but before reply leaves validated with no ACK.
Neither result grants authentication or permits automatic retry/reinitialization.

## Validation and next gate

Nine regressions cover empty/partial/full unsealed states, each fixed sealed pair,
orphan/partial/mismatch/unknown seals, seal flush failure, existing seal refusal,
oversize/missing baseline, actual abrupt exits before seal/reply, hardlinked or
changed records and invalid expected input before I/O. Local Windows suite:
111 cases,108 passed,3 deliberate platform skips. CI evidence is recorded
separately. Child processes are owned, waited with ten-second bounds, outputs
read with64/4096-byte ceilings and killed only on test cleanup if still alive.

Before using any state-selection or auth protocol, select the logical commit
point, reader/admission serialization and uncertainty/recovery contract. Exact
bytes without ACK do not establish a durable transition. Native root/record pins,
protected metadata, multiple generations/capacity, rollback, caller cancellation
and power loss remain separate gates. Existing replacing ProtectedStore remains
unchanged and its negative race evidence remains valid. No remote endpoint or
production authority is added.

Prior #215 exactc7e8792 has general CI747 / fixture25 green.
