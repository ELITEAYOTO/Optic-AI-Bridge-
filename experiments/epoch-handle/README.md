# Principal + epoch + opaque handle — zero-authority experiment

Owners #172/#173. Stacked on #187 exact head
`d27500b79a1ca4206bde602b2109cb43a7517e93`; remote main checked unchanged at
`156653a406f01383e2fc14cada513963453b04f3`. No production Rust or authority changes.
Keep Draft; #178 and H-01 grouped instrumentation remain separate.

## Contract before implementation promotion

Exactly two remote tools: `bound_create_handle` and
`bound_consume_handle(application_handle)`. They operate only on in-memory dummy
handoff state: no real project, filesystem, process, Git, mutation, session_cancel,
session-grant or remote control tool. No external I/O is performed by the core.
These are NOT production SessionHandles or a production SessionResolver.

Reuse strict #187 per-request dummy Bearer/custom-header checks and epoch recheck
before SDK dispatch. Transport ids/tunnel URLs do not select an owner. Ownership
is the server-derived immutable `(owner, independent principal, auth epoch)`
identity. Tokens are minted by the server with256 random bits and fixed `oph_`
prefix, stored only as SHA-256 digests with that identity and monotonic deadline.
Create returns the token for model handoff; consume never echoes it. Logs contain
safe proof markers, never token/credential values or secret fingerprints.

Core create/consume and Registry rotate/revoke share the SAME lock. Authentication
is checked again while holding it. Each complete in-memory operation is atomic
relative to control changes. Rotate preserves principal but advances epoch;
therefore the NEW valid credential cannot resolve an OLD epoch's handle. Revoke
denies stale identities and bindings. A completed operation may have its response
delivered after a later control event; this zero-authority response does not prove
production file/process revocation or quiescence. No lock is held over network I/O.

Wrong owner fails without consuming another active owner's handle. Successful
consume deletes its active binding exactly once; replay fails. TTL120s, max32
live handles globally, fail closed on live capacity, no eviction. Lazy cleanup on
every core create/consume removes expired/inactive-epoch bindings; invalidated
entries may remain until the next operation, but cannot be resolved. Retain at
most128 digest-only minted-token tombstones: retired-token collisions fail closed
instead of recycling consumed/expired strings. After128 successful mints, further
create fails with probe.mint_budget. No unbounded history or raw token retention.

Process reset loses every handle/tombstone and regenerates diagnostic principals;
old bindings cannot be recovered. Crypto randomness, not persisted history, avoids
token reissuance across fresh processes. The PUBLIC dummy credential registry
still resets at restart: durable production authentication/revocation is not
implemented. Operator controls remain fixed local stdin only (one rotation per
owner, max16 bounded command lines), never remote tools. Existing body64KiB,
headers16KiB/64, guardedactive8/HTTPtasks32, request20s, requests512, lifetime25min
bounds remain. Default loopback host protection; explicit synthetic tunnel switch.

## Exact proof categories

Automated core proof: owner/exact-input/single-use/replay, forged identity,
epoch change invalidates old handle, revoke denies create/consume, B remains
independent, exact TTL boundary, capacity cleanup, process reset, collision after
retirement, finite mint budget, one racing consume, rotate waiting for a suspended
atomic create and revoke waiting for a suspended atomic consume. Revoke returning
prevents subsequent synthetic operations; no stronger production claim.

Real local MCP proof: exact two-tool discovery, forbidden control/old-tool denial,
canary auth refusal, cross-owner denial without consuming A token, unchanged
handoff across fresh connection, replay refusal, old credential401 on same socket,
new credential valid BUT old handle denied, new bound handle accepted, A revoke
denial while B's own handle succeeds, later reconnect still denies revoked A,
all returned tool markers matched to local logs, raw token/credential absence.
Inherited #186/#187 tests and both MCP smokes are also run by matrix CI.

