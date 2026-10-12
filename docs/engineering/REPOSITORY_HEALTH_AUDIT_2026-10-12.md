# Repository health audit — 2026-10-12

## Audit boundary

Reviewed current main, draft PRs #169/#170/#178, issues #171–#183 and relevant discussions, workflow results/job summaries, documentation entry points/history, and selected production/experiment source seams. This is a repository-state and authority-boundary audit, not an exhaustive line-by-line security certification. No local Rust tests were rerun; CI evidence below was read from GitHub. No production Rust, workflow or experiment code is changed by this documentation pass.

## Exact baseline and CI

| Ref / scope | Audited SHA | Latest relevant verified result |
| --- | --- | --- |
| main | `156653a406f01383e2fc14cada513963453b04f3` | CI #602: success; Windows, Ubuntu, dependency policy job summaries inspected |
| #169 docs consolidation, before this pass | `8eeecd9ba0a16fa49980fba06be215a2c61a1b97` | CI #666: success; job/step summaries inspected |
| #170 H-01 probe | `d8ebc0b5d7f46ce61a9aad737bc64690b7ca584b` | H-01 #37 and CI #650: success |
| #178 Real Link | `ef5bc84144670135d13f55a59a782611aeae762f` | Real Link #11 and CI #653: success |

These greens belong to those SHAs. CI for later documentation commits must be reported separately. The recent cancelled #169 runs were superseded by a successful run; cancellation is not a failed security gate. CI defines `cancel-in-progress: true`. Green tests do not close the newly recorded #176/#180–#183 findings.

## Branch and review health

- #169: open Draft, base main, `docs/host-autonomy-roadmap-2026-10-10`; docs-only diff. This pass belongs here.
- #170: open Draft, base #169's branch, `h01/host-capability-probe`; authority-free Python probe/workflow/scripts, no production Rust changes.
- #178: open Draft, base #170's branch, `experiment/real-optic-mcp-link`; synthetic adapter/smoke/workflow/scripts, no production Rust changes.
- None is merged by this audit. A later integration must review the stack in order and rerun CI after any synchronization; this pass does not rewrite/rebase child branches.
- New H-track/security documentation is currently on #169, not main. A clean main CI is not evidence that those draft features are shipped.

## Issue disposition

| Issue | Current state | What is done / remaining |
| --- | --- | --- |
| #171 H-01 | Open | Probe implemented and automated green; real transport reachable. Fresh v2 new-chat/reconnect/overlap/header/auth captures pending. Today's Real Link evidence is separate. Outage is historical, not the present blocker. |
| #172 H-02 | Open | Requirements defined; FranceStudent now exposes bearer/secret + secure-header UI, but exact on-wire auth behavior and zero-cost auth/revoke/rotate proof are missing. |
| #173 H-03 | Open | Architecture/refactor seams identified; no production resolver. Server-correlated seq25/26 evidence across two user-reported chats on one live adapter reinforces the missing isolation. Per-session lease provisioning, principal binding, bootstrap rollback, expiry/revoke semantics remain. |
| #174 H-04 | Open | Review/autonomy mode invariants drafted; grants and lifecycle contract/implementation pending. |
| #175 H-05 | Open | Local approval contract proposed; pending ownership/bounds/replay/revalidation proof and implementation. |
| #176 read/list | Open | Classifier decision missing; canonical listing authorization and non-progressing cursor are concrete independent coding candidates. |
| #177 Real Link experiment | Open | Automated implementation and real FranceStudent read/denial/new-chat evidence exist in #178; completion/review remains separate from production readiness. Do not auto-close from a draft. |
| #178 | Open Draft PR | Synthetic bridge works; no production transport/auth/session resolver. |
| #179 dependency freeze | Closed | Full shared version constraints, pip check/freeze evidence. Version freezing is not a claim of hash-verified supply-chain integrity. |
| #180 Git-control mutation namespace | Open | Architectural authority gap documented; focused actual-runtime fixture proof and canonical protected-control policy/tests needed. No blanket exploit claim against every `.git` file. |
| #181 Unix native path round-trip | Open | Representation hazard documented; lossless conversion/error contract and alias/read/list/mutation regression proof needed. |
| #182 GitRead sensitivity | Open | Status/diff disclosure relationship needs explicit policy decision and tracked/staged/untracked tests. |
| #183 read revoke/quiescence | Open | Snapshot-only read/list/Git paths lack effect-spanning admission; deterministic revoke race and timeout/error cleanup proof needed. |

