# FranceStudent host evidence checkpoint

Recorded: 2026-10-18. This supersedes the unmeasured checklist in the original
2026-10-08 candidate where observations below apply. It does not promote the
experimental adapter or its public dummy credentials into production support.

## Evidence source

The operator supplied actual tool results, server console traces and screenshots
in the development conversation, principally from October 13–15. These are
operator-reported host observations, not a new independent replay, a current
provider guarantee or a complete reconstruction of every retained attachment.
No raw token, application handle, personal file content or temporary tunnel URL
is needed in this checkpoint. Local SDK/CI tests are separate evidence.

| Area | Observed host behavior | Limit of the observation |
| --- | --- | --- |
| Authentication headers | POST requests carried a single Bearer authorization and single matching `x-optic-probe` / `x-optic-probe-secret` dummy headers. A later failing attempt had a matching Bearer but both custom headers absent. | This explains that attempt's `dummy_custom_headers` 401; it does not prove all models/configurations preserve custom headers. FS's generic “expired token” text did not identify the exact failed gate. |
| Rotation and revocation | Fixed-read instance `0d3aee74e6cf` accepted tool calls, denied the old dummy credential after local rotation, accepted calls again with the updated credential, then denied POST/GET/DELETE after local revocation. Denials were recorded at the auth gate without tool dispatch. | Prototype dummy-auth lifecycle, not production credential enrollment, token storage or conversation identity. The upstream runtime policy epoch is a different domain from adapter auth epoch. |
| Synthetic application handles | On instance `edcd1ea1d1fc`, two opens of the same ticket returned the same session reference and synthetic ID; two probes succeeded with that reference. Later probes and an old-ticket open returned `probe.invalid_session`. | Zero-authority synthetic session behavior. Later invalidity alone does not identify its cause without elapsed-time/control evidence. No production exactly-once or cross-conversation isolation claim. |
| Grouped calls | FS returned two completed waits for one group, both `active_on_entry=1`; measured intervals did not overlap and the report had `completed_pair_overlap=false`. | A successful sequential control. The separate local SDK concurrent proof does not establish FS parallel dispatch, nor does this control establish that FS can never dispatch concurrently. |
| Real file access | The fixed-read host returned session info, listing and generated `public/hello.txt` through the Rust backend. Generated-project alpha runs created/updated HTML, CSS and JS; the operator then exercised the resulting interfaces successfully. | Generated public test projects. Existing sensitive-project access, all filesystem edge cases and all UI behavior are not thereby validated. |
| Node tasks | With installed Node22.15.1, actual `test` jobs returned exit0/stdout markers; a deliberate model error returned exit1/assertion diagnostics, then restoration/check/test returned exit0. A generated expense application also passed its Node tests. | Fixed isolated profiles and authorized generated files. No arbitrary command, dependency installation, Cargo, general compiler or model-authored Electron main/preload authority. |
| Read pagination | Reads in1024-byte pages reached EOF with stable versions and allowed larger generated files to be verified. A later fixed diagnostic payload series was reported without explicit truncation through6656 text bytes in one observed sequence, while7168/7680/8192 showed explicit truncation. Another7168 result retained its marker despite truncation. | Payload shape and metadata placement matter. These are not an exact universal byte/token ceiling or a file-size limit. Keep cursor/version/EOF metadata visible and support bounded pages; do not require every file to be1024 bytes. |
| Inference interruptions | Some server traces completed tool calls with HTTP200 and0–16ms handler time before FS displayed an inference-service error. The operator reported an FS administrator reproduced the interruption. A10-second delayed diagnostic call also succeeded. | Server completion does not prove client delivery. These observations do not locate a universal idle timeout or distinguish every tunnel/client/provider failure. A public SDK setting alone is not proof of the deployed cause. |
| Electron | The operator launched and exercised generated interfaces using the fixed local viewer. Separate native compatibility work reproduced ordinary Electron startup failure under the outer AppContainer. | Local fixed viewer use is not arbitrary Electron-process admission. The explicit sandbox-disabled comparison is not a production fallback or proof that normal isolated startup works. |

## Session and recovery interpretation

The operator accepted a shared single-session alpha and deferred multisession.
The reported administrator reply did not supply a stable FS conversation
identifier. The earlier question also covered cancellation, so do not interpret
the short reply as a precise specification of `/chats/{id}/stream/cancel`.
Cancellation of already-running MCP effects remains unmeasured at FS level.

Reopening a conversation, reconnecting a transport and resuming a saved project
are distinct. Files survived an explicit `ResumeRun` of the same generated run;
that is not restoration of expired authority or recovery of the model's memory.
The local supervised mode is an implementation option, not a new host observation
in this checkpoint. Private-task journal correlation on main is host-only; it
does not give the model access to journal state or permission to replay a task.

## Remaining integration work

1. Keep the current main/stdio core distinct from the experimental FS adapter.
   Production remote credentials, principal/session binding, rate/concurrency
   limits and read-only real-workspace enrollment still need their own decision
   and exact end-to-end validation. A tunnel URL is never authority.
2. Compose the existing capture, owned task, journal and publication services for
   a real authorized build path. Direct fixed Rust compile/link/run is proven
   locally/CI; FS has not yet driven that path. Cargo/build scripts, dependencies
   and writable build storage remain separate from fixed Node checks.
3. Resolve the measured Electron startup boundary before claiming general GUI
   execution. Avoid automatic sandbox downgrades or a mandatory UI/runtime for
   the Rust core.
4. Ask the operator only for the next missing end-to-end behavior. Do not repeat
   dummy-header, sequential-control or tiny-file fixtures merely because the
   original October8 checklist still calls them unknown.
