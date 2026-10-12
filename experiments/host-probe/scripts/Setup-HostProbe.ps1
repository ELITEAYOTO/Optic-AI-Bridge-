$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Venv = Join-Path $Root '.venv'
$Requirements = Join-Path $Root 'requirements.txt'
$Constraints = Join-Path $Root 'constraints.txt'

if (-not (Get-Command py -ErrorAction SilentlyContinue)) {
    throw 'Python launcher (py.exe) was not found. Install Python 3.10 or newer.'
}

& py -3 -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)"
if ($LASTEXITCODE -ne 0) {
    throw 'The default Python 3 selected by py.exe is older than 3.10. Install/select Python 3.10 or newer.'
}

if (-not (Test-Path -LiteralPath $Requirements -PathType Leaf)) {
    throw "H-01 requirements file not found: $Requirements"
}
if (-not (Test-Path -LiteralPath $Constraints -PathType Leaf)) {
    throw "H-01 constraints file not found: $Constraints"
}

if (-not (Test-Path (Join-Path $Venv 'Scripts\python.exe'))) {
    & py -3 -m venv $Venv
    if ($LASTEXITCODE -ne 0) { throw 'Failed to create the H-01 virtual environment.' }
}

$Python = Join-Path $Venv 'Scripts\python.exe'
& $Python -m pip install -r $Requirements -c $Constraints
if ($LASTEXITCODE -ne 0) { throw 'Failed to install the frozen H-01 dependency graph.' }
& $Python -m pip check
if ($LASTEXITCODE -ne 0) { throw 'The installed H-01 dependency graph is inconsistent.' }
Write-Host 'H-01 host probe is installed with the frozen tested dependency graph.'
