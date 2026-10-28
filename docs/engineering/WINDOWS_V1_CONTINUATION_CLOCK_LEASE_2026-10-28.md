# Windows V1 continuation — clock provenance and original lease stop

28 October 2026. Repository: **ELITEAYOTO/Optic-AI-Bridge- only**. Windows V1 **NO-GO**.

## Read current evidence before changing anything

Last independently closed checkpoint when this source was written:
`main=eea5438873293243cd747b0f646ed903fb9f1282`, tree
`81ef82229f52bebf62eafaae05c1a2b0756d91f5`, [push/main CI #40752232971](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40752232971)
SUCCESS 4/4. This continuation's own technical PR advances main after validation:
**read live main and the latest [#323 receipts](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) for its actual final SHA, tree and independent CI.**
Do not treat a pending workflow or this source's historical checkpoint as completion.

Read AGENTS, STATUS, ROADMAP, CHANGELOG, GITHUB_HANDOFF,
ISSUE323_NATIVE_COMMIT_AUDIT_2026-10-28 and NATIVE_GIT_READ_WITNESS_CONTRACT.
Old prompts are historical. One PR at a time; no new PR before the previous
squash and independent push/main CI are fully verified.

## Established slices, with limits

- #416: original transport clock identity, no retrofitting after admission.
- #417: original clock-issued session expiry and counted clocked admission;
  raw renewal strips future provenance, old permits retain original deadline.
- #418: original TaskLease generation/clock witness; foreign or absent origin
  rejected by the inert service-clock preparation. Legacy MCP remains unchanged.
- #419: actual executable identity IO moved outside TaskLease revoke mutex for
  lookup and renewal. Original pins retained; final generation/set/expiry
  rechecks; renewal publication all-or-nothing.
- #420: actual debug renewal panic reproduced (200 vs 100) and fixed without
  removing current validation or refreshing the original permit deadline.
- Current pre-drain lease-stop slice: behavioral RED was obtained on Windows
  and Ubuntu at cad47990342c8aa84a9ce6bcdb62943f7e90bdfa (#40753355684 and
  #40753355716): original lease stop waited on admission drainage. The corrected
  source publishes only flags early, retains records/pins/capacity and repeats
  the lease sweep after drainage. Require final corrected-head and independent
  main receipts in #323 before treating this slice as integrated.

All native status preparation/custody is still **non-executing**. No native
GitRead root has been launched by this owner. Existing real empty Job/watchdog
tests are not evidence of Git/descendant/root/stdio physical termination.
Production GitRead still uses HardenedCommandRunner and its reader joins remain
a known unbounded-liveness risk.

## Next concrete P0 boundary

Analyze actual service-owned host shutdown and policy authority before any
CreateProcessW/ResumeThread integration. Avoid a new caller-supplied boolean,
clock, identity or fresh lookup masquerading as original authority.

Observed on the checked production source:
- app main constructs `Arc<dyn Clock>` from StdClock and issues legacy numeric
  session/lease expiries; MCP workers still use the legacy admission interfaces.
  The internal clock-coherent native constructors are not wired into that path.
- app main invokes lifecycle revoke after transport termination explicitly
  inside `if args.supervise_session`. Audit default vs supervised shutdown and
  all early-error/drop paths; this observation alone does not establish what
  physically happens to native jobs when the application runtime exits.
- policy_epoch is carried from startup configuration/session/lease/envelope;
  equality of captured integers is not a live policy-revocation issuer.
- session and lease mutexes remain distinct. Positive sequential observations
  are not an atomic launch permission. Define where revoke or launch wins,
  retain ownership across stalled Win32 calls and never hold sensitive locks
  during IO, process creation/resume/waits/termination.

Prefer the smallest real service integration with non-substitutable original
authority, then prove its races. Do not add redundant empty-Job fixtures or
activate native GitRead merely because these inert prerequisites pass.

## Preserved failures and delivery gates

#418 initial HEAD 4e57f565 had natural Node/AppContainer RED in CI #40746209363:
10s timeout, empty Node output, no observed network attempt. The three built-in
later comparisons passed but did not explain or repair the failure. Original
cold Windows tests now emit fixed launcher phases; targeted native tests run
independently before specialized Node proofs. #185 remains OPEN. No retry-to-
green or timeout/ACL/network/sandbox weakening.

#319 natural MCP startup intermittence remains distinct. #176/#182 sensitive
read/provenance, Windows 10/11 actual platform and installed MCP acceptance,
physical termination/EOF, crash/uncertainty custody and explicit delivery
acceptance still block GO. No release/tag/installer publication.

## Operator scope and finish procedure

No project code/build/test/server/installer on the operator PC, no self-hosted
runner, Desktop or REL changes. Hosted Windows + Ubuntu only. Squash merges
were explicitly authorized by the owner; repository security-policy changes
and delivery still require specific authorization. Self-review is not an
independent approval.

At the end, verify final PR HEAD, every applicable job/log, source/test-merge/
squash tree identity, live main and independent push/main 4/4. Update #323 with
actual evidence, limitations and failures. Preserve any unfinished draft;
do not claim a simulated or non-executing observation is physical proof.

Owner requested usage monitoring and a clean finish around 8–10% remaining.
Read live account limits; a past percentage is not current. Keep enough margin
to finish CI, receipts and a self-contained handoff. No automation was created.
