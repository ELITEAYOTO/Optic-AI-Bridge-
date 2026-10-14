# Single-session alpha integration gate

Status: GENERATED-FIXTURE READ/EDIT IMPLEMENTED; LIVE FS EDITING TEST PENDING
Date: 2026-10-13
Scope: [ADR-0013](../decisions/ADR-0013-SINGLE_SESSION_REMOTE_ALPHA.md)

## 2026-10-14 delivery checkpoint

The original three-tool read stage was measured in FS in test9. The next
generated-project editing stage is implemented on the alpha delivery branch:
six tools with strict parameter schemas, exact read versions, typed Rust write
preconditions, one retained serialized slot, dummy-auth rechecks and real local
stop. The CI package preserves generated output for inspection. See
[generated-project alpha](GENERATED_PROJECT_ALPHA.md) and the packaged
experiments/project-alpha/HOST_TEST_STEPS.md. This is a public disposable exercise,
not existing-user-project enrollment or credential-storage completion.

The historical requirements below describe the original read stage. Broader
filesystem edits are now automated integration evidence; live FS mini-project
creation/modification still needs its manual result. Process/build/Java runtime
eligibility and actual user-project authority remain separate gates.

## Existing reusable proof

Draft #178 (`experiment/real-optic-mcp-link`) already connects one long-lived
real Rust stdio MCP server to a Python HTTP adapter and generated temporary
files. Its fixed remote allowlist is `optic_session_info`, `optic_fs_list`,
`optic_fs_read`. The HTTP surface never forwards arbitrary upstream tools.
The real binary receives only the generated workspace; caller-selected project
paths are absent. This is the starting integration fixture, not a product server.

Its old public generated-fixture endpoint is unauthenticated. Do not present it
as alpha enrollment or reuse it for user projects. Existing long-lived stdio
ownership also does not prove credential revocation, cancellation of a queued
read or a reliable local stop. These must be composed explicitly.

## Required implementation before a new host test

| Boundary | Required behavior | Evidence |
| --- | --- | --- |
| Enrollment | One locally selected binding; no remote project/session selector | Fixed argument schema; rebinding denied |
| Authentication | Bounded exact parsing; duplicate/malformed credentials denied before SDK dispatch | Initial discovery, call and reconnect negative tests |
| Admission | Recheck current owner/epoch after body ingestion and after queue admission | Rotate/revoke while body or forward slot waits |
| Shared session | Same real session on new HTTP transports; no implicit mint/renew | Real `session_info` identity comparison |
| Rotation | Old credential denied; new valid credential only reaches locally valid binding | Exact version/epoch test; no authority from rotation |
| Local revoke | Reject later calls and suppress late results; drain already admitted work to known outcome | Held upstream call, local revoke, reconnect denial |
| Limits | Bounded request body/header/queue/concurrency/output/deadlines and lifetime | Overflow/timeout/cancel cleanup cases |
| Diagnostics | Fixed errors and correlation markers; no credential, absolute workspace path or session reference logs | Captured logs checked against canaries |
| Tool boundary | Exactly three read-only tools through real Rust policy | List verification and unknown tool/traversal denial |

Credential revocation is distinct from revoking the real application session.
An epoch change must not silently transfer session authority; choose and test
the local binding update/closure rule before wiring the adapter. A cancelled
HTTP wait cannot be assumed to cancel an admitted upstream action. Even this
read-only fixture must retain its concurrency accounting until the outcome is
known; mutation/process tools remain excluded.

## User participation

No FS admin response or fresh manual test is required for local implementation.
After automated real-binary integration checks pass, provide a new bounded
generated-fixture package with exact prompts and console correlation markers.
That host test checks compatibility and reconnect/revoke behavior, not arbitrary
real-project access or all tools. Broader alpha tooling follows the remaining
read, metadata, Git, process, storage and local-approval gates separately.
