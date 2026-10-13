$ErrorActionPreference = 'Stop'
$Experiments = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
& (Join-Path $Experiments 'francestudent-probe\scripts\Setup-FranceStudentProbe.ps1')
Write-Host 'GROUPED PARALLEL SETUP PASS - dummy-only, no persistent state.'
