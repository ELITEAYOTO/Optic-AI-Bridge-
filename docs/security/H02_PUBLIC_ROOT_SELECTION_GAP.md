# H-02 public root selection gap characterization

Date: 2026-10-14. Negative public-fixture evidence, not a security fix.

The relative leaf reader protects references after its root opens but has no
trusted expected root identity. A deterministic entry hook renames the generated
selected directory aside and replaces it with another generated valid fixture
immediately before the native root open. Actual filesystem renames/open/read
occur; this is an injected interleaving, not a competing-process timing proof.

The original fixture contains rotate-a; the replacement contains revoke-b.
RelativePublicPair accepts revoke-b with authority none. Both valid public pairs
and the original directory remain intact, and observed root identities differ.
Passing this test confirms an initial-selection gap; it does not mean selection
is secure. A second native test shows ordinary root rename denied while the root
is held and permitted after close. This narrower sharing property does not fix
selection before open or hostile ancestor trust.

All operations stay inside one test-owned generated temporary parent. No user
folder, network configuration, credential, arbitrary process or production file
is changed. Local Windows154 cases /151 pass /3 deliberate platform skips.

## Next design gate

Before accepting real state, define the trust anchor and enrollment relationship
for the directory, including how expected identity is acquired and validated on
the same owned reference used for child opens. A path string or a hash of known
public content is insufficient. An identity copied from the same untrusted late
path is not an independent anchor. Fresh creation under a trusted owned parent,
reopen/recovery identity, volume changes and threat scope need explicit contracts.
This characterization selects no production enrollment or recovery design.

Prior pins, local admission and root-relative reads remain public-only research.
No durable auth commit, authenticity, antirollback, credential journal or
production Store/runtime/Effect changes. Neither a second path precheck nor
silent fallback is introduced as a purported fix.
