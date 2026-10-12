# H-02 dummy credential lifecycle — experiment only

Owning issue #172, diagnostic dependency #186. Stacked on #186 exact head
`106c98939c2c463fea0b2c984d6087b017993017`; main still
`156653a406f01383e2fc14cada513963453b04f3` at initial remote-ref check.
Keep Draft; no Rust changes or real workspace/runtime exposure.

## Contract

One remote tool: `credential_probe`, returning only zero-authority metadata plus
independent principal id and authentication epoch. No handle/session tools,
process, files, Git, mutation, session_cancel or remote revoke/rotate tool. A
principal is a diagnostic connection identity, never a grant. The original H-01
and #178 experiments are separate; nothing here proves host parallelism.

Reuse #186's bounded guard/secret-free proof channel and frozen SDK. Two optional
guard dependencies (header observer and allowed no-argument tool names, plus
optional handle tool) let this independent server use epoch-aware authentication;
defaults retain #186 behavior. Recheck epoch after bounded body ingestion before
SDK dispatch, and again in the harmless tool. A previously checked body waiting
for ingestion cannot recover revoked/rotated authentication. A concurrently
already-admitted metadata response may finish; this does not claim effect-spanning
production revocation/quiescence for files/processes. Production must define its
own admitted-effect contract.

Two fixed synthetic owners A/B, each with a random independent diagnostic
principal, epoch, active bit and version. Credentials are published dummy values,
not production secrets or useful access control. Only exact single Bearer plus
the two exact custom headers pass, per request, with constant-time dummy-byte
comparison and no positive-auth cache. New credential preserves principal but
increments epoch. Revoke disables current credential and increments epoch.
Each owner permits **one rotation only**, old->v2; no cycling back to an old
credential. A second rotation fails closed. Other owner is unaffected.

Local operator controls are ONLY fixed stdin lines in the server console:
`rotate-a`, `revoke-a`, `rotate-b`, `revoke-b`. No HTTP control endpoint, no MCP
control tools. At most16 command lines per process, at most64 bytes/chars read per
line; oversized line closes this input channel, invalid input logs only a constant
error. Logs never echo raw commands or credential values. State is guarded by one
lock; two-owner registry/history is fixed size. Existing remote budget: body64KiB,
headers16KiB/64, active8/HTTPtasks32, request20s, requests512, lifetime25min.

**Process restart deliberately resets this dummy experiment**, including revoked
state and dummy initial version. Reconnect/tunnel restart while the same server
is alive does not change its registry. No production durable revocation, secret
generation, verifier-only storage, DPAPI, OAuth, rate-limiter certification,
SessionResolver or H-02 completion is claimed. Never reuse public dummy values
or restart semantics for production. Tokens/epochs confer no project authority.

## Automated proof vs unknowns

Deterministic registry tests: revoke/rotate invalidates old identity; new epoch
keeps independent principal; wrong/malformed values fail; ownerB unaffected;
one concurrent rotation succeeds; old credential never recycles; fixed bounds;
stale epoch denied before SDK dispatch. Actual loopback HTTP/MCP smoke: exact
one-tool discovery, remote control/handle calls rejected, rotate then old fails
on the **same HTTP keep-alive socket**, new succeeds, old remains denied after
fresh connection, revoke then both A versions fail, ownerB succeeds, revoked
credential remains denied after reconnect, raw canary/credential absence in logs
and returned error data. Uses modern MCP envelope and matching method/name
headers; no real host is contacted. Inherited #186 unit/protocol tests also run.

Still unmeasured: actual FranceStudent behavior on rotate/revoke, caching/retry/UI
error semantics; tunnel restart during lifecycle; real secrets/durable storage;
production auth/session provisioning. #186's two live sessions prove the header
carrier and model handle feasibility, not these lifecycle behaviors. Historical
missing-header requests remain unresolved. New live results must have new instance
and independent server proof; do not reuse old seq/markers.

## Windows — stop after each step for output review

Use the new experiment archive, extracted in a separate folder. Python3.11+;
no Rust required. The setup delegates to the sibling probe's isolated venv and
same frozen requirements; no global install. One step at a time with the assistant.

1. In the new root folder:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\credential-lifecycle\scripts\Setup-CredentialLifecycle.ps1
   ```
   Expected `LIFECYCLE SETUP PASS`. Send output and wait.
2. Run:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\credential-lifecycle\scripts\Test-CredentialLifecycle.ps1
   ```
   Expected `LIFECYCLE TEST PASS`. This starts/stops its own loopback test server.
   Send output and wait.
3. Guided manual start (only after reviewing tests):
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\credential-lifecycle\scripts\Start-CredentialLifecycle.ps1 -AllowRandomTunnelHost
   ```
   Expected Authority NONE/dummy-lifecycle-v1/port8002. Keep console open; it owns
   stdin for local commands. Send output and wait before tunnel or host setup.
4. Separate window, temporary tunnel: `cloudflared tunnel --url http://127.0.0.1:8002`.
   Configure a separate connector with printed HTTPS URL + `/mcp`; dedicated
   authorization field **`Bearer optic-probe-dummy-owner-a`** and secure JSON:
   ```json
   {"X-Optic-Probe":"optic-probe-dummy-header","X-Optic-Probe-Secret":"optic-probe-dummy-secret"}
   ```
   No real tokens. Review setup logs before the first `credential_probe` request.
5. After matching initial tool proof, enter `rotate-a` in SERVER console only.
   Review `[LOCAL]` acknowledgement, then ask for an actual call with the unchanged
   old connector credential: expected HTTP401, no new tool proof. Host may cache
   or fabricate results; compare server rows, not model summary.
6. After reviewing old refusal, update dedicated field to
   **`Bearer optic-probe-dummy-owner-a-v2`**. Compare real `credential_probe`
   principal/epoch and matching server proof: same principal, higher epoch.
7. After reviewing new success, enter `revoke-a` in server console. Review local
   acknowledgement, then actual call must fail401. Connector toggle/reconnect
   cannot restore validity while this same server remains alive. Every step gets
   output review; do not restart the server mid-lifecycle to circumvent refusal.
8. Ctrl+C both windows and disable the dummy connector. Record tested head,
   instance, local acknowledgements and safe HTTP/TOOL markers, not raw captures.

Further testing never grants real authority. #172/#173 and production security
gates remain open; keep this experiment Draft and do not merge it by declaring
the dummy lifecycle proof a complete authentication implementation.
