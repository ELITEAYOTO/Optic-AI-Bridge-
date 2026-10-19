param([string]$PythonCommand = 'python', [switch]$RebuildEnvironment)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Venv = [IO.Path]::GetFullPath((Join-Path $Root '.venv'))
$Previous = [IO.Path]::GetFullPath((Join-Path $Root '.venv.previous'))
$Python = Join-Path $Venv 'Scripts\python.exe'

function Test-ExperimentPython([string]$Executable) {
    # An executable can remain after its base interpreter was uninstalled.
    # Probe in isolated mode; this does not install or change global Python.
    $ErrorActionPreference = 'Continue'
    $PSNativeCommandUseErrorActionPreference = $false
    try {
        $Resolved = @(Get-Command -Name $Executable -CommandType Application -ErrorAction Stop)
        if ($Resolved.Count -ne 1) { return $false }
        & $Resolved[0].Source -I -c "import sys; raise SystemExit(0 if sys.version_info >= (3,11) else 1)" 2>$null | Out-Null
        return $LASTEXITCODE -eq 0
    }
    catch { return $false }
}

function Assert-OrdinaryEnvironmentPath([string]$Path) {
    $Current = [IO.Path]::GetFullPath($Path)
    while ($Current) {
        if (Test-Path -LiteralPath $Current) {
            $Item = Get-Item -LiteralPath $Current -Force
            if (($Item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw 'Environment setup refuses a redirected path; use an ordinary local experiment folder.'
            }
        }
        $Parent = Split-Path -Parent $Current
        if ($Parent -eq $Current) { break }
        $Current = $Parent
    }
}

Assert-OrdinaryEnvironmentPath $Venv
if ($RebuildEnvironment -or -not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    if (-not (Test-ExperimentPython $PythonCommand)) { throw 'An installed Python 3.11 or newer is required; select it with -PythonCommand.' }
    if (Test-Path -LiteralPath $Venv) {
        if (-not $RebuildEnvironment) { throw 'The experiment environment is incomplete. Re-run setup with -RebuildEnvironment to preserve it and create a new one.' }
        Assert-OrdinaryEnvironmentPath $Previous
        $ExpectedParent = [IO.Path]::GetFullPath($Root)
        if ((Split-Path -Parent $Venv) -ne $ExpectedParent -or (Split-Path -Parent $Previous) -ne $ExpectedParent) { throw 'Environment backup escaped its experiment root.' }
        if (-not (Test-Path -LiteralPath $Venv -PathType Container)) { throw 'The environment path is not a directory.' }
        if (Test-Path -LiteralPath $Previous) { throw 'A previous environment backup already exists. Preserve or move it before requesting another rebuild; nothing was replaced.' }
        # Same-parent rename only, after absolute target and reparse checks.
        # No recursive deletion, cross-volume move or project/output cleanup.
        Move-Item -LiteralPath $Venv -Destination $Previous
        Write-Host 'Previous experiment environment preserved as .venv.previous.'
    }
    & $PythonCommand -I -m venv $Venv
    if ($LASTEXITCODE -ne 0) { throw 'Could not create isolated environment.' }
}
Assert-OrdinaryEnvironmentPath $Python
if (-not (Test-ExperimentPython $Python)) { throw 'The experiment environment cannot start; its base Python may have been removed. Re-run setup with -RebuildEnvironment and -PythonCommand pointing to an installed Python. Project outputs are preserved.' }
$HostProbe = Join-Path (Split-Path -Parent $Root) 'host-probe'
& $Python -I -m pip install -r (Join-Path $HostProbe 'requirements.txt') -c (Join-Path $HostProbe 'constraints.txt')
if ($LASTEXITCODE -ne 0) { throw 'Frozen dependency installation failed.' }
& $Python -I -m pip check
if ($LASTEXITCODE -ne 0) { throw 'Dependency check failed.' }
Write-Host 'SETUP PASS - isolated dummy-only probe is ready.'
