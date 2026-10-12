$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path $Root '.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-FranceStudentProbe.ps1 first.' }
$PreviousPath = $env:PYTHONPATH
try {
    $env:PYTHONPATH = $Root
    & $Python -m unittest discover -s (Join-Path $Root 'tests') -v
    if ($LASTEXITCODE -ne 0) { throw 'Deterministic tests failed.' }
    & $Python (Join-Path $Root 'smoke.py')
    if ($LASTEXITCODE -ne 0) { throw 'Real local MCP smoke failed.' }
} finally { $env:PYTHONPATH = $PreviousPath }
Write-Host 'TEST PASS - local proof only; new host diagnostics still need manual evidence.'
