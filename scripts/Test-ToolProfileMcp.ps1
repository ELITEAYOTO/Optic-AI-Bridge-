[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$BridgePath,
    [Parameter(Mandatory = $true)][string]$NodePath,
    [Parameter(Mandatory = $true)][string]$FixtureRoot,
    [int]$TimeoutMs = 15000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Quote-ProcessArgument {
    param([Parameter(Mandatory = $true)][string]$Value)
    return '"' + ($Value -replace '"', '\"') + '"'
}

function Stop-OwnedProcess {
    param($Process)
    if ($Process) {
        if (-not $Process.HasExited) {
            try { $Process.Kill() } catch { }
            try { $Process.WaitForExit(2000) | Out-Null } catch { }
        }
        $Process.Dispose()
    }
}

function Read-McpResponse {
    param(
        [Parameter(Mandatory = $true)]$Process,
        [Parameter(Mandatory = $true)][int]$Id,
        [Parameter(Mandatory = $true)][int]$Timeout,
        [switch]$AllowApproval
    )

    $deadline = [DateTime]::UtcNow.AddMilliseconds($Timeout)
    while ([DateTime]::UtcNow -lt $deadline) {
        $remaining = [Math]::Max(1, [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds)
        $task = $Process.StandardOutput.ReadLineAsync()
        if (-not $task.Wait($remaining)) { throw "Timed out waiting for MCP response id=$Id." }
        $line = $task.Result
        if ($null -eq $line) {
            $stderr = $Process.StandardError.ReadToEnd()
            throw "Bridge closed before MCP response id=$Id. stderr: $stderr"
        }
        if ([string]::IsNullOrWhiteSpace($line)) { continue }

        $message = $line | ConvertFrom-Json
        if ($message.PSObject.Properties.Name -contains 'method' -and [string]$message.method -eq 'elicitation/create') {
            if (-not $AllowApproval) { throw 'Unexpected elicitation for a request that must fail before approval.' }
            if (-not ($message.PSObject.Properties.Name -contains 'id')) { throw 'Elicitation request had no JSON-RPC id.' }
            $text = [string]$message.params.message
            foreach ($expected in @('node-version', '--version', 'Denied', 'timeout=10000ms')) {
                if ($text -notmatch [Regex]::Escape($expected)) {
                    throw "Approval text omitted exact contract fragment '$expected'. text=$text"
                }
            }
            $reply = [ordered]@{
                jsonrpc = '2.0'
                id = $message.id
                result = [ordered]@{ action = 'accept'; content = @{} }
            } | ConvertTo-Json -Depth 8 -Compress
            $Process.StandardInput.WriteLine($reply)
            $Process.StandardInput.Flush()
            continue
        }

        if ($message.PSObject.Properties.Name -contains 'id' -and [int]$message.id -eq $Id) {
            return $message
        }
    }
    throw "Timed out waiting for MCP response id=$Id."
}

function Invoke-McpTool {
    param(
        [Parameter(Mandatory = $true)]$Process,
        [Parameter(Mandatory = $true)][int]$Id,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)]$Arguments,
        [switch]$AllowApproval
    )
    $request = [ordered]@{
        jsonrpc = '2.0'
        id = $Id
        method = 'tools/call'
        params = [ordered]@{ name = $Name; arguments = $Arguments }
    } | ConvertTo-Json -Depth 12 -Compress
    $Process.StandardInput.WriteLine($request)
    $Process.StandardInput.Flush()
    return Read-McpResponse -Process $Process -Id $Id -Timeout $TimeoutMs -AllowApproval:$AllowApproval
}

function Get-ToolPayload {
    param([Parameter(Mandatory = $true)]$Response)
    if ($Response.PSObject.Properties.Name -contains 'error') { throw "MCP request failed: $($Response.error.message)" }
    if ($Response.result.PSObject.Properties.Name -contains 'isError' -and $Response.result.isError) {
        $detail = (@($Response.result.content) | ForEach-Object {
            if ($_.PSObject.Properties.Name -contains 'text') { [string]$_.text }
        }) -join "`n"
        throw "MCP tool returned an error: $detail"
    }
    foreach ($name in @('structuredContent', 'structured_content')) {
        if ($Response.result.PSObject.Properties.Name -contains $name -and $Response.result.$name) { return $Response.result.$name }
    }
    foreach ($item in @($Response.result.content)) {
        if ($item -and $item.PSObject.Properties.Name -contains 'text') {
            try { return ($item.text | ConvertFrom-Json) } catch { }
        }
    }
    throw 'MCP result did not contain structured JSON content.'
}

function Get-ToolErrorText {
    param([Parameter(Mandatory = $true)]$Response)
    if ($Response.PSObject.Properties.Name -contains 'error') { return [string]$Response.error.message }
    if ($Response.PSObject.Properties.Name -contains 'result' -and
        $Response.result.PSObject.Properties.Name -contains 'isError' -and $Response.result.isError) {
        return ((@($Response.result.content) | ForEach-Object {
            if ($_.PSObject.Properties.Name -contains 'text') { [string]$_.text }
        }) -join "`n")
    }
    return $null
}