## Documentation consistency

Fixed by this pass: stale current-outage claims; older “no header UI visible” treated as current; missing real-host evidence promotion; confusion between #170 and #178; unsupported admission-order/overlap inference; missing current facts/unknowns and issue-by-issue priorities; previously incomplete seq-26 console provenance in the evidence ledger.

Preserved deliberately: all historical docs, `STATUS.md` and `docs/product/SCOPE_AND_ROADMAP.md`. These contain older baseline/current-next paragraphs (including a much older main SHA and completed-gate wording) alongside later entries. They are useful history but must not be the sole active-state source. The active front door is the current-state snapshot, project-health checkpoint and this audit. Future surgical synchronization should label dated entries and update only live summaries after reconciling merge/CI provenance; do not erase historical evidence.

Residual documentation work:
- reconcile `docs/architecture/MULTI_SESSION_CONCURRENCY.md`'s older Phase 3D / proposed Phase 4 summary with the later implementation ledger;
- trace each product readiness claim to an exact merged commit/test scope, including the selected Windows profile versus unadmitted toolchains;
- document the chosen sensitivity/Git/control-namespace contracts after design decisions;
- freeze authentication, per-session authority provisioning, lifetime/renewal, local approval and error compatibility contracts before their production implementation;
- retain a complete sanitized manual evidence bundle. The seq-26 host JSON and exact matching local console line are now retained in `FRANCESTUDENT_REAL_LINK_EVIDENCE_2026-10-12.md`; the exact local checkout/binary hash for that manual run is still missing.

## Source cross-checks and limits

At audited main:
- `crates/optic-bridge-core/src/read_scope.rs` explicitly defers classification; policy checks content scope, and app uses `workspace_all_non_sensitive()`. That name is not secret filtering.
- `crates/optic-bridge-core/src/path.rs` normalizes backslashes and accepts `.git` paths; native authority conversions require #181, repository-control exclusion requires #180.
- MCP `server.rs` holds one fixed `SessionHandle` and session-bound authority sets. Git read uses that session; production public session creation/renewal is absent.
- App initial TTL is 30 minutes, reap interval 5 seconds; lifecycle renewal existence is not production renewal UX.
- CI covers formatting, Clippy, locked tests, dependency policy and Windows isolation/Node/approval/Git/packaging smokes. These are selected tested guarantees, not universal sandbox or real-remote readiness.

At #178, `server.py` has one lifespan-owned upstream; proofs are minted after success. `client_smoke.py` gathers two calls and accepts any exception for traversal. Improve exact error assertions and overlap measurement before broadening these evidence claims; production Rust stays untouched in this pass.

## Next coding candidates (proposals, not changes in this audit)

1. #183 admission across read/list/Git effects, with deterministic revoke race and failure-path tests.
2. #176 canonical prepared directory listing and strict pagination progress; test prefix aliases, reparse behavior and response-budget exhaustion.
3. #181 one lossless native-path conversion boundary, with Unix literal-backslash alias/read/list/mutation regressions.
4. #180 protected Git-control namespace after canonical observation; test normal/worktree `.git` forms, Windows case behavior and ordinary `.gitignore`.
5. Joint #176/#182 sensitivity contract, then classifier/capability enforcement across filesystem and Git outputs.
6. Experiment-only exact traversal-code checks and overlap/failed-call evidence instrumentation.
7. H-02 proof harness once dummy auth/header transport is measured; H-03 contract with explicit opaque-handle fallback and transactional authority bundles; then H-04/H-05 and H-06.
8. Optional synthetic mutation loop only after auditing the real mutation service; keep disposable/no-Git/no-process/no caller workspace path and route create/update/delete through real Optic transactional mutation.

Items 1–4 and experiment proof hardening can progress locally without waiting for a host conversation-id answer. Production remote real-project use remains blocked by applicable security/auth/session/local-control gates. Full desktop UX comes after stable contracts.
