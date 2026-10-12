# Pre-implementation audit and authority-free follow-up — 2026-10-12

Live GitHub refs queried before implementation (not cached conversation state):

| Ref | Exact head | State / exact-head runs |
| --- | --- | --- |
| main | `156653a406f01383e2fc14cada513963453b04f3` | clone agrees; commit workflow API returned no runs (no inferred green) |
| #169 | `bb3ff5b5fccf25654b43b32674b93ec0a2966a29` | Open Draft, main base; CI #685 / 38285398127 success |
| #170 | `d8ebc0b5d7f46ce61a9aad737bc64690b7ca584b` | Open Draft, #169 branch base; H-01 #37 / 38176710196 and CI #650 / 38176710198 success |
| #178 | `ef5bc84144670135d13f55a59a782611aeae762f` | Open Draft, #170 base; Real Link #11 / 38177111139 and CI #653 / 38177111106 success |
| #184 | `1e7695f12d214c504f84870dfe1f861f5d7d6792` | Open Draft, main base; CI #680 / 38256421685 success |

#172/#173 remain open. Their bodies have historical wire-unknown wording;
later issue comments record the admin's exact Bearer and secure-header answers.
#171's admin supplement is product-contract evidence, not a server trace.
#169's body contains an older e5d4408 validation supplement; the actual latest head
is bb3ff5b, with independently successful #685. No stale-head success is reused.
#185 preserves an earlier same-SHA Windows intermittent startup timeout/retry;
the green retry did not establish root cause or authorize weaker security tests.

The new branch stacks on #170 rather than #178 or #184. This preserves the H-01
instrumentation and avoids importing Rust/session changes or real runtime access.
The focused diff is an independent three-tool dummy-only MCP server, tests,
PowerShell scripts, matrix CI and its guide. Production files are untouched.

Canonical experiment contract and facts/unknowns:
`../../experiments/francestudent-probe/README.md`.

Production gates #176 and #180–#183, and H-02–H-05, remain applicable. #184 is an
unmerged hardening candidate, not evidence that those gates are closed. No drafts
are merged, no real projects exposed, and no auth/SessionResolver completion is
claimed by a harmless model handoff test.

## Consolidated follow-up

The implementation advanced on focused Draft #186, retaining the same #170 base.
Exact implementation head `54ca313e3a88e8ce61800966c5818efff2de372b` has green
probe matrix #3 / 38292476806 and general CI #688 / 38292476838. That proves this
head, not any later documentation-only commit; latest-head CI belongs in the PR.

Two user-run manual sessions are now recorded in
`../../experiments/francestudent-probe/LIVE_EVIDENCE_2026-10-12.md`, with exact
instances/markers and explicit user-log/model provenance. The no-prefix Bearer
guide was corrected to measured full-prefix UI input. Model-mediated handoff,
requested replay refusal, new call after connector toggle and delayed refusal
beyond TTL are corroborated. V2 exposes safe method/stage diagnostics and proves
its DELETE405 rows occur after authentication; it does not explain historical
v1 missing-header requests or prove universal delivery/TCP reconnect.

The user confirmed both sessions stopped and connectors disabled. Live owner
change, credential lifecycle and production SessionResolver remain unproven;
automated owner/lifecycle/race proof is explicitly separate. No further question
blocks this documentation consolidation; none of these results authorize merge
or real-project access. #178 remains distinct from H-01 grouped instrumentation.