Live FranceStudent baseline handoff in THIS endpoint is now corroborated. An A
rotation attempt produced no new token; a B attempt produced a real refusal but
789 seconds after creation, so it cannot isolate epoch invalidation from expiry.
See `LIVE_EVIDENCE_2026-10-12.md`. Live epoch-before-expiry attribution remains
unmeasured; no pass is inferred from these attempts. Prior live
header/handle and credential lifecycle sessions remain separate evidence; do not
reuse their markers for this new experiment. No proof of per-chat isolation,
future host behavior, production credentials/storage/OAuth or transactional
capability/lease/session provisioning. #172/#173 stay open.

## Windows — one step and one output review at a time

Use a new extracted folder; Python3.11+, no Rust required. Setup reuses the frozen
sibling environment. Do not overwrite previous evidence folders or resume old
tunnels. Stop after each step and send output to the accompanying assistant.

1. Setup:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\epoch-handle\scripts\Setup-EpochHandleProbe.ps1
   ```
   Expected EPOCH HANDLE SETUP PASS. Send output and wait.
2. Test:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\epoch-handle\scripts\Test-EpochHandleProbe.ps1
   ```
   Expected7 tests and EPOCH HANDLE TEST PASS; test-only loopback server is stopped.
3. Start only after review:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\epoch-handle\scripts\Start-EpochHandleProbe.ps1 -AllowRandomTunnelHost
   ```
   Expected Authority NONE/dummy-epoch-handle-v1/port8003. Keep console open.
4. Second window: `cloudflared tunnel --url http://127.0.0.1:8003`. Configure the
   new HTTPS URL + `/mcp`, dedicated authorization `Bearer optic-probe-dummy-owner-a`
   and the same two dummy secure JSON headers from #187. Disable older connectors
   to avoid endpoint confusion. Review initial HTTP rows and exact two tools.
5. Baseline model test: actual create then immediate consume with the returned
   application_handle unchanged. Match both server/tool proofs before continuing.
6. Prepare the entire rotation trial BEFORE creating a pending token: server
   console, connector edit screen and the two exact model prompts must be ready.
   Take any breaks now. Ask the model to create one NEW pending token only and
   leave it unconsumed. WAIT for the complete actual response containing
   `ok=true` and `application_handle`; if it fails or no token appears, STOP the
   trial without issuing rotate, saving a new credential or requesting consume.
   After successful create, run this short trial without pauses or intermediate
   output exchanges with the accompanying assistant: locally enter rotate-a,
   verify the LOCAL ok/epoch2 acknowledgement, update the connector to full
   `Bearer optic-probe-dummy-owner-a-v2`, wait for successful save, then ask the
   model to consume exactly that NEW pending token without minting another.
   Stop on any failed acknowledgement/save. Send all results together afterward.
   Expected authenticated HTTP200 with tool ok=false/probe.invalid_handle.
   To attribute this to epoch change rather than expiry, admission timestamps
   must demonstrate the attempted consume occurs comfortably within TTL120 after
   creation, with no intervening consume. If the guided exchange takes too long,
   record ambiguity instead of claiming epoch proof or increasing the timeout.
   Automated fake-clock proof remains valid even if manual timing is ambiguous.
   If the operator has to pause, abandon this trial and record that; do not reuse
   its token as proof of epoch invalidation. The one-rotation-per-owner limit still
   applies: another clean trial requires an untouched owner or a fresh process.
   Process restart is an explicit test reset, not recovery of production revocation
   state; new server instance and initial dummy UI credential are required, and
   no previous handle may be reused.
7. Only after reviewing old-binding denial: new-epoch create/consume should work.
   Cross-owner control follows the same TTL discipline: original B dummy cannot
   consume a pending A handle; restore active A and consume within TTL to verify
   B did not consume it. Revoke locally invalidates remaining A access; B can only
   use its own minted bindings. Each change and call gets output review.
8. Ctrl+C server/tunnel and disable connector. Share only safe logs/results, no
   raw credentials/captures. Record tested commit, instance and invocation markers.

This phase is authority-free contract integration, not promotion to real projects.
Durable production credentials and full SessionResolver authority provisioning
remain gated and require separate contracts/review. No draft is merged here.
