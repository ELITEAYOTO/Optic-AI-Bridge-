# H-02 held-root relative public pair read

Date: 2026-10-14. Explicit Windows x64 disposable public read candidate.

RelativePublicPair retains the prior three-file pin/validation/use/close protocol
but opens each fixed leaf through NtCreateFile RootDirectory using the already
owned root handle. Only baseline.public, successor.0001.public and
successor.0001.seal are permitted. FILE_OPEN refuses absence without creation;
FILE_GENERIC_READ/share-read adds no write/delete access. Synchronous nonalert,
non-directory and open-reparse options keep the observed object subject to the
existing metadata rejection. There is no path-based fallback for leaf reads.

Windows x64 layout checks cover pointer/wchar and UnicodeName/ObjectAttributes/
IoStatus sizes. Exact status0, IOstatus0 and FILE_OPENED information1 plus a valid
returned handle are required. A nonnegative returned valid handle is owned and
closed on ambiguous completion; a negative status never adopts output storage.
No handle supplied by a caller is accepted. Root selection remains path-based.

Five regressions cover actual held-root reads after mutating the directory string,
write sharing refusal while held, missing seal never created with prior pins
released, unknown leaf rejection before loading, successful-owned ambiguous
status/IO/information cleanup without fallback, and negative output not adopted.
The completion anomalies are simulated; actual opened handles and cleanup are
used for successful-owned cases. Native real relative reads are separately tested.
Local full Windows152 cases /149 passed /3 deliberate platform skips.

This opens a fixed name relative to the held root, not a proof that initial root
selection excludes hostile ancestors. Sequential observations are not atomic;
mapped writers/privileged access and durable authenticity remain unproved.
The variant is deliberately separate from NativePublicAdmission selection, which
still uses the earlier explicit path-based pinned reader. No silent integration
or automatic migration is added. No credential journal, rollback protection,
production Store/runtime/Effect or remote endpoint changes.

Primary reference: [NtCreateFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntcreatefile)
for RootDirectory, FILE_OPEN and FILE_OPENED completion. Tests establish the
observed Windows fixture behavior; the reference is not a whole threat-model proof.
