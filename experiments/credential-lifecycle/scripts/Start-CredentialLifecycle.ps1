param([ValidateRange(1024,65535)][int]$Port = 8002, [switch]$AllowRandomTunnelHost)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-CredentialLifecycle.ps1 first.' }
$PreviousPort = $env:OPTIC_LIFECYCLE_PORT
$PreviousTunnel = $env:OPTIC_LIFECYCLE_TUNNEL
try {
    $env:OPTIC_LIFECYCLE_PORT = "$Port"
    $env:OPTIC_LIFECYCLE_TUNNEL = if ($AllowRandomTunnelHost) { '1' } else { '0' }
    & $Python (Join-Path $Root 'lifecycle_server.py')
    if ($LASTEXITCODE -ne 0) { throw 'Lifecycle server failed.' }
} finally {
    $env:OPTIC_LIFECYCLE_PORT = $PreviousPort
    $env:OPTIC_LIFECYCLE_TUNNEL = $PreviousTunnel
}
