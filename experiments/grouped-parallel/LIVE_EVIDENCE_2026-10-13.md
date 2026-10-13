# FranceStudent grouped trial — sequential sample, not parallel proof

Tested #195 code0877b8d7dce620807139e7f10e3bf91842ace7b3. CI704, H-01 Host
Probe#38 and Guarded Grouped Parallel#1 green. Operator native setup,8tests and
local SDK smoke passed; those local positive/negative controls are separate from
this FranceStudent evidence. Dummy-only h01-grouped-v1 on8007 through temporary
HTTPS tunnel; server_instance09bd97b62707, fs-probe-v3. Sources: user-provided
model output, screenshots and console. No raw bearer/group token values here.
Operator confirmed final server/tunnel shutdown and disabled connector.

## Interrupted first attempt

Discovery/initialize/notification successful. Seq7 parallel_create_group marker
0dae947234803c91 elapsed151125ms. One parallel_wait seq8 marker2a50f09403b1b404
HTTP admission158500ms, TOOL/STATUS/COMPLETE161500ms (request_elapsed3000ms).
GROUP30a71a749c3bc656 invocationa3d52320d155d348 entered158516/finished161516ms,
statecompleted, active_on_entry1, overlapfalse. GROUP timestamps have their own
baseline, not Guard's. Seq9 DELETE309047ms405 after authentication.

Screenshot shows create/wait marked completed, then interface technical-service
error instead of final response. No second wait or report in supplied console.
No complete model-visible four-result chain, delivery or overlap claim. Operator
reports current FS issues with such errors; that report is not independent status
verification. Cause unknown: no attribution to prompt, parallel dispatch, server,
tunnel, model or host policy. Server HTTP200/ASGI completion is not peer receipt.

## Completed sequential sample in user-reported new conversation

All4 model TOOL proofs match console on same instance:

|seq|tool|marker|Guard elapsed_ms|
|---|---|---|---:|
|13|parallel_create_group|2f356ca2555bab75|429672|
|14|parallel_wait|c8af07abe91c8cc1|437797|
|15|parallel_wait|98decac2530cb7e8|451312|
|16|parallel_report|0dedc62d65000c67|456969|

Group727e762295284a53 had EXACTLY2 completed records:

* seq14 invocationde4e6135253ec949: entered434813, finished437813ms,
  duration3000ms, active_on_entry1, overlap_observedfalse.
* seq15 invocationf9167c22993eb83a: entered448313, finished451328ms,
  duration3015ms, active_on_entry1, overlap_observedfalse.

Each invocation matches local GROUP enter/exit, group_id and request_sequence.
Second entry is10500ms AFTER first exit. Strict interval intersection is empty;
report completed_pair_overlapfalse, active0, remainingTTL93. Create->report Guard
delta27.297s, comfortably beforeTTL120; no expiry confound. Model explicitly
reported sequential execution. All supplied authenticated requests matched single
Bearer and both dummy custom headers. DELETE seq17 returned405 at mcp; no native
cancel semantics. No raw tool args in logs; exact handle threading is model/user
account plus matching group evidence, not raw argument capture.

## Conclusion / admin comparison

Verified finite sequential dispatch despite a prompt requesting parallel calls.
Not evidence that FranceStudent cannot parallelize; the admin says host permits
parallel calls and models choose whether to use them. No contradiction established.
The approximately8 host limit remains unmeasured: our local8 ceiling is separate.
No real host overlap positive, CPU concurrency, native cancellation, stable chat
id, per-chat isolation, or TCP reconnect conclusion. New conversation recovery is
operator observation, not root-cause proof. #178 remains distinct from H-01.

Existing h01-v2 correlation/reconnect/group instrumentation remains intact; this
ledger supplements experiments/host-probe/H01_RESULT_TEMPLATE.md and #171, does
not close its exit criteria. H-02/H-03 production credential storage, permissions,
HostContext/real SessionHandle gates remain open. Keep drafts; no merge.
