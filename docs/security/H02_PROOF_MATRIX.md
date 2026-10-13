# H-02 proof and remaining integration gates

[FranceStudent SDK review](FRANCESTUDENT_SDK_REVIEW_2026-10-13.md) records the
user-supplied client's exact source revision and API/backend distinctions.
Client chat IDs, cancel/OAuth routes and approval fields are new research leads,
not closure of external MCP host/session/cancellation/approval gates.

Reviewed2026-10-13 on the draft experiment stack. This is not a declaration of
merged/production authentication. Remote main was observed at156653a4 unchanged.
Owning issues: #171 host measurements, #172 auth, #173 sessions. No issue closes
from this matrix. The [storage contract](H02_LOCAL_CREDENTIAL_STORAGE_CONTRACT.md)
defines the proposed production decisions; experiments contain only public dummy
values and disposable state.

| Boundary | Measured evidence | Remaining limit/gate |
| --- | --- | --- |
| Wire authentication | #186 real FS Bearer/custom header observations, markers matched to server | Real high-entropy credential enrollment and protected verifier design; not MCP OAuth conformance |
| Real fixed-read host link | #205–207 Windows/Linux real-binary integration; [2026-10-14 FS measurement](FRANCESTUDENT_REAL_FIXED_READ_2026-10-14.md) correlates generated-file metadata/list/read, rotation/reconnect/revoke and local stop ack | Only one generated fixture binding; real project, native chat identity, local enrollment and production storage remain gated |
| Per-request lifecycle | #187 old credential401 after rotation, same principal/new epoch succeeds; revoke401; B works | Production admission binding, exact operator recovery policy |
| Handles | #188 exact handoff, epoch invalidation withinTTL, other-owner refusal then original owner consumption | No project authority; not a native host conversation ID |
| Persistence | #189 dummy rotate/revoke after native process restart, plus user orderly restarts | Not power-loss durability or secure production verifier storage |
| Rate/body bounds | #190/#191 bounded synthetic gate, controlled429 and release recovery | Not natural FS retry behavior, Retry-After obedience, per-principal fairness or denial-of-service immunity |
| Session bootstrap | #193 idempotent same-ticket session reuse, repeated probes and close denial withinTTL | Synthetic session only; real Optic binding, adapter ownership and local grant UI |
| Host concurrency | #195 local overlap positive/control negative; real FS sequential sample | No positive host temporal overlap, no contradiction of model-dependent parallel support |
| Windows native protection | #196 default-user API roundtrip, same-user fresh process, tamper and cleanup cases | No cross-user/machine test, real secret handling, ACL/recovery/rollback guarantee |
| Protected composition | #197 abrupt process exit after acknowledged commit and before replace | Valid old-state replay is accepted; pending/write/lock paths need dedicated native boundary |
| Decoded state/live drift | #198 exact schema and impossible state refusal; changed valid disk state preserved on stale commit | Pre-write check is not atomic compare-and-replace or external revoke notification |
| Protection purposes | #199 distinct fixed public labels, wrong-purpose/relabelled prefix refused natively | Earlier disposable blobs not migrated; labels are not secret keys or authority |
| Prepared protected read | #200 same observed native handle held through decode; write/rename/link/size negatives | Partial read proof, not full ancestor/hostile-writer/ACL/write-commit containment |

## Sequenced next work

1. Finish native storage policy: directory/file ownership and ACL contract,
   lock identity, pending creation and final replacement; retain failure/no-ACK
   behavior. Prove only the selected threat model, not universal same-user/admin
   malware resistance. No real credentials before review of that contract.
2. Decide verifier construction/enrollment, local show/copy/recovery and rollback
   policy. Explicitly distinguish integrity/confidentiality from freshness.
3. Bind authenticated principal/epoch to a real application-owned Optic session
   with bounded admission/revoke behavior. Keep session references independent
   of a tunnel URL/host-provided ID. Do not call it per-conversation isolation
   unless the adapter proves that mapping.
4. Prove local operator project/profile authorization independent of remote
   model consent. No generic MCP operation can widen policy/leases/authority.
5. Add bounded Rust HTTP adapter only after owning H-gates permit it. First real
   tool test: generated disposable project, restricted read surface, no secrets,
   Git, mutations or processes. Broaden effects separately with security tests.

Before real projects/autonomy, resolve #176 and #180–183 audit gates, including
sensitive filesystem/Git content, protected repository metadata and revocation
admission. Fixes in draft branches are not equivalent to merged closure.

## Validation reporting rule

Record exact SHA, native versus simulated versus host evidence and skips. A
rerun that passes leaves the initial unexplained failure on record; never remove
an assertion/increase timeout merely to declare green. Same-head checks do not
automatically apply to descendant commits. No generic percentage-complete or
calendar completion promise follows from these experiments.
