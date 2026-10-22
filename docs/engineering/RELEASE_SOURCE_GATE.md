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
A read-only live API check uses historical known-green main c9446144 to verify
schema and API access, explicitly not this candidate's acceptance evidence.

CI on the candidate and independent post-merge CI are separate required checks.
No production release workflow is triggered for testing. Native release build/
signing/distribution and Windows10/11 installation acceptance remain unproved.
The guard trusts GitHub's repository/Actions metadata and does not replace PR
review, protected branches, reproducible builds, content security or V1 gates.
Release contents:write still spans its existing build job; finer publish-only
token scope is a future independent hardening option, not silently claimed here.

## Current stabilization checkpoint and next priority

#318 is squash-integrated at main c94461446a49ee61dc527a74f4e198afc8e55b4d;
main CI39716573989 and native doctor39716574009 succeeded attempt1.
The #317 Rust native slice at d295d92ec813612a3a919b56143ed23dcbb89485
passed CI39718651583 and supervised native39718651623 attempt1 (8 cases).
It is research only; proposed ADR0035 and the risk matrix live on branch
bootstrap/issue-317-windows-rust-owner. Production runner still has pipe-join risk.
See issue317 comment6332249104. No #317/#319 bulk merge or Linux experiment.

Next #317 work: prove native stdin cancellation/retention and OS failure paths;
then bounded service-owned Session/HostStartup registry, stop-before-drain,
seal/reap proof and a single GitRead migration. Keep CAS outcome separate.
#176/#182 sensitive/provenance, #319 startup and #185 AppContainer remain open.
No real-project remote exposure or feature unfreeze follows from this guard.
