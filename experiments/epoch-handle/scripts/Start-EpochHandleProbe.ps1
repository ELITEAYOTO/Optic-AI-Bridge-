param([ValidateRange(1024,65535)][int]$Port = 8003, [switch]$AllowRandomTunnelHost)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-EpochHandleProbe.ps1 first.' }
$PreviousPort = $env:OPTIC_EPOCH_HANDLE_PORT
$PreviousTunnel = $env:OPTIC_EPOCH_HANDLE_TUNNEL
try {
    $env:OPTIC_EPOCH_HANDLE_PORT = "$Port"
    $env:OPTIC_EPOCH_HANDLE_TUNNEL = if ($AllowRandomTunnelHost) { '1' } else { '0' }
    & $Python (Join-Path $Root 'bound_server.py')
    if ($LASTEXITCODE -ne 0) { throw 'Bound-handle server failed.' }
} finally {
    $env:OPTIC_EPOCH_HANDLE_PORT = $PreviousPort
    $env:OPTIC_EPOCH_HANDLE_TUNNEL = $PreviousTunnel
}
