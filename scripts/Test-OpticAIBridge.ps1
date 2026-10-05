[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BridgePath,

    [Parameter(Mandatory = $true)]
    [string]$Workspace,

    [string]$GitPath,
    [string]$GitIntegrationPath,
    [string]$GitIntegrationRoot,
    [string]$GitIntegrationRef,
    [switch]$EnableGitIntegrate,
    [string]$StateDir,
    [string]$WritePrefix,
    [string]$DeletePrefix,
    [int]$TimeoutMs = 10000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Quote-ProcessArgument {
    param([Parameter(Mandatory = $true)][string]$Value)
    return '"' + ($Value -replace '"', '\"') + '"'
}

function Read-McpResponse {
    param(
        [Parameter(Mandatory = $true)]$Process,
        [Parameter(Mandatory = $true)][int]$Id,
        [Parameter(Mandatory = $true)][int]$Timeout
    )

    $deadline = [DateTime]::UtcNow.AddMilliseconds($Timeout)
    while ([DateTime]::UtcNow -lt $deadline) {
        $remaining = [Math]::Max(1, [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds)
        $task = $Process.StandardOutput.ReadLineAsync()
        if (-not $task.Wait($remaining)) {
            throw "Timed out waiting for MCP response id=$Id."
        }

        $line = $task.Result
        if ($null -eq $line) {
            $stderr = $Process.StandardError.ReadToEnd()
            throw "Optic AI Bridge closed before MCP response id=$Id. stderr: $stderr"
        }

        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $message = $line | ConvertFrom-Json
        if ($message.PSObject.Properties.Name -contains 'id' -and [int]$message.id -eq $Id) {
            return $message
        }
    }

    throw "Timed out waiting for MCP response id=$Id."
}

$BridgePath = (Resolve-Path -LiteralPath $BridgePath).Path
$Workspace = (Resolve-Path -LiteralPath $Workspace).Path

if ($GitPath) { $GitPath = (Resolve-Path -LiteralPath $GitPath).Path }
$integrationParts = @($GitIntegrationPath, $GitIntegrationRoot, $GitIntegrationRef | Where-Object { $_ }).Count
if ($integrationParts -ne 0 -and $integrationParts -ne 3) {
    throw 'Git integration doctor configuration requires path, root, and ref together.'
}
if ($EnableGitIntegrate -and $integrationParts -ne 3) {
    throw '-EnableGitIntegrate requires GitIntegrationPath, GitIntegrationRoot, and GitIntegrationRef.'
}
if ($GitIntegrationPath) { $GitIntegrationPath = (Resolve-Path -LiteralPath $GitIntegrationPath).Path }
if ($GitIntegrationRoot) {
    New-Item -ItemType Directory -Force -Path $GitIntegrationRoot | Out-Null
    $GitIntegrationRoot = (Resolve-Path -LiteralPath $GitIntegrationRoot).Path
}
if (($WritePrefix -or $DeletePrefix) -and -not $StateDir) {
    throw 'StateDir is required when write or delete authority is enabled.'
}
if ($StateDir) {
    New-Item -ItemType Directory -Force -Path $StateDir | Out-Null
    $StateDir = (Resolve-Path -LiteralPath $StateDir).Path
}

$bridgeArgs = New-Object System.Collections.Generic.List[string]
if ($GitPath) { $bridgeArgs.Add("--git-executable=$GitPath") }
if ($integrationParts -eq 3) {
    $bridgeArgs.Add("--git-integration-executable=$GitIntegrationPath")
    $bridgeArgs.Add("--git-integration-root=$GitIntegrationRoot")
    $bridgeArgs.Add("--git-integration-ref=$GitIntegrationRef")
}
if ($EnableGitIntegrate) { $bridgeArgs.Add('--allow-git-integrate') }
if ($StateDir) { $bridgeArgs.Add("--mutation-state-dir=$StateDir") }
if ($WritePrefix) { $bridgeArgs.Add("--allow-write-scope=prefix:$WritePrefix") }
if ($DeletePrefix) { $bridgeArgs.Add("--allow-delete-scope=prefix:$DeletePrefix") }
$bridgeArgs.Add($Workspace)

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $BridgePath
$psi.Arguments = (($bridgeArgs | ForEach-Object { Quote-ProcessArgument $_ }) -join ' ')
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.CreateNoWindow = $true

$process = New-Object System.Diagnostics.Process
$process.StartInfo = $psi

try {
    if (-not $process.Start()) { throw 'Failed to start Optic AI Bridge.' }

    $initialize = [ordered]@{
        jsonrpc = '2.0'
        id = 1
        method = 'initialize'
        params = [ordered]@{
            protocolVersion = '2025-06-18'
            capabilities = @{}
            clientInfo = [ordered]@{ name = 'optic-installer-doctor'; version = '1' }
        }
    } | ConvertTo-Json -Depth 8 -Compress
    $process.StandardInput.WriteLine($initialize)
    $process.StandardInput.Flush()

    $initResponse = Read-McpResponse -Process $process -Id 1 -Timeout $TimeoutMs
    if ($initResponse.PSObject.Properties.Name -contains 'error') {
        throw "MCP initialize failed: $($initResponse.error.message)"
    }
    if ($initResponse.result.protocolVersion -ne '2025-06-18') {
        throw "Unexpected MCP protocol version: $($initResponse.result.protocolVersion)"
    }

    $initialized = [ordered]@{
        jsonrpc = '2.0'
        method = 'notifications/initialized'
        params = @{}
    } | ConvertTo-Json -Depth 4 -Compress
    $process.StandardInput.WriteLine($initialized)
    $process.StandardInput.Flush()

    $listTools = [ordered]@{
        jsonrpc = '2.0'
        id = 2
        method = 'tools/list'
        params = @{}
    } | ConvertTo-Json -Depth 4 -Compress
    $process.StandardInput.WriteLine($listTools)
    $process.StandardInput.Flush()

    $toolsResponse = Read-McpResponse -Process $process -Id 2 -Timeout $TimeoutMs
    if ($toolsResponse.PSObject.Properties.Name -contains 'error') {
        throw "MCP tools/list failed: $($toolsResponse.error.message)"
    }

    $toolNames = @($toolsResponse.result.tools | ForEach-Object { [string]$_.name })
    $required = New-Object System.Collections.Generic.List[string]
    $required.Add('fs_list')
    $required.Add('fs_read')
    if ($GitPath) {
        $required.Add('git_status')
        $required.Add('git_diff')
        $required.Add('git_log')
    }
    if ($EnableGitIntegrate) {
        $required.Add('git_integration_status')
        $required.Add('git_integrate')
    }
    if ($WritePrefix) {
        $required.Add('fs_write')
        $required.Add('fs_apply_patch')
    }
    if ($DeletePrefix) { $required.Add('fs_delete') }

    $missing = @($required | Where-Object { $_ -notin $toolNames })
    if ($missing.Count -gt 0) {
        throw "Missing expected MCP tools: $($missing -join ', ')"
    }

    [pscustomobject]@{
        Ok = $true
        ProtocolVersion = [string]$initResponse.result.protocolVersion
        ServerName = [string]$initResponse.result.serverInfo.name
        ServerVersion = [string]$initResponse.result.serverInfo.version
        ToolCount = $toolNames.Count
        RequiredTools = @($required)
    }
}
finally {
    if ($process -and -not $process.HasExited) {
        try { $process.Kill() } catch { }
        try { $process.WaitForExit(2000) | Out-Null } catch { }
    }
    if ($process) { $process.Dispose() }
}
