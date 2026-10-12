param([string]$PythonCommand = 'python')
$ErrorActionPreference = 'Stop'
$Experiments = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
& (Join-Path $Experiments 'francestudent-probe\scripts\Setup-FranceStudentProbe.ps1') -PythonCommand $PythonCommand
if ($LASTEXITCODE -ne 0) { throw 'Setup failed.' }
Write-Host 'EPOCH HANDLE SETUP PASS - synthetic-only.'
