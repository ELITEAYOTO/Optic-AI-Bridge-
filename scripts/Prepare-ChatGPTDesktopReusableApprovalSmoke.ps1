[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Workspace,

    [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'OpticAIBridge-A08E'),
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

function Get-CodexCommand {
    $desktopCodex = Join-Path $env:USERPROFILE '.codex\plugins\.plugin-appserver\codex.exe'
    if (Test-Path -LiteralPath $desktopCodex -PathType Leaf) {
        return (Resolve-Path -LiteralPath $desktopCodex).Path
    }
    $command = Get-Command codex.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { return $command.Source }
    throw 'ChatGPT Desktop/Codex plugin manager was not found. Open an up-to-date ChatGPT Desktop once, then rerun.'
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

$Workspace = Require-Directory -Path $Workspace -Label 'Workspace'
$bundleRoot = (Resolve-Path -LiteralPath $PSScriptRoot).Path
$bridge = Require-Leaf -Path (Join-Path $bundleRoot 'optic-bridge.exe') -Label 'Bridge binary'
$helper = Require-Leaf -Path (Join-Path $bundleRoot 'optic-bridge-isolation-launcher.exe') -Label 'Isolation launcher'
$installer = Require-Leaf -Path (Join-Path $bundleRoot 'Install-OpticAIBridge.ps1') -Label 'Installer'
$preSmoke = Require-Leaf -Path (Join-Path $bundleRoot 'Test-ReusableApprovalMcp.ps1') -Label 'Reusable approval pre-smoke'
$plugin = Require-Directory -Path (Join-Path $bundleRoot 'plugin') -Label 'Plugin template'

if ($NodePath) {
    if (-not [IO.Path]::IsPathRooted($NodePath)) { throw '-NodePath must be absolute when supplied.' }
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
    SkipDoctor = $true
    SkipPluginRegistration = $true
}

Write-Host '[Optic A-08E] Installing the exact CI-built binaries into a separate validation root...' -ForegroundColor Cyan
& $installer @installerParams

$InstallRoot = Require-Directory -Path $InstallRoot -Label 'Installed validation root'
$validationDir = Join-Path $InstallRoot 'a08e-validation'
New-Item -ItemType Directory -Force -Path $validationDir | Out-Null
$profilePath = Join-Path $validationDir 'tool-profiles.json'
$configPath = Join-Path $validationDir 'optic-config.json'
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

$profileDocument = [ordered]@{
    version = 1
    profiles = @(
        [ordered]@{
            name = 'node-version'
            executable = $NodePath
            args = @('--version')
            env_allowlist = @()
            resources = [ordered]@{
                timeout_ms = 30000
                output_bytes = 1048576
                memory_bytes = 536870912
                process_count = 1
            }
            require_approval = $true
        },
        [ordered]@{
            name = 'node-help'
            executable = $NodePath
            args = @('--help')
            env_allowlist = @()
            resources = [ordered]@{
                timeout_ms = 30000
                output_bytes = 1048576
                memory_bytes = 536870912
                process_count = 1
            }
            require_approval = $true
        }
    )
}
[IO.File]::WriteAllText($profilePath, ($profileDocument | ConvertTo-Json -Depth 10), $utf8NoBom)
$profilePath = Require-Leaf -Path $profilePath -Label 'A-08E ToolProfile file'

$persistentConfig = [ordered]@{
    version = 1
    policy_epoch = 1
    workspace = $Workspace
    executables = @([ordered]@{ class = 'interpreter'; path = $NodePath })
    process_read_grants = @()
    isolated_node_executables = @($NodePath)
    tool_profile_file = $profilePath
    environment_grants = @()
    mutation = $null
    git = $null
}
[IO.File]::WriteAllText($configPath, ($persistentConfig | ConvertTo-Json -Depth 10), $utf8NoBom)
$configPath = Require-Leaf -Path $configPath -Label 'A-08E persistent config'

$configJson = Get-Content -Raw -LiteralPath $configPath | ConvertFrom-Json
if ([int64]$configJson.policy_epoch -ne 1) { throw 'A-08E config policy_epoch changed unexpectedly.' }
if ([string]$configJson.tool_profile_file -ine $profilePath) { throw 'A-08E config does not reference the exact ToolProfile file.' }
if (@($configJson.process_read_grants).Count -ne 0) { throw 'A-08E config unexpectedly grants process workspace read authority.' }
if (@($configJson.environment_grants).Count -ne 0) { throw 'A-08E config unexpectedly grants process environment authority.' }

$marketplaceRoot = Require-Directory -Path (Join-Path $InstallRoot 'marketplace') -Label 'A-08E marketplace root'
$mcpPath = Join-Path $marketplaceRoot 'plugins\optic-ai-bridge-local\.mcp.json'
$mcpPath = Require-Leaf -Path $mcpPath -Label 'Installed MCP config'
$mcp = Get-Content -Raw -LiteralPath $mcpPath | ConvertFrom-Json
$server = $mcp.mcpServers.optic
if (-not $server) { throw 'Installed MCP config has no optic server.' }
$installedBridge = Require-Leaf -Path ([string]$server.command) -Label 'Installed bridge command'
$installedHelper = Require-Leaf -Path (Join-Path (Split-Path -Parent $installedBridge) 'optic-bridge-isolation-launcher.exe') -Label 'Installed isolation launcher'

$server.args = @("--config=$configPath")
$server.enabled_tools = @(
    'process_start',
    'process_read',
    'process_result',
    'process_stop',
    'session_info',
    'session_cancel'
)
$server.default_tools_approval_mode = 'approve'
$server.tools = [pscustomobject][ordered]@{
    process_start = [ordered]@{ approval_mode = 'prompt' }
    session_cancel = [ordered]@{ approval_mode = 'prompt' }
}
[IO.File]::WriteAllText($mcpPath, ($mcp | ConvertTo-Json -Depth 12), $utf8NoBom)

$verified = Get-Content -Raw -LiteralPath $mcpPath | ConvertFrom-Json
$verifiedServer = $verified.mcpServers.optic
$args = @($verifiedServer.args | ForEach-Object { [string]$_ })
if ($args.Count -ne 1 -or $args[0] -ine "--config=$configPath") {
    throw "A-08E installed MCP args are not exclusive persistent-config mode: $($args -join ', ')"
}
$expectedTools = @('process_read','process_result','process_start','process_stop','session_cancel','session_info') | Sort-Object
$actualTools = @($verifiedServer.enabled_tools | ForEach-Object { [string]$_ } | Sort-Object)
if (($actualTools -join "`n") -cne ($expectedTools -join "`n")) {
    throw "A-08E host tool surface mismatch: $($actualTools -join ', ')"
}
if ([string]$verifiedServer.tools.process_start.approval_mode -ne 'prompt') {
    throw 'A-08E process_start is not independently host-prompt-gated.'
}
if ([string]$verifiedServer.tools.session_cancel.approval_mode -ne 'prompt') {
    throw 'A-08E session_cancel is not independently host-prompt-gated.'
}

Write-Host '[Optic A-08E] Running direct installed-binary reusable-approval proof before Desktop...' -ForegroundColor Cyan
$preSmokeRoot = Join-Path $validationDir 'direct-smoke'
$preSmokeResult = & $preSmoke -BridgePath $installedBridge -NodePath $NodePath -FixtureRoot $preSmokeRoot
if (-not $preSmokeResult.Ok) { throw 'A-08E direct reusable-approval pre-smoke failed.' }

$codex = Get-CodexCommand
Write-Host '[Optic A-08E] Temporarily switching the optic-ai-bridge marketplace to the A-08E validation root...' -ForegroundColor Cyan
Invoke-Codex -Command $codex -Arguments @('plugin','remove','optic-ai-bridge-local@optic-ai-bridge','--json') -AllowFailure | Out-Null
Invoke-Codex -Command $codex -Arguments @('plugin','marketplace','remove','optic-ai-bridge') -AllowFailure | Out-Null
Invoke-Codex -Command $codex -Arguments @('plugin','marketplace','add',$marketplaceRoot,'--json') | Out-Null
Invoke-Codex -Command $codex -Arguments @('plugin','add','optic-ai-bridge-local@optic-ai-bridge','--json') | Out-Null
$pluginListText = (Invoke-Codex -Command $codex -Arguments @('plugin','list','--json')) -join "`n"
$pluginList = $pluginListText | ConvertFrom-Json
$registered = @($pluginList.installed | Where-Object { $_.pluginId -eq 'optic-ai-bridge-local@optic-ai-bridge' })
if ($registered.Count -ne 1 -or -not $registered[0].enabled) {
    throw 'The expected A-08E Optic plugin is not uniquely installed and enabled.'
}
$expectedPluginSource = (Resolve-Path -LiteralPath (Join-Path $marketplaceRoot 'plugins\optic-ai-bridge-local')).Path
$registeredPluginSource = [IO.Path]::GetFullPath([string]$registered[0].source.path).TrimEnd('\','/')
if ($registeredPluginSource -ine $expectedPluginSource.TrimEnd('\','/')) {
    throw "ChatGPT Desktop active plugin source mismatch. expected=$expectedPluginSource observed=$registeredPluginSource"
}

$mcpList = (Invoke-Codex -Command $codex -Arguments @('mcp','list')) -join "`n"
if ($mcpList -notmatch '(?m)^optic\s') {
    throw "The A-08E Optic MCP server is not visible to ChatGPT Desktop.`n$mcpList"
}

$bridgeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installedBridge).Hash
$helperHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installedHelper).Hash
$profileHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $profilePath).Hash
$configHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $configPath).Hash

