# H-01 guarded grouped temporal overlap — synthetic only

Separate supplement to #171/#170. Existing experiments/host-probe h01-v2 grouped
instrumentation/correlation/reconnect tools remain intact. This carrier narrows
the next measurement to actual SAME-GROUP pending tool overlap using measured
entry/exit intervals, not two displayed results or #178's shared upstream.
#178 is NOT H-01 and provides no temporal-overlap proof. Canonical result template:
../host-probe/H01_RESULT_TEMPLATE.md plus this follow-up's own fields.

Main reread2026-10-13:156653a406f01383e2fc14cada513963453b04f3, unchanged.
Stacked on #194 head8db9a889f4c247dffaa4a62de9f9aec2a6e8d449 whose CI703 and
all6 experiment workflows green. No production Rust changes, real project/file/
process/Git/mutation/session_cancel APIs, shell/network-effect authority or remote
controls. Exactly3 harmless tools on separate loopback8007, h01-grouped-v1.

## Contract

parallel_create_group mints an opaque application_handle and diagnostic group_id.
Same owner/principal/epoch required on every operation. Handles are correlation,
never capabilities or sessions. HMAC/fresh per-process key, SHA256 digest-only
lookup; no raw handles/credential values in logs. No implicit group creation,
caller-controlled group name, delay or concurrency level. Group expires120s
without renewal. New process has new key/empty groups; no durable group recovery.

parallel_wait(application_handle) performs fixed3s cooperative async sleep only.
Records random invocation_id, trusted request_sequence, entered_ms/finished_ms,
active_on_entry, state and overlap_observed. All group timestamps share Groups
instance monotonic baseline (different from Guard's baseline; do not subtract
timestamps across those domains). First entry may initially mark no overlap, then
be marked overlapped when another same-group call enters before it completes.
GROUP enter snapshot reflects entry; exit reflects final record. These measure
pending coroutine overlap, not simultaneous CPU execution or effect authority.

parallel_report(application_handle) snapshots at most8 records and computes
completed_pair_overlap from strict intersection of COMPLETED same-group intervals:
max(entered_ms) < min(finished_ms). Unrelated groups cannot contribute. Cancelled/
failed/clock-invalid records do not qualify. Call count/zero records alone is not
proof of parallel capability or absence. Other owner/expired/stale epoch/malformed
handle fail with same invalid_group response, never expose another group's state.

State ceilings:16 live groups,64 lifetime issued-digest tombstones,8 records/group,
4 pending/group,8 global pending. No eviction or record cycling. Reaping inactive/
expired groups while waits exist leaves at most8 active-retained group objects,
still counted in the global pending ceiling until finally cleanup. Cooperative
task cancellation/failure/expiry/revoke denies/cleans admission; no MCP cancellation
feature/tool is added. A valid wait can still be timed out by Guard or host.
Our8 local ceiling is NOT FranceStudent's announced approximate8-call limit.

Inherited strict dummy Bearer +2 custom headers, Recheck before SDK, Guard bounds
and fs-probe-v3 stage timings, bounded ProofChannel output. Only schema-controlled
GROUP ids/times/counters/states and TOOL proofs logged, no opaque handle/raw args.
Incomplete/dropped logs are not proof. Request timeout20s is cooperative, not a
preemptive OS/SDK/network wall-clock guarantee. Lifetime25min, loopback host policy
unless explicit synthetic tunnel switch. No local stdin controls on this carrier.

## Automated proof

Eight deterministic tests: same-group overlap, unrelated-group false control,
sequential false control, cancellation cleanup/record ceiling, owner/epoch/expiry/
malformed rejection, capacity/budget/nonfinite time, revoke-during-wait cleanup,
per-group/global pending ceilings. Fake clock and controlled async events avoid
timing-based unit conclusions. No caller content used as authorization metadata.

Real local HTTP/MCP smoke discovers exactly3 tools, uses SDK gather for two
same-group waits -> measured completed overlap, uses sequential waits in new group
-> false, denies B and reuses group from fresh A carrier. All GROUP entry/exit ids
and request sequences match wait payloads; TOOL markers match console. No raw
credential/group tokens in output. Temporary local port, child killed at exit,
bounded log retention; smoke never accepts an external URL/real credential.
These are LOCAL SDK measurements, NOT FranceStudent host proof.

## Windows — review output after each setup step

Extract a fresh test8 folder; keep prior fixtures untouched. Setup helper delegates
frozen isolated probe dependencies; Test-GroupedParallelProbe.ps1 expects8 unit
tests plus actual SDK/HTTP overlap and sequential controls. Start with
Start-GroupedParallelProbe.ps1 -AllowRandomTunnelHost on8007; new tunnel to8007.
No initialization or persistent data. Configure HTTPS/mcp, full-prefix original
dummy A and existing2 secure dummy headers, allow ONLY the3 parallel tools.
Do not expose production Optic or use real credentials.

Prepare prompt BEFORE starting group TTL. One single-response attempt:

```text
Appelle réellement parallel_create_group une seule fois. Si la création échoue, arrête-toi. Avec exactement son application_handle, lance deux appels parallel_wait en parallèle si le host te le permet, sans attendre le résultat du premier avant de lancer le deuxième. Une fois les deux terminés, appelle parallel_report avec le même handle. Affiche les quatre résultats complets et indique honnêtement si tu as lancé les deux attentes simultanément ou séquentiellement. N'invente pas de chevauchement, ne crée aucun autre groupe et ne change pas le handle. Si une attente échoue ou expire, arrête-toi et affiche l'erreur réelle.
```

Verify model Tool markers/instance/sequences; exact same group_id, exactly2
completed records and GROUP enter/exit proofs; completed_pair_overlap plus
interval intersection establishes a finite successful pair. If sequential/false,
record that sample without inferring FranceStudent cannot parallelize. No8-call
limit, host CPU concurrency, cancellation, correlation-id or universal guarantee.
Follow-up report from a fresh user turn with SAME handle before TTL can measure
explicit group continuity but not socket reuse or per-conversation isolation.
Use fresh group for a separately prepared sequential control if useful; do not
conflate trials or keep retrying after timeout. Final Ctrl+C server/tunnel and
disableconnector after evidence review. H-01 correlation/reconnect questions still
require existing host characterization; H-02/H-03 production gates remain open.
Keep Draft; do not merge.
