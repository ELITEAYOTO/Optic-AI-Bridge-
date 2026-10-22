param([string]$Python, [string]$Fixture, [string]$Root)
$ErrorActionPreference = 'Stop'
'ready' | Set-Content (Join-Path $Root 'ready')
$clock = [Diagnostics.Stopwatch]::StartNew()
while (-not (Test-Path (Join-Path $Root 'go'))) {
    if ($clock.ElapsedMilliseconds -gt 8000) { throw 'Ownership gate missing' }
    Start-Sleep -Milliseconds 5
}
$results = @()
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
        if ($fixed) {
            if ($psi.PSObject.Properties.Name -contains 'StandardInputEncoding') {
                $psi.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
                $null = $p.Start()
            } else {
                try {
                    [Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
                    $null = $p.Start()
                } finally { [Console]::InputEncoding = $previous }
            }
        } else { $null = $p.Start() }
        $restored = ([Console]::InputEncoding.CodePage -eq $previous.CodePage -and
            [Convert]::ToBase64String([Console]::InputEncoding.GetPreamble()) -ceq [Convert]::ToBase64String($previous.GetPreamble()))
        $p.StandardInput.WriteLine('{"probe":1}')
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
        }
    } finally {
        if (-not $p.HasExited) { $p.Kill(); $null = $p.WaitForExit(1000) }
        $p.Dispose()
    }
}
$results | ConvertTo-Json | Set-Content (Join-Path $Root 'result.json') -Encoding utf8
