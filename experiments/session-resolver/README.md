# H-03 reusable synthetic resolver — local contract candidate

Prior #188 proved single-use handoff, #189 persisted dummy credential lifecycle,
#191 controlled429/recovery. None is a reusable Optic session resolver. This
experiment tests that missing contract entirely locally, no network endpoint,
production Rust, real workspace, capability, lease, effect, policy or remote
close/cancel control. No architectural invariant changed; existing AI_HANDOFF
rules remain. Full H-03 exit criteria are not met by this Python proof.

Trusted adapter derives Identity from dummy Registry, selects a fixed adapter
label, calls open, then resolves application_handle on later requests. Raw
caller metadata never becomes Identity or adapter authority; no HostContext is
accepted here. Fixed allowlist francestudent/synthetic-host is a domain boundary,
not proof that either host supplies a conversation id. UI wording remains
current Optic session, not this conversation. internal_id is synthetic-only
immutable metadata, never a production SessionHandle or effect authorization.

Every explicit open creates a NEW zero-authority synthetic session. No implicit
open from missing/invalid handle; no attach-by-conversation or bootstrap-key map.
Concurrent opens are capacity-limited but deliberately not deduplicated. Exactly-
once bootstrap/idempotent retry with authority is an unresolved H-03 contract;
do not interpret these independent empty sessions as exactly-once creation.

Reusable opaque handle has ops_ plus random process generation128bit and fresh
suffix256bit. Only its SHA256 digest is retained, bound to server-derived
owner/principal/epoch and fixed adapter. Same valid handle resolves the same
immutable session repeatedly without consumption or sliding TTL. Absolute
monotonic TTL120 seconds; clock regression/nonfinite values refuse. Owner/adapter
mismatch denies without deletion. Local in-process close removes entry; replay
does not reopen. Complete open/resolve/close runs under Registry lock shared with
rotate/revoke, so stale epoch rechecks and cleanup serialize with local controls.
Returned metadata is not a lease: this does NOT prove admitted-effect quiescence.

Live capacity32, process mint budget128, issued-digest tombstones128 maximum,
no eviction, lazy bounded cleanup on operations. Expiry/inactive epoch removes
live entries but never replenishes mint budget. Collision denies without token
recycling; process generation prevents reusing old handles even when injected
mint suffix repeats on fresh resolver. No persistent sessions or recovery.
Process generation is random, not a durable incarnation sequence; no power-loss
or snapshot-rollback claim. Public dummy Registry restart semantics remain its
own experiment; a future persistent registry adapter requires separate tests.

Eight native deterministic tests: stable repeat/fixed expiry, other owner/adapter/
malformed denial, local close/replay/collision, rotation/revoke/new handle/B,
capacity/no eviction/mint ceiling, generation-separated restart, parallel opens
bounded/distinct, forged principal/nonfinite clock denial. Tests use fake clock
and disposable in-memory registry. This adds no FranceStudent measurement.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\session-resolver\scripts\Test-SyntheticSessionResolver.ps1
```

No new operator host test is needed for the local contract. Before future remote
integration, specify bounded HostContext normalization, trustworthy adapter
selection and exactly-once bootstrap semantics; then test actual repeated model
handoff, expiry, local close and credential epoch transitions on a harmless
endpoint. Production verifier/storage/integrity/recovery and H-01 conversation/
parallel measurements remain open; #178 is not H-01. Keep Draft, do not merge.
