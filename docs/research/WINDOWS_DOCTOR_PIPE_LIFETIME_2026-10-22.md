# Windows doctor pipe-lifetime proof — 2026-10-22

Priority: Windows10/11 V1, existing Ubuntu CI retained for shared-code regression.
No new Linux confinement research, user-PC project execution, permissions or
security limits changed. Research branch starts at PR318's exact head
49fe3efd34c9ffe9c10c1c5945b37da2c6a74028; no #317/#319 bulk merge.

## Verified baseline and epistemic limits

main80fe108b288d2b0b6676d6128d9e9beb922eda3f; only PR318 open, unchanged octal fix.
CI39585600089 attempt1 failed Windows119173829254 after process-started8ms,
initialize-sent34ms, no response30000ms. Its full log shows nine short startup
markers ending read-authorities-provisioned at2ms and no later milestone.
This localizes absence of progress before provisioning completion but cannot
prove which synchronous operation stalled. Prior reads of the pinned ~87MB Node
binary measured seconds inside native Read; not proven cause of that failure.

Critical path: Tokio runtime/entry -> arguments/executable eligibility -> optional
Git/recovery (not configured by this TEMP-only smoke) -> session/read authorities
-> sequential process lease registration (preflight lock released, identity
capture, locked recheck) -> canonical path/native pin/full bounded BLAKE3 -> server
-> stdio -> RMCP receive initialize/handle/send -> serve. No initialized-notification
gate exists in inspected RMCP3.4.0. Existing traces lack in-flight native-read
progress. CPU/filesystem scheduling/filter effects are plausible, not identified.
No hash, pin, byte ceiling, ACL/network rule, or30s command deadline is changed.

## Independent demonstrated-risk candidate

Production doctor has two unbounded stderr.ReadToEnd calls: timeout after direct
exit, and stdout EOF even while the process remains alive. A descendant may retain
stderr beyond direct exit. Research branch49b64e's400ms Task.Wait bounds its caller
but can leave the underlying ReadAsync pending; not accepted as a complete lifetime
fix. The new native proof compares real inherited pipes against the old script.

Candidate is deliberately confined to the WINDOWS doctor: sole stderr consumer
uses existing anonymous pipe SafePipeHandle, PeekNamedPipe then ReadFile of at most
already-available bytes. No background task/thread or duplicated handle is created.
Decode UTF-8 only after byte capture, so partial code units cannot wait for another
write. Byte cap32768, poll budget400ms, preview4096chars, safe phase queue24.
Every stderr read site uses it; no simultaneous StreamReader consumer is allowed.
Response deadline uses Stopwatch rather than wall-clock correction. Poll budget
does not promise progress during OS scheduling pauses; native operations assume
one reader and synchronous local anonymous pipes. Never use on a shared pipe with
another reader; error capture is best effort and cannot replace primary failure.

This fixes an error-path diagnostic lifetime contract only IF native proof passes.
It is not a proposed cure for natural MCP startup delay, AppContainer185 or runner317.
No early-ready response, buffer/cache optimization or timeout extension.

## Native proof

Python test fixture (test-only hosted dependency) closes stdout but holds stderr,
retains stderr in a descendant after parent exit, times out with retained stderr,
writes partial UTF-8 or excessive synthetic stderr. Normal MCP result retained.
PowerShell worker AST-imports functions only, never executes installer/profile setup.
External supervisor assigns worker to kill-on-close Job before opening a file gate
that permits any fixture creation. Fixture sleepers have8s autonomous ceiling.
Baseline must still be blocked at3s; candidate outer limit5s. Guard terminates and
confirms Job empty on success AND failure. No arbitrary machine PID stop.
Four repeated silent captures test handle growth; no read worker is introduced.
Published JSON has fixed modes, durations/caps/results, no raw paths/logs/secrets.

Validation is not preclaimed. Each result includes exact SHA/run/attempt.
Normal CI remains required independently on the same SHA. Original red attempts
are preserved; no retry merely to turn a badge green. PR318 remains unmerged until
its actual candidate checks and acceptance decision justify integration.

## Reprise

Reverify main/PR318/research branch/Actions. Read this file and result JSON.
1. Review native pipe proof and normal CI on exact candidate SHA.
2. If justified, isolate doctor diagnostics/regression as a directly relevant
   validation complement, retaining PR318's octal tests and original red CI.
3. Do not close319 from a passing doctor. Capture natural startup failure with
   reviewed bounded in-flight markers before claiming cause.185 remains distinct.
4.317 Windows work needs retained ownership/reservations/session revoke/seal/reap
   integration; Python experiments are not production code. No new Linux work.
