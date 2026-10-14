# H-02 one-slot create-only successor election

Date: 2026-10-14. Research fixture only; no journal format or auth commit chosen.

The replacement-gap tests show that a later writer can overwrite a modification
inserted after the old-state precheck. This candidate tests a narrower alternative:
two locally selected fixed public operations reserve the same absent successor
slot using exclusive creation, with no replacement or truncation path.

There is one existing fixed public baseline, two compiled-in public record values
and one fixed successor filename. No arbitrary payload, slot number, state parser,
counter selection, implicit initialization, repair, delete or remote endpoint is
provided. Each candidate makes one attempt. The single slot is a lifetime capacity
of one: every subsequent claim fails, including an empty or partially written slot.

Expected baseline must equal the fixed public bytes before any I/O. Baseline
reads and record readback are capped at64 bytes. Basic file kind/link/size checks
reject malformed fixtures; they are not a native root-handle containment proof.
The path-based Python exclusive open is the election point. A successful claim
requires exact write count, flush, readback, file metadata, close and a final
baseline check. Failure can leave the slot present without success; it is never
automatically adopted, completed or deleted. OS I/O latency is not bounded here.

## Evidence

Ten regressions cover input rejection before I/O, single-slot exhaustion, a
competitor inserted after baseline check, real two-thread and two-process races,
flush failure, partial write, existing empty/file/directory/hardlink preservation
and missing/corrupt baseline refusal. Local Windows full suite:102 cases,
99 passed,3 deliberate platform skips. Process tests use exactly two owned
children, bounded readiness/output reads and waits, and kill only those children
on failure. Thread barriers and future waits have finite test timeouts, not a
general filesystem-operation deadline guarantee.

Both competing operations reach the create point after checking the same
baseline. Exactly one wins; the loser cannot overwrite the winner's operation.
The deterministic late-competitor case preserves the revoke-b public marker
when a rotate-a claim arrives later. These are operation markers, not actual
credential rotation/revocation or a persisted authorization acknowledgement.

## Limits and next gate

The [public successor/seal reader](H02_PUBLIC_SUCCESSOR_READER.md) now classifies
reserved versus exact validated public pairs. It explicitly measures that a
validated pair can exist without writer ACK and grants no authentication or
durability authority. It is not a chosen journal/commit protocol.

No existing ProtectedStore or initial publisher behavior changes. This does not
fix #214's replacing fixture or provide atomic CAS on an arbitrary mutable state.
Other same-user programs can still modify/delete/replay files. Path/ancestor
races, native root-relative creation, protected record encoding, state selection,
multiple generations, capacity recovery, partial-record handling, live admission,
antirollback and power-loss durability remain separate proofs.

Slot existence is a reservation, never permission to authenticate or select a
new state. A valid full record can be visible before its writer announces success;
no reader protocol is implemented. Do not promote this election into a journal
or production commit without defining and proving that protocol. Exhaustion must
not cause an implicit reset or garbage collection of old authority.

Prior #214 exact952d423 is green: CI746/native fixture24. Current candidate CI
is recorded separately when available.
