param([ValidateRange(1024,65535)][int]$Port = 8007, [switch]$AllowRandomTunnelHost)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-GroupedParallelProbe.ps1 first.' }
$PreviousPort = $env:OPTIC_GROUPED_PORT
$PreviousTunnel = $env:OPTIC_GROUPED_TUNNEL
try {
    $env:OPTIC_GROUPED_PORT = "$Port"
    $env:OPTIC_GROUPED_TUNNEL = if ($AllowRandomTunnelHost) { '1' } else { '0' }
    & $Python (Join-Path $Root 'server.py')
    if ($LASTEXITCODE -ne 0) { throw 'Grouped probe failed.' }
} finally {
    $env:OPTIC_GROUPED_PORT = $PreviousPort
    $env:OPTIC_GROUPED_TUNNEL = $PreviousTunnel
}
