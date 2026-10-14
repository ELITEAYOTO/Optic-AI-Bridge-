# H-02 held native public pair

Date: 2026-10-14. Windows disposable public fixture only.

NativePublicPair holds a directory plus exactly three read handles: fixed
baseline.public, successor.0001.public and successor.0001.seal. Existing files
only; read-sharing permits ordinary readers but denies ordinary write/delete
opens while these references remain live. No caller file selector or fallback.

Each handle observes regular non-reparse, single-link metadata and a fixed byte
ceiling64/64/96 plus one growth-detection byte. Final names must match the held
root and fixed leaves. Exact baseline and one compiled record/seal pair are
required. All three handles remain owned through a fixed public digest probe.
Re-entry fails before releasing ownership. Exit closes files then root. Invalid
entry attempts all owned closes and never repairs existing bytes. Close failure
is sticky, with no retry. Output always fixture_only=true / authority=none.

Four added native regressions measure actual separate-process writes refused for
all three files, ordinary write succeeding after owner close, re-entry preserving
live pins and invalid-pair cleanup preserving invalid bytes. Child waits are
ten-second bounded and stdout/stderr discarded; only children created by the
test are involved. Explicit NativePublicAdmission composition also verifies
ordinary write and busy-close refusal during the pinned calculation, then write
availability after close. Injected uncertainty after actual native cleanup
suppresses the result and blocks coordination. This injection is not an actual
OS close failure. Local full Windows147 cases /144 passed /3 platform skips.

## Limits

Root and leaves are opened through the existing path-based native helper. Exact
final-name observations do not prove hostile ancestor containment or authorize
the selected root. Sequential opens are not an atomic snapshot. Ordinary
CreateFile sharing observations are not a claim against pre-existing writable
memory mappings, privileged kernel access or every filesystem/Windows version.
NativePublicAdmission explicitly selects pinned work with no portable work
fallback; its publication and before/after observations remain the earlier
portable path-based variant. The admission lock spans native use and close;
file pins span computation but close before the coordinator's final observation.
This does not extend native ownership across every observation. There is no credential state, action lease,
durability, recovery or production Effect integration. A digest computed while
held grants no permission after close. Existing replacement-gap evidence remains
valid and production Store/runtime are unchanged.
