$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Experiments = Split-Path -Parent $Root
$Python = Join-Path $Experiments 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-CredentialLifecycle.ps1 first.' }
$PreviousPath = $env:PYTHONPATH
try {
    $env:PYTHONPATH = $Root
    & $Python -m unittest discover -s (Join-Path $Root 'tests') -v
    if ($LASTEXITCODE -ne 0) { throw 'Lifecycle unit tests failed.' }
    & $Python (Join-Path $Root 'smoke_lifecycle.py')
    if ($LASTEXITCODE -ne 0) { throw 'Lifecycle HTTP smoke failed.' }
} finally { $env:PYTHONPATH = $PreviousPath }
Write-Host 'LIFECYCLE TEST PASS - local-only proof.'
