[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BridgePath,

    [Parameter(Mandatory = $true)]
    [string]$IsolationLauncherPath,

    [Parameter(Mandatory = $true)]
    [string]$PluginTemplatePath,

    [Parameter(Mandatory = $true)]
    [string]$FixtureRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)][string]$Git,
        [Parameter(Mandatory = $true)][string]$Repository,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [switch]$Capture
    )

    $output = & $Git -C $Repository @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Git command failed (exit $LASTEXITCODE): git -C $Repository $($Arguments -join ' ')`n$($output -join "`n")"
    }
    if ($Capture) { return (($output -join "`n").Trim()) }
}

function Read-McpConfig {
    param([Parameter(Mandatory = $true)][string]$InstallRoot)
    $path = Join-Path $InstallRoot 'marketplace\plugins\optic-ai-bridge-local\.mcp.json'
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Generated MCP config not found: $path"
    }
    return (Get-Content -Raw -LiteralPath $path | ConvertFrom-Json)
}

$BridgePath = (Resolve-Path -LiteralPath $BridgePath).Path
$IsolationLauncherPath = (Resolve-Path -LiteralPath $IsolationLauncherPath).Path
$PluginTemplatePath = (Resolve-Path -LiteralPath $PluginTemplatePath).Path
$sourceIsolationHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $IsolationLauncherPath).Hash
New-Item -ItemType Directory -Force -Path $FixtureRoot | Out-Null
$FixtureRoot = (Resolve-Path -LiteralPath $FixtureRoot).Path
$installer = Join-Path $PSScriptRoot 'Install-OpticAIBridge.ps1'
$gitCommand = Get-Command git.exe -ErrorAction Stop | Select-Object -First 1
$git = (Resolve-Path -LiteralPath $gitCommand.Source).Path
$nodeCommand = Get-Command node.exe -ErrorAction Stop | Select-Object -First 1
$node = (Resolve-Path -LiteralPath $nodeCommand.Source).Path
$fixture = Join-Path $FixtureRoot ('installer-profiles-' + [Guid]::NewGuid().ToString('N'))
$repo = Join-Path $fixture 'repo'
$defaultInstall = Join-Path $fixture 'default-install'
$nodeInstall = Join-Path $fixture 'node-install'
$nodePathRejectedInstall = Join-Path $fixture 'node-path-rejected-install'
$optInInstall = Join-Path $fixture 'integration-install'
$rejectedInstall = Join-Path $fixture 'rejected-install'
$symbolicInstall = Join-Path $fixture 'symbolic-install'
$unsafeRoot = Join-Path $fixture 'unsafe-root'
$targetRef = 'refs/optic/integration/chatgpt'

