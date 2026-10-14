param([ValidateRange(1024,65535)][int]$Port = 8009, [switch]$AllowRandomTunnelHost, [string]$NodeExecutable, [ValidateRange(1,50)][int]$SessionMinutes = 50)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Experiments = Split-Path -Parent $Root
$Repository = Split-Path -Parent $Experiments
$Python = Join-Path $Experiments 'francestudent-probe\.venv\Scripts\python.exe'
$Binary = Join-Path $Repository 'packaging\project-alpha\optic-bridge.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-ProjectAlpha.ps1 first.' }
if (-not (Test-Path -LiteralPath $Binary)) { throw 'Use the matching verified project-alpha package.' }
$Manifest = Get-Content -Raw -LiteralPath (Join-Path $Repository 'ALPHA_MANIFEST.json') | ConvertFrom-Json
if ($Manifest.binary_sha256 -notmatch '^[0-9a-fA-F]{64}$' -or $Manifest.source_commit -notmatch '^[0-9a-f]{40}$') { throw 'Invalid alpha package manifest.' }
$BinaryStream = [IO.File]::OpenRead($Binary)
$BinaryHasher = $null
try {
    if ($BinaryStream.Length -le 0 -or $BinaryStream.Length -gt 64MB) { throw 'Alpha binary size bound.' }
    $BinaryHasher = [Security.Cryptography.SHA256]::Create()
    $ActualHash = [BitConverter]::ToString($BinaryHasher.ComputeHash($BinaryStream)).Replace('-', '')
} finally {
    if ($null -ne $BinaryHasher) { $BinaryHasher.Dispose() }
    $BinaryStream.Dispose()
}
if ($ActualHash -ne $Manifest.binary_sha256) { throw 'Alpha binary hash mismatch.' }
if ($NodeExecutable) {
    $NodeItem = Get-Item -LiteralPath $NodeExecutable -ErrorAction Stop
    if ($NodeItem.PSIsContainer -or $NodeItem.Name -ne 'node.exe') { throw 'Select an installed node.exe explicitly.' }
    $NodeExecutable = $NodeItem.FullName
    $Helper = Join-Path $Repository 'packaging\project-alpha\optic-bridge-isolation-launcher.exe'
    if ($Manifest.helper_sha256 -notmatch '^[0-9a-fA-F]{64}$' -or -not (Test-Path -LiteralPath $Helper -PathType Leaf)) { throw 'Matching verified isolation helper required for Node mode.' }
    $HelperStream = [IO.File]::OpenRead($Helper)
    $HelperHasher = $null
    try {
        if ($HelperStream.Length -le 0 -or $HelperStream.Length -gt 64MB) { throw 'Alpha helper size bound.' }
        $HelperHasher = [Security.Cryptography.SHA256]::Create()
        $HelperHash = [BitConverter]::ToString($HelperHasher.ComputeHash($HelperStream)).Replace('-', '')
    } finally {
        if ($null -ne $HelperHasher) { $HelperHasher.Dispose() }
        $HelperStream.Dispose()
    }
    if ($HelperHash -ne $Manifest.helper_sha256) { throw 'Alpha helper hash mismatch.' }
}
$Output = Join-Path $Repository 'output'
if (-not (Test-Path -LiteralPath $Output)) { New-Item -ItemType Directory -Path $Output | Out-Null }
$RunnerArguments = @((Join-Path $Root 'project_runner.py'), '--binary', $Binary, '--output-root', $Output, '--port', "$Port", '--lifetime', "$($SessionMinutes * 60)")
if ($AllowRandomTunnelHost) { $RunnerArguments += '--allow-random-tunnel-host' }
if ($NodeExecutable) { $RunnerArguments += @('--node', $NodeExecutable) }
& $Python @RunnerArguments
if ($LASTEXITCODE -ne 0) { throw 'Project alpha failed; inspect the retained generated run without retrying uncertain writes.' }