$BridgePath = (Resolve-Path -LiteralPath $BridgePath).Path
$NodePath = (Resolve-Path -LiteralPath $NodePath).Path
if ([IO.Path]::GetFileName($NodePath) -ine 'node.exe') { throw "NodePath must resolve to node.exe: $NodePath" }
$launcher = Join-Path (Split-Path -Parent $BridgePath) 'optic-bridge-isolation-launcher.exe'
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) { throw "Isolation launcher sibling is missing: $launcher" }

New-Item -ItemType Directory -Force -Path $FixtureRoot | Out-Null
$FixtureRoot = (Resolve-Path -LiteralPath $FixtureRoot).Path
$fixture = Join-Path $FixtureRoot ('tool-profile-mcp-' + [Guid]::NewGuid().ToString('N'))
$workspace = Join-Path $fixture 'workspace'
$profilePath = Join-Path $fixture 'profiles.json'
$process = $null

try {
    New-Item -ItemType Directory -Force -Path $workspace | Out-Null
    $profile = [ordered]@{
        version = 1
        profiles = @([ordered]@{
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
        })
    }
    $profile | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $profilePath -Encoding utf8

    $bridgeArgs = @(
        "--allow-executable=interpreter:$NodePath",
        "--allow-isolated-node=$NodePath",
        "--tool-profile-file=$profilePath",
        $workspace
    )
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
    if (-not $process.Start()) { throw 'Failed to start Optic AI Bridge.' }

    $initialize = [ordered]@{
        jsonrpc = '2.0'; id = 1; method = 'initialize'
        params = [ordered]@{
            protocolVersion = '2025-06-18'
            capabilities = [ordered]@{ elicitation = @{} }
            clientInfo = [ordered]@{ name = 'optic-tool-profile-smoke'; version = '1' }
        }
    } | ConvertTo-Json -Depth 8 -Compress
    $process.StandardInput.WriteLine($initialize); $process.StandardInput.Flush()
    $init = Read-McpResponse -Process $process -Id 1 -Timeout $TimeoutMs
    if ($init.PSObject.Properties.Name -contains 'error') { throw "MCP initialize failed: $($init.error.message)" }
    $process.StandardInput.WriteLine((@{jsonrpc='2.0';method='notifications/initialized';params=@{}} | ConvertTo-Json -Compress)); $process.StandardInput.Flush()

    $drift = Invoke-McpTool -Process $process -Id 2 -Name 'process_start' -Arguments ([ordered]@{
        executable = $NodePath; args = @('--version','unexpected'); cwd = $null; env_allowlist = @(); network = $false; timeout_ms = 10000; process_count = 1
    })
    $driftError = Get-ToolErrorText -Response $drift
    if ($driftError -notmatch 'optic\.process_profile_not_authorized') {
        throw "Invocation drift was not rejected by ToolProfile resolution. observed=$driftError"
    }

    $start = Invoke-McpTool -Process $process -Id 3 -Name 'process_start' -Arguments ([ordered]@{
        executable = $NodePath; args = @('--version'); cwd = $null; env_allowlist = @(); network = $false; timeout_ms = 10000; process_count = 1
    }) -AllowApproval
    $jobId = [string](Get-ToolPayload -Response $start).job_id
    if ([string]::IsNullOrWhiteSpace($jobId)) { throw 'Profiled process_start returned no job_id.' }

    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    $nextId = 4
    do {
        $result = Get-ToolPayload -Response (Invoke-McpTool -Process $process -Id $nextId -Name 'process_result' -Arguments ([ordered]@{job_id=$jobId}))
        $nextId++
        if ([string]$result.status -ne 'running') { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    if ([string]$result.status -ne 'exited' -or [int]$result.exit_code -ne 0) {
        throw "Profiled Node did not exit successfully. status=$($result.status) exit=$($result.exit_code)"
    }

    $read = Get-ToolPayload -Response (Invoke-McpTool -Process $process -Id $nextId -Name 'process_read' -Arguments ([ordered]@{job_id=$jobId;stream='stdout';cursor=0;max_bytes=65536}))
    $stdout = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$read.data)).Trim()
    if ($stdout -notmatch '^v\d+\.\d+\.\d+') { throw "Unexpected Node version output: $stdout" }

    [pscustomobject]@{
        Ok = $true
        DriftDeniedBeforeApproval = $true
        ExactProfileElicited = $true
        ApprovalAccepted = $true
        NodeExited = $true
        NodeVersion = $stdout
        ProfileFile = $profilePath
    }
}
finally {
    Stop-OwnedProcess -Process $process
    if (Test-Path -LiteralPath $fixture) { Remove-Item -LiteralPath $fixture -Recurse -Force }
}
