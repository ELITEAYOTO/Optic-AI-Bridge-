# Issue #185 — micro-phases Node / AppContainer (5 November 2026)

**Status: research-only, not a fix.** Current experimental parent is `a1a7eeae1f7d4a648c6975a94c50f0fe2821a02f` on `bootstrap/185-process-memory-checkpoints-2026-11-04`. The original Windows V1 gate is NO-GO. PR #511 is not merged and its first-attempt Windows failure remains authoritative.

## Historical Windows-hosted evidence, same image

All six reviewed Windows jobs used Microsoft Windows Server 2025 image `windows-2025-vs2026` version `20261030.294.1`. This controls the **image version**, not background scheduling, Defender scan state, host resources, or executable file-cache state.

| Source | Workflow / Windows job | Initial Node proof | Observations |
|---|---|---|---|
| docs-only PR #491 | 41521859700 / 125539627855 | RED, TimedOut | target resumed at 219ms, no JS marker |
| PR #499 | 41794726225 / 126456012820 | RED, TimedOut | target resumed at 382ms, no JS marker; a serial comparison later needed 7524ms but did exit 42 |
| PR #511 | 41885824035 / 126769880625 | RED, TimedOut | target resumed at 230ms, no JS marker; the following serial attempt was also TimedOut |
| PR #500 | 41804090326 / 126486698078 | GREEN | target resumed at 16ms, normal Exited/42 |
| push/main | 41857747040 / 126670829555 | GREEN | target resumed at 17ms, normal Exited/42 |
| #185 memory research | 41906684964 / 126843111308 | GREEN | target resumed at 17ms, normal Exited/42 |

**Important separate phenotype:** the original `push/main` run [#40084443275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40084443275) emitted `NODE_PROBE_STARTED` and `NETWORK_ATTEMPT` and then timed out. The first group above was mostly pre-JS, which must not be conflated with a synchronous network-call stall.

## Test-only discriminator

The real Node script in `crates/optic-bridge-app/tests/appcontainer_network_isolation_windows.rs` previously printed `NETWORK_ATTEMPT` **before** `net.createConnection` and armed a 1500ms JS timeout **after** the call. This meant a future failure with `NETWORK_ATTEMPT` alone could be due either to a synchronous call which did not return, or to a subsequent event-loop/timer stall.

The research fixture adds `NETWORK_CREATE_RETURNED` immediately after `net.createConnection` returns, then `NODE_TIMER_ARMED` immediately after `setTimeout` returns. The original mandatory `NODE_PROBE_STARTED`, `NETWORK_ATTEMPT`, denied network, no accepted socket, and normal exit-code 42 or 43 requirements are **unchanged**. A passing attempt must now also present the two new markers. The output increase is a few dozen fixed bytes within the same original output budget; no new environment, ACL, SID, network right, file grant, timeout extension, process route or CPU quota change. Printing is part of the **test script only**, not the production runtime.

Diagnostic interpretation of a *future natural failure*:
- no `NODE_PROBE_STARTED`: failure before JS first marker; do not ascribe it to socket code;
- `NETWORK_ATTEMPT` but no `NETWORK_CREATE_RETURNED`: synchronous call may not have returned, but missing stdout delivery is a competing explanation;
- `NETWORK_CREATE_RETURNED` but no `NODE_TIMER_ARMED`: failure between return and timer installation, or output missing;
- `NODE_TIMER_ARMED`, but no terminal `NETWORK_DENIED` / `NETWORK_TIMEOUT`: investigate event loop, callback delivery, OS scheduling and launcher/pipe observations. Markers alone do not diagnose root cause.

## Acceptance and preservation

RED on real Windows 10: 1 test actually executed, normal `Exited/42` network-denial proof but new mandatory marker missing; assertion fails exit **101**. GREEN: real Node/AppContainer network test exit 0, observed both new markers and no accepted socket. Exclude any attempt with zero tests or Cargo help output. Pinned Rust 1.99.0 `--locked --offline`, no self-hosted runner.

#185 root cause **unknown**. The first-attempt CI failure of PR #511 is not transformed into a pass by a successful independent run on this research branch. Main and the daily installed Bridge stay unchanged. Windows V1 remains NO-GO.
