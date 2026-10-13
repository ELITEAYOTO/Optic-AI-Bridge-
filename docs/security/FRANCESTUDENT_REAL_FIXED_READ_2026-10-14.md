# FranceStudent real fixed-read fixture measurement — 2026-10-14

Status: OBSERVED GENERATED-FIXTURE HOST EVIDENCE / NOT PRODUCTION ALPHA

## Setup and provenance

The user ran the tested Windows package from draft #207 at branch419dd1c,
workflow38476906798, artifact11854671497. Packaged checkout is
35a617d3c32528e13b6e5e107da632d124947229 (workflow merge commit). The downloaded
artifact digest and inner binary/manifest were separately verified locally.

Path: FranceStudent Luna -> temporary Quick Tunnel -> loopback8008/mcp -> bounded
three-tool adapter -> one long-lived real Rust stdio Optic server -> generated
temporary files only. All credentials/headers are public dummy fixtures.
No user project, real credential, process/Git/mutation authority or persistence.
Server instance throughout the observed run:0d3aee74e6cf.

## Correlated observations

| Phase | Console request(s) | Result / marker |
| --- | --- | --- |
| Discovery | 1–3 | initialize200, initialized202, tools/list200; single matching Bearer/custom headers |
| Real session metadata | 7 | success; d55c642a30496ed9 |
| Fixed listing | 8 | public/hello.txt only; e09ab4feb17e2285 |
| Real fixed read | 9 | success; b9c48b624751d350 |
| Local rotate A | local | active true, epoch2 |
| Old credential attempt | 11–13 | POST/DELETE/GET401 at dummy_bearer; no MCP dispatch/tool event |
| Updated credential discovery | 14–16 | 200/202/200 on same instance |
| Metadata after reconnect | 20 | success; f89da3dfeacf1783 |
| Read after reconnect | 21 | success; 56ea1131549735fa |
| Local revoke A | local | active false, epoch3 |
| Revoked credential attempt | 23–25 | POST/DELETE/GET401 at dummy_bearer; no MCP dispatch/tool event |
| Local stop | local | ok true, drained true, session_revoked true; returned PowerShell prompt |

All five success markers above match the independent pasted console tool events.
The base64 read decodes to `PUBLIC_GENERATED_OPTIC_HOST_FIXTURE` plus newline,
36bytes, eof true. Optic metadata before/after rotation is identical:
principal local-stdio, project local-workspace, capabilities file_read/file_search,
policy_epoch1, expires_at_monotonic_ms1800000. No deadline extension is observed.

This is unchanged metadata and known long-lived upstream composition, not a
proof that a hidden application handle is unique or that FS supplies chat identity.
The local stop acknowledges the real Rust session_cancel call. Independent CI
additionally tested exact optic.session_inactive for a later direct Rust read;
the manual host run did not perform that post-stop direct read.

## Limits

No host concurrency, in-flight host cancellation, alternate-owner live-host
measurement, crash recovery, real credential enrollment or user-project security
closure is claimed. A tunnel/bearer does not grant authority; all host chats using
this test connection share its one locally selected fixture session. The local
dummy registry epoch is distinct from the Rust policy epoch.

## Stop boundary audit

The stop path was traced through server exit, gate drain, session_cancel,
session/lease/approval revoke, owner-scoped job cancellation and normal stdio
child/context cleanup. No DNS/route/interface/firewall/router modification occurs
in that path. No process authority was granted in this fixture. The separately
started cloudflared process is not selected by Optic session job cancellation.