$prompt1 = @"
@Optic AI Bridge Use process_start to run exactly "$NodePath" with args ["--version"], cwd null, env_allowlist [], network false, timeout_ms 10000 and process_count 1. When Optic asks for approval scope, I will choose "Allow for current Optic session". Then use process_result and process_read on stdout and report the Node version. Do not run any other command.
"@.Trim()
$prompt2 = @"
@Optic AI Bridge Repeat exactly the same Node --version process_start contract as the previous request, with a new tool call. Report the Node version. Note whether Optic itself asks for approval scope again; a separate ChatGPT host-level confirmation does not count as an Optic elicitation.
"@.Trim()
$prompt3 = @"
@Optic AI Bridge Try process_start for exactly "$NodePath" with args ["--help"], cwd null, env_allowlist [], network false, timeout_ms 10000 and process_count 1. This is a different ToolProfile. Stop when Optic asks for approval scope so I can cancel it; do not substitute another command.
"@.Trim()
$prompt4 = @"
@Optic AI Bridge Try process_start for "$NodePath" with args ["--version","unexpected"], cwd null, env_allowlist [], network false, timeout_ms 10000 and process_count 1. This invocation is outside every configured ToolProfile and must fail before any Optic approval-scope elicitation.
"@.Trim()
$prompt5 = @"
@Optic AI Bridge Call session_info and report the current Optic session policy_epoch. Then call session_cancel once. After cancellation, try the exact Node --version process_start again. It must fail as an inactive session without reusing the previous profile approval. Do this revocation test last.
"@.Trim()

