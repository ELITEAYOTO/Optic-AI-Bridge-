[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Workspace,

    [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'OpticAIBridge'),
    [string]$NodePath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Require-Leaf {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Label not found: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

function Require-Directory {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "$Label not found: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

$Workspace = Require-Directory -Path $Workspace -Label 'Workspace'
$bundleRoot = (Resolve-Path -LiteralPath $PSScriptRoot).Path
$bridge = Require-Leaf -Path (Join-Path $bundleRoot 'optic-bridge.exe') -Label 'Bridge binary'
$helper = Require-Leaf -Path (Join-Path $bundleRoot 'optic-bridge-isolation-launcher.exe') -Label 'Isolation launcher'
$installer = Require-Leaf -Path (Join-Path $bundleRoot 'Install-OpticAIBridge.ps1') -Label 'Installer'
$plugin = Require-Directory -Path (Join-Path $bundleRoot 'plugin') -Label 'Plugin template'

if ($NodePath) {
    if (-not [IO.Path]::IsPathRooted($NodePath)) {
        throw '-NodePath must be absolute when supplied.'
    }
    $NodePath = Require-Leaf -Path $NodePath -Label 'Node executable'
}
else {
    $nodeCommand = Get-Command node.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $nodeCommand) {
        throw 'node.exe was not found. Install Node.js or rerun with -NodePath <absolute-node.exe>.'
    }
    $NodePath = (Resolve-Path -LiteralPath $nodeCommand.Source).Path
}
if ([IO.Path]::GetFileName($NodePath) -ine 'node.exe') {
    throw "NodePath must resolve to node.exe: $NodePath"
}

$installerParams = @{
    Workspace = $Workspace
    BinaryPath = $bridge
    IsolationLauncherPath = $helper
    PluginTemplatePath = $plugin
    InstallRoot = $InstallRoot
    ReadOnly = $true
    EnableIsolatedNode = $true
    NodePath = $NodePath
}

Write-Host '[Optic C5G] Installing the exact CI-built isolated Node profile...' -ForegroundColor Cyan
& $installer @installerParams

$configPath = Join-Path $InstallRoot 'marketplace\plugins\optic-ai-bridge-local\.mcp.json'
$configPath = Require-Leaf -Path $configPath -Label 'Installed MCP config'
$config = Get-Content -Raw -LiteralPath $configPath | ConvertFrom-Json
$server = $config.mcpServers.optic
if (-not $server) { throw 'Installed MCP config has no optic server.' }

$args = @($server.args | ForEach-Object { [string]$_ })
$tools = @($server.enabled_tools | ForEach-Object { [string]$_ })
$expectedArgs = @(
    "--allow-executable=interpreter:$NodePath",
    "--allow-isolated-node=$NodePath"
)
foreach ($argument in $expectedArgs) {
    if ($argument -notin $args) {
        throw "Installed Node profile is missing exact startup argument: $argument"
    }
}
if ($args | Where-Object { $_ -like '--allow-process-read-file*' }) {
    throw 'Installed Node profile unexpectedly contains process workspace-read authority.'
}
foreach ($tool in @('process_start', 'process_read', 'process_result', 'process_stop')) {
    if ($tool -notin $tools) {
        throw "Installed Node profile is missing tool: $tool"
    }
}
if ($server.tools.process_start.approval_mode -ne 'prompt') {
    throw 'Installed process_start is not prompt-gated.'
}

$installedBridge = Require-Leaf -Path ([string]$server.command) -Label 'Installed bridge command'
$installedHelper = Require-Leaf -Path (Join-Path (Split-Path -Parent $installedBridge) 'optic-bridge-isolation-launcher.exe') -Label 'Installed isolation launcher'
$bridgeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installedBridge).Hash
$helperHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installedHelper).Hash

$desktopCodex = Join-Path $env:USERPROFILE '.codex\plugins\.plugin-appserver\codex.exe'
if (Test-Path -LiteralPath $desktopCodex -PathType Leaf) {
    $codex = (Resolve-Path -LiteralPath $desktopCodex).Path
}
else {
    $codexCommand = Get-Command codex.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $codexCommand) { throw 'ChatGPT Desktop/Codex plugin manager disappeared after installation.' }
    $codex = $codexCommand.Source
}
$pluginListText = (& $codex plugin list --json 2>&1) -join "`n"
if ($LASTEXITCODE -ne 0) { throw "Unable to verify registered ChatGPT plugin source.`n$pluginListText" }
$pluginList = $pluginListText | ConvertFrom-Json
$registered = @($pluginList.installed | Where-Object { $_.pluginId -eq 'optic-ai-bridge-local@optic-ai-bridge' })
if ($registered.Count -ne 1 -or -not $registered[0].enabled) {
    throw 'The expected optic-ai-bridge-local@optic-ai-bridge plugin is not uniquely installed and enabled.'
}
$expectedPluginSource = (Resolve-Path -LiteralPath (Join-Path $InstallRoot 'marketplace\plugins\optic-ai-bridge-local')).Path
$registeredPluginSource = [IO.Path]::GetFullPath([string]$registered[0].source.path).TrimEnd('\','/')
if ($registeredPluginSource -ine $expectedPluginSource.TrimEnd('\','/')) {
    throw "ChatGPT Desktop active plugin source mismatch. expected=$expectedPluginSource observed=$registeredPluginSource"
}

$prompt = @"
@Optic AI Bridge Use process_start to run exactly "$NodePath" with args ["--version"], network false and process_count 1. If ChatGPT asks for approval, ask me to approve only this exact Node version command. Then use process_result and process_read on stdout and report the Node version. Do not access workspace files, do not request network, and do not run any other command.
"@.Trim()

Write-Host ''
Write-Host 'C5G preparation succeeded.' -ForegroundColor Green
Write-Host 'Now fully close ChatGPT Desktop, reopen it, start a new normal Chat, then send:' -ForegroundColor Yellow
Write-Host ''
Write-Host $prompt -ForegroundColor White
Write-Host ''

[pscustomobject]@{
    Ok = $true
    Workspace = $Workspace
    InstallRoot = (Resolve-Path -LiteralPath $InstallRoot).Path
    ConfigPath = $configPath
    NodePath = $NodePath
    BridgeSha256 = $bridgeHash
    HelperSha256 = $helperHash
    ProcessStartApproval = [string]$server.tools.process_start.approval_mode
    ProcessWorkspaceReadGrantAbsent = $true
    RegisteredPluginSource = $registeredPluginSource
    Prompt = $prompt
}
