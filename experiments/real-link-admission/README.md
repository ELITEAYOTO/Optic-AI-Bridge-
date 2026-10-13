# Experimental fixed read-link admission

The contract module itself exposes no HTTP server, real Rust process, user
workspace, real credential, persistence or product authority. The separate
stdio smoke below composes it with a generated-workspace Rust process; HTTP
composition and actual session cleanup remain next gates. Python is an experiment
harness, not a new core dependency.

`real_smoke.py` separately launches the actual Rust stdio binary against its
own generated temporary files. Exact head7f535cb15005c5ad4d93d0f9e3776ef3e36b349b
passed dedicated workflow run38474093643 on Windows and Linux: fixed listing,
two fixture reads, stable session metadata after dummy credential rotation,
other-owner/stale/revoked denial and completed gate drain. No HTTP, real-project
or actual Rust session-revoke evidence is inferred. Local compilation hit missing
MSVC `msvcrt.lib`; no machine installation was modified.

One application-selected dummy principal owns one fixed binding. Only session
information, a four-entry listing of `public` and a1024byte read of
`public/hello.txt` are selected internally. A caller cannot supply upstream
tool names, file arguments, project paths or session selectors. Another owner
is refused. Local credential rotation preserves principal identity while old
requests/late results fail epoch checks; rotation does not mint authority.

Eight retained upstream tasks maximum, with immediate refusal and no waiter
queue. Caller timeout (maximum15seconds) or cancellation leaves the admitted
task and capacity reservation intact until completion. Completion callbacks
retrieve detached failures without logging exception strings. Local stop closes
admission first, suppresses late results and waits at most5seconds; incomplete
drain returns false and never resets/reopens the gate.

Seven deterministic/local async tests cover fixed calls, owner denial, rotation
before worker dispatch and while awaiting results, redacted exceptions, invalid
limits, caller cancellation, timeout accounting, twelve-call concurrent pressure
and local-stop suppression. These are fake-upstream contract tests, not evidence
of actual Rust cancellation or remote FranceStudent behavior.

This gate is loop-owned. Dummy registry checks are snapshots; they do not
serialize an actual Rust effect against local session revoke. Integration must
retain the Rust lifecycle/admission barrier and distinguish credential denial
from real application-session cleanup. An upstream task that never completes
keeps its slot; the service lifetime and child teardown remain separate gates.
No automatic retry or replacement task may hide uncertain cleanup. Output bounds
and structured parsing remain obligations of the actual fixed read adapter.

## Loopback HTTP composition and local real-session stop

`http_adapter.py` composes the existing bounded dummy-header Guard with the
fixed gate. Its three tools take no arguments. Only the locally selected dummy
principal is accepted; B, malformed/extra tool arguments, old epochs and stopped
bindings are refused. The post-ingress check runs before SDK dispatch. Replies
carry the bounded existing console correlation proof; arbitrary input is never
included in diagnostics. This is a generated-fixture experiment, not enrollment.

`http_smoke.py` starts a loopback-only ephemeral listener and the real Rust binary
on generated temporary files. It checks exact discovery, listing/read data,
reconnect with unchanged session metadata after local rotation, stale/revoked
credential denial and log canaries. The local-only `local_stop` closes admission,
drains retained reads for at most5seconds, then calls real Rust `session_cancel`
with a5second acknowledgement deadline. No remote control tool is exposed.
Failure/timeout returns a fixed error and leaves admission stopped; it is not
acknowledged as successful revocation or followed by automatic retry/reopening.
The smoke requires a subsequent direct Rust read to fail with the exact MCP
error `optic.session_inactive`, not a generic transport error.

Four local SDK/stop regressions cover actual HTTP discovery, fixed arguments,
owner/stale/reconnect denial, drain ordering, SDK failure and negative revoke
acknowledgements. These pass locally; exact cross-platform real-binary evidence
passed at exact6703dcac47b43669b4f1ae2709cf01c08720ab49 in workflow38475057579
on Windows/Linux. No public tunnel, interactive driver/package,
host cancellation propagation or real-project support is implemented here.

## Manual generated-fixture test package — 2026-10-14

The separate `host_runner.py` now supplies the bounded manual test driver and
an explicit random-tunnel-host experiment switch. Only generated files and
public dummy A credentials are supported. Local rotate/revoke change dummy
authentication; local stop/lifetime close admission, drain and revoke the real
session before normal child teardown. ProofChannel bounds console output; missing
or dropped proof is never success evidence. No remote control tool is added.

Exact branch head419dd1c64065022ba66ec2a9e58b96ecc24086cc passed all dedicated
Windows/Linux jobs in run38476906798, including the actual runner child. Windows
artifact11854671497 contains selected tracked sources plus the same tested binary.
The checkout/package source commit is35a617d3c32528e13b6e5e107da632d124947229
(the workflow merge commit). Its manifest/hash were verified after download;
the downloaded binary also passed a local generated-fixture runner read/stop.
The local readiness harness needed to tolerate a bounded initial ConnectTimeout
on Windows; full MCP read and stop acknowledgement are still mandatory.

`Start-FixedReadHost.ps1` checks package manifest and binary hash; this detects
mixups, not malicious replacement/signature authenticity. CI creates the ZIP only
after successful real-binary tests. No installed Python environment, account
credential or user files are packaged. See HOST_TEST_STEPS.md; perform one step
at a time with Codex. This is a test artifact, not a signed product release.
