# FranceStudent dummy headers + application handoff experiment

Owners: #171 / #172 / #173. Stacked on the exact #170 H-01 head
`d8ebc0b5d7f46ce61a9aad737bc64690b7ca584b`. Keep Draft. No Rust changes.

## Boundary and interpretation

This server exposes exactly `probe_headers`, `probe_create_handle`, and
`probe_consume_handle`. It imports no Optic runtime, accepts no workspace/project,
and offers no file/process/Git/mutation/session_cancel tool. The scripts and smoke
runner start only test infrastructure. No remote tool can launch a process.

Only the two published **dummy** Bearer values below are accepted. These are test
selectors, not production authentication. Unknown/missing/duplicate auth and
missing/wrong/duplicate required custom headers fail before MCP dispatch, on each
request. Public dummy values are no access-control guarantee against strangers;
the safety boundary is zero authority plus bounded resources. Never configure a
real token. No bearer/header values or their fingerprints leave the server. Only
presence/single/scheme/exact-dummy-match bits and independent invocation markers
are logged/returned. Third-party logging and Uvicorn access logs are disabled.
Rejected requests never echo values. Tool arguments are narrowly validated before
SDK dispatch; unknown tools (including session_cancel) and extra arguments fail.

The handle has 256 random bits, a fixed `oph_` prefix, and **zero authority**.
The server retains only its SHA-256 digest with synthetic owner A/B and monotonic
deadline. Maximum 32 active handles, fixed 120-second TTL, lazy deterministic reap
on create/consume. Capacity fails closed (no eviction of live handles); collision
fails closed. Owner mismatch fails without consuming the other owner's token.
Successful consume atomically removes state; a racing/repeated consume fails.
Expiry/replay/unknown/wrong owner share `probe.invalid_handle`; invalid raw shape
is refused at ingress. Restart loses all handles; no persistence or resurrection.
No raw handle is logged or echoed by consume, but create must return it to test
model handoff. This is **not** an internal SessionHandle, production SessionResolver,
per-chat isolation, approval, capability or lease. Same-owner conversations can
reuse a not-yet-consumed token if someone explicitly supplies it; no chat-id exists
to distinguish them. Owner A/B illustrates principal scoping only.

Bounds: 64 KiB body (streamed/cumulative), 16 KiB aggregate headers / 64 parsed
headers, 8 guarded active requests with no waiting queue, 32 Uvicorn admitted HTTP
tasks, 20-second guarded request timeout, 5-second keep-alive, 512 observed requests
per process (then quiet HTTP 429), 25-minute server lifetime plus at most 5-second
graceful shutdown. Header parser also has a 16 KiB incomplete-event ceiling.
Tunnel/provider limits and low-level rejected-header logging are outside the safe
proof channel. DNS-rebinding protection defaults to loopback; the explicit random
tunnel switch relaxes only this authority-free experiment. Do not copy it to core.

## Known facts vs missing evidence (2026-10-12)

- Product/admin statements recorded on #171/#172/#173: `Authorization: Bearer <value>`,
  custom secure headers on every request and hidden from model, no chat-id, global
  connector, no native call confirmation/cancellation, model-directed parallelism.
- Local automated proof: exact dummy-match comparisons on admitted MCP discovery,
  initialization, tool and reconnect requests; same-owner handoff over a fresh
  connection; other-owner denial; one atomic consume; log canary absence.
- User-supplied live server/model evidence now corroborates one `probe_headers`
  call, model-mediated unchanged create-to-consume handoff and subsequent refusal
  when asked to reuse the consumed handle. See `LIVE_EVIDENCE_2026-10-12.md` for
  exact markers, source qualifications, rejected requests and proof limits.
- In the tested UI configuration the dedicated authorization field required the
  full `Bearer <dummy>` entry. The initial no-prefix guide failed; this is measured
  UI-entry behavior, not proof of the raw value of earlier rejected headers.
- Universal secure-header delivery remains **unproven**: two observed requests
  had matching Bearer but both custom headers absent. Their method/source was not
  captured. Explicit connector reconnect, live expiry/owner-denial, timeout,
  header conflict precedence and grouped temporal overlap remain unmeasured.
  Secure-field invisibility is an admin statement, not something the server can
  establish. Local lifecycle tests remain distinct from live host proof.
- H-02 credential generation/storage/revoke/rotation/OAuth and H-03 production
  authority provisioning remain open. This deliberately small probe cannot close
  those issues. It adds no production architecture invariant, so no new ADR.

## Windows — one step at a time

The current server labels its proof rows `fs-probe-v2`. Before MCP/body parsing,
HTTP rows contain an allowlisted HTTP method (or `other`), endpoint-match and
query-presence booleans, plus elapsed monotonic milliseconds since process start.
STATUS rows label the response stage: header bounds, endpoint, dummy Bearer,
dummy custom headers, admission, body bounds, request shape, MCP, timeout or
internal error. No arbitrary method/path/query/IP/user-agent/exception is echoed.
The rejection body stays generic and checks stay strict. These fields distinguish
an absent-header GET from an authenticated SDK DELETE rejection in local tests;
they do not identify a caller or retroactively explain the older live 401/405 rows.
Source attribution remains unknown until a controlled new host run.

