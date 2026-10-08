[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BridgePath,

    [Parameter(Mandatory = $true)]
    [string]$NodePath,

    [Parameter(Mandatory = $true)]
    [string]$FixtureRoot,

    [int]$TimeoutMs = 15000
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

function Invoke-McpTool {
    param(
        [Parameter(Mandatory = $true)]$Process,
        [Parameter(Mandatory = $true)][int]$Id,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)]$Arguments
    )

    $request = [ordered]@{
        jsonrpc = '2.0'
        id = $Id
        method = 'tools/call'
        params = [ordered]@{ name = $Name; arguments = $Arguments }
    } | ConvertTo-Json -Depth 12 -Compress
    $Process.StandardInput.WriteLine($request)
    $Process.StandardInput.Flush()
    return Read-McpResponse -Process $Process -Id $Id -Timeout $TimeoutMs
}

function Get-ToolPayload {
    param([Parameter(Mandatory = $true)]$Response)

    if ($Response.PSObject.Properties.Name -contains 'error') {
        throw "MCP tool call failed: $($Response.error.message)"
    }
    if ($Response.result.PSObject.Properties.Name -contains 'isError' -and $Response.result.isError) {
        $detail = (@($Response.result.content) | ForEach-Object {
            if ($_.PSObject.Properties.Name -contains 'text') { [string]$_.text }
        }) -join "`n"
        throw "MCP tool returned an error result: $detail"
    }

    foreach ($structuredName in @('structuredContent', 'structured_content')) {
        if ($Response.result.PSObject.Properties.Name -contains $structuredName) {
            $structured = $Response.result.$structuredName
            if ($structured) { return $structured }
        }
    }

    foreach ($item in @($Response.result.content)) {
        if ($item -and $item.PSObject.Properties.Name -contains 'text') {
            try { return ($item.text | ConvertFrom-Json) } catch { }
        }
    }
    throw 'MCP tool result did not contain structured JSON content.'
}

function Get-ToolErrorText {
    param([Parameter(Mandatory = $true)]$Response)

    if ($Response.PSObject.Properties.Name -contains 'error') {
        return [string]$Response.error.message
    }
    if ($Response.PSObject.Properties.Name -contains 'result' -and
        $Response.result.PSObject.Properties.Name -contains 'isError' -and
        $Response.result.isError) {
        return ((@($Response.result.content) | ForEach-Object {
            if ($_.PSObject.Properties.Name -contains 'text') { [string]$_.text }
        }) -join "`n")
    }
    return $null
}

