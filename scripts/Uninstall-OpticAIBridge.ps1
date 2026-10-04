[CmdletBinding()]
param(
    [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'OpticAIBridge')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-CodexCommand {
    $desktopCodex = Join-Path $env:USERPROFILE '.codex\plugins\.plugin-appserver\codex.exe'
    if (Test-Path -LiteralPath $desktopCodex -PathType Leaf) {
        return (Resolve-Path -LiteralPath $desktopCodex).Path
    }
    $command = Get-Command codex -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { return $command.Source }
    return $null
}

$codex = Get-CodexCommand
if ($codex) {
    & $codex plugin remove 'optic-ai-bridge-local@optic-ai-bridge' --json 2>$null | Out-Null
    & $codex plugin marketplace remove 'optic-ai-bridge' 2>$null | Out-Null
}

if (Test-Path -LiteralPath $InstallRoot) {
    Remove-Item -LiteralPath $InstallRoot -Recurse -Force
}

Write-Host 'Optic AI Bridge user installation removed.' -ForegroundColor Green
Write-Host 'Restart ChatGPT Desktop if it is currently open.'
