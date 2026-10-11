$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$Python = Join-Path $RepoRoot 'experiments\host-probe\.venv\Scripts\python.exe'
$Client = Join-Path $RepoRoot 'experiments\optic-mcp-link\client_smoke.py'

if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    throw 'Pinned Python environment not found. Run .\Setup-RealOpticLink.ps1 first.'
}
if (-not (Test-Path -LiteralPath $Client -PathType Leaf)) {
    throw "Real-link client smoke not found: $Client"
}

& $Python $Client 'http://127.0.0.1:8000/mcp'
if ($LASTEXITCODE -ne 0) {
    throw 'Real Optic MCP link smoke failed.'
}
