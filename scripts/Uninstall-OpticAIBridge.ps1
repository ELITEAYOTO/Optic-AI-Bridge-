[CmdletBinding()]
param(
    [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'OpticAIBridge'),
    [string]$Workspace,
    [switch]$RemoveGitIntegrationRef,
    [switch]$SkipPluginRegistration
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$gitIntegrationRef = 'refs/optic/integration/chatgpt'

function Get-CodexCommand {
    $desktopCodex = Join-Path $env:USERPROFILE '.codex\plugins\.plugin-appserver\codex.exe'
    if (Test-Path -LiteralPath $desktopCodex -PathType Leaf) {
        return (Resolve-Path -LiteralPath $desktopCodex).Path
    }
    $command = Get-Command codex -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { return $command.Source }
    return $null
}

function Normalize-PathText {
    param([Parameter(Mandatory = $true)][string]$Path)
    return ([IO.Path]::GetFullPath($Path)).TrimEnd('\', '/')
}

$defaultInstallRoot = Normalize-PathText (Join-Path $env:LOCALAPPDATA 'OpticAIBridge')
$installFullPath = [IO.Path]::GetFullPath($InstallRoot)
$installPathRoot = [IO.Path]::GetPathRoot($installFullPath)
if ($installPathRoot -and
    (Normalize-PathText $installPathRoot) -ieq (Normalize-PathText $installFullPath)) {
    throw 'InstallRoot must not be a filesystem root.'
}
$InstallRoot = Normalize-PathText $installFullPath
if (Test-Path -LiteralPath $InstallRoot -PathType Leaf) {
    throw 'InstallRoot must be a directory path, not a file.'
}
$installMarkerPath = Join-Path $InstallRoot '.optic-ai-bridge-install.json'
if (Test-Path -LiteralPath $InstallRoot -PathType Container) {
    $hasExistingEntries = $null -ne (Get-ChildItem -LiteralPath $InstallRoot -Force | Select-Object -First 1)
    if ($hasExistingEntries) {
        if (Test-Path -LiteralPath $installMarkerPath -PathType Leaf) {
            try {
                $marker = Get-Content -Raw -LiteralPath $installMarkerPath | ConvertFrom-Json
                $markerRoot = Normalize-PathText ([string]$marker.install_root)
                if ([string]$marker.product -ne 'optic-ai-bridge' -or
                    [int]$marker.schema_version -ne 1 -or
                    $markerRoot -ine $InstallRoot) {
                    throw 'marker mismatch'
                }
            }
            catch {
                throw 'InstallRoot contains an invalid Optic installation marker; refusing recursive removal.'
            }
        }
        else {
            $legacyDefault = $InstallRoot -ieq $defaultInstallRoot -and (
                (Test-Path -LiteralPath (Join-Path $InstallRoot 'bin\optic-bridge.exe') -PathType Leaf) -or
                (Test-Path -LiteralPath (Join-Path $InstallRoot 'marketplace\plugins\optic-ai-bridge-local\.codex-plugin\plugin.json') -PathType Leaf)
            )
            if (-not $legacyDefault) {
                throw 'InstallRoot is non-empty and is not a recognized Optic AI Bridge installation root; refusing recursive removal.'
            }
        }
    }
}

if ($RemoveGitIntegrationRef) {
    if (-not $Workspace) {
        throw '-RemoveGitIntegrationRef requires an explicit -Workspace repository path.'
    }

    $Workspace = (Resolve-Path -LiteralPath $Workspace).Path
    $gitCommand = Get-Command git.exe -ErrorAction Stop | Select-Object -First 1
    $git = (Resolve-Path -LiteralPath $gitCommand.Source).Path
    $gitRoot = (& $git -C $Workspace rev-parse --show-toplevel 2>$null)
    if ($LASTEXITCODE -ne 0 -or -not $gitRoot -or
        (Normalize-PathText $gitRoot) -ine (Normalize-PathText $Workspace)) {
        throw '-RemoveGitIntegrationRef requires Workspace to be exactly the Git repository root.'
    }

    $hooks = Join-Path $InstallRoot 'uninstall-hooks'
    New-Item -ItemType Directory -Force -Path $hooks | Out-Null
    if (Get-ChildItem -LiteralPath $hooks -Force | Select-Object -First 1) {
        throw 'Git uninstall hooks directory must be empty.'
    }

    $previousGitNoSystem = [Environment]::GetEnvironmentVariable('GIT_CONFIG_NOSYSTEM', 'Process')
    $previousGitGlobal = [Environment]::GetEnvironmentVariable('GIT_CONFIG_GLOBAL', 'Process')
    $previousGitPrompt = [Environment]::GetEnvironmentVariable('GIT_TERMINAL_PROMPT', 'Process')
    $previousGitReplace = [Environment]::GetEnvironmentVariable('GIT_NO_REPLACE_OBJECTS', 'Process')
    try {
        $env:GIT_CONFIG_NOSYSTEM = '1'
        $env:GIT_CONFIG_GLOBAL = 'NUL'
        $env:GIT_TERMINAL_PROMPT = '0'
        $env:GIT_NO_REPLACE_OBJECTS = '1'
        $gitBaseArgs = @('--no-pager', '--literal-pathspecs', '-c', "core.hooksPath=$hooks", '-C', $Workspace)

        & $git @gitBaseArgs symbolic-ref -q $gitIntegrationRef 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            throw "Refusing to remove symbolic Git integration ref $gitIntegrationRef."
        }

        $observed = (& $git @gitBaseArgs show-ref --verify --hash $gitIntegrationRef 2>$null)
        if ($LASTEXITCODE -eq 0 -and $observed) {
            $observed = ([string]$observed).Trim()
            if (($observed.Length -ne 40 -and $observed.Length -ne 64) -or
                $observed -notmatch '^[0-9a-fA-F]+$') {
                throw 'Git integration ref resolved to an invalid object id.'
            }

            & $git @gitBaseArgs update-ref --no-deref -d $gitIntegrationRef $observed
            if ($LASTEXITCODE -ne 0) {
                throw "Failed to remove $gitIntegrationRef without overwriting concurrent state."
            }
        }
    }
    finally {
        if ($null -eq $previousGitNoSystem) { Remove-Item Env:GIT_CONFIG_NOSYSTEM -ErrorAction SilentlyContinue } else { $env:GIT_CONFIG_NOSYSTEM = $previousGitNoSystem }
        if ($null -eq $previousGitGlobal) { Remove-Item Env:GIT_CONFIG_GLOBAL -ErrorAction SilentlyContinue } else { $env:GIT_CONFIG_GLOBAL = $previousGitGlobal }
        if ($null -eq $previousGitPrompt) { Remove-Item Env:GIT_TERMINAL_PROMPT -ErrorAction SilentlyContinue } else { $env:GIT_TERMINAL_PROMPT = $previousGitPrompt }
        if ($null -eq $previousGitReplace) { Remove-Item Env:GIT_NO_REPLACE_OBJECTS -ErrorAction SilentlyContinue } else { $env:GIT_NO_REPLACE_OBJECTS = $previousGitReplace }
    }
}

if (-not $SkipPluginRegistration) {
    $codex = Get-CodexCommand
    if ($codex) {
        & $codex plugin remove 'optic-ai-bridge-local@optic-ai-bridge' --json 2>$null | Out-Null
        & $codex plugin marketplace remove 'optic-ai-bridge' 2>$null | Out-Null
    }
}

if (Test-Path -LiteralPath $InstallRoot) {
    Remove-Item -LiteralPath $InstallRoot -Recurse -Force
}

Write-Host 'Optic AI Bridge user installation removed.' -ForegroundColor Green
if (-not $RemoveGitIntegrationRef) {
    Write-Host 'Repository Git integration refs are left untouched by default.' -ForegroundColor Yellow
    Write-Host 'If Git integration was enabled and you also want to remove its internal ref, rerun with:' -ForegroundColor Yellow
    Write-Host '.\Uninstall-OpticAIBridge.ps1 -Workspace "C:\path\to\repo" -RemoveGitIntegrationRef' -ForegroundColor Yellow
}
Write-Host 'Restart ChatGPT Desktop if it is currently open.'
