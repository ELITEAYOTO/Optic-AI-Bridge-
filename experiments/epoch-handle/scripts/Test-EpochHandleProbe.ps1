$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-EpochHandleProbe.ps1 first.' }
$PreviousPath = $env:PYTHONPATH
try {
    $env:PYTHONPATH = $Root
    & $Python -m unittest discover -s (Join-Path $Root 'tests') -v
    if ($LASTEXITCODE -ne 0) { throw 'Bound-handle tests failed.' }
    & $Python (Join-Path $Root 'smoke_bound.py')
    if ($LASTEXITCODE -ne 0) { throw 'Bound-handle MCP smoke failed.' }
} finally { $env:PYTHONPATH = $PreviousPath }
Write-Host 'EPOCH HANDLE TEST PASS - local synthetic proof only.'
