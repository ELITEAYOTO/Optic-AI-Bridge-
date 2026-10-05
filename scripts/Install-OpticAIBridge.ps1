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
    [switch]$EnableGitIntegration,
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
if ($ReadOnly -and $EnableGitIntegration) {
    throw '-EnableGitIntegration is a mutating Git capability and cannot be combined with -ReadOnly.'
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
                $existingMarker = Get-Content -Raw -LiteralPath $installMarkerPath | ConvertFrom-Json
                $markerRoot = Normalize-PathText ([string]$existingMarker.install_root)
                if ([string]$existingMarker.product -ne 'optic-ai-bridge' -or
                    [int]$existingMarker.schema_version -ne 1 -or
                    $markerRoot -ine $InstallRoot) {
                    throw 'marker mismatch'
                }
            }
            catch {
                throw 'InstallRoot contains an invalid Optic installation marker; refusing to overwrite it.'
            }
        }
        else {
            $legacyDefault = $InstallRoot -ieq $defaultInstallRoot -and (
                (Test-Path -LiteralPath (Join-Path $InstallRoot 'bin\optic-bridge.exe') -PathType Leaf) -or
                (Test-Path -LiteralPath (Join-Path $InstallRoot 'marketplace\plugins\optic-ai-bridge-local\.codex-plugin\plugin.json') -PathType Leaf)
            )
            if (-not $legacyDefault) {
                throw 'InstallRoot is non-empty and is not a recognized Optic AI Bridge installation root.'
            }
        }
    }
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

$codex = $null
if (-not $SkipPluginRegistration) {
    $codex = Get-CodexCommand
}
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
$gitIntegrationRoot = Join-Path $InstallRoot 'git-integration'
$gitBootstrapHooks = Join-Path $InstallRoot 'git-bootstrap-hooks'
$gitIntegrationRef = 'refs/optic/integration/chatgpt'
$marketplaceRoot = Join-Path $InstallRoot 'marketplace'
$pluginSource = Join-Path $marketplaceRoot 'plugins\optic-ai-bridge-local'
$pluginManifestDir = Join-Path $pluginSource '.codex-plugin'
$pluginAssetsDir = Join-Path $pluginSource 'assets'
$marketplaceManifestDir = Join-Path $marketplaceRoot '.agents\plugins'
$installedBridge = Join-Path $binDir 'optic-bridge.exe'

# Mark ownership before creating any integration bootstrap state/ref. If a later
# install step fails, the partial user-scoped root remains safely identifiable.
New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$installMarker = [ordered]@{
    product = 'optic-ai-bridge'
    schema_version = 1
    install_root = $InstallRoot
}
[IO.File]::WriteAllText(
    $installMarkerPath,
    ($installMarker | ConvertTo-Json -Compress),
    $utf8NoBom
)

if ($EnableGitIntegration) {
    if (-not $enableGit -or -not $gitPath) {
        throw '-EnableGitIntegration requires Git and a workspace that is exactly the Git repository root.'
    }

    $workspaceNormalized = Normalize-PathText $Workspace
    $integrationNormalized = Normalize-PathText $gitIntegrationRoot
    if ($integrationNormalized -ieq $workspaceNormalized -or
        $integrationNormalized.StartsWith($workspaceNormalized + '\', [StringComparison]::OrdinalIgnoreCase) -or
        $workspaceNormalized.StartsWith($integrationNormalized + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Git integration root must remain outside and non-overlapping with the configured workspace/repository.'
    }

    New-Item -ItemType Directory -Force -Path $gitBootstrapHooks | Out-Null
    if (Get-ChildItem -LiteralPath $gitBootstrapHooks -Force | Select-Object -First 1) {
        throw 'Git integration bootstrap hooks directory must be empty.'
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

        $gitBaseArgs = @('--no-pager', '--literal-pathspecs', '-c', "core.hooksPath=$gitBootstrapHooks", '-c', 'commit.gpgSign=false', '-C', $Workspace)
        $head = (& $gitPath @gitBaseArgs rev-parse --verify HEAD 2>$null)
        if ($LASTEXITCODE -ne 0 -or -not $head) {
            throw '-EnableGitIntegration requires a repository with an existing HEAD commit.'
        }
        $head = ([string]$head).Trim()
        if ($head.Length -ne 40 -and $head.Length -ne 64) {
            throw 'Git returned an unsupported HEAD object id length.'
        }

        & $gitPath @gitBaseArgs symbolic-ref -q $gitIntegrationRef 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            throw "Refusing symbolic Git integration ref $gitIntegrationRef."
        }

        & $gitPath @gitBaseArgs show-ref --verify --quiet $gitIntegrationRef
        if ($LASTEXITCODE -ne 0) {
            $zeroOld = '0' * $head.Length
            & $gitPath @gitBaseArgs update-ref --no-deref $gitIntegrationRef $head $zeroOld
            if ($LASTEXITCODE -ne 0) {
                throw "Failed to create operator-owned Git integration ref $gitIntegrationRef without overwriting concurrent state."
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

Write-Step 'Installing Optic AI Bridge in the current user profile...'
New-Item -ItemType Directory -Force -Path $binDir, $stateDir, $pluginManifestDir, $pluginAssetsDir, $marketplaceManifestDir | Out-Null
if ($EnableGitIntegration) {
    New-Item -ItemType Directory -Force -Path $gitIntegrationRoot | Out-Null
}
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

if ($EnableGitIntegration) {
    $mcpArgs.Add("--git-integration-executable=$gitPath")
    $mcpArgs.Add("--git-integration-root=$gitIntegrationRoot")
    $mcpArgs.Add("--git-integration-ref=$gitIntegrationRef")
    $mcpArgs.Add('--allow-git-integrate')
    $enabledTools.Add('git_integration_status')
    $enabledTools.Add('git_integrate')
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
if ($EnableGitIntegration) {
    $toolApprovals.git_integrate = [ordered]@{ approval_mode = 'prompt' }
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
    if ($EnableGitIntegration) {
        $doctorParams.GitIntegrationPath = $gitPath
        $doctorParams.GitIntegrationRoot = $gitIntegrationRoot
        $doctorParams.GitIntegrationRef = $gitIntegrationRef
        $doctorParams.EnableGitIntegrate = $true
    }
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
Write-Host "Git read  : $enableGit"
Write-Host "Git integrate: $EnableGitIntegration"
Write-Host "Mode      : $(if ($ReadOnly) { 'read-only' } elseif ($EnableGitIntegration) { 'read + Git + scratch mutations + explicit Git integration' } else { 'read + Git + scratch mutations' })"
Write-Host ''
Write-Host 'Final step: fully close and reopen ChatGPT Desktop, create a new normal Chat, then type:' -ForegroundColor Yellow
Write-Host '@Optic AI Bridge Inspect the current workspace without modifying anything.' -ForegroundColor White
