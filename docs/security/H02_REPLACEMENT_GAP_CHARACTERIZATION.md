# H-02 replacement gap — measured negative evidence

Date: 2026-10-14. Disposable public Windows fixtures, not production storage.

The initial publication proof is green on exact head
`a64d21bb60c63fe37db8943ac67932e5082cd4f9`: general CI745 / native fixture23.
It uses no replacement. The following two tests characterize why replacement
cannot be approved merely by adding a replace flag or an extra pre-read.

## Native file pin and unlocked destination gap

While the current state is held by PreparedFixtureRead, another write and a
normal replacing NT rename both fail. Once the read context closes, a same-user
writer ignoring writer.lock successfully alters state while the cooperating
writer pin is still held. A replacing NT rename then overwrites those bytes.

Replacement is enabled only in this negative test's explicit packet. The
initial publication wrapper remains no-replace, unchanged. No POSIX sharing
bypass, arbitrary external path or production replacement API is introduced.

## Protected fixture revocation lost after precheck

The existing ProtectedStore compares current decoded owners with expected state,
then closes the read pin before protection and path-based replacement. A test
hook deterministically inserts an actual valid protected B revocation after
that comparison. Actual protection/write/replace for an A rotation then completes
and reports persisted. Reopen accepts B again: the inserted revocation was lost.

The hook schedules the interleaving; it is not a simultaneous external process.
The state mutation, DPAPI bytes and replacement are actual Windows operations.
No real token is used or printed. Prior tests that reject drift already present
before the comparison remain valid, but do not exclude this later gap.

Local suite: 92 cases, 89 passed, 3 deliberate platform skips. These are passing
negative characterizations, not a successful concurrency-security verdict.

## Consequence and next gate

A [single-slot create-only election](H02_CREATE_ONLY_SUCCESSOR_ELECTION.md) now
tests whether a late claimant can avoid overwriting a competitor. It is only
public operation-marker reservation; no state-selection/auth commit protocol or
production journal is chosen. The replacing fixture's measured gap remains.

Keep ProtectedStore's path-based replacement and the new NT replace trial out
of production enrollment. A read/hash/check followed by ordinary replacement
does not provide atomic compare-and-swap. Rechecking just before replacement
would move the gap, not prove its exclusion.

Before a replacement implementation, select and review one truthful model:

- A single cooperating writer with explicit local-account trust, and actual
  isolation of untrusted repository processes from the storage boundary; prove
  that all application transitions serialize and cannot lose acknowledgement.
- A different commit representation/primitive with an independently proved
  stale-state exclusion contract. An append-only finite fixture is a research
  option, not a chosen production format; capacity, partial creation, crash,
  state selection and rollback still need explicit fail-closed behavior.

Do not describe ACL counters, DPAPI or writer.lock as excluding all same-account
programs. Recovery/old-valid-state replay, hostile ancestors and power loss stay
separate. No production guarantee or architectural decision is made by this note.

Reference: [FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)
describes replace and sharing semantics; our result is the specific native
interleaving above, not general filesystem conformance.
