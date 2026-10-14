# H-02 volatile expected public root identity

Date: 2026-10-14. Public fixture only; not production trusted enrollment.

An immutable two-integer snapshot records volume serial and64-bit file index
from an active validated RelativePublicPair's owned root reference. Capture
refuses unopened/closed pairs. No supplied OS handle is accepted. The expected
snapshot has exact type/range checks before directory/native I/O.

ExpectedRootPublicPair compares metadata on its actual opened root against that
snapshot before opening any child. The same owned root then anchors the existing
fixed relative leaf reads. A mismatch refuses and releases owned references;
no fallback, repair, replacement or adoption occurs.

Three additional regressions cover malformed snapshots before I/O, unchanged
root reopen and capture refusal after close, and actual same-name substitution
refused before any leaf open. The original and alternate valid public fixtures
remain intact. Local Windows157 cases /154 passed /3 platform skips.

## Conditional boundary

This is an expected-state comparison only. A snapshot captured from an already
substituted path would describe the wrong root; caller-fabricated snapshots are
not authentic. No trusted enrollment source, protected snapshot persistence or
recovery is chosen. Volume serial/file index are observed metadata, not global
cryptographic identities, immutable generation counters or antirollback proof;
recycling and remount/recovery require separate contracts.

The test's anchor is deliberately acquired before substitution from an owned
generated fixture. It does not establish production root trust or close the
initial capture gap. Existing public root-selection negative evidence remains
valid for the unanchored reader. Root selection still opens by path; expected
identity checks the opened reference rather than adding another path precheck.
No production authentication, Store/runtime/Effect changes or remote tools.
