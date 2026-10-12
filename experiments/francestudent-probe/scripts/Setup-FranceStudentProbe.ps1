param([string]$PythonCommand = 'python')
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path $Root '.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $Python)) {
    & $PythonCommand -c "import sys; raise SystemExit(0 if sys.version_info >= (3,11) else 1)"
    if ($LASTEXITCODE -ne 0) { throw 'Python 3.11 or newer is required.' }
    & $PythonCommand -m venv (Join-Path $Root '.venv')
    if ($LASTEXITCODE -ne 0) { throw 'Could not create isolated environment.' }
}
$HostProbe = Join-Path (Split-Path -Parent $Root) 'host-probe'
& $Python -m pip install -r (Join-Path $HostProbe 'requirements.txt') -c (Join-Path $HostProbe 'constraints.txt')
if ($LASTEXITCODE -ne 0) { throw 'Frozen dependency installation failed.' }
& $Python -m pip check
if ($LASTEXITCODE -ne 0) { throw 'Dependency check failed.' }
Write-Host 'SETUP PASS - isolated dummy-only probe is ready.'
