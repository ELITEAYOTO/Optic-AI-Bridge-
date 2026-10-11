param(
    [switch]$AllowRandomTunnelHost
)

$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$Python = Join-Path $RepoRoot 'experiments\host-probe\.venv\Scripts\python.exe'
$Server = Join-Path $RepoRoot 'experiments\optic-mcp-link\server.py'
$Bridge = Join-Path $RepoRoot 'target\debug\optic-bridge.exe'

if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    throw 'Pinned Python environment not found. Run .\Setup-RealOpticLink.ps1 first.'
}
if (-not (Test-Path -LiteralPath $Server -PathType Leaf)) {
    throw "Real-link server not found: $Server"
}
if (-not (Test-Path -LiteralPath $Bridge -PathType Leaf)) {
    throw 'Real optic-bridge binary not found. Run .\Setup-RealOpticLink.ps1 first.'
}

$oldApp = $env:OPTIC_REAL_LINK_OPTIC_APP
$oldRandomHost = $env:OPTIC_REAL_LINK_ALLOW_RANDOM_TUNNEL_HOST
$oldHost = $env:OPTIC_REAL_LINK_HOST
$oldPort = $env:OPTIC_REAL_LINK_PORT

try {
    $env:OPTIC_REAL_LINK_OPTIC_APP = $Bridge
    $env:OPTIC_REAL_LINK_HOST = '127.0.0.1'
    $env:OPTIC_REAL_LINK_PORT = '8000'
    $env:OPTIC_REAL_LINK_ALLOW_RANDOM_TUNNEL_HOST = if ($AllowRandomTunnelHost) { '1' } else { '0' }

    Write-Host 'Starting synthetic real Optic MCP link experiment...'
    Write-Host 'Endpoint: http://127.0.0.1:8000/mcp'
    Write-Host 'Workspace: generated temporary fixture only'
    if ($AllowRandomTunnelHost) {
        Write-Warning 'Random tunnel host relaxation is enabled for the synthetic fixture experiment only.'
    }
    Write-Host 'Press Ctrl+C to stop.'

    & $Python $Server
}
finally {
    if ($null -eq $oldApp) { Remove-Item Env:OPTIC_REAL_LINK_OPTIC_APP -ErrorAction SilentlyContinue }
    else { $env:OPTIC_REAL_LINK_OPTIC_APP = $oldApp }

    if ($null -eq $oldRandomHost) { Remove-Item Env:OPTIC_REAL_LINK_ALLOW_RANDOM_TUNNEL_HOST -ErrorAction SilentlyContinue }
    else { $env:OPTIC_REAL_LINK_ALLOW_RANDOM_TUNNEL_HOST = $oldRandomHost }

    if ($null -eq $oldHost) { Remove-Item Env:OPTIC_REAL_LINK_HOST -ErrorAction SilentlyContinue }
    else { $env:OPTIC_REAL_LINK_HOST = $oldHost }

    if ($null -eq $oldPort) { Remove-Item Env:OPTIC_REAL_LINK_PORT -ErrorAction SilentlyContinue }
    else { $env:OPTIC_REAL_LINK_PORT = $oldPort }
}
