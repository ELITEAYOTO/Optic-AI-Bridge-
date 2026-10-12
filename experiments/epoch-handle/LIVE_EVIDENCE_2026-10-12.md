# Bound-handle manual run — qualified evidence

Source: user-supplied Luna responses, pasted PowerShell transcripts and screenshot,
not independent access to the driveE manual process. Provided source archive
`5c13e9d722ba09f032c381b2e4ad5b1bb82a5ae2`; extracted files not independently
re-hashed. User Setup/Test passed:7 bound-handle tests and actual local MCP smoke.
Server banner dummy-epoch-handle-v1, port8003, shared proof schema fs-probe-v2;
instance `13e8ddcfabfc`. Connector display name reused Optic-Bridge-Last-Test with
this new endpoint. No raw credential/header captures or raw application handles
are retained here. Independent proof markers are not authority.

## Baseline — accepted

- Seq1–3 POST initialize200 / initialized202 / tools/list200 at admission
  elapsed102609–102734ms, exact single dummy Bearer and both custom headers.
- Seq4–6 repeat initialization/discovery at175719–175859ms, all-match.
- Seq7 `bound_create_handle`, marker `670dfeeabab9cad4`, POST200 at181344ms.
  Model result ok/single_use/TTL120/principal `cc2a058716dfa830`/epoch1/authority none.
- Seq8 `bound_consume_handle`, marker `8e81fe94353e1e08`, POST200 at186188ms.
  Model result ok/passed_unchanged/consumed=true, same principal/epoch, authority none.
  Both markers exactly match local TOOL tool/sequence/instance; admission gap4.844s.
- Seq9 DELETE198891ms, all headers match, stage=mcp405.

This validates one model-mediated unchanged bound handoff under the same dummy
principal/epoch. Result fields/principal/epoch are user-supplied tool data under
the inspected implementation; local TOOL proof independently corroborates only
tool/sequence/marker/instance. No production SessionResolver or per-chat claim.

## A rotation trial — aborted before a new handle existed

The user reported performing the prescribed sequence in under one minute, but
the supplied server transcript contains local `rotate-a` acknowledgement
active=true/epoch2 BEFORE seq10POST403703ms,11DELETE403781ms,12GET403844ms.
All three have Bearer present but not accepted, exact custom headers and
dummy_bearer401. No new create TOOL exists in this interval. The model reported
initialization/fallback Unauthorized and no handle, then declined consume because
the only previously displayed token was already consumed by the baseline.

Exact user submission timing is not logged; a submitted request may still have
been in flight when rotation occurred. This is not a handle-epoch test or a TTL
test. The guide failed to require aborting the trial on creation failure; that
requirement is now explicit. After the new A credential was saved, seq13–18
initialization/discovery succeeded; seq19 DELETE452063ms->mcp405. These prove
new credential admission, not an old-binding denial.

## B rotation trial — real denial, expiry-confounded

- User switched to untouched original B; seq20–22 initialization/discovery at
  593281–593469ms succeeds on the same instance, all header matches.
- Seq26 `bound_create_handle`, marker `2ef894ab07910c2e`, admission674031ms,
  POST200. Model result ok/TTL120/single_use/principal `b923c323d2dd1f4c`/epoch1.
  Exact local/model proof match; model instructed not to consume yet.
- Local `rotate-b` acknowledgement active=true/epoch2, then accepted setup with
  v2 credential seq28–33, at1452859–1457391ms, same instance.
- Seq34 `bound_consume_handle`, marker `afe7965af4ad13a1`, admission1463031ms,
  POST200; model result ok=false/probe.invalid_handle, exact local/model proof.
- Seq35 DELETE1470641ms, all headers match, stage=mcp405; 25-minute stop close.

Admission gap **789000ms =789s**, substantially beyond TTL120. Expiry independently
explains refusal, so this is NOT accepted as isolated live epoch invalidation.
No intermediate consume supplied, but raw argument identity is user/model account
because tokens are not logged. Ingress times are not exact mint/effect times;
the large gap does not support a before-expiry claim. Do not erase this failed
trial, infer a cause for the delay or extend TTL to manufacture a pass.

