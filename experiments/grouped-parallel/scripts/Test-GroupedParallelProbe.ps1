$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-GroupedParallelProbe.ps1 first.' }
$PreviousPath = $env:PYTHONPATH
try {
    $env:PYTHONPATH = $Root
    & $Python -m unittest discover -s (Join-Path $Root 'tests') -v
    if ($LASTEXITCODE -ne 0) { throw 'Grouped unit proof failed.' }
    & $Python (Join-Path $Root 'smoke_groups.py')
    if ($LASTEXITCODE -ne 0) { throw 'Grouped MCP smoke failed.' }
} finally { $env:PYTHONPATH = $PreviousPath }
Write-Host 'GROUPED PARALLEL TEST PASS - local SDK proof only; host evidence pending.'
