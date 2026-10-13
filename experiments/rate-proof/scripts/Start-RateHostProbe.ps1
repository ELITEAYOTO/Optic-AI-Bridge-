param([ValidateRange(1024,65535)][int]$Port = 8005, [switch]$AllowRandomTunnelHost)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-RateHostProbe.ps1 first.' }
$PreviousPort = $env:OPTIC_RATE_PORT
$PreviousTunnel = $env:OPTIC_RATE_TUNNEL
try {
    $env:OPTIC_RATE_PORT = "$Port"
    $env:OPTIC_RATE_TUNNEL = if ($AllowRandomTunnelHost) { '1' } else { '0' }
    & $Python (Join-Path $Root 'host_server.py')
    if ($LASTEXITCODE -ne 0) { throw 'Rate host probe failed.' }
} finally {
    $env:OPTIC_RATE_PORT = $PreviousPort
    $env:OPTIC_RATE_TUNNEL = $PreviousTunnel
}
