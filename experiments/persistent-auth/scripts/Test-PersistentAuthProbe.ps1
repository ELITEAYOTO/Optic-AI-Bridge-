$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-PersistentAuthProbe.ps1 first.' }
$PreviousPath = $env:PYTHONPATH
try {
    $env:PYTHONPATH = $Root
    & $Python -m unittest discover -s (Join-Path $Root 'tests') -v
    if ($LASTEXITCODE -ne 0) { throw 'Persistence tests failed.' }
    & $Python (Join-Path $Root 'smoke_persistent.py')
    if ($LASTEXITCODE -ne 0) { throw 'Restart MCP smoke failed.' }
} finally { $env:PYTHONPATH = $PreviousPath }
Write-Host 'PERSISTENT AUTH TEST PASS - disposable local fixture only.'
