$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Venv = Join-Path $Root '.venv'

if (-not (Get-Command py -ErrorAction SilentlyContinue)) {
    throw 'Python launcher (py.exe) was not found. Install Python 3.10 or newer.'
}

if (-not (Test-Path (Join-Path $Venv 'Scripts\python.exe'))) {
    & py -3 -m venv $Venv
}

$Python = Join-Path $Venv 'Scripts\python.exe'
& $Python -m pip install --upgrade pip
& $Python -m pip install -r (Join-Path $Root 'requirements.txt')
Write-Host 'H-01 host probe is installed.'
