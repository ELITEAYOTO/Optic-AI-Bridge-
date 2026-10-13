$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-FranceStudentProbe.ps1 first.' }
$PreviousPath = $env:PYTHONPATH
try {
    $env:PYTHONPATH = $Root
    & $Python -m unittest discover -s (Join-Path $Root 'tests') -v
    if ($LASTEXITCODE -ne 0) { throw 'Resolver proof failed.' }
    & $Python (Join-Path $Root 'smoke_bootstrap.py')
    if ($LASTEXITCODE -ne 0) { throw 'Bootstrap MCP proof failed.' }
} finally { $env:PYTHONPATH = $PreviousPath }
Write-Host 'SYNTHETIC RESOLVER PASS - local zero-authority contract only.'
