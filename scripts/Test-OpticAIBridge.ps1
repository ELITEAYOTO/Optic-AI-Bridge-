[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BridgePath,

    [Parameter(Mandatory = $true)]
    [string]$Workspace,

    [string]$IsolationLauncherPath,
    [string]$IsolatedNodePath,
    [string]$GitPath,
    [string]$GitIntegrationPath,
    [string]$GitIntegrationRoot,
    [string]$GitIntegrationRef,
    [switch]$EnableGitIntegrate,
    [string]$StateDir,
    [string]$WritePrefix,
    [string]$DeletePrefix,
    [ValidateRange(1000, 120000)]
    [int]$TimeoutMs = 30000,

    [switch]$StartupTiming
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Quote-ProcessArgument {
    param([Parameter(Mandatory = $true)][string]$Value)
    return '"' + ($Value -replace '"', '\"') + '"'
}

function Throw-McpTimeout {
    param(
        [Parameter(Mandatory = $true)]$Process,
        [Parameter(Mandatory = $true)][int]$Id,
        [Parameter(Mandatory = $true)][int]$Timeout
    )

    $wasRunning = -not $Process.HasExited
    if ($wasRunning) {
        try { $Process.Kill() } catch { }
        try { $Process.WaitForExit(2000) | Out-Null } catch { }
    }

    $finalState = if ($Process.HasExited) {
        "exited with code $($Process.ExitCode)"
    }
    else {
        'still running after bounded kill attempt'
    }

    # Bound failed-run stderr collection as well as the diagnostic output.
    # Child descendants can retain the pipe: no unbounded ReadToEnd or wait.
    $rawStderr = ''
    if ($Process.HasExited) {
        try {
            $chars = New-Object char[] 32768
            $used = 0
            $readClock = [Diagnostics.Stopwatch]::StartNew()
            while ($used -lt $chars.Length -and $readClock.ElapsedMilliseconds -lt 400) {
                $remaining = [Math]::Max(1, 400 - [int]$readClock.ElapsedMilliseconds)
                $readTask = $Process.StandardError.ReadAsync($chars, $used, $chars.Length - $used)
                if (-not $readTask.Wait($remaining)) { break }
                if ($readTask.Result -eq 0) { break }
                $used += $readTask.Result
            }
            $rawStderr = [string]::new($chars, 0, $used)
        }
        catch {
            # Best-effort diagnostic collection never changes timeout failure.
        }
    }
    $stderr = $rawStderr
    if ([string]::IsNullOrWhiteSpace($stderr)) {
        $stderr = '<empty>'
    }
    else {
        $stderr = $stderr.Trim()
        if ($stderr.Length -gt 4096) {
            $stderr = $stderr.Substring(0, 4096) + '...[truncated]'
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($stderr) -and $stderr -ne '<empty>') {
        # Opt-in labels and numeric durations only; never echo raw diagnostic data.
        $printed = 0
        foreach ($phaseLine in ($rawStderr -split "\r?\n")) {
            if ($printed -ge 24) { break }
            if ($phaseLine -cmatch '^OPTIC_STARTUP phase=(entry|arguments-parsed|process-manager-created|executable-profile-checked|git-services-ready|session-provisioned|read-authorities-provisioned|process-lease-entered|process-lease-eligibility-checked|process-lease-id-started|process-lease-id-generated|process-lease-registration-started|process-lease-registered|process-leases-provisioned|server-created|transport-created|mcp-service-started) elapsed_ms=([0-9]{1,12})$') {
                Write-Host "OPTIC_DOCTOR_TIMEOUT_PHASE phase=$($Matches[1]) elapsed_ms=$($Matches[2])"
                $printed++
            }
            elseif ($phaseLine -cmatch '^OPTIC_PROCESS_LEASE_TIMING phase=(entered|eligibility-checked|id-started|id-generated|registration-started|registered) elapsed_ms=([0-9]{1,12})$') {
                Write-Host "OPTIC_DOCTOR_TIMEOUT_LEASE_TIMING phase=$($Matches[1]) elapsed_ms=$($Matches[2])"
                $printed++
            }
            elseif ($phaseLine -cmatch '^OPTIC_PROCESS_IDENTITY_TIMING path_ms=([0-9]{1,12}) pin_ms=([0-9]{1,12}) hash_ms=([0-9]{1,12}) total_ms=([0-9]{1,12})$') {
                Write-Host "OPTIC_DOCTOR_TIMEOUT_IDENTITY_TIMING path_ms=$($Matches[1]) pin_ms=$($Matches[2]) hash_ms=$($Matches[3]) total_ms=$($Matches[4])"
                $printed++
            }
            elseif ($phaseLine -cmatch '^OPTIC_PROCESS_IDENTITY_IO_TIMING read_ms=([0-9]{1,12}) nonread_ms=([0-9]{1,12}) total_ms=([0-9]{1,12}) calls=([0-9]{1,12})$') {
                Write-Host "OPTIC_DOCTOR_TIMEOUT_IDENTITY_IO read_ms=$($Matches[1]) nonread_ms=$($Matches[2]) total_ms=$($Matches[3]) calls=$($Matches[4])"
                $printed++
            }
        }
    }

    $initialState = if ($wasRunning) { 'still running' } else { 'already exited' }
    throw "Timed out after ${Timeout}ms waiting for MCP response id=$Id; bridge was $initialState; final state: $finalState; stderr: $stderr"
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
            Throw-McpTimeout -Process $Process -Id $Id -Timeout $Timeout
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

    Throw-McpTimeout -Process $Process -Id $Id -Timeout $Timeout
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

$BridgePath = (Resolve-Path -LiteralPath $BridgePath).Path
$Workspace = (Resolve-Path -LiteralPath $Workspace).Path
if ($IsolationLauncherPath) {
    $IsolationLauncherPath = (Resolve-Path -LiteralPath $IsolationLauncherPath).Path
    $expectedLauncher = Join-Path (Split-Path -Parent $BridgePath) 'optic-bridge-isolation-launcher.exe'
    if ([IO.Path]::GetFullPath($IsolationLauncherPath) -ine [IO.Path]::GetFullPath($expectedLauncher)) {
        throw 'Isolation launcher must be installed as the canonical sibling of optic-bridge.exe.'
    }
    if (-not (Test-Path -LiteralPath $IsolationLauncherPath -PathType Leaf)) {
        throw "Isolation launcher is not a regular file: $IsolationLauncherPath"
    }
}
if ($IsolatedNodePath) {
    if (-not $IsolationLauncherPath) {
        throw 'IsolatedNodePath requires the canonical isolation launcher sibling.'
    }
    $IsolatedNodePath = (Resolve-Path -LiteralPath $IsolatedNodePath).Path
    if ([IO.Path]::GetFileName($IsolatedNodePath) -ine 'node.exe') {
        throw 'IsolatedNodePath must resolve to node.exe.'
    }
}

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
if ($IsolatedNodePath) {
    $bridgeArgs.Add("--allow-executable=interpreter:$IsolatedNodePath")
    $bridgeArgs.Add("--allow-isolated-node=$IsolatedNodePath")
}
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
if ($StartupTiming) {
    # Child-only diagnostic opt-in: do not alter the operator shell's environment.
    # Fixed phase names/times only; no MCP messages, capabilities, paths or secrets.
    $psi.Environment['OPTIC_STARTUP_TIMING'] = '1'
}

$process = New-Object System.Diagnostics.Process
$process.StartInfo = $psi
$startupClock = [Diagnostics.Stopwatch]::StartNew()
$doctorCompleted = $false

try {
    if (-not $process.Start()) { throw 'Failed to start Optic AI Bridge.' }
    if ($StartupTiming) {
        Write-Host "OPTIC_DOCTOR_TIMING phase=process-started elapsed_ms=$($startupClock.ElapsedMilliseconds)"
    }

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
    if ($StartupTiming) {
        Write-Host "OPTIC_DOCTOR_TIMING phase=initialize-sent elapsed_ms=$($startupClock.ElapsedMilliseconds)"
    }

    $initResponse = Read-McpResponse -Process $process -Id 1 -Timeout $TimeoutMs
    if ($StartupTiming) {
        Write-Host "OPTIC_DOCTOR_TIMING phase=initialize-received elapsed_ms=$($startupClock.ElapsedMilliseconds)"
    }
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
    if ($IsolatedNodePath) {
        foreach ($tool in @('process_start', 'process_read', 'process_result', 'process_stop')) {
            $required.Add($tool)
        }
    }

    $missing = @($required | Where-Object { $_ -notin $toolNames })
    if ($missing.Count -gt 0) {
        throw "Missing expected MCP tools: $($missing -join ', ')"
    }

    $isolatedNodeVersion = $null
    if ($IsolatedNodePath) {
        $start = Invoke-McpTool -Process $process -Id 3 -Name 'process_start' -Arguments ([ordered]@{
            executable = $IsolatedNodePath
            args = @('--version')
            cwd = $null
            env_allowlist = @()
            network = $false
            timeout_ms = 10000
            output_budget = 16384
            memory_bytes = 268435456
            process_count = 1
        })
        $startPayload = Get-ToolPayload -Response $start
        $jobId = [string]$startPayload.job_id
        if ([string]::IsNullOrWhiteSpace($jobId)) { throw 'Node doctor process_start returned no job_id.' }

        $nextId = 4
        $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
        $resultPayload = $null
        do {
            $result = Invoke-McpTool -Process $process -Id $nextId -Name 'process_result' -Arguments ([ordered]@{ job_id = $jobId })
            $nextId++
            $resultPayload = Get-ToolPayload -Response $result
            if ([string]$resultPayload.status -ne 'running') { break }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $deadline)
        if ($null -eq $resultPayload -or [string]$resultPayload.status -eq 'running') {
            throw 'Timed out waiting for isolated Node doctor process.'
        }
        if ([string]$resultPayload.status -ne 'exited' -or [int]$resultPayload.exit_code -ne 0) {
            throw "Isolated Node doctor process failed. status=$($resultPayload.status) exit=$($resultPayload.exit_code)"
        }

        $read = Invoke-McpTool -Process $process -Id $nextId -Name 'process_read' -Arguments ([ordered]@{
            job_id = $jobId
            stream = 'stdout'
            cursor = 0
            max_bytes = 4096
        })
        $readPayload = Get-ToolPayload -Response $read
        $isolatedNodeVersion = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$readPayload.data)).Trim()
        if ($isolatedNodeVersion -notmatch '^v[0-9]+\.') {
            throw "Unexpected isolated Node version output: $isolatedNodeVersion"
        }
    }

    $doctorCompleted = $true
    [pscustomobject]@{
        Ok = $true
        ProtocolVersion = [string]$initResponse.result.protocolVersion
        ServerName = [string]$initResponse.result.serverInfo.name
        ServerVersion = [string]$initResponse.result.serverInfo.version
        ToolCount = $toolNames.Count
        RequiredTools = @($required)
        IsolationLauncherPresent = [bool]$IsolationLauncherPath
        IsolatedNodeProfile = [bool]$IsolatedNodePath
        IsolatedNodeVersion = $isolatedNodeVersion
    }
}
finally {
    if ($process -and -not $process.HasExited) {
        try { $process.Kill() } catch { }
        try { $process.WaitForExit(2000) | Out-Null } catch { }
    }
    # The bridge already emits fixed, opt-in startup phase timings to
    # stderr. Previously successful doctor runs discarded them, leaving no
    # phase baseline for comparing intermittent slow initialization.
    # Consume at most 4096 characters after stopping the owned bridge,
    # never print arbitrary stderr or project data, and never block cleanup
    # for longer than 400ms on an inherited or unexpectedly live pipe.
    if ($StartupTiming -and $doctorCompleted -and $process -and $process.HasExited) {
        try {
            $chars = New-Object char[] 4096
            $used = 0
            $readClock = [Diagnostics.Stopwatch]::StartNew()
            while ($used -lt $chars.Length -and $readClock.ElapsedMilliseconds -lt 400) {
                $remaining = [Math]::Max(1, 400 - [int]$readClock.ElapsedMilliseconds)
                $readTask = $process.StandardError.ReadAsync($chars, $used, $chars.Length - $used)
                if (-not $readTask.Wait($remaining)) { break }
                if ($readTask.Result -eq 0) { break }
                $used += $readTask.Result
            }
            if ($used -gt 0) {
                $startupLines = [string]::new($chars, 0, $used) -split "\r?\n"
                $seenPhases = New-Object 'System.Collections.Generic.HashSet[string]'
                $seenProcessIdentityTiming = $false
                foreach ($startupLine in $startupLines) {
                    if ($startupLine -cmatch '^OPTIC_STARTUP phase=(entry|arguments-parsed|process-manager-created|executable-profile-checked|git-services-ready|session-provisioned|read-authorities-provisioned|process-lease-entered|process-lease-eligibility-checked|process-lease-id-started|process-lease-id-generated|process-lease-registration-started|process-lease-registered|process-leases-provisioned|server-created|transport-created|mcp-service-started) elapsed_ms=([0-9]{1,12})$') {
                        if ($seenPhases.Add($Matches[1])) {
                            Write-Host "OPTIC_DOCTOR_CHILD_TIMING phase=$($Matches[1]) elapsed_ms=$($Matches[2])"
                        }
                    }
                    elseif ($startupLine -cmatch '^OPTIC_PROCESS_LEASE_TIMING phase=(entered|eligibility-checked|id-started|id-generated|registration-started|registered) elapsed_ms=([0-9]{1,12})$') {
                        $leasePhase = $Matches[1]
                        $leaseDuration = $Matches[2]
                        if ($seenPhases.Add("lease-$leasePhase")) {
                            Write-Host "OPTIC_DOCTOR_LEASE_TIMING phase=$leasePhase elapsed_ms=$leaseDuration"
                        }
                    }
                    elseif (-not $seenProcessIdentityTiming -and $startupLine -cmatch '^OPTIC_PROCESS_IDENTITY_TIMING path_ms=([0-9]{1,12}) pin_ms=([0-9]{1,12}) hash_ms=([0-9]{1,12}) total_ms=([0-9]{1,12})$') {
                        $seenProcessIdentityTiming = $true
                        Write-Host "OPTIC_DOCTOR_IDENTITY_TIMING path_ms=$($Matches[1]) pin_ms=$($Matches[2]) hash_ms=$($Matches[3]) total_ms=$($Matches[4])"
                    }
                    elseif ($startupLine -cmatch '^OPTIC_PROCESS_IDENTITY_IO_TIMING read_ms=([0-9]{1,12}) nonread_ms=([0-9]{1,12}) total_ms=([0-9]{1,12}) calls=([0-9]{1,12})$') {
                        if ($seenPhases.Add('identity-io')) {
                            Write-Host "OPTIC_DOCTOR_IDENTITY_IO read_ms=$($Matches[1]) nonread_ms=$($Matches[2]) total_ms=$($Matches[3]) calls=$($Matches[4])"
                        }
                    }
                }
            }
        }
        catch {
            # Observability is best effort; never reinterpret a successful
            # MCP functional proof as a failure due to timing collection.
        }
    }
    if ($process) { $process.Dispose() }
}
