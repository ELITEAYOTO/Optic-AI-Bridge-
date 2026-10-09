[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$BridgePath,
    [Parameter(Mandatory = $true)][string]$NodePath,
    [Parameter(Mandatory = $true)][string]$FixtureRoot,
    [int]$TimeoutMs = 15000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:ElicitationCount = 0

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

function Assert-ApprovalSchema {
    param([Parameter(Mandatory = $true)]$Params)
    $schema = $Params.requestedSchema
    if (-not $schema) { throw 'Reusable approval elicitation omitted requestedSchema.' }
    if (@($schema.required) -notcontains 'approval_scope') {
        throw 'Reusable approval elicitation did not require approval_scope.'
    }
    $scope = $schema.properties.approval_scope
    if (-not $scope) { throw 'Reusable approval elicitation omitted approval_scope schema.' }
    if ([string]$scope.type -ne 'string') { throw 'approval_scope is not a string enum.' }
    if ([string]$scope.default -ne 'once') { throw 'approval_scope least-authority default is not once.' }
    $values = @($scope.oneOf | ForEach-Object { [string]$_.const })
    if (($values -join "`n") -cne (@('once', 'current_session') -join "`n")) {
        throw "Unexpected approval_scope values: $($values -join ', ')"
    }
}

function Read-McpResponse {
    param(
        [Parameter(Mandatory = $true)]$Process,
        [Parameter(Mandatory = $true)][int]$Id,
        [Parameter(Mandatory = $true)][int]$Timeout,
        [ValidateSet('none','once','current_session','cancel')][string]$Approval = 'none'
    )

    $before = $script:ElicitationCount
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
            if ($Approval -eq 'none') { throw "Unexpected elicitation while waiting for MCP response id=$Id." }
            if (-not ($message.PSObject.Properties.Name -contains 'id')) { throw 'Elicitation request had no JSON-RPC id.' }
            Assert-ApprovalSchema -Params $message.params
            $script:ElicitationCount++

            if ($Approval -eq 'cancel') {
                $result = [ordered]@{ action = 'cancel' }
            }
            else {
                $result = [ordered]@{
                    action = 'accept'
                    content = [ordered]@{ approval_scope = $Approval }
                }
            }
            $reply = [ordered]@{
                jsonrpc = '2.0'
                id = $message.id
                result = $result
            } | ConvertTo-Json -Depth 10 -Compress
            $Process.StandardInput.WriteLine($reply)
            $Process.StandardInput.Flush()
            continue
        }

        if ($message.PSObject.Properties.Name -contains 'id' -and [int]$message.id -eq $Id) {
            if ($Approval -ne 'none' -and $script:ElicitationCount -eq $before) {
                throw "Expected one reusable approval elicitation for MCP response id=$Id but none was observed."
            }
            if ($script:ElicitationCount -gt ($before + 1)) {
                throw "Observed more than one elicitation while waiting for MCP response id=$Id."
            }
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
        [ValidateSet('none','once','current_session','cancel')][string]$Approval = 'none'
    )
    $request = [ordered]@{
        jsonrpc = '2.0'
        id = $Id
        method = 'tools/call'
        params = [ordered]@{ name = $Name; arguments = $Arguments }
    } | ConvertTo-Json -Depth 12 -Compress
    $Process.StandardInput.WriteLine($request)
    $Process.StandardInput.Flush()
    return Read-McpResponse -Process $Process -Id $Id -Timeout $TimeoutMs -Approval $Approval
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

function Wait-JobExit {
    param(
        [Parameter(Mandatory = $true)]$Process,
        [Parameter(Mandatory = $true)][string]$JobId,
        [Parameter(Mandatory = $true)][ref]$NextId
    )
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    do {
        $id = $NextId.Value
        $NextId.Value++
        $result = Get-ToolPayload -Response (Invoke-McpTool -Process $Process -Id $id -Name 'process_result' -Arguments ([ordered]@{job_id=$JobId}))
        if ([string]$result.status -ne 'running') { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    if ([string]$result.status -ne 'exited' -or [int]$result.exit_code -ne 0) {
        throw "Profiled Node did not exit successfully. status=$($result.status) exit=$($result.exit_code)"
    }
}

$BridgePath = (Resolve-Path -LiteralPath $BridgePath).Path
$NodePath = (Resolve-Path -LiteralPath $NodePath).Path
if ([IO.Path]::GetFileName($NodePath) -ine 'node.exe') { throw "NodePath must resolve to node.exe: $NodePath" }
$launcher = Join-Path (Split-Path -Parent $BridgePath) 'optic-bridge-isolation-launcher.exe'
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) { throw "Isolation launcher sibling is missing: $launcher" }

New-Item -ItemType Directory -Force -Path $FixtureRoot | Out-Null
$FixtureRoot = (Resolve-Path -LiteralPath $FixtureRoot).Path
$fixture = Join-Path $FixtureRoot ('reusable-approval-mcp-' + [Guid]::NewGuid().ToString('N'))
$workspace = Join-Path $fixture 'workspace'
$profilePath = Join-Path $fixture 'profiles.json'
$configPath = Join-Path $fixture 'optic.json'
$process = $null

try {
    New-Item -ItemType Directory -Force -Path $workspace | Out-Null
    $profiles = [ordered]@{
        version = 1
        profiles = @(
            [ordered]@{
                name = 'node-version'
                executable = $NodePath
                args = @('--version')
                env_allowlist = @()
                resources = [ordered]@{ timeout_ms=30000; output_bytes=1048576; memory_bytes=536870912; process_count=1 }
                require_approval = $true
            },
            [ordered]@{
                name = 'node-help'
                executable = $NodePath
                args = @('--help')
                env_allowlist = @()
                resources = [ordered]@{ timeout_ms=30000; output_bytes=1048576; memory_bytes=536870912; process_count=1 }
                require_approval = $true
            }
        )
    }
    $profiles | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $profilePath -Encoding utf8

    $config = [ordered]@{
        version = 1
        policy_epoch = 7
        workspace = $workspace
        executables = @([ordered]@{ class='interpreter'; path=$NodePath })
        process_read_grants = @()
        isolated_node_executables = @($NodePath)
        tool_profile_file = $profilePath
        environment_grants = @()
        mutation = $null
        git = $null
    }
    $config | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $configPath -Encoding utf8

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $BridgePath
    $psi.Arguments = Quote-ProcessArgument "--config=$configPath"
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
            clientInfo = [ordered]@{ name='optic-reusable-approval-smoke'; version='1' }
        }
    } | ConvertTo-Json -Depth 8 -Compress
    $process.StandardInput.WriteLine($initialize); $process.StandardInput.Flush()
    $init = Read-McpResponse -Process $process -Id 1 -Timeout $TimeoutMs
    if ($init.PSObject.Properties.Name -contains 'error') { throw "MCP initialize failed: $($init.error.message)" }
    $process.StandardInput.WriteLine((@{jsonrpc='2.0';method='notifications/initialized';params=@{}} | ConvertTo-Json -Compress)); $process.StandardInput.Flush()

    $nextId = 2
    $drift = Invoke-McpTool -Process $process -Id $nextId -Name 'process_start' -Arguments ([ordered]@{
        executable=$NodePath; args=@('--version','unexpected'); cwd=$null; env_allowlist=@(); network=$false; timeout_ms=10000; process_count=1
    })
    $nextId++
    $driftError = Get-ToolErrorText -Response $drift
    if ($driftError -notmatch 'optic\.process_profile_not_authorized') {
        throw "Invocation drift was not rejected before approval. observed=$driftError"
    }

    $first = Invoke-McpTool -Process $process -Id $nextId -Name 'process_start' -Arguments ([ordered]@{
        executable=$NodePath; args=@('--version'); cwd=$null; env_allowlist=@(); network=$false; timeout_ms=10000; process_count=1
    }) -Approval current_session
    $nextId++
    $firstJob = [string](Get-ToolPayload -Response $first).job_id
    if ([string]::IsNullOrWhiteSpace($firstJob)) { throw 'First profiled process_start returned no job_id.' }
    Wait-JobExit -Process $process -JobId $firstJob -NextId ([ref]$nextId)
    if ($script:ElicitationCount -ne 1) { throw "Expected exactly one elicitation after first action; observed $script:ElicitationCount." }

    $second = Invoke-McpTool -Process $process -Id $nextId -Name 'process_start' -Arguments ([ordered]@{
        executable=$NodePath; args=@('--version'); cwd=$null; env_allowlist=@(); network=$false; timeout_ms=10000; process_count=1
    })
    $nextId++
    $secondJob = [string](Get-ToolPayload -Response $second).job_id
    if ([string]::IsNullOrWhiteSpace($secondJob)) { throw 'Second profiled process_start returned no job_id.' }
    Wait-JobExit -Process $process -JobId $secondJob -NextId ([ref]$nextId)
    if ($script:ElicitationCount -ne 1) { throw 'Matching reusable grant unexpectedly prompted again.' }

    $otherProfile = Invoke-McpTool -Process $process -Id $nextId -Name 'process_start' -Arguments ([ordered]@{
        executable=$NodePath; args=@('--help'); cwd=$null; env_allowlist=@(); network=$false; timeout_ms=10000; process_count=1
    }) -Approval cancel
    $nextId++
    $otherError = Get-ToolErrorText -Response $otherProfile
    if ($otherError -notmatch 'optic\.approval_cancelled') {
        throw "Different ToolProfile did not require separate approval. observed=$otherError"
    }
    if ($script:ElicitationCount -ne 2) { throw "Expected second elicitation for a different profile; observed $script:ElicitationCount." }

    $cancel = Invoke-McpTool -Process $process -Id $nextId -Name 'session_cancel' -Arguments ([ordered]@{})
    $nextId++
    $null = Get-ToolPayload -Response $cancel

    $afterCancel = Invoke-McpTool -Process $process -Id $nextId -Name 'process_start' -Arguments ([ordered]@{
        executable=$NodePath; args=@('--version'); cwd=$null; env_allowlist=@(); network=$false; timeout_ms=10000; process_count=1
    })
    $afterCancelError = Get-ToolErrorText -Response $afterCancel
    if ($afterCancelError -notmatch 'optic\.session_(revoked|inactive)') {
        throw "Revoked session unexpectedly reached reusable approval. observed=$afterCancelError"
    }

    [pscustomobject]@{
        Ok = $true
        DriftDeniedBeforeApproval = $true
        CurrentSessionChoiceObserved = $true
        DistinctSecondActionSkippedOpticElicitation = $true
        DifferentProfileRequiredNewElicitation = $true
        RevokedSessionRejectedWithoutElicitation = $true
        ElicitationCount = $script:ElicitationCount
        PolicyEpoch = 7
        PolicyEpochMismatchCoveredByCoreTests = $true
        ProfileFile = $profilePath
        ConfigFile = $configPath
    }
}
finally {
    Stop-OwnedProcess -Process $process
    if (Test-Path -LiteralPath $fixture) { Remove-Item -LiteralPath $fixture -Recurse -Force }
}
