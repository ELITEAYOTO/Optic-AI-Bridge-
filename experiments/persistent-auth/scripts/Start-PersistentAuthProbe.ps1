param([ValidateRange(1024,65535)][int]$Port = 8004, [switch]$AllowRandomTunnelHost)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run setup and tests first.' }
$PreviousPort = $env:OPTIC_PERSISTENT_AUTH_PORT
$PreviousTunnel = $env:OPTIC_PERSISTENT_AUTH_TUNNEL
try {
    $env:OPTIC_PERSISTENT_AUTH_PORT = "$Port"
    $env:OPTIC_PERSISTENT_AUTH_TUNNEL = if ($AllowRandomTunnelHost) { '1' } else { '0' }
    & $Python (Join-Path $Root 'persistent_server.py')
    if ($LASTEXITCODE -ne 0) { throw 'Startup blocked; do not delete/reset state to bypass it.' }
} finally {
    $env:OPTIC_PERSISTENT_AUTH_PORT = $PreviousPort
    $env:OPTIC_PERSISTENT_AUTH_TUNNEL = $PreviousTunnel
}
