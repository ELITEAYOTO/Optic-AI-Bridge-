$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Venv = Join-Path $Root '.venv'

if (-not (Get-Command py -ErrorAction SilentlyContinue)) {
    throw 'Python launcher (py.exe) was not found. Install Python 3.10 or newer.'
}

& py -3 -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)"
if ($LASTEXITCODE -ne 0) {
    throw 'The default Python 3 selected by py.exe is older than 3.10. Install/select Python 3.10 or newer.'
}

if (-not (Test-Path (Join-Path $Venv 'Scripts\python.exe'))) {
    & py -3 -m venv $Venv
    if ($LASTEXITCODE -ne 0) { throw 'Failed to create the H-01 virtual environment.' }
}

$Python = Join-Path $Venv 'Scripts\python.exe'
& $Python -m pip install -r (Join-Path $Root 'requirements.txt')
if ($LASTEXITCODE -ne 0) { throw 'Failed to install the pinned H-01 dependencies.' }
Write-Host 'H-01 host probe is installed.'
