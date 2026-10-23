# #317 — authority-owned gate and native owner continuation

Experimental, no production wiring. Base 92574db93aa41de4582d972665275ad7f12f7826;
branch codex/317-authority-native-owner. Main and PR327 are unchanged.

## First increment: stale close versus record replacement

The previous commit checked gate identity before launch, but not before close.
Two deterministic regressions now demonstrate a real defect in that TEST-ONLY
candidate: a direct revoke or prompt close snapshots the old gate, waits on it,
the already-closed/admission-free record is removed and re-registered, and a
launch holds the new record's distinct gate. The stale closer used to publish
revoked through the OLD gate despite the in-progress new guarded effect.

The exact handshake uses Arc ownership counts: old record + test-held gate +
blocked closer snapshot. All channels are finite and threads scoped/joined.
Initial native-Windows unit result: two expected regression failures, each
observed Ok(true) instead of rejecting the retired generation. An initial
compiler borrowing error is separately preserved; it is not the defect proof.

Both close paths now revalidate Arc::ptr_eq under the registry lock before
publication. A stale close returns UnknownSession for the captured, retired
generation and does NOT claim that a replacement session was closed. Normal
provisioning generates a fresh handle; deliberate reuse through public register
is an adversarial test. Production API semantics are unchanged: additional
synchronization and checks remain compiled only in research test builds.

This is not a complete ABA model or a production fix. A future public authority
must define immutable session generations and receipt scope explicitly rather
than silently treating an obsolete close as success on a replacement record.
Poison recovery remains allowed for closure only; new commitments still fail.
