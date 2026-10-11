$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$HostProbeSetup = Join-Path $RepoRoot 'experiments\host-probe\scripts\Setup-HostProbe.ps1'
$Bridge = Join-Path $RepoRoot 'target\debug\optic-bridge.exe'

if (-not (Test-Path -LiteralPath $HostProbeSetup)) {
    throw "H-01 setup helper not found: $HostProbeSetup"
}

& $HostProbeSetup
if ($LASTEXITCODE -ne 0) {
    throw 'Failed to prepare the pinned H-01 Python environment.'
}

Push-Location $RepoRoot
try {
    & cargo build --locked -p optic-bridge-app --bin optic-bridge
    if ($LASTEXITCODE -ne 0) {
        throw 'Failed to build the real optic-bridge binary.'
    }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $Bridge -PathType Leaf)) {
    throw "Built optic-bridge binary not found: $Bridge"
}

Write-Host 'Real Optic MCP link experiment is ready.'
Write-Host "Binary: $Bridge"
Write-Host 'Next: .\Start-RealOpticLink.ps1'
