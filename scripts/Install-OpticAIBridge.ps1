[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Workspace,

    [string]$BinaryPath = (Join-Path $PSScriptRoot 'optic-bridge.exe'),
    [string]$PluginTemplatePath,
    [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'OpticAIBridge'),
    [string]$WritePrefix = 'scratch',
    [string]$DeletePrefix = 'scratch',
    [switch]$ReadOnly,
    [switch]$SkipDoctor,
    [switch]$SkipPluginRegistration
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Write-Step {
    param([string]$Message)
    Write-Host "[Optic] $Message" -ForegroundColor Cyan
}

function Get-CodexCommand {
    $desktopCodex = Join-Path $env:USERPROFILE '.codex\plugins\.plugin-appserver\codex.exe'
    if (Test-Path -LiteralPath $desktopCodex -PathType Leaf) {
        return (Resolve-Path -LiteralPath $desktopCodex).Path
    }

    $command = Get-Command codex -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { return $command.Source }

    throw 'ChatGPT Desktop/Codex plugin manager was not found. Install or update ChatGPT Desktop, open it once, then rerun this installer.'
}

function Invoke-Codex {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [switch]$AllowFailure
    )

    $output = & $Command @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0 -and -not $AllowFailure) {
        throw "Codex command failed (exit $exitCode): codex $($Arguments -join ' ')`n$($output -join "`n")"
    }
    return @($output)
}

function Normalize-PathText {
    param([Parameter(Mandatory = $true)][string]$Path)
    return ([IO.Path]::GetFullPath($Path)).TrimEnd('\', '/')
}

if ($PSVersionTable.PSEdition -eq 'Core' -and -not $IsWindows) {
    throw 'This installer currently supports Windows only.'
}

Write-Step 'Validating workspace and installation bundle...'
$Workspace = (Resolve-Path -LiteralPath $Workspace).Path
if (-not (Test-Path -LiteralPath $Workspace -PathType Container)) {
    throw "Workspace is not a directory: $Workspace"
}
$BinaryPath = (Resolve-Path -LiteralPath $BinaryPath).Path

if (-not $PluginTemplatePath) {
    $releasePlugin = Join-Path $PSScriptRoot 'plugin'
    $repoPlugin = Join-Path (Split-Path -Parent $PSScriptRoot) 'packaging\chatgpt-plugin'
    if (Test-Path -LiteralPath $releasePlugin -PathType Container) {
        $PluginTemplatePath = $releasePlugin
    }
    elseif (Test-Path -LiteralPath $repoPlugin -PathType Container) {
        $PluginTemplatePath = $repoPlugin
    }
    else {
        throw 'Plugin template not found. Use the official release bundle or provide -PluginTemplatePath.'
    }
}
$PluginTemplatePath = (Resolve-Path -LiteralPath $PluginTemplatePath).Path

$manifestTemplate = Join-Path $PluginTemplatePath '.codex-plugin\plugin.json'
$logoBase64 = Join-Path $PluginTemplatePath 'assets\optic-ai-bridge.png.b64'
if (-not (Test-Path -LiteralPath $manifestTemplate -PathType Leaf)) { throw "Missing plugin manifest: $manifestTemplate" }
if (-not (Test-Path -LiteralPath $logoBase64 -PathType Leaf)) { throw "Missing plugin logo payload: $logoBase64" }

$codex = Get-CodexCommand
$gitCommand = Get-Command git.exe -ErrorAction SilentlyContinue | Select-Object -First 1
$gitPath = $null
$enableGit = $false
if ($gitCommand) {
    $gitPath = (Resolve-Path -LiteralPath $gitCommand.Source).Path
    $gitRoot = (& $gitPath -C $Workspace rev-parse --show-toplevel 2>$null)
    if ($LASTEXITCODE -eq 0 -and $gitRoot) {
        $enableGit = (Normalize-PathText $gitRoot) -ieq (Normalize-PathText $Workspace)
    }
}

$binDir = Join-Path $InstallRoot 'bin'
$stateDir = Join-Path $InstallRoot 'state'
$marketplaceRoot = Join-Path $InstallRoot 'marketplace'
$pluginSource = Join-Path $marketplaceRoot 'plugins\optic-ai-bridge-local'
$pluginManifestDir = Join-Path $pluginSource '.codex-plugin'
$pluginAssetsDir = Join-Path $pluginSource 'assets'
$marketplaceManifestDir = Join-Path $marketplaceRoot '.agents\plugins'
$installedBridge = Join-Path $binDir 'optic-bridge.exe'

Write-Step 'Installing Optic AI Bridge in the current user profile...'
New-Item -ItemType Directory -Force -Path $binDir, $stateDir, $pluginManifestDir, $pluginAssetsDir, $marketplaceManifestDir | Out-Null
Copy-Item -LiteralPath $BinaryPath -Destination $installedBridge -Force
Copy-Item -LiteralPath $manifestTemplate -Destination (Join-Path $pluginManifestDir 'plugin.json') -Force
[IO.File]::WriteAllBytes(
    (Join-Path $pluginAssetsDir 'optic-ai-bridge.png'),
    [Convert]::FromBase64String((Get-Content -LiteralPath $logoBase64 -Raw).Trim())
)

if (-not $ReadOnly) {
    $scratch = Join-Path $Workspace $WritePrefix
    if ($WritePrefix -eq 'scratch') {
        New-Item -ItemType Directory -Force -Path $scratch | Out-Null
    }
}

$mcpArgs = New-Object System.Collections.Generic.List[string]
$enabledTools = New-Object System.Collections.Generic.List[string]
$enabledTools.Add('fs_list')
$enabledTools.Add('fs_read')

if ($enableGit) {
    $mcpArgs.Add("--git-executable=$gitPath")
    $enabledTools.Add('git_status')
    $enabledTools.Add('git_diff')
    $enabledTools.Add('git_log')
}

if (-not $ReadOnly) {
    $mcpArgs.Add("--mutation-state-dir=$stateDir")
    if ($WritePrefix) {
        $mcpArgs.Add("--allow-write-scope=prefix:$WritePrefix")
        $enabledTools.Add('fs_write')
        $enabledTools.Add('fs_apply_patch')
    }
    if ($DeletePrefix) {
        $mcpArgs.Add("--allow-delete-scope=prefix:$DeletePrefix")
        $enabledTools.Add('fs_delete')
    }
}
$mcpArgs.Add($Workspace)

$toolApprovals = [ordered]@{}
if (-not $ReadOnly -and $WritePrefix) {
    $toolApprovals.fs_write = [ordered]@{ approval_mode = 'prompt' }
    $toolApprovals.fs_apply_patch = [ordered]@{ approval_mode = 'prompt' }
}
if (-not $ReadOnly -and $DeletePrefix) {
    $toolApprovals.fs_delete = [ordered]@{ approval_mode = 'prompt' }
}

$mcpConfig = [ordered]@{
    mcpServers = [ordered]@{
        optic = [ordered]@{
            command = $installedBridge
            args = @($mcpArgs)
            enabled = $true
            startup_timeout_sec = 15
            tool_timeout_sec = 30
            default_tools_approval_mode = 'approve'
            enabled_tools = @($enabledTools)
            tools = $toolApprovals
        }
    }
}
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$mcpJson = $mcpConfig | ConvertTo-Json -Depth 12
[IO.File]::WriteAllText((Join-Path $pluginSource '.mcp.json'), $mcpJson, $utf8NoBom)

$marketplace = [ordered]@{
    name = 'optic-ai-bridge'
    interface = [ordered]@{ displayName = 'Optic AI Bridge Local' }
    plugins = @(
        [ordered]@{
            name = 'optic-ai-bridge-local'
            source = [ordered]@{ source = 'local'; path = './plugins/optic-ai-bridge-local' }
            policy = [ordered]@{ installation = 'AVAILABLE'; authentication = 'ON_INSTALL' }
            category = 'Productivity'
        }
    )
}
$marketplaceJson = $marketplace | ConvertTo-Json -Depth 8
[IO.File]::WriteAllText((Join-Path $marketplaceManifestDir 'marketplace.json'), $marketplaceJson, $utf8NoBom)

if (-not $SkipPluginRegistration) {
    Write-Step 'Registering the local plugin with ChatGPT Desktop...'
    $marketplacesJson = (Invoke-Codex -Command $codex -Arguments @('plugin','marketplace','list','--json')) -join "`n"
    $marketplaces = $marketplacesJson | ConvertFrom-Json
    $existingMarketplace = @($marketplaces.marketplaces | Where-Object { $_.name -eq 'optic-ai-bridge' })
    if ($existingMarketplace.Count -eq 0) {
        Invoke-Codex -Command $codex -Arguments @('plugin','marketplace','add',$marketplaceRoot,'--json') | Out-Null
    }

    Invoke-Codex -Command $codex -Arguments @('plugin','remove','optic-ai-bridge-local@optic-ai-bridge','--json') -AllowFailure | Out-Null
    Invoke-Codex -Command $codex -Arguments @('plugin','add','optic-ai-bridge-local@optic-ai-bridge','--json') | Out-Null
}
else {
    Write-Step 'Skipping ChatGPT plugin registration (validation mode).'
}

