# H-02 closed-stage public seal publication

Date:2026-10-14. Optional public fixture variant, no auth/journal decision.

The direct seal writer can expose a complete seal before its own flush/close.
This variant prepares the same fixed65-byte public seal under one fixed pending
name, validates write count/readback/metadata, flushes and closes it, then
publishes using one explicit os.link operation. No copy/rename/replace fallback
is attempted. Existing pending names are never adopted, truncated or repaired.
Failed stage entry closes its owned descriptor; failed close prevents publication.

The final seal temporarily has two names/links. The observer rejects that state.
Only after the known staged name is removed does the final seal have one link
and become acceptable to the public-pair observer. If link publication collides,
the existing target is preserved. If unlink fails or the process exits between
link and unlink, the reader refuses both-link state rather than auto-repairing it.

This fixture uses paths and requires hard-link support. Identity observations are
not an atomic unlink authorization or hostile-ancestor/same-user writer exclusion.
Only owned disposable generated roots are tested; no production file deletion
or native root-bound publishing authority is introduced. Directory flush,
power-loss durability, antirollback and credential enrollment remain unproved.

## Evidence

An [explicit Windows handle-relative variant](H02_NATIVE_PUBLIC_SEAL.md) now
retains the NT source through relative link and source disposition/close, without
Path.unlink. It is a separate selection; this portable path-based variant is
unchanged and is never used as a native fallback.

Eight new regressions cover reserved observation before publication/after close,
failed stage metadata entry, failed close, failed flush, existing stage refusal,
link collision, failed staged unlink and actual abrupt child exits at three
publication boundaries. Local full Windows suite:119 cases,116 passed,
3 deliberate platform skips. The child harness has ten-second waits and64/4096
byte output ceilings; only its owned children are terminated on failure.

| Actual child exit point | Reopened observer |
| --- | --- |
| Closed stage, before link | reserved, no visible seal |
| Link created, before staged unlink | fixed refusal, two links retained |
| Final one-link seal, before reply | validated public pair, no ACK emitted |

Publication therefore follows prepared close in this variant, but a lost reply
can still leave a validated pair. This is expected uncertainty, not proof that
no operation happened. Observer output stays authority=none and does not grant
authentication or establish a durable transaction. The original direct-seal
candidate remains available as the explicit negative visibility characterization.

Reference: [Python os.link](https://docs.python.org/3.11/library/os.html#os.link)
documents the hard-link interface and platform support. Collision and link-count
behavior above are actual fixture observations, not all-filesystem conformance.
