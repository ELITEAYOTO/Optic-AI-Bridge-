# FranceStudent: measured findings vs admin statements

Admin input: #171 comment6129335914 (product statement, not raw wire capture).
Manual sources: user-supplied Luna outputs, local-console transcripts and screenshots;
see the separate live ledgers in experiments/francestudent-probe,
experiments/credential-lifecycle and experiments/epoch-handle. Raw secrets/tokens
are omitted; invocation markers correlate actual tool execution to server logs.

| Subject | Admin statement | Evidence / limit |
| --- | --- | --- |
| Authorization | Bearer wire shape | Exact dummy Bearer accepted. In tested UI, full prefix entry worked; no-prefix entry failed. Initial raw rejected syntax was not captured, so do not claim a universal verbatim-field contract. |
| Secure custom headers | Every request, hidden from model | Exact dummy delivery on enumerated recent setup/tool/denied POST/DELETE/GET. Original instance f347353f08ae requests8/9 lacked custom headers, source/method unknown. Universal delivery and model visibility not established. |
| Conversation identifier | Not supplied | Product input; current tests neither invent nor rely on chat-id. Explicit Optic handle handoff works. No claim of universal absence from every host field. |
| Global connector | Account-wide | Product input consistent with earlier separate #178 shared experimental context; not independently proven as per-chat isolation semantics by these probes. |
| Native confirmation | None | Earlier harmless calls showed no native confirmation. These guided runs do not prove absence for every tool/host configuration. Local approvals remain a separate gate. |
| Parallelism | Permitted, model decides | Current create->consume calls are sequential dependencies. No current real-host temporal-overlap proof; H-01 grouped experiment remains separate. |
| Roughly8 calls | Current limit, may change | Admin estimate, not measured or relied on; independent Optic admission limits retained. |
| Cancellation | Not supported | Admin statement, no cancellation experiment here. HTTP DELETE405 is not proof of call cancellation. |
| Reconnect | Unspecified | User toggle then new valid call succeeds; toggle does not restore revoked dummy auth. Toggle alone may produce no traffic. TCP identity/handle continuity across toggle not established live. |
| Host timeouts | Unknown | Not measured. TTL120 and lifetime25min are our own test controls, not host timeout facts. |

## Additional Optic compatibility/security findings

1. Modern local SDK and live host setup/discovery/tool path reach the guarded
   Streamable HTTP endpoint through a temporary tunnel. Provider identity grants
   no project authority and is not connection authentication.
2. Actual tool results are validated by independent instance/sequence/marker,
   not model narration. One earlier fabricated H-01 sample remains excluded.
3. Model-mediated explicit opaque handoff works without user token copying.
4. A consumed handle is subsequently refused. Exact replay arguments are user/
   model account because no raw argument capture is retained.
5. Delayed consume after309.563s is refused against TTL120; ingress timestamps are
   not exact mint times. Exact boundary is independently tested with fake clocks.
6. Local rotation invalidates old dummy credentials; new credential preserves
   principal but advances epoch. Native same-socket proof is automated, not live
   socket identification. Actual host POST/DELETE/GET attempts are refused.
7. Local revoke disables current dummy credential; connector reactivation does
   not restore it while the same server remains alive. Process restart of these
   initial experiments deliberately resets auth; durable revoke is still missing.
8. Independent B succeeds with a different principal and initial epoch despite A
   rotation/revoke. This is principal separation, not per-chat isolation.
9. Fresh b4f20cc47845 trial: old epoch1 binding refused under reported A-v2 at
   +49.453s, then fresh epoch2 handoff succeeds at+4.563s. Earlier aborted and
   expiry-confounded attempts remain recorded; 789s refusal is not epoch proof.
10. Corrected ownership trial: reported B refusal at+30.875s; restored A success
    on same requested token at+63.172s. All three invocation markers match;
    other-owner denial did not consume the active A binding under this config.
11. V2 authenticated DELETE405 rows are downstream MCP-app rejection. Revoked
    POST/DELETE/GET401 rows occur at dummy_bearer before SDK dispatch, with no tool.
12. Automated Windows/Linux proof additionally covers exact tool allowlists,
    malformed/duplicate/missing/wrong auth, forged/stale identities, monotonic TTL,
    one racing consume/rotation, atomic control vs handle operations, collision,
    retired-digest budget, active-state capacity, streamed body/header limits,
    timeout/cancel cleanup and absence of raw credentials/handles from server logs.

No authenticated test principal grants capabilities, leases, filesystem/Git/process
or mutation access. No session_cancel, production SessionResolver or real workspace
is exposed. All manual processes are stopped/connector disabled by user confirmation.
#186/#187/#188 stay Draft; #172/#173 and production security gates remain open.

## Next safe engineering gate

Local persistence of PUBLIC DUMMY auth metadata can test whether an acknowledged
rotate/revoke survives process restart, and whether corrupt/missing state fails
closed. It must use an explicit fixture directory, exclusive local writer lock,
bounded exact-schema data, no persisted raw secrets, local-only controls, atomic
commit before acknowledgement and no automatic initialization on missing state.
This is a process-crash/restart experiment, not production DPAPI/verifier storage,
power-loss durability, hostile filesystem containment or release authorization.