Write-Host ''
Write-Host 'A-08E preparation succeeded.' -ForegroundColor Green
Write-Host 'The exact installed binary/config passed the direct reusable-approval smoke.' -ForegroundColor Green
Write-Host 'Now fully close ChatGPT Desktop, reopen it, create ONE new normal Chat, and send these prompts in order.' -ForegroundColor Yellow
Write-Host 'Host-level ChatGPT confirmations are independent; record them separately from Optic elicitation.' -ForegroundColor Yellow
Write-Host 'This validation temporarily replaces the registered optic-ai-bridge marketplace. After the smoke, rerun the normal Optic installer to restore your usual profile.' -ForegroundColor Yellow
Write-Host ''
foreach ($entry in @(
    [pscustomobject]@{ Label='1 - mint current-session approval'; Text=$prompt1 },
    [pscustomobject]@{ Label='2 - same profile must reuse'; Text=$prompt2 },
    [pscustomobject]@{ Label='3 - different profile must prompt'; Text=$prompt3 },
    [pscustomobject]@{ Label='4 - out-of-profile must fail before prompt'; Text=$prompt4 },
    [pscustomobject]@{ Label='5 - revoke session last'; Text=$prompt5 }
)) {
    Write-Host "[$($entry.Label)]" -ForegroundColor Cyan
    Write-Host $entry.Text -ForegroundColor White
    Write-Host ''
}
Write-Host 'Policy-epoch note: this build has no live epoch-rotation API. Epoch mismatch remains proven by core/runtime tests; restarting with a new epoch would also clear in-memory grants and would not isolate that invariant.' -ForegroundColor DarkYellow

[pscustomobject]@{
    Ok = $true
    Workspace = $Workspace
    InstallRoot = $InstallRoot
    ConfigPath = $configPath
    ToolProfilePath = $profilePath
    NodePath = $NodePath
    BridgeSha256 = $bridgeHash
    HelperSha256 = $helperHash
    ToolProfileSha256 = $profileHash
    ConfigSha256 = $configHash
    RegisteredPluginSource = $registeredPluginSource
    DirectReusableApprovalSmoke = [bool]$preSmokeResult.Ok
    PolicyEpoch = 1
    LivePolicyEpochRotationAvailable = $false
    NormalMarketplaceRestoreRequired = $true
    Prompts = @($prompt1, $prompt2, $prompt3, $prompt4, $prompt5)
}
