$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-FranceStudentProbe.ps1 first.' }
$PreviousPath = $env:PYTHONPATH
try {
    $Experiments = Split-Path -Parent $Root
    $env:PYTHONPATH = @($Root, (Join-Path $Experiments 'persistent-auth'), (Join-Path $Experiments 'credential-lifecycle')) -join [IO.Path]::PathSeparator
    & $Python -m unittest discover -s (Join-Path $Root 'tests') -v
    if ($LASTEXITCODE -ne 0) { throw 'Public dummy protection proof failed.' }
} finally { $env:PYTHONPATH = $PreviousPath }
Write-Host 'PUBLIC DUMMY PROTECTION PASS - native Windows fixture only; no server or real credentials.'