try {
    New-Item -ItemType Directory -Force -Path $repo | Out-Null
    & $git init --quiet $repo
    if ($LASTEXITCODE -ne 0) { throw 'Failed to initialize installer fixture repository.' }
    Invoke-Git -Git $git -Repository $repo -Arguments @('config', 'user.email', 'optic-installer@example.invalid')
    Invoke-Git -Git $git -Repository $repo -Arguments @('config', 'user.name', 'Optic Installer Test')
    Set-Content -LiteralPath (Join-Path $repo 'tracked.txt') -Value 'alpha' -Encoding ascii
    Invoke-Git -Git $git -Repository $repo -Arguments @('add', 'tracked.txt')
    Invoke-Git -Git $git -Repository $repo -Arguments @('commit', '--quiet', '-m', 'initial')
    $head = Invoke-Git -Git $git -Repository $repo -Arguments @('rev-parse', '--verify', 'HEAD') -Capture

    # A non-empty custom path without an Optic marker must never become a recursively removable install root.
    New-Item -ItemType Directory -Force -Path $unsafeRoot | Out-Null
    $sentinel = Join-Path $unsafeRoot 'keep.txt'
    Set-Content -LiteralPath $sentinel -Value 'do-not-delete' -Encoding ascii
    $unsafeInstallRejected = $false
    try {
        & $installer `
            -Workspace $repo `
            -BinaryPath $BridgePath `
            -IsolationLauncherPath $IsolationLauncherPath `
            -PluginTemplatePath $PluginTemplatePath `
            -InstallRoot $unsafeRoot `
            -ReadOnly `
            -SkipDoctor `
            -SkipPluginRegistration
    }
    catch {
        $unsafeInstallRejected = $true
    }
    if (-not $unsafeInstallRejected -or -not (Test-Path -LiteralPath $sentinel -PathType Leaf)) {
        throw 'Installer did not fail closed on an unmarked non-empty custom InstallRoot.'
    }

    $unsafeUninstallRejected = $false
    try {
        & (Join-Path $PSScriptRoot 'Uninstall-OpticAIBridge.ps1') `
            -InstallRoot $unsafeRoot `
            -SkipPluginRegistration
    }
    catch {
        $unsafeUninstallRejected = $true
    }
    if (-not $unsafeUninstallRejected -or -not (Test-Path -LiteralPath $sentinel -PathType Leaf)) {
        throw 'Uninstaller did not protect an unmarked non-empty custom InstallRoot.'
    }

    # Default/read-only packaging must not silently grant GitIntegrate.
    & $installer `
        -Workspace $repo `
        -BinaryPath $BridgePath `
        -IsolationLauncherPath $IsolationLauncherPath `
        -PluginTemplatePath $PluginTemplatePath `
        -InstallRoot $defaultInstall `
        -ReadOnly `
        -SkipPluginRegistration

    $defaultInstalledLauncher = Join-Path $defaultInstall 'bin\optic-bridge-isolation-launcher.exe'
    if (-not (Test-Path -LiteralPath $defaultInstalledLauncher -PathType Leaf)) {
        throw 'Default installer profile did not install the AppContainer isolation helper.'
    }
    $defaultIsolationHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $defaultInstalledLauncher).Hash
    if ($defaultIsolationHash -ne $sourceIsolationHash) {
        throw 'Installed AppContainer isolation helper does not match the supplied helper bytes.'
    }

    $defaultConfig = Read-McpConfig -InstallRoot $defaultInstall
    $defaultServer = $defaultConfig.mcpServers.optic
    $defaultTools = @($defaultServer.enabled_tools | ForEach-Object { [string]$_ })
    $defaultArgs = @($defaultServer.args | ForEach-Object { [string]$_ })
    foreach ($integrationTool in @('git_integration_status', 'git_integrate')) {
        if ($integrationTool -in $defaultTools) {
            throw "Default installer profile unexpectedly enabled $integrationTool."
        }
    }
    if ($defaultArgs | Where-Object { $_ -like '--git-integration-*' -or $_ -eq '--allow-git-integrate' }) {
        throw 'Default installer profile unexpectedly emitted Git integration startup authority.'
    }
    if ($defaultArgs | Where-Object { $_ -like '--allow-isolated-node=*' -or $_ -like '--allow-executable=interpreter:*' }) {
        throw 'Default installer profile unexpectedly emitted isolated Node authority.'
    }
    foreach ($processTool in @('process_start', 'process_read', 'process_result', 'process_stop')) {
        if ($processTool -in $defaultTools) {
            throw "Default installer profile unexpectedly enabled $processTool."
        }
    }
    & $git -C $repo show-ref --verify --quiet $targetRef
    $defaultRefExists = $LASTEXITCODE -eq 0
    $global:LASTEXITCODE = 0
    if ($defaultRefExists) {
        throw 'Default installer profile unexpectedly created the integration ref.'
    }

    # NodePath alone must never imply authority; the dedicated switch is mandatory.
    $nodePathWithoutOptInRejected = $false
    try {
        & $installer `
            -Workspace $repo `
            -BinaryPath $BridgePath `
            -IsolationLauncherPath $IsolationLauncherPath `
            -PluginTemplatePath $PluginTemplatePath `
            -InstallRoot $nodePathRejectedInstall `
            -ReadOnly `
            -NodePath $node `
            -SkipDoctor `
            -SkipPluginRegistration
    }
    catch {
        $nodePathWithoutOptInRejected = $true
    }
    if (-not $nodePathWithoutOptInRejected) {
        throw 'Installer accepted -NodePath without explicit -EnableIsolatedNode.'
    }

    # Explicit Node opt-in auto-discovers the exact hosted node.exe, emits only
    # Node eligibility/process authority, exposes the process lifecycle, and the
    # doctor proves a real isolated Node --version start through the installed helper.
    & $installer `
        -Workspace $repo `
        -BinaryPath $BridgePath `
        -IsolationLauncherPath $IsolationLauncherPath `
        -PluginTemplatePath $PluginTemplatePath `
        -InstallRoot $nodeInstall `
        -ReadOnly `
        -EnableIsolatedNode `
        -SkipPluginRegistration

    $nodeConfig = Read-McpConfig -InstallRoot $nodeInstall
    $nodeServer = $nodeConfig.mcpServers.optic
    $nodeTools = @($nodeServer.enabled_tools | ForEach-Object { [string]$_ })
    $nodeArgs = @($nodeServer.args | ForEach-Object { [string]$_ })
    foreach ($argument in @(
        "--allow-executable=interpreter:$node",
        "--allow-isolated-node=$node"
    )) {
        if ($argument -notin $nodeArgs) { throw "Node installer profile is missing expected argument: $argument" }
    }
    if ($nodeArgs | Where-Object { $_ -like '--allow-process-read-file*' }) {
        throw 'Node installer profile silently widened workspace read authority.'
    }
    foreach ($processTool in @('process_start', 'process_read', 'process_result', 'process_stop')) {
        if ($processTool -notin $nodeTools) { throw "Node installer profile is missing expected tool: $processTool" }
    }
    if ($nodeServer.tools.process_start.approval_mode -ne 'prompt') {
        throw 'process_start must require prompt approval in the generated ChatGPT plugin config.'
    }

    & (Join-Path $PSScriptRoot 'Uninstall-OpticAIBridge.ps1') `
        -InstallRoot $nodeInstall `
        -SkipPluginRegistration
    if (Test-Path -LiteralPath $nodeInstall) {
        throw 'Node profile uninstall did not remove its installation root.'
    }

    # ReadOnly is intentionally incompatible with a mutating Git capability.
    $readOnlyRejected = $false
    try {
        & $installer `
            -Workspace $repo `
            -BinaryPath $BridgePath `
            -IsolationLauncherPath $IsolationLauncherPath `
            -PluginTemplatePath $PluginTemplatePath `
            -InstallRoot $rejectedInstall `
            -ReadOnly `
            -EnableGitIntegration `
            -SkipDoctor `
            -SkipPluginRegistration
    }
    catch {
        $readOnlyRejected = $true
    }
    if (-not $readOnlyRejected) {
        throw 'ReadOnly + EnableGitIntegration was not rejected.'
    }

    # A pre-existing symbolic internal ref must be rejected before any integration authority is emitted.
    $headRef = Invoke-Git -Git $git -Repository $repo -Arguments @('symbolic-ref', 'HEAD') -Capture
    Invoke-Git -Git $git -Repository $repo -Arguments @('symbolic-ref', $targetRef, $headRef)
    $symbolicRejected = $false
    try {
        & $installer `
            -Workspace $repo `
            -BinaryPath $BridgePath `
            -IsolationLauncherPath $IsolationLauncherPath `
            -PluginTemplatePath $PluginTemplatePath `
            -InstallRoot $symbolicInstall `
            -WritePrefix '' `
            -DeletePrefix '' `
            -EnableGitIntegration `
            -SkipDoctor `
            -SkipPluginRegistration
    }
    catch {
        $symbolicRejected = $true
    }
    if (-not $symbolicRejected) {
        throw 'Installer accepted a symbolic operator-owned Git integration ref.'
    }
    Invoke-Git -Git $git -Repository $repo -Arguments @('symbolic-ref', '--delete', $targetRef)

    # Explicit opt-in gets its own integration runtime/authority and prompt policy.
    & $installer `
        -Workspace $repo `
        -BinaryPath $BridgePath `
        -IsolationLauncherPath $IsolationLauncherPath `
        -PluginTemplatePath $PluginTemplatePath `
        -InstallRoot $optInInstall `
        -WritePrefix '' `
        -DeletePrefix '' `
        -EnableGitIntegration `
        -SkipPluginRegistration

    $optConfig = Read-McpConfig -InstallRoot $optInInstall
    $optServer = $optConfig.mcpServers.optic
    $optTools = @($optServer.enabled_tools | ForEach-Object { [string]$_ })
    $optArgs = @($optServer.args | ForEach-Object { [string]$_ })
    $integrationRoot = (Resolve-Path -LiteralPath (Join-Path $optInInstall 'git-integration')).Path

    foreach ($tool in @('git_status', 'git_diff', 'git_log', 'git_integration_status', 'git_integrate')) {
        if ($tool -notin $optTools) { throw "Opt-in installer profile is missing expected tool: $tool" }
    }
    foreach ($argument in @(
        "--git-executable=$git",
        "--git-integration-executable=$git",
        "--git-integration-root=$integrationRoot",
        "--git-integration-ref=$targetRef",
        '--allow-git-integrate'
    )) {
        if ($argument -notin $optArgs) { throw "Opt-in installer profile is missing expected argument: $argument" }
    }
    if ($optServer.tools.git_integrate.approval_mode -ne 'prompt') {
        throw 'git_integrate must require prompt approval in the generated ChatGPT plugin config.'
    }

    $observedRef = Invoke-Git -Git $git -Repository $repo -Arguments @('rev-parse', '--verify', $targetRef) -Capture
    if ($observedRef -ne $head) {
        throw "Installer initialized integration ref to unexpected head. expected=$head observed=$observedRef"
    }

    # Reinstall must preserve an existing direct integration ref rather than resetting it to current HEAD.
    Set-Content -LiteralPath (Join-Path $repo 'tracked.txt') -Value 'beta' -Encoding ascii
    Invoke-Git -Git $git -Repository $repo -Arguments @('add', 'tracked.txt')
    Invoke-Git -Git $git -Repository $repo -Arguments @('commit', '--quiet', '-m', 'second')
    $secondHead = Invoke-Git -Git $git -Repository $repo -Arguments @('rev-parse', '--verify', 'HEAD') -Capture
    Invoke-Git -Git $git -Repository $repo -Arguments @('update-ref', '--no-deref', $targetRef, $secondHead, $head)

    & $installer `
        -Workspace $repo `
        -BinaryPath $BridgePath `
        -IsolationLauncherPath $IsolationLauncherPath `
        -PluginTemplatePath $PluginTemplatePath `
        -InstallRoot $optInInstall `
        -WritePrefix '' `
        -DeletePrefix '' `
        -EnableGitIntegration `
        -SkipPluginRegistration

    $reinstalledLauncher = Join-Path $optInInstall 'bin\optic-bridge-isolation-launcher.exe'
    if (-not (Test-Path -LiteralPath $reinstalledLauncher -PathType Leaf)) {
        throw 'Reinstall did not preserve the installed AppContainer isolation helper.'
    }
    $reinstalledIsolationHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $reinstalledLauncher).Hash
    if ($reinstalledIsolationHash -ne $sourceIsolationHash) {
        throw 'Reinstalled AppContainer isolation helper does not match the supplied helper bytes.'
    }

    $preservedRef = Invoke-Git -Git $git -Repository $repo -Arguments @('rev-parse', '--verify', $targetRef) -Capture
    if ($preservedRef -ne $secondHead) {
        throw "Reinstall reset existing integration ref. expected=$secondHead observed=$preservedRef"
    }

    $bootstrapHooks = Join-Path $optInInstall 'git-bootstrap-hooks'
    if (-not (Test-Path -LiteralPath $bootstrapHooks -PathType Container)) {
        throw 'Installer did not create its controlled bootstrap-hooks directory.'
    }
    if (Get-ChildItem -LiteralPath $bootstrapHooks -Force | Select-Object -First 1) {
        throw 'Installer bootstrap-hooks directory is not empty.'
    }

    $uninstaller = Join-Path $PSScriptRoot 'Uninstall-OpticAIBridge.ps1'
    & $uninstaller `
        -InstallRoot $optInInstall `
        -Workspace $repo `
        -RemoveGitIntegrationRef `
        -SkipPluginRegistration

    if (Test-Path -LiteralPath $optInInstall) {
        throw 'Explicit uninstall did not remove its installation root.'
    }
    & $git -C $repo show-ref --verify --quiet $targetRef
    $remainingRefExists = $LASTEXITCODE -eq 0
    $global:LASTEXITCODE = 0
    if ($remainingRefExists) {
        throw 'Explicit uninstall did not remove the operator-owned Git integration ref.'
    }

    [pscustomobject]@{
        Ok = $true
        UnsafeCustomInstallRootRejected = $true
        UnsafeCustomUninstallRootProtected = $true
        DefaultGitIntegrationToolsAbsent = $true
        DefaultIsolatedNodeAuthorityAbsent = $true
        NodePathWithoutOptInRejected = $true
        OptInNodeProcessToolsPresent = $true
        OptInNodeProcessStartPromptApproval = $true
        OptInNodeDoctorPassed = $true
        OptInNodeDidNotGrantWorkspaceRead = $true
        ReadOnlyIntegrationRejected = $true
        SymbolicIntegrationRefRejected = $true
        OptInGitIntegrationStatusPresent = $true
        OptInGitIntegratePresent = $true
        PromptApprovalConfigured = $true
        IntegrationRefInitializedToHead = $true
        ExistingIntegrationRefPreserved = $true
        DoctorPassed = $true
        IsolationHelperInstalledByteExact = $true
        IsolationHelperReinstallPreserved = $true
        ExplicitUninstallRemovedIntegrationRef = $true
        ExplicitUninstallRemovedInstallRoot = $true
    }
}
finally {
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
