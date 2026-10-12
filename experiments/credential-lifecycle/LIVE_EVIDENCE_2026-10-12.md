# Dummy credential lifecycle — live FranceStudent evidence

Source: user-supplied PowerShell logs, Luna responses and connector screenshot,
not independent access to the manual server on drive E. User identified model as
Luna. Provided archive head `63e9cec9c9ed883adfea90d2ec147fe5f4c61090`; extracted
files not independently re-hashed. User Setup/Test scripts passed: five lifecycle
tests and real local HTTP/MCP smoke. Server banner `dummy-lifecycle-v1`, port8002;
shared guard proof schema `fs-probe-v2`; instance `db5b300f3bb9` throughout.
The connector display name remained `Optic-Bridge-Last-Test` with the new tunnel
URL; do not claim that a separate connector was created. No raw credentials,
credential fingerprints or request captures are included in this ledger.

## Observed transitions

All supplied HTTP requests1–33 show a single Bearer, both exact dummy custom
headers, matching endpoint and no query. Bearer acceptance changes as below.
The local acknowledgements do not have ingress timestamps; ordering is supplied
by the user's console transcript. Elapsed ms below are HTTP admission times.

| Step | Requests / evidence | Result |
| --- | --- | --- |
| Initial setup | 1–3, POST initialize / initialized / tools/list, elapsed131265–131422ms | 200/202/200, stage=mcp, all matches. |
| Baseline discovery | 4–6, same POST methods, elapsed207797–207922ms | 200/202/200, all matches. |
| A baseline | 7 POST credential_probe at212093ms, marker `d59f5dc2a8dafdfd` | Exact local/model TOOL proof; result principal `f7995841ff16007c`, epoch1, authority none. |
| Subsequent request | 8 DELETE at219547ms | 405 inside MCP after header acceptance. |
| Local rotate | Console `rotate-a` | LOCAL ok=true, active=true, ownerA, epoch2. |
| Old A unchanged | 9 POST336172ms / 10 DELETE336250ms / 11 GET336328ms | Bearer present but not accepted; all401 at dummy_bearer; no DISPATCH/TOOL. |
| Host error | Luna's reported initialization and HTTP/SSE attempts | Unauthorized; no fabricated success supplied. Methods/sequence are observed, not a general fallback algorithm contract. |
| Update dedicated auth to v2 dummy | 12–14 POST initialize / initialized / tools/list at455187–455343ms | 200/202/200, all matches. URL/custom headers/server unchanged by user account. |
| New A discovery | 15–17 POST initialize / initialized / tools/list at519781–519922ms | 200/202/200, all matches. |
| A rotated | 18 POST credential_probe at523922ms, marker `d2cc776d5359783f` | Exact local/model TOOL proof; same principal `f7995841ff16007c`, epoch2, authority none. |
| Subsequent request | 19 DELETE531265ms | 405 inside MCP after header acceptance. |
| Local revoke | Console `revoke-a` | LOCAL ok=true, active=false, ownerA, epoch3. |
| Revoked A | 20 POST632562ms / 21 DELETE632625ms / 22 GET632687ms | All401 at dummy_bearer, custom headers match, no DISPATCH/TOOL; Luna reports Unauthorized. |
| Connector toggle | User disabled/re-enabled same connector without parameter/server changes | Supplied immediate excerpt still ends at22; no new request observed from toggle alone. |
| New request after toggle | 23 POST784437ms / 24 DELETE784500ms / 25 GET784578ms | All401 at dummy_bearer, no DISPATCH/TOOL; Luna again reports Unauthorized. |
| Switch dedicated auth to original B dummy | 26–28 POST initialize / initialized / tools/list at881843–882000ms | 200/202/200; no local B command or server restart reported. |
| B discovery | 29–31 POST initialization/discovery group | All header matches; user-supplied transcript precedes B call. |
| B independent | 32 POST credential_probe at952328ms, marker `cf9f74b88c2136a9` | Exact local/model TOOL proof; principal `a98ac87e14d4cc6b`, epoch1, authority none. |
| Subsequent request | 33 DELETE959593ms | 405 inside MCP after header acceptance. |

Principal/epoch values are user-supplied tool-result fields under the inspected
implementation. Local TOOL rows independently corroborate tool/instance/marker/
sequence, not a second direct emission of principal/epoch. Do not overstate that
distinction. The server recognizes only fixed published dummy credential pairs;
the logs intentionally report accept bits, not the actual entered header bytes.

## Accepted finite proof

- Old A is denied before MCP on the observed requests after local rotation.
- Rotated A is accepted with unchanged diagnostic principal and higher epoch.
- After revoke, A is denied before MCP; user connector reactivation does not
  restore access while the same server remains alive.
- Independent B still succeeds with its own principal and unchanged epoch1.
- Required custom headers remain present on all requests supplied in this run,
  including denied POST/DELETE/GET. No claim for all future requests or earlier
  #186 missing-header traffic.
- The live DELETE405 rows occur after authentication inside MCP. They are not
  missing/wrong credential failures; exact caller intent is not authenticated.

No live TCP-socket identity was instrumented: SAME keep-alive socket denial and
fresh-connection denial have separate automated local proof, not this host run.
The local tests also separately cover stale epoch/paused body races, constant-time
dummy comparisons, one-rotation bound, owner independence and log canary absence.

The user confirmed Ctrl+C for server/tunnel and connector disabled after the final
B check. No manual endpoint is claimed active and no additional user action is
required for this evidence consolidation.

## Remaining production work and next safe boundary

Process restart deliberately resets this public-dummy registry. Durable
revocation, locally generated high-entropy credentials/verifier storage, DPAPI
or another reviewed storage contract, local reset/rotation recovery, standards
OAuth compatibility, host caching/retry guarantees and tunnel-restart observation
remain unimplemented/unmeasured. Production effect-spanning revocation is not
proved by a metadata-only probe. #172 stays open; real project gates stay open.

The next authority-free integration candidate is principal+epoch-bound opaque
handle resolution: same-owner handoff, stale-epoch rejection after rotate/revoke,
cross-owner denial and no resurrection on process reset, with finite state and
fake clocks. This requires a separate experiment contract before implementation;
it does not replace H-03's transactional production authority provisioning.
#186/#187 remain Draft; #178/H-01 grouped parallelism are distinct measurements.
