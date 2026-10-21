# Hosted synthetic test: imports only the timeout function; launches no child.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$doctor = Join-Path $PSScriptRoot '..\Test-OpticAIBridge.ps1'
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $doctor, [ref]$tokens, [ref]$parseErrors
)
if ($parseErrors.Count) { throw 'Doctor script has parse errors.' }
$definitions = @($ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst]
}, $true) | Where-Object { $_.Name -eq 'Throw-McpTimeout' })
if ($definitions.Count -ne 1) { throw 'Exactly one timeout function is required.' }
. ([scriptblock]::Create($definitions[0].Extent.Text))

# Capture only the function's fixed-label diagnostic host messages.
$script:timingLines = [System.Collections.Generic.List[string]]::new()
function Write-Host {
    param([string]$Object)
    $script:timingLines.Add($Object)
}
function Assert-TimedOut {
    param([Parameter(Mandatory=$true)][string]$InputText)
    $reader = [System.IO.StringReader]::new($InputText)
    try {
        $fake = [pscustomobject]@{
            HasExited = $true
            ExitCode = -1
            StandardError = $reader
        }
        $failure = $null
        try {
            Throw-McpTimeout -Process $fake -Id 1 -Timeout 30000
        }
        catch {
            $failure = $_.Exception.Message
        }
        if ($null -eq $failure -or $failure -notlike '*Timed out after 30000ms*') {
            throw 'Timeout gate was not preserved.'
        }
        return [pscustomobject]@{ Message = $failure; Remaining = $reader.ReadToEnd().Length }
    }
    finally { $reader.Dispose() }
}

$marker = 'OPTIC_PROCESS_IDENTITY_IO_TIMING read_ms=123 nonread_ms=4 total_ms=127 calls=13'
$noise = 'UNTRUSTED_SECRET=' + ('X' * 4600)
$sample = $noise + [Environment]::NewLine + $marker + [Environment]::NewLine
$result = Assert-TimedOut -InputText $sample
$expected = 'OPTIC_DOCTOR_TIMEOUT_IDENTITY_IO read_ms=123 nonread_ms=4 total_ms=127 calls=13'
if ($script:timingLines.Count -ne 1 -or $script:timingLines[0] -cne $expected) {
    throw 'The phase marker after 4096 characters was not preserved.'
}
if ($script:timingLines[0] -match 'UNTRUSTED|SECRET|XXXXX') {
    throw 'Untrusted stderr escaped fixed-label diagnostics.'
}
if ($result.Message -notmatch '\[truncated\]') {
    throw 'Existing 4096-character error preview cap was not preserved.'
}

$script:timingLines.Clear()
$invalid = @(
    'OPTIC_STARTUP phase=entry elapsed_ms=1 secret=raw',
    'OPTIC_STARTUP phase=unknown elapsed_ms=1',
    'OPTIC_STARTUP phase=entry elapsed_ms=-1'
) -join [Environment]::NewLine
$result = Assert-TimedOut -InputText $invalid
if ($script:timingLines.Count -ne 0) { throw 'Malformed/unknown timing data was published.' }

$script:timingLines.Clear()
$repeated = (('OPTIC_STARTUP phase=entry elapsed_ms=1' + [Environment]::NewLine) * 40)
$result = Assert-TimedOut -InputText $repeated
if ($script:timingLines.Count -ne 24) { throw 'The 24-marker output limit changed.' }

$script:timingLines.Clear()
$longInput = 'Z' * 40000
$result = Assert-TimedOut -InputText $longInput
if ($result.Remaining -ne (40000 - 32768)) {
    throw 'Timeout diagnostic reader consumed beyond its 32768-character ceiling.'
}
if ($script:timingLines.Count -ne 0) { throw 'Untrusted filler became a diagnostic marker.' }
Microsoft.PowerShell.Utility\Write-Host 'OPTIC_DOCTOR_TIMEOUT_SYNTHETIC bounded_read=32768 marker_beyond_4096=ok fixed_labels=ok max_lines=24'
