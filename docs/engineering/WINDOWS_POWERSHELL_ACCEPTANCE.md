# Windows PowerShell doctor acceptance

Scope: hosted native Windows Server, Windows PowerShell 5.1 (Desktop/.NET
Framework) and current pwsh. This is a component/runtime acceptance slice, **not**
certification of all Windows10/11 installations, client profiles or installers.

## Why this check is new

The existing inherited-pipe harness used ProcessStartInfo.ArgumentList, which is
available in its pwsh runtime but not the .NET Framework runtime of Windows
PowerShell. It could not exercise the doctor under that runtime unchanged.
The test worker now retains ArgumentList on modern .NET and uses Windows CRT
argument quoting on the older runtime. Synthetic fixture paths include spaces.
The worker's warm compilation also runs after the outer ownership gate; this
covers .NET Framework compiler subprocesses too. A second fixture gate preserves
the existing 3s baseline/5s candidate supervisor budgets after compilation.
Only test harness/coverage changes; the production doctor is byte-for-byte unchanged.

The native workflow explicitly selects powershell.exe and checks the reported
version is 5.1.* with PSEdition=Desktop for all ten candidate cases. The two
original blocked controls remain independently supervised; no baseline result
is relabeled as a production timeout or AppContainer proof.

## Acceptance

- The existing twelve-case real-pipe proof must also pass on pwsh, independently.
- Windows PowerShell proof: normal MCP response, root/descendant stderr retention,
  short synthetic timeout, partial UTF-8, oversized output, repeated handles,
  diagnostic compilation failures and late approved markers. Every fixture tree
  is Job-owned before launch gate and confirmed empty; fixed output is bounded.
- Standard CI additionally runs the actual unchanged doctor with the freshly
  built Bridge and isolated Node under Windows PowerShell 5.1 in a synthetic
  workspace whose path contains spaces. No installer or ChatGPT profile setup.
- Keep the existing pwsh TEMP-only smoke, AppContainer network-denial gate,
  Ubuntu checks, release-source tests and all red attempts. No timeout extended,
  privilege changed, containment bypass or arbitrary process stop.

The native JSON reports exact source SHA separately from PR checkout SHA,
workflow/attempt, shell version/edition, case results and tree confirmation.
The small ZIP contains only sanitized proof, this contract and relevant sources.
Raw logs/environment/PIDs/private paths/binaries are excluded.
A passing functions-only fixture does not substitute for the real Bridge smoke.
The actual Bridge smoke still does not certify installation on user PCs.

## Known limits and open V1 risks

No production Rust/doctor change is proposed in this slice. #317 production
runner remains uncorrected: the native Rust owner proposal at d295d92e passed
eight cases but lacks stdin, retained registry/reservations and session/CAS gates.
[Checkpoint](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6332249104).
#319 natural first initialize30s and #185 Node/AppContainer10s remain distinct
intermittences; repeated passing CI does not prove their causes or resolution.
#176/#182 remain sensitive/provenance gates, especially before real remote use.

PR320 integrated release source verification at751bc72a3090b934ee8e20c860bc7293ea357c29.
Source b0c9b9fa had all four CI checks successful in run39719495246 attempt1.
Independent main CI39720011889 subsequently succeeded attempt1 on that exact
merge SHA. The initial actual API/schema proof is preserved there; ordinary CI
keeps the 26 deterministic fixtures rather than coupling every future candidate
to an old SHA's latest rerun or GitHub API availability. The real release job's
live fail-closed check is unchanged. See RELEASE_SOURCE_GATE.md.

## Reprise

Reverify main/PR/source checks and the native proof workflow. Require first
actual failure signature before a functional patch. Read both powerShell edition
fields and complete/cleanup fields, not only workflow badge. Native and normal CI
must pass on the final exact PR source; then verify independent main results.
Do not call this Windows10/11 certification. No PC tests or settings are authorized.
Next engineering priority remains Windows #317 ownership/stdin/lifecycle proof,
then #319/#185 diagnostics and the protected-content/recovery boundaries. Avoid
further redundant cache benchmarks, Linux confinement studies or feature work.
