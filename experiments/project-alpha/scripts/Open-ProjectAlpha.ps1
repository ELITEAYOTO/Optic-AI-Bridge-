param(
    [Parameter(Mandatory=$true)][ValidatePattern('^alpha-[a-zA-Z0-9_-]{1,64}$')][string]$RunName,
    [string]$ElectronExecutable = (Join-Path $env:USERPROFILE 'node_modules\electron\dist\electron.exe')
)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Repository = Split-Path -Parent (Split-Path -Parent $Root)
$Output = Join-Path $Repository 'output'
$Run = Join-Path $Output $RunName
if (-not (Test-Path -LiteralPath (Join-Path $Run 'viewer-ready.json') -PathType Leaf)) { throw 'Stop the completed Node alpha first; viewer-ready.json is required.' }
$Manifest = Get-Content -Raw -LiteralPath (Join-Path $Repository 'ALPHA_MANIFEST.json') | ConvertFrom-Json
$Viewer = Join-Path $Root 'viewer\electron_main.cjs'
$Policy = Join-Path $Root 'viewer\viewer_policy.cjs'
foreach ($Entry in @(@{Path=$Viewer;Hash=$Manifest.viewer_main_sha256}, @{Path=$Policy;Hash=$Manifest.viewer_policy_sha256})) {
    if ($Entry.Hash -notmatch '^[0-9a-fA-F]{64}$' -or -not (Test-Path -LiteralPath $Entry.Path -PathType Leaf)) { throw 'Use the verified Electron viewer package.' }
    if ((Get-Item -LiteralPath $Entry.Path).Length -gt 32KB -or (Get-FileHash -LiteralPath $Entry.Path -Algorithm SHA256).Hash -ne $Entry.Hash) { throw 'Viewer hash mismatch.' }
}
$Executable = Get-Item -LiteralPath $ElectronExecutable
if ($Executable.PSIsContainer -or $Executable.Name -ne 'electron.exe') { throw 'Select the installed electron.exe explicitly.' }
$Info = New-Object System.Diagnostics.ProcessStartInfo
$Info.FileName = $Executable.FullName
$Info.UseShellExecute = $false
$Info.WorkingDirectory = $Root
$Info.Arguments = '"' + $Viewer + '" "' + $Run + '" "' + $Output + '"'
$Info.EnvironmentVariables.Remove('NODE_OPTIONS')
$Info.EnvironmentVariables.Remove('ELECTRON_RUN_AS_NODE')
$Process = [System.Diagnostics.Process]::Start($Info)
try {
    Write-Host 'Local operator Electron window. AI cannot select main/preload code or launch this viewer.'
    Write-Host ('Saved application data: ' + (Join-Path $Run 'electron-data'))
    Write-Host ('JSON exports (maximum 8 per opening, 64KB each): ' + (Join-Path $Run 'exports'))
    $Process.WaitForExit()
    if ($Process.ExitCode -ne 0) { throw 'Viewer refused startup or failed. Keep the sources and report the error.' }
} finally { $Process.Dispose() }
