# H-02 native commit gate after fixed scratch creation

Status: PROPOSED TEST SEQUENCE / NOT IMPLEMENTED
Date: 2026-10-14
Owner: #172; complements H02_LOCAL_CREDENTIAL_STORAGE_CONTRACT.md

## Evidence available

#209 pins one existing public writer.lock and observes root/file identity.
#210 creates and writes one fixed public scratch file but still uses a path.
#211 explicitly creates the fixed leaf relative to a held root handle, without
absolute-create fallback. These are disposable public-marker fixtures. None
publishes committed auth state, stores a real credential or closes production
storage. DPAPI integrity remains distinct from state freshness/rollback.

A native composition regression now creates the private dummy child, opens its
fixed writer marker, creates/writes the root-relative scratch, and observes the
root/lock/scratch descriptors while the handles remain live. Root owner and
protected DACL, three parsed entries and no other write-allow category are
asserted. This is measured descriptor composition, not effective-access proof
against another user or an uncooperative same-user writer. No state is published.

## Next implementation order

1. Prove initial publication to an absent fixed destination separately from
   rotation/revocation replacement. It must refuse every existing target rather
   than toggling replace-on-exist after a sharing or collision failure.
   Initialization requires a fresh locally created fixture root in that same
   operation. A missing state in a previously enrolled/existing root is not
   initialization authority and must not trigger credential resurrection/reset.
2. Specify the selected writer/threat model and ACL/owner policy for both lock
   and state. A cooperating lock alone does not exclude a writer ignoring it.
3. Select and test the actual replacement primitive against expected committed
   state. A pre-write read/hash followed by ordinary replace is not atomic CAS.
   Do not claim stale-state exclusion without its native interleaving proof.
4. Compose exact public-dummy protected encoding/schema with the proven primitive,
   then repeat failure, abrupt-exit and reopened-registry auth cases. The fixed
   marker from #210/#211 must never be accepted as registry metadata.
5. Only after review, design real verifier/enrollment and the local operator
   recovery flow. No permissive migration/reset or resurrection of old auth.

## Acceptance table for generated fixtures only

| Scenario | Required outcome |
| --- | --- |
| Absent initial state | Fixed expected destination appears, validates fully before acknowledgement |
| Existing initial target: file/dir/link | Failure, existing bytes/identity preserved; no delete/repair/fallback |
| Missing state in an existing enrolled root | Failure, no implicit initialization or recovery by absence |
| Partial write or failed flush | No success acknowledgement; scratch inert, auth blocked if composing a transition |
| Native publish failure | No success acknowledgement; committed state preserved or explicit uncertainty |
| Old/current state changed during admission | Refusal or proven serialization, never silently overwritten |
| Caller cancellation after admission | Retain accounting to known completion/uncertainty, not assumed rollback |
| Abrupt exit before publication | Reopen uses committed state; scratch never auto-promoted |
| Abrupt exit after acknowledged transition | Reopen retains new epoch/revoked denial; unrelated dummy owner unaffected |
| Native close or completion uncertain | No false success or implicit unlock/retry/reset |
| Existing handles/sharing conflict | Fixed failure; never enable bypass flags merely to obtain green tests |
| Restored old valid protected state | Record measured rollback behavior; decryption is not freshness evidence |

Current root-relative scratch handles intentionally lack DELETE access. Rename
needs a separately selected internal fixture authority and validation; do not
quietly widen the existing primitive. Rename sharing rules and replacement
semantics must be proved with actual Windows handles. Root-relative naming
alone does not authorize a destination or verify its expected version.

Primary reference reviewed:
[FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)
describes relative root/name, rename DELETE access and refusal of an existing
destination when replacement is disabled. Driver documentation is a research
lead, not a selected user-mode implementation or general conformance proof.

## Exit boundary

No generalized file API, caller-selected path/flags, external credentials,
remote endpoint or production Store refactor is part of this next fixture gate.
Record exact SHA, actual/simulated/host evidence and optional skips separately.
