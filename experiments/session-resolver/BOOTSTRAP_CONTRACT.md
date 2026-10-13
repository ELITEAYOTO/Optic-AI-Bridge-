# H-03 bounded idempotent bootstrap — synthetic candidate

Follow-up to #192 local reusable resolver. No production invariant changes;
AI_HANDOFF authority/ownership/bounds rules remain. Separate BootstrapResolver
and port8006 listener, public dummy auth, zero authority. Existing Resolver stays
as prior direct-open contract; no production Rust SessionResolver implementation.

## Contract and limits

Three remote tools ONLY:

* session_bootstrap(): server mints a ticket, returned as application_handle with
  kind bootstrap_ticket. No session is created at this stage.
* session_open(application_handle): accepts THAT ticket. First eligible open
  creates one empty synthetic session. Repeat/concurrent open with SAME ticket,
  same authenticated principal/epoch and server-selected adapter returns SAME
  session reference and diagnostic synthetic_session_id, reusedtrue.
* session_probe(application_handle): accepts SESSION reference from open, resolves
  repeatedly and returns diagnostic id/principal/epoch/remaining TTL, no effects.

No caller-selected bootstrap key, owner, adapter, project, capabilities, leases,
profile, HostContext or internal SessionHandle is accepted. Adapter is fixed by
server to francestudent. An opaque value is correlation input; current Registry
authentication and ownership checks remain necessary on every request and core
operation. Internal id is synthetic metadata, never production authority.

At-most-one mapping per minted ticket per process generation, with recoverable
response after a lost OPEN reply while ticket remains eligible. Not globally
exactly-once effects, not durable transactions. If the BOOTSTRAP reply itself is
lost, another bootstrap mints a distinct unused ticket within the finite budget;
it does not deduplicate that earlier request. No session/effect yet exists for
an unused ticket. Distinct explicit tickets deliberately create distinct empty
sessions; a future authority-bearing bootstrap needs an additional reviewed
local session/permission contract. No implicit open on malformed/missing token.

Public ticket/session grammar uses existing oph_ plus64hex but cryptographic
domains differ: HMAC-SHA256 with a fresh256bit per-process key and fixed ticket/
session prefixes. Ticket derives from fresh256bit random input. Session derives
from ticket DIGEST. Raw ticket/session tokens are not retained; deterministic
session derivation permits same-token re-emission on retry. Process key is
ephemeral in memory, never persisted/logged. This is not production verifier
storage/DPAPI or authenticated state persistence. Restart loses all sessions and
tickets, with new key preventing old-token reissue even with repeated mint seed.

TTL120 seconds starts when ticket is minted, NOT each open/probe/retry. Absolute
monotonic deadline shared by session; no sliding renewal. Regression/nonfinite
clock refuses. Live sessions32, lifetime ticket budget128; terminal tickets stay
as bounded tombstones and cannot be reopened. No eviction/recycling. Capacity
failure leaves eligible ticket unused so retry may succeed when another owner's
session frees space. Missing/expired/closed/other-owner/other-adapter tokens use
same invalid_session code. Local close-a/b closes all that owner's tickets AND
sessions, including unused tickets, preventing same-ticket resurrection.

Complete bootstrap/open/probe/local-close runs under Registry lock shared with
rotate/revoke. Rotate/revoke invalidates old epoch's tickets/sessions. Already
completed synthetic metadata may reach the caller after a control; it is not a
lease or admitted-effect quiescence promise. Persistent dummy auth adapter is
not used here; auth reset on process restart stays explicitly synthetic.

Guard retains exact dummy Bearer/custom headers, body64KiB/header16KiB, active8,
requests512/lifetime25min, timeout20s. Opt-in extra_handle_tools tuple permits a
second fixed tool with exact sole application_handle grammar; defaults unchanged
for all earlier probes. Narrow extra allowlist regression test rejects extra
arguments/unknown name/malformed token. No remote controls, cancellation/project/
file/process/Git/mutation tools. Local stdin fixed6 commands,16 lines/read65.
No rate limiter integrated into this session test, to avoid conflating proofs.

## Evidence

17 local native tests:8 prior reusable tests plus9 bootstrap/guard tests. New
proofs include20 concurrent same-ticket opens with exactlyone new mapping, lost
reply recovery, fixed TTL, owner/adapter/domain separation, local close/epoch
replay denial, capacity retry without eviction, bounded tickets, collision and
restart, malformed clock/input denial and paused open serializing with rotation.
The real loopback HTTP/MCP smoke discovers exactly3 tools, repeats open and probe
on same session after fresh transport, denies B without consuming A reference,
local-closes A then denies old reference AND same-ticket open, and permits an
explicit fresh ticket/new empty session. All output markers match logs; no raw
credentials/tickets/session handles logged. Child killed at smoke exit.
12 inherited Guard tests/actual MCP smoke also pass natively after opt-in change.
CI runs Windows/Linux and inherited regressions. FranceStudent behavior pending.

## Windows guided phase — one output review per setup step

Extract fresh source ZIP in test7, preserve old test5 fixture. Setup script:
Setup-SessionBootstrapProbe.ps1. Test: Test-SyntheticSessionResolver.ps1 expects17
tests and actual MCP smoke. Start: Start-SessionBootstrapProbe.ps1
-AllowRandomTunnelHost, loopback8006. No Initialize step or persistent state.
Tunnel targets8006. Connector full-prefix original dummy A and same2 dummy secure
headers; authorize ONLY session_bootstrap, session_open, session_probe.

Prepare BOTH prompts and local close-a BEFORE timed ticket creation. Once enabled,
one full sequential chain within same response avoids human round trips:

```text
Effectue réellement ces appels dans l'ordre. Appelle session_bootstrap une fois et conserve son application_handle comme ticket. Appelle session_open avec exactement ce ticket, puis session_open une deuxième fois avec le même ticket. Vérifie que les deux résultats ont le même application_handle de session et le même synthetic_session_id. Appelle ensuite session_probe deux fois avec exactement ce handle de session. Affiche les cinq résultats complets. Si un appel échoue, arrête-toi; n'invente aucun résultat, ne crée aucun ticket supplémentaire et ne remplace pas le ticket par le handle de session lors du deuxième open.
```

Only AFTER all5 actually succeed, operator immediately types close-a, verifies
LOCAL oktrue/operationclose ACK, then SAME conversation sends:

```text
Appelle réellement session_probe avec exactement le handle de session du dernier enchaînement réussi. Puis appelle session_open avec exactement le ticket initial de ce même enchaînement, pas le handle de session. Affiche les deux résultats complets. Ne crée aucun nouveau ticket. Si le premier appel renvoie probe.invalid_session, effectue tout de même le deuxième appel prévu; si la connexion échoue, arrête-toi.
```

Expected invalid_session twice. No intermediate assistant round trip after mint;
if initial chain fails, STOP and report, no control based on absent handle. Retain
full server log + model outputs. Compare admission elapsed times: if close trial
falls beyond TTL, report expiry-confounded rather than a clean close proof.
Operator controls are harmless local lifecycle only, never model authorization.
End Ctrl+C server/tunnel/disableconnector after evidence review. Real session
authority, host conversation mapping, retry policy, durability, fairness and
H-01 overlap remain open; #178 is separate. Keep Draft; do not merge.
