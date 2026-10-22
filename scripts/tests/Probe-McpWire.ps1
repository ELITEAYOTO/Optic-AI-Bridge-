param([string]$Python, [string]$Fixture, [string]$Root)
# Import only the actual production start helper, never deployment/profile code.
$tokens = $null; $errors = $null
$doctor = Join-Path $PSScriptRoot '..\Test-OpticAIBridge.ps1'
$ast = [Management.Automation.Language.Parser]::ParseFile($doctor, [ref]$tokens, [ref]$errors)
$function = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Start-OpticMcpProcess' }, $false)
if ($null -eq $function) { throw 'Production MCP start helper missing' }
. ([scriptblock]::Create($function.Extent.Text))
$ErrorActionPreference = 'Stop'
'ready' | Set-Content (Join-Path $Root 'ready')
$clock = [Diagnostics.Stopwatch]::StartNew()
while (-not (Test-Path (Join-Path $Root 'go'))) {
    if ($clock.ElapsedMilliseconds -gt 8000) { throw 'Ownership gate missing' }
    Start-Sleep -Milliseconds 5
}
$results = @()
$originalConsole = [Console]::InputEncoding
# Deterministic regression input, confined to this owned worker and restored.
[Console]::InputEncoding = [Text.UTF8Encoding]::new($true)
try {
foreach ($fixed in @($false, $true)) {
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = $Python
    $psi.Arguments = '-I "' + $Fixture + '"'
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $p = New-Object Diagnostics.Process
    $p.StartInfo = $psi
    $previous = [Console]::InputEncoding
    try {
        if ($fixed) { $null = Start-OpticMcpProcess -Process $p }
        else { $null = $p.Start() }
        $restored = ([Console]::InputEncoding.CodePage -eq $previous.CodePage -and
            [Convert]::ToBase64String([Console]::InputEncoding.GetPreamble()) -ceq [Convert]::ToBase64String($previous.GetPreamble()))
        $unicode = ([char]0x00e9).ToString() + [char]0x96ea
        $payload = @{ probe = 1; unicode = $unicode } | ConvertTo-Json -Compress
        $p.StandardInput.WriteLine($payload)
        $p.StandardInput.Flush()
        $read = $p.StandardOutput.ReadLineAsync()
        if (-not $read.Wait(2000)) { throw 'Wire fixture deadline' }
        $value = $read.Result | ConvertFrom-Json
        if (-not $p.WaitForExit(2000)) { throw 'Wire fixture exit deadline' }
        $results += [pscustomobject]@{
            fixed = $fixed
            powershell = $PSVersionTable.PSVersion.ToString()
            edition = $PSVersionTable.PSEdition
            stdin_codepage = $p.StandardInput.Encoding.CodePage
            stdin_preamble_bytes = $p.StandardInput.Encoding.GetPreamble().Length
            console_restored = $restored
            prefix_hex = $value.prefix_hex
            valid_json = $value.valid_json
            unicode_roundtrip = ($value.echo -ceq $unicode)
        }
    } finally {
        if (-not $p.HasExited) { $p.Kill(); $null = $p.WaitForExit(1000) }
        $p.Dispose()
    }
}
$bad = New-Object Diagnostics.Process
try {
    $bad.StartInfo.FileName = Join-Path $Root 'known-absent-synthetic.exe'
    $bad.StartInfo.UseShellExecute = $false
    $bad.StartInfo.RedirectStandardInput = $true
    $bad.StartInfo.RedirectStandardOutput = $true
    $bad.StartInfo.RedirectStandardError = $true
    $previous = [Console]::InputEncoding
    $failed = $false
    try { $null = Start-OpticMcpProcess -Process $bad } catch { $failed = $true }
    $restored = ([Console]::InputEncoding.CodePage -eq $previous.CodePage -and
        [Convert]::ToBase64String([Console]::InputEncoding.GetPreamble()) -ceq [Convert]::ToBase64String($previous.GetPreamble()))
    if (-not $failed -or -not $restored) { throw 'Failed start did not restore encoding' }
    $results += [pscustomobject]@{ fixed = $true; failed_start = $true; console_restored = $restored; edition = $PSVersionTable.PSEdition }
} finally { $bad.Dispose() }
} finally { [Console]::InputEncoding = $originalConsole }
$results | ConvertTo-Json | Set-Content (Join-Path $Root 'result.json') -Encoding utf8