function Assert-BridgeStartupRejected {
    param(
        [Parameter(Mandatory = $true)][string]$BridgeExecutable,
        [Parameter(Mandatory = $true)][string[]]$BridgeArguments,
        [Parameter(Mandatory = $true)][string]$ExpectedMessage,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $BridgeExecutable
    $psi.Arguments = (($BridgeArguments | ForEach-Object { Quote-ProcessArgument $_ }) -join ' ')
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $psi
    try {
        if (-not $process.Start()) { throw "$Label failed to start the bridge fixture." }
        if (-not $process.WaitForExit(5000)) {
            try { $process.Kill() } catch { }
            throw "$Label unexpectedly remained running instead of failing closed at startup."
        }
        $stderr = $process.StandardError.ReadToEnd()
        if ($process.ExitCode -eq 0) {
            throw "$Label unexpectedly exited successfully. stderr=$stderr"
        }
        if ($stderr -notmatch [Regex]::Escape($ExpectedMessage)) {
            throw "$Label failed for an unexpected reason. expected=$ExpectedMessage stderr=$stderr"
        }
    }
    finally {
        if ($process -and -not $process.HasExited) {
            try { $process.Kill() } catch { }
            try { $process.WaitForExit(2000) | Out-Null } catch { }
        }
        if ($process) { $process.Dispose() }
    }
}

function Start-OpticBridge {
    param(
        [Parameter(Mandatory = $true)][string]$Workspace,
        [Parameter(Mandatory = $true)][bool]$EnableNodeEligibility
    )

    $bridgeArgs = New-Object System.Collections.Generic.List[string]
    $bridgeArgs.Add("--allow-executable=interpreter:$NodePath")
    $bridgeArgs.Add("--allow-process-read-file=$NodePath")
    $bridgeArgs.Add('allowed.txt')
    if ($EnableNodeEligibility) {
        $bridgeArgs.Add("--allow-isolated-node=$NodePath")
    }
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
    if (-not $process.Start()) { throw 'Failed to start Optic AI Bridge.' }

    $initialize = [ordered]@{
        jsonrpc = '2.0'
        id = 1
        method = 'initialize'
        params = [ordered]@{
            protocolVersion = '2025-06-18'
            capabilities = @{}
            clientInfo = [ordered]@{ name = 'optic-isolated-node-smoke'; version = '1' }
        }
    } | ConvertTo-Json -Depth 8 -Compress
    $process.StandardInput.WriteLine($initialize)
    $process.StandardInput.Flush()
    $initResponse = Read-McpResponse -Process $process -Id 1 -Timeout $TimeoutMs
    if ($initResponse.PSObject.Properties.Name -contains 'error') {
        throw "MCP initialize failed: $($initResponse.error.message)"
    }

    $initialized = [ordered]@{
        jsonrpc = '2.0'
        method = 'notifications/initialized'
        params = @{}
    } | ConvertTo-Json -Depth 4 -Compress
    $process.StandardInput.WriteLine($initialized)
    $process.StandardInput.Flush()
    return $process
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

$BridgePath = (Resolve-Path -LiteralPath $BridgePath).Path
$NodePath = (Resolve-Path -LiteralPath $NodePath).Path
if ([IO.Path]::GetFileName($NodePath) -ine 'node.exe') {
    throw "NodePath must resolve to node.exe: $NodePath"
}
$launcher = Join-Path (Split-Path -Parent $BridgePath) 'optic-bridge-isolation-launcher.exe'
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) {
    throw "Isolation launcher sibling is missing: $launcher"
}

New-Item -ItemType Directory -Force -Path $FixtureRoot | Out-Null
$FixtureRoot = (Resolve-Path -LiteralPath $FixtureRoot).Path
$fixture = Join-Path $FixtureRoot ('isolated-node-mcp-' + [Guid]::NewGuid().ToString('N'))
$workspace = Join-Path $fixture 'workspace'
$control = $null
$eligible = $null

try {
    New-Item -ItemType Directory -Force -Path $workspace | Out-Null
    $allowedPath = Join-Path $workspace 'allowed.txt'
    $deniedPath = Join-Path $workspace 'denied.txt'
    [IO.File]::WriteAllText($allowedPath, 'optic-node-allowed-sentinel', [Text.Encoding]::ASCII)
    [IO.File]::WriteAllText($deniedPath, 'optic-node-denied-sentinel', [Text.Encoding]::ASCII)

    # Operator profile validation is fail-closed before any session/lease is
    # published: wrong class, non-node basename and missing helper are rejected.
    Assert-BridgeStartupRejected `
        -BridgeExecutable $BridgePath `
        -BridgeArguments @(
            "--allow-executable=repository-code:$NodePath",
            "--allow-isolated-node=$NodePath",
            $workspace
        ) `
        -ExpectedMessage '--allow-isolated-node requires the same executable to be authorized as --allow-executable=interpreter:<absolute-node-path>' `
        -Label 'wrong-class Node profile'

    $fakeNode = Join-Path $fixture 'python.exe'
    Copy-Item -LiteralPath $NodePath -Destination $fakeNode -Force
    Assert-BridgeStartupRejected `
        -BridgeExecutable $BridgePath `
        -BridgeArguments @(
            "--allow-executable=interpreter:$fakeNode",
            "--allow-isolated-node=$fakeNode",
            $workspace
        ) `
        -ExpectedMessage '--allow-isolated-node accepts only an exact node.exe interpreter path' `
        -Label 'non-node basename profile'

    $bridgeOnlyDir = Join-Path $fixture 'bridge-without-helper'
    New-Item -ItemType Directory -Force -Path $bridgeOnlyDir | Out-Null
    $bridgeWithoutHelper = Join-Path $bridgeOnlyDir 'optic-bridge.exe'
    Copy-Item -LiteralPath $BridgePath -Destination $bridgeWithoutHelper -Force
    Assert-BridgeStartupRejected `
        -BridgeExecutable $bridgeWithoutHelper `
        -BridgeArguments @(
            "--allow-executable=interpreter:$NodePath",
            "--allow-isolated-node=$NodePath",
            $workspace
        ) `
        -ExpectedMessage '--allow-isolated-node requires the installed sibling optic-bridge-isolation-launcher.exe' `
        -Label 'missing-helper Node profile'

    # Control: executable + exact-file authority alone must remain denied without
    # the operator-owned Node eligibility profile.
    $control = Start-OpticBridge -Workspace $workspace -EnableNodeEligibility $false
    $controlStart = Invoke-McpTool -Process $control -Id 2 -Name 'process_start' -Arguments ([ordered]@{
        executable = $NodePath
        args = @('--version')
        cwd = $null
        env_allowlist = @()
        network = $false
        timeout_ms = 10000
    })
    $controlError = Get-ToolErrorText -Response $controlStart
    if ($controlError -notmatch 'optic\.process_isolation_unavailable') {
        throw "High-risk Node start was not denied without operator eligibility. observed=$controlError"
    }

    # A caller-supplied field cannot mint the server-owned marker. It may be
    # rejected by parameter validation or ignored, but it must never start a job.
    $spoof = Invoke-McpTool -Process $control -Id 3 -Name 'process_start' -Arguments ([ordered]@{
        executable = $NodePath
        args = @('--version')
        cwd = $null
        env_allowlist = @()
        network = $false
        isolation_eligible = $true
        timeout_ms = 10000
    })
    if ($null -eq (Get-ToolErrorText -Response $spoof)) {
        throw 'Caller-supplied isolation_eligible unexpectedly minted execution authority.'
    }
    Stop-OwnedProcess -Process $control
    $control = $null

    # Positive path: the exact operator-selected Node executable is admitted,
    # routed through the sibling AppContainer helper, reads exactly the granted
    # file and remains denied the ungranted sibling.
    $eligible = Start-OpticBridge -Workspace $workspace -EnableNodeEligibility $true

    # C5E proves only a single logical isolated target. The eligible lease is
    # therefore deliberately capped to process_count=1; descendant budgets stay
    # fail-closed until a separate containment gate proves them.
    $descendantBudget = Invoke-McpTool -Process $eligible -Id 2 -Name 'process_start' -Arguments ([ordered]@{
        executable = $NodePath
        args = @('--version')
        cwd = $null
        env_allowlist = @()
        network = $false
        timeout_ms = 10000
        process_count = 2
    })
    if ($null -eq (Get-ToolErrorText -Response $descendantBudget)) {
        throw 'Eligible Node unexpectedly received an unproven multi-process budget.'
    }

    $nodeScript = "const fs=require('fs'); const ok=fs.readFileSync(process.argv[1],'utf8'); let denied='READ'; try { fs.readFileSync(process.argv[2],'utf8'); } catch(e) { denied=(e&&e.code)||'DENIED'; } process.stdout.write('ALLOWED='+ok+';DENIED='+denied);"
    $start = Invoke-McpTool -Process $eligible -Id 3 -Name 'process_start' -Arguments ([ordered]@{
        executable = $NodePath
        args = @('-e', $nodeScript, $allowedPath, $deniedPath)
        cwd = $null
        env_allowlist = @()
        network = $false
        timeout_ms = 10000
        output_budget = 65536
        memory_bytes = 268435456
        process_count = 1
    })
    $startPayload = Get-ToolPayload -Response $start
    $jobId = [string]$startPayload.job_id
    if ([string]::IsNullOrWhiteSpace($jobId)) { throw 'process_start returned no job_id.' }

    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    $resultPayload = $null
    $nextId = 4
    do {
        $result = Invoke-McpTool -Process $eligible -Id $nextId -Name 'process_result' -Arguments ([ordered]@{ job_id = $jobId })
        $nextId++
        $resultPayload = Get-ToolPayload -Response $result
        if ([string]$resultPayload.status -ne 'running') { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)

    if ($null -eq $resultPayload -or [string]$resultPayload.status -eq 'running') {
        throw 'Timed out waiting for isolated Node MCP process result.'
    }
    if ([string]$resultPayload.status -ne 'exited' -or [int]$resultPayload.exit_code -ne 0) {
        throw "Isolated Node did not exit successfully. status=$($resultPayload.status) exit=$($resultPayload.exit_code)"
    }

    $read = Invoke-McpTool -Process $eligible -Id $nextId -Name 'process_read' -Arguments ([ordered]@{
        job_id = $jobId
        stream = 'stdout'
        cursor = 0
        max_bytes = 65536
    })
    $readPayload = Get-ToolPayload -Response $read
    $stdout = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$readPayload.data))
    if ($stdout -notmatch 'ALLOWED=optic-node-allowed-sentinel') {
        throw "Granted exact file was not readable through real MCP isolated Node path. stdout=$stdout"
    }
    if ($stdout -match 'DENIED=READ') {
        throw "Ungrant sibling file was readable through isolated Node path. stdout=$stdout"
    }
    if ($stdout -notmatch 'DENIED=[A-Z0-9_]+') {
        throw "Ungrant sibling denial was not observed. stdout=$stdout"
    }

    [pscustomobject]@{
        Ok = $true
        WrongClassProfileRejected = $true
        NonNodeBasenameRejected = $true
        MissingHelperRejected = $true
        ControlWithoutEligibilityDenied = $true
        CallerCannotMintEligibility = $true
        ExactNodeEligibilityAdmitted = $true
        MultiProcessBudgetDenied = $true
        GrantedFileReadable = $true
        UngrantedSiblingDenied = $true
        IsolationLauncher = $launcher
        Node = $NodePath
    }
}
finally {
    Stop-OwnedProcess -Process $control
    Stop-OwnedProcess -Process $eligible
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
