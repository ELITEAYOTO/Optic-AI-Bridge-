# Native GitRead registered-lease witness — proposed contract (#317)

**Status: inert runtime prerequisite, not a production native process launcher. Windows V1 NO-GO.**

The `verify_registered_native_git_read_preflight` helper combines two controls which were previously only independent prerequisites on `main`:

1. `TaskLeaseRegistry::get_active_with_revocation_watch` returns the actual active, executable-identity-verified registry record and an original-generation revocation witness under the same lock.
2. `PolicyEngine::evaluate_native_git_read_preflight` checks the real typed ActionEnvelope/session/epoch, GitRead capability, exact canonical repository and FixedTool executable scope, nonzero resource ceiling and full monotonic watchdog horizon. Relative targets and legacy lease-less envelopes are rejected by this new helper.

The resulting `NativeGitReadLeaseWitness` is read-only and generation-bound, **never an authority to spawn**. Revocation may publish during/after preflight; `is_revoked() == false` does not grant permission to create a process. A future integration must first prove actual MCP session admission and protected Git metadata handling, opened/pinned exact Git executable, pre-created and registered native Job/owner with retained quotas, and perform a final policy/deadline/revocation/OS-identity check at the correct pre-spawn boundary. Failures and `TerminationUncertain` must retain ownership/capacity.

This PR deliberately does **not** change `authorize_git_read`, `GitReadService`, `HardenedCommandRunner`, stdio, session admission, the Windows owner or legacy GitRead permissions. It does not solve original #355 (real post-revocation Git spawn) or #352 (actual elapsed pipe completion). The contract has synthetic, non-executing fixture tests for legacy rejection, unknown registry lease, wrong root/session/epoch/budget, and revocation followed by same-ID reuse. It makes no claim about physical host-loss recovery, real EOF, AppContainer #185 or Windows 10/11 release acceptance #323.

Review before promotion: verify the exact PR HEAD Windows+Ubuntu CI, retain any initial failures; do not bulk-merge #353/#358 and do not close #317/#323. Independently verify push/main CI if merged.

## 25 October 2026 — subsequent admitted-session preflight slice

The *additional* `begin_admitted_native_git_read_preflight` binds the trusted
application-owned `SessionHandle` (never a client-supplied handle) to a **genuine
counted `OwnedSessionAdmissionPermit`**, then uses the previous registry+policy
preflight. It returns a non-cloneable `NativeGitReadAdmittedPreflight` retaining
both original-generation session and TaskLease stop conditions, including their
captured monotonic expirations. Failed preflight releases the session permit;
successful preflight must be retained by a future native Job guardian through
real terminal evidence. An unrelated or forged `ActionEnvelope.session` fails
before admission, and native-only lease-less requests still fail.

Tests deliberately create **no native process**. They exercise wrong/unknown
session, missing lease, exact grant, original expiration, revoke-before-removal,
session capacity held while a witness exists and same-ID TaskLease replacement.
This step does NOT connect the live MCP GitRead handler or guarantee race-free
pre-spawn authority. It does NOT satisfy either original behavioral acceptance
#352 or #355, nor Windows guardian loss, Job-empty, real EOF or V1 installation.
No legacy GitRead permission change, no unsafe additions, no release.

The admitted helper now takes a `NativeGitReadTrustedContext` grouping the server-owned session handle, registries and policy engine, rather than a long unstructured parameter list. The `ActionEnvelope` and the trusted session are still compared independently before any counted admission.
