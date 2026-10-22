# Windows release source gate

The tag-triggered release workflow builds/publishes a **prerelease**, not a V1
acceptance decision. Creating a tag or publishing an artifact still needs the
operator's release decision. No tag/release is created by the regression tests.

## Demonstrated configuration gap and correction

Before this guard, any matching v* tag with a matching VERSION could enter the
build/publication job; it did not require ancestry in main or a successful
independent main CI on the tag's exact commit. This is a release provenance gap,
not proof of an unauthorized attacker or a released vulnerable binary.

A read-only verify-source job now runs before release-windows. It requires:
- official repository, exact 40-hex source, constrained version and exact tag;
- GitHub compare source...main status ahead/identical and merge base=source;
- latest matching push/main .github/workflows/ci.yml run on the EXACT source;
- completed successful run, all current jobs successful, exactly one Windows,
  Ubuntu and dependency-policy job;
- complete job evidence (single page <=100); missing, pending, failed, skipped,
  ambiguous or unavailable evidence denies publication, with no retry-to-green.

The gate queries at most three API resources, each with a 30-second request
timeout, under a five-minute hosted job cap. It samples evidence at the time of
verification. A later workflow rerun may produce different evidence; the guard
does not claim future reproducibility. An older green run never substitutes for
a newer matching failure.

Both release checkouts explicitly select github.sha, so build and verification
consume the event commit rather than following a mutable tag name. Tag protection
and a concurrent operator tag move remain separate repository controls.

The verify job has contents:read/actions:read only; release-windows keeps the
existing contents:write needed for gh release. Ref/repository values enter
PowerShell as environment data, not interpolated script source. No branch
protection, release tag, runtime permissions, dependencies or security limits
are changed. This does not authorize publishing from a research branch.

## Validation and limits

26 offline fixtures exercise positive ancestry, source/tag/repository mismatch,
pending/red/wrong-branch/wrong-event/wrong-workflow CI, absent/duplicate/skipped/
failed jobs, latest-failure selection, API failure and incomplete pagination.
CI runs the same fixtures on hosted pwsh/Windows PowerShell and Ubuntu pwsh.
A one-time read-only live API proof was verified in PR320 and independent main
CI39720011889 against historical known-green main c9446144. It is not repeated
in every ordinary CI: future candidate acceptance must not depend on an old
SHA's latest rerun or live API availability. Deterministic API-boundary mocks
remain in the permanent 26-case suite. The ACTUAL release verify-source job
still requires live exact-source evidence and fails closed on API failure.

CI on the candidate and independent post-merge CI are separate required checks.
No production release workflow is triggered for testing. Native release build/
signing/distribution and Windows10/11 installation acceptance remain unproved.
The guard trusts GitHub's repository/Actions metadata and does not replace PR
review, protected branches, reproducible builds, content security or V1 gates.
Release contents:write still spans its existing build job; finer publish-only
token scope is a future independent hardening option, not silently claimed here.

## Current stabilization checkpoint and next priority

PR [#320](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/320) introduced this release guard and squash-integrated it at `751bc72a3090b934ee8e20c860bc7293ea357c29`; [CI #39720011889](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39720011889) passed. PR [#321](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/321) subsequently integrated at `main=65aa5db0dbaad24894f2cf4b050d5ac0bd61eac9`; independent [CI #39722724572](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724572), [pwsh #39722724496](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724496), [Desktop5.1 #39722724477](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39722724477) passed attempt 1. **No release or tag created**; this gate does not establish release readiness.

The #321 no-BOM stdin fix is distinct from unresolved #319 natural MCP initialization and #185 AppContainer startup. The #317 native Rust eight-case research head `d295d92ec813612a3a919b56143ed23dcbb89485` ([native #39718651623](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39718651623)) has NOT changed production `HardenedCommandRunner`: stdin cancellation, native failure/handles, service owner accounting, session/HostStartup stop-before-drain, seal/reap and CAS uncertainty are still pending. Sensitive/provenance #176/#182 gates remain open for real-project remote access. `main` branch protection and publish-job token lifetime need separately authorized security decisions. Always refresh live status before work.
