param(
    [int]$Port = 8000,
    [switch]$AllowRandomTunnelHost
)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path $Root '.venv\Scripts\python.exe'
if (-not (Test-Path $Python)) { throw 'Run Setup-HostProbe.ps1 first.' }
$env:OPTIC_HOST_PROBE_PORT = "$Port"
$env:OPTIC_HOST_PROBE_ALLOW_RANDOM_TUNNEL_HOST = if ($AllowRandomTunnelHost) { '1' } else { '0' }
& $Python (Join-Path $Root 'server.py')
