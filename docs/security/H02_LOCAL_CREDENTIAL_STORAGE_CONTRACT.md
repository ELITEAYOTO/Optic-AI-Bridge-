# H-02 local credential storage — proposed integration contract

Status: design only, 2026-10-13. Owning gate: issue #172. Production HTTP
remains H-06. This contract grants no capability and does not close H-02.

## Evidence and boundaries

The persistent public-dummy registry (#189) proves acknowledged rotation and
revocation across process restart. The Windows public-dummy fixture (#196,
code/test head d954653) proves CurrentUser DPAPI roundtrips and bounded native
cleanup. Its fixture workflow #2 and general CI #707 passed. These are separate
proofs: neither implements protected production registry persistence.

Draft #197 composes the two mechanisms in temporary public-dummy fixtures.
Its exact head fc0e8c3 passed fixture workflow #5 and CI #710 on rerun after
one unexplained initial MCP doctor timeout. The composition proves abrupt
process-exit and pre-replace pending-state behavior, not production integration.
Old valid protected state replay is measured as accepted; freshness remains open.

The next integration must remain disposable and public-dummy-only until its
schema, commit failures and restart behavior are independently tested. Do not
reuse its unsalted public hashes as a production credential verifier.

## Required state and ownership

Application-owned state must carry an explicit schema version, bounded principal
records, active/revoked status, credential generation and authorization epoch.
Principal identity is independent of the credential and stays stable on rotation.
Remote input cannot choose a principal, epoch, filesystem path or reset policy.
The local operator owns enrollment, rotation and revocation. Authentication
confers connection identity only; session mapping and capabilities stay separate.

Only verifier material necessary for checking high-entropy random credentials
may persist. A later production design must specify verifier construction and
credential generation before implementation. Copy/show behavior belongs to a
local UI contract and must not leak through MCP, activity or diagnostics.

## Commit and restart contract

Use one cooperating writer, bounded exact-schema decoding and a fixed owned
storage location. Protected bytes are not enough: reject missing, duplicate,
unknown-version, oversized or semantically invalid state. Never repair or reset
invalid state implicitly. A previous pending file must not override committed
state. Native protection failure must not cause plaintext fallback.

Persist the next state before acknowledging a rotation or revocation. Serialize
authentication/effect admission with that transition. On an uncertain commit,
block further authentication until locally resolved; do not acknowledge success
or continue with old authority. Restart must load the committed state and reject
previously invalidated credentials. Define cancellation and writer-lock cleanup
without claiming that a cancelled caller reverses a committed transition.

Filesystem ownership, ACLs and replacement races need native Windows evidence
for the actual persistence primitive. A successful file replacement/fsync or
orderly restart is not a power-loss durability claim.

## Rollback and recovery remain unresolved

DPAPI protection does not establish freshness. A valid older protected registry
can still be replayed unless an independent freshness mechanism detects it.
Do not advertise rollback prevention based on an epoch stored in that same file.
The next dummy proof must explicitly demonstrate this limit, not silently treat
successful decryption as current authorization.

Profile loss/password recovery, backup restore and migration require an explicit
operator recovery policy. No automatic credential resurrection, permissive
fallback or reset-on-decrypt-error is allowed. Recovery and antirollback are
separate decisions; no production guarantee is selected here.

## Acceptance cases for the next disposable proof

- Native protected registry write/reopen, with no plaintext verifier file.
- Acknowledged rotate/revoke survive a new process; principal stays stable.
- Invalid protected or decoded state blocks startup without overwrite/reset.
- Protection/write/replace failure returns no success acknowledgment and blocks
  later authentication; cooperating writer cleanup remains bounded.
- Raw dummy credential and decoded protected payload never appear in logs/results.
- Restoring a valid earlier fixture is recorded as a rollback limitation.

No remote endpoint, real credential enrollment or production refactor is needed
for these acceptance cases. The isolated fixture may write only its own temporary
test directory. Host tests resume only after a reviewed harmless endpoint exists.