if (-not $SkipDoctor) {
    Write-Step 'Running local MCP self-test...'
    $doctorScript = Join-Path $PSScriptRoot 'Test-OpticAIBridge.ps1'
    if (-not (Test-Path -LiteralPath $doctorScript -PathType Leaf)) {
        throw "Doctor script missing from installation bundle: $doctorScript"
    }

    $doctorParams = @{
        BridgePath = $installedBridge
        Workspace = $Workspace
    }
    if ($enableGit) { $doctorParams.GitPath = $gitPath }
    if (-not $ReadOnly) {
        $doctorParams.StateDir = $stateDir
        $doctorParams.WritePrefix = $WritePrefix
        $doctorParams.DeletePrefix = $DeletePrefix
    }
    $doctor = & $doctorScript @doctorParams
    if (-not $doctor.Ok) { throw 'Optic MCP self-test failed.' }
}

if (-not $SkipPluginRegistration) {
    $pluginListJson = (Invoke-Codex -Command $codex -Arguments @('plugin','list','--json')) -join "`n"
    $pluginList = $pluginListJson | ConvertFrom-Json
    $installedPlugin = @($pluginList.installed | Where-Object { $_.pluginId -eq 'optic-ai-bridge-local@optic-ai-bridge' })
    if ($installedPlugin.Count -eq 0 -or -not $installedPlugin[0].enabled) {
        throw 'Plugin installation could not be verified in Codex/ChatGPT Desktop.'
    }

    $mcpList = (Invoke-Codex -Command $codex -Arguments @('mcp','list')) -join "`n"
    if ($mcpList -notmatch '(?m)^optic\s') {
        throw "The Optic MCP server is not visible to ChatGPT Desktop.`n$mcpList"
    }
}

Write-Host ''
Write-Host 'Optic AI Bridge is ready.' -ForegroundColor Green
Write-Host "Workspace : $Workspace"
Write-Host "Git tools : $enableGit"
Write-Host "Mode      : $(if ($ReadOnly) { 'read-only' } else { 'read + Git + scratch mutations' })"
Write-Host ''
Write-Host 'Final step: fully close and reopen ChatGPT Desktop, create a new normal Chat, then type:' -ForegroundColor Yellow
Write-Host '@Optic AI Bridge Inspect the current workspace without modifying anything.' -ForegroundColor White
