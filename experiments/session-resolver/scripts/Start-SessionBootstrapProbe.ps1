param([ValidateRange(1024,65535)][int]$Port = 8006, [switch]$AllowRandomTunnelHost)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-SessionBootstrapProbe.ps1 first.' }
$PreviousPort = $env:OPTIC_SESSION_PROBE_PORT
$PreviousTunnel = $env:OPTIC_SESSION_PROBE_TUNNEL
try {
    $env:OPTIC_SESSION_PROBE_PORT = "$Port"
    $env:OPTIC_SESSION_PROBE_TUNNEL = if ($AllowRandomTunnelHost) { '1' } else { '0' }
    & $Python (Join-Path $Root 'bootstrap_server.py')
    if ($LASTEXITCODE -ne 0) { throw 'Session bootstrap probe failed.' }
} finally {
    $env:OPTIC_SESSION_PROBE_PORT = $PreviousPort
    $env:OPTIC_SESSION_PROBE_TUNNEL = $PreviousTunnel
}
