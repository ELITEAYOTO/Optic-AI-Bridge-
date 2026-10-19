param([string]$PythonCommand = 'python', [switch]$RebuildEnvironment)
$ErrorActionPreference = 'Stop'
$Experiments = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
& (Join-Path $Experiments 'francestudent-probe\scripts\Setup-FranceStudentProbe.ps1') -PythonCommand $PythonCommand -RebuildEnvironment:$RebuildEnvironment
Write-Host 'PROJECT ALPHA SETUP PASS - generated project editing only.'