The 64 KiB cumulative body bound is checked before appending each new chunk.
Regression proof includes split/chunked overflow, timeout/cancellation capacity
cleanup, exception redaction and concurrent owner-context isolation. All three
tool names, owner/TTL/single-use semantics, credentials and resource ceilings
are unchanged. The user reports the first manual server/tunnel stopped and the
connector disabled; do not silently resume or reuse that old endpoint/instance.

Use a fresh checkout of `experiment/francestudent-header-handle` (prefer the exact
PR head). No Rust installation/build is required. Python 3.11+ is required. If
needed install it from https://www.python.org/downloads/windows/ and reopen
PowerShell. Optional public test transport uses `cloudflared`; only install that
when the local tests pass (`winget install --id Cloudflare.cloudflared`).

**Stop after each numbered step and send the output/screenshot to the accompanying
assistant. Do not run the next step until it has checked the result.** Screenshots
should show only the dummy probe, never other connectors' secrets.

1. Open PowerShell in the fresh repository folder and run:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\francestudent-probe\scripts\Setup-FranceStudentProbe.ps1
   ```
   Expected: `SETUP PASS`. If `python` is unavailable, the script accepts
   `-PythonCommand 'C:\path\to\python.exe'`. Do not change permanent execution policy.
2. Run the deterministic tests and local-only real MCP smoke:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\francestudent-probe\scripts\Test-FranceStudentProbe.ps1
   ```
   Expected: `TEST PASS`. It starts/stops its own loopback server, accepts no remote
   URL or user credential, and checks exact discovery plus owner/replay/concurrency.
3. Start a fresh manual server and keep that window open:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\francestudent-probe\scripts\Start-FranceStudentProbe.ps1 -AllowRandomTunnelHost
   ```
   Expected: `Authority NONE`, endpoint `http://127.0.0.1:8001/mcp`.
4. In a second PowerShell window start the temporary tunnel:
   ```powershell
   cloudflared tunnel --url http://127.0.0.1:8001
   ```
   Share the printed HTTPS URL with the assistant. Keep both windows open. Stop
   both with Ctrl+C after testing. Never point this tunnel at a real Optic server.
5. Create a separate FranceStudent connector named `Optic-Dummy-Probe` using
   `https://<printed-host>/mcp`. Dedicated **Bearer token ou secret** field:
   `Bearer optic-probe-dummy-owner-a` (include the prefix, as measured in this
   FranceStudent UI configuration). **Headers sécurisés (JSON)**:
   ```json
   {"X-Optic-Probe":"optic-probe-dummy-header","X-Optic-Probe-Secret":"optic-probe-dummy-secret"}
   ```
   Do not add Authorization in the JSON. Keep other Optic connectors disabled in
   this test chat to avoid confusing endpoints. Show discovery plus safe console
   HTTP/STATUS/DISPATCH rows; every admitted request must have all match bits true.
6. New FranceStudent chat, ask: **Call the real `probe_headers` tool once. Return
   its complete result and probe_invocation. Do not invent tool output.** Compare
   server_instance/request_sequence/marker with the console TOOL + HTTP rows.
7. Ask: **Call `probe_create_handle` once. Then immediately call
   `probe_consume_handle` with exactly the returned application_handle. Do not
   ask me to copy it. Return both actual tool results including probe_invocation.**
   Accepted evidence: matching local proof for both calls and consume
   `ok=true`, `passed_unchanged=true`, `consumed=true` within 120 seconds.
8. Ask for consume of the same handle again. Expected `probe.invalid_handle` with
   matching local proof. Share the results; an assertion by the model alone is
   insufficient. If it never called the tool, record that, not a server failure.
9. Optional guided reconnect: create a fresh handle; disable/re-enable the same
   dummy connector (do not restart the server); consume within 120 seconds.
   Optional owner test: create fresh A handle, change dedicated dummy token to
   `Bearer optic-probe-dummy-owner-b`, try consume (denied), restore full Bearer A
   and consume (allowed).
   Optional expiry: wait at least 120 seconds before consume (denied). Each is a
   separate step with output review. Restart always invalidates prior handles.

Record tested commit, Python/pip check, timestamp, model/host, server instance,
safe HTTP/STATUS/DISPATCH rows and tool markers. Never upload raw HTTP captures,
request dumps or credentials. Do not claim "every request forever": only the
observed test window, including discovery and reconnect, has been measured.

## H-01 and #178 remain separate

The inherited `experiments/host-probe` files/workflow are unchanged. Use that
fresh H-01 v2 process for grouped `probe_parallel` (same new run_id), correlation
and reconnect. Its endpoint defaults to port 8000; this probe defaults to 8001.
Never expose both accidentally on the same tunnel or reuse server proof across
them. #178 remains the real Rust read-only synthetic link and is **not H-01**.
This endpoint intentionally offers no parallel/correlation tool and does not
claim host temporal overlap.
