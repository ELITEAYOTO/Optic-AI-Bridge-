# Windows PowerShell doctor acceptance and UTF-8 framing

Scope: hosted Windows Server, Windows PowerShell 5.1/Desktop and current pwsh.
This is not Windows10/11 installation/client/profile certification.

## Demonstrated failure and mechanism

Source766d4aa6818db61efdebc54c5d715a106aa8d159 failed its first real doctor
smoke in CI39720869986 attempt1 (Windows job119633850436): process-started5ms,
initialize-sent856ms, then EOF with ConnectionClosed("initialize request").
The preceding pwsh smoke on the same binary succeeded. Native functions-only
pipe cases passed on both runtimes; they did not inspect initialize stdin bytes.

Independent supervised wire probe39721750628 attempt1, source
e38e43cbfd6011bf464863360b7a108e1cfbb484, observed NATURALLY:
- Desktop5.1.26100.33438: stdin codepage65001, preamble3, prefix efbbbf, JSON rejected.
- Scoped no-BOM prototype: same codepage, preamble0, valid JSON, console restored.
- Core7.6.6: legacy and explicit no-BOM both valid JSON, preamble0.
All outer fixture Jobs confirmed empty. This establishes the managed stdin
framing defect, not the unrelated natural30s startup319 or AppContainer185.
The original Windows failure is retained; no same-SHA retry-to-green.

.NET Framework builds/flushed redirected stdin from Console.InputEncoding;
with UTF-8 this may emit a BOM during Process.Start, before the JSON request.
Primary source: https://github.com/microsoft/referencesource/blob/main/System/services/monitoring/system/diagnosticts/Process.cs

## Narrow correction and tradeoff

Start-OpticMcpProcess chooses explicit UTF-8 without BOM for redirected stdin,
stdout and stderr. Modern .NET uses StandardInputEncoding. Framework lacks that
property: scope Console.InputEncoding to no-BOM UTF-8 only around synchronous
Start and restore the exact previous encoding in finally, including start failure.
No persistent console/system/security setting is changed. This private CLI helper
assumes no concurrent Console.In consumer or concurrent encoding changes; it is
not a general thread-safe process factory. Target interactive Windows10/11 still
needs separate acceptance. Do not use protocol-parser BOM tolerance as a workaround.

Cleanup now tolerates HasExited throwing when native creation never happened,
preserving the primary start error. Production Rust, full identity/pinning, MCP
30s deadline, AppContainer, task permissions and resource ceilings are unchanged.

## Permanent native regressions

The byte-level test AST-imports the ACTUAL helper, not a test-only replacement.
It deliberately selects UTF-8 with BOM inside an independently owned worker,
then restores its initial encoding. Desktop legacy output must show efbbbf and
invalid JSON. Fixed input must be no-BOM UTF-8 with actual non-ASCII roundtrip;
failed native start must restore codepage and preamble. Core controls retained.

The existing twelve-case real-pipe proof runs independently on both shells;
the Desktop job requires all ten candidate cases to report5.1.* and Desktop.
ArgumentList remains on modern .NET; the test worker uses CRT quoting on Framework.
Fixture paths contain spaces. Warm C# compilation starts only AFTER outer Job
ownership (including any compiler subprocess); a second fixture gate preserves
the existing per-case budgets. Every tree is recovered and confirmed empty.

Standard CI also exercises the real rebuilt Bridge/isolated Node with the doctor
on Windows PowerShell, in addition to the pwsh TEMP-only smoke. Functions-only
success cannot substitute for this gate. No installer/profile/user-PC execution.
Validation requires exact final candidate and independent post-merge results.
The ZIP records sanitized byte/case evidence and sources; no raw logs/PIDs/env.

## Other campaign results and reprise

Release guard PR320 integrated at751bc72a3090b934ee8e20c860bc7293ea357c29.
Source CI39719495246 and independent main CI39720011889 succeeded attempt1.
Its one-time API proof is retained; ordinary CI now uses the26 deterministic
fixtures, avoiding a permanent dependency on an old SHA's reruns/API availability.
The actual release job's fail-closed live exact-source verification is unchanged.

#317 remains OPEN and uncorrected in production. Native Rust slice d295d92e
passed8 cases plus full CI; its proposed ADR0035/risk matrix are on isolated branch
bootstrap/issue-317-windows-rust-owner, not bulk-merged.
https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6332249104
Next: native stdin/OS failure proof, bounded Session/HostStartup owner registry,
stop-before-drain and seal/reap gates; CAS outcome stays separate.
#176/#182 sensitive/provenance, #319 startup and #185 AppContainer stay open.
No release, new feature, Linux experiment or real-project remote exposure.

Reverify current main/PR/source/attempt, read complete JSON and cleanup fields,
preserve the first red and distinguish the synthetic forced-BOM regression from
the naturally observed wire probe. Never claim all Windows targets supported.