## Disposition and clean retry

User confirmed stopping server/tunnel and disabling connector after the run.
Both owners already rotated once; the next clean trial needs an explicit fresh
test process and initial dummy credentials, new instance and freshly minted token.
Old handles are never reused. This reset loses test state and is not durable
production credential recovery.

Before minting, prepare all UI/console/prompts and finish any discussion. Only
after a complete successful create result containing an actual handle may the
operator rotate; stop immediately on failure. Perform local control, successful
credential save and exact pending-token consume without intermediate assistant
exchanges. Send the entire trial afterward for server-proof and timestamp review.
If the attempt exceeds TTL120, record ambiguity again, not live epoch success.

Automated proof remains accepted independently:7 core tests and real local MCP
epoch-crossing smoke, plus17 inherited tests and2 inherited smokes on Windows/Linux.
Implementation head5c13e9d has green epoch#1, lifecycle#3 and general CI#692.
Later docs heads require their own CI. Live stale-epoch-before-expiry, live
cross-owner handle denial and production SessionResolver/credential persistence
remain unproven. Keep #188 Draft, #172/#173 open and real-project gates intact.

## Clean fresh retry — accepted finite integration proof

New instance `b4f20cc47845`, same unchanged implementation archive5c13e9d; new
tunnel and initial A dummy configuration. Seq1–3 initialization/discovery all
accepted, exact headers, elapsed140765–140937ms. Earlier failures remain above.

| Step | Admission ms / markers | Result |
| --- | --- | --- |
| A epoch1 create7 | 284969 / `10eb6737c80f65b3` | ok, principal `c541ee52597030b3`, epoch1, TTL120, unconsumed by user/model account. |
| Local rotate-a | LOCAL active=true/epoch2 | After successful create, before saved A-v2 setup. |
| Old handle consume15 under reported A-v2 | 334422 / `a14eda59074e43d2` | Authenticated delivery200, tool invalid_handle; gap49.453s comfortably within TTL. |
| New epoch2 create19 | 474515 / `873335e0ede7f282` | Same principal, epoch2, new token. |
| New epoch2 consume20 | 479078 / `f7c5d8c93b21c323` | ok/passed_unchanged/consumed=true; gap4.563s, positive control. |
| Mistaken ownership trial create24 | A epoch2 / `64c7215cd08cb08a` | A token minted; not an ownership denial. |
| Mistaken trial create32 | 664703 / `cfa92c5bf7c4c37b` | B principal `94ceca80716433b3`, epoch1 token minted; no consume in supplied trial. Assistant omitted exact copyable consume prompt. |
| Corrected ownership trial A create39 | 870812 / `5b2b62d611c2bf55` | A principal/epoch2, fresh token. |
| Reported B consume46 | 901687 / `d549cf3feefdcfa6` | invalid_handle at+30.875s. |
| Restored A-v2 consume53, same requested token | 933984 / `49681e046bfba4d4` | ok/passed_unchanged/consumed=true, same A principal/epoch2; +63.172s withinTTL. |

Every listed TOOL marker/instance/sequence exactly matches the user-supplied model
result. Principal/epoch and booleans are result fields under inspected code, not
independent local TOOL fields. Raw token/credential bytes are not logged; exact
token identity and UI owner switches rely on the user/model account. HTTP200 on
a denied consume is successful MCP delivery, not successful consumption.

Under that recorded configuration, old-epoch refusal now meets the before-expiry
timing gate, with a succeeding new-epoch control. Corrected ownership trial has
both denied-other-owner and successful-original-owner control, demonstrating
the live active A binding remained consumable after the refused attempt. The
mistaken two-create trial is retained, not counted as a pass. Observed DELETE405
rows remain downstream MCP refusals, not credential failures.

User confirmed stopping the fresh server/tunnel and disabling the connector.
No endpoint is claimed active. These finite synthetic proofs do not establish
per-chat isolation, transactional production authority, durable credentials,
live effect-spanning revoke, universal host behavior or H-03 completion. The
first checkpoint's unproven live epoch/ownership items are superseded only by
this fresh trial; all other limits remain. #188 stays Draft; no real project
exposed and no draft merged.
