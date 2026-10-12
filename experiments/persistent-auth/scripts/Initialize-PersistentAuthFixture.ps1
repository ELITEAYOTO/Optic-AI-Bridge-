$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path (Split-Path -Parent $Root) 'francestudent-probe\.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run setup and tests first.' }
& $Python (Join-Path $Root 'persistent_server.py') --initialize
if ($LASTEXITCODE -ne 0) { throw 'Fixture initialization refused; existing state is never overwritten.' }
