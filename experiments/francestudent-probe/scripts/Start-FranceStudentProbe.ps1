param([ValidateRange(1024,65535)][int]$Port = 8001, [switch]$AllowRandomTunnelHost)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path $Root '.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-FranceStudentProbe.ps1 first.' }
$PreviousPort = $env:OPTIC_FS_PROBE_PORT
$PreviousTunnel = $env:OPTIC_FS_PROBE_RANDOM_TUNNEL
try {
    $env:OPTIC_FS_PROBE_PORT = "$Port"
    $env:OPTIC_FS_PROBE_RANDOM_TUNNEL = if ($AllowRandomTunnelHost) { '1' } else { '0' }
    Write-Host 'No real tokens. No workspace. Stops after 25 minutes or Ctrl+C.'
    & $Python (Join-Path $Root 'server.py')
    if ($LASTEXITCODE -ne 0) { throw 'Probe server failed.' }
} finally {
    $env:OPTIC_FS_PROBE_PORT = $PreviousPort
    $env:OPTIC_FS_PROBE_RANDOM_TUNNEL = $PreviousTunnel
}
