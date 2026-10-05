[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BridgePath,

    [Parameter(Mandatory = $true)]
    [string]$GitPath,

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

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)][string]$Git,
        [Parameter(Mandatory = $true)][string]$Repository,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [switch]$Capture
    )

    if ($Capture) {
        $output = & $Git -C $Repository @Arguments 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "Git command failed: git -C $Repository $($Arguments -join ' ')`n$output"
        }
        return ($output -join "`n").Trim()
    }

    & $Git -C $Repository @Arguments | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Git command failed: git -C $Repository $($Arguments -join ' ')"
    }
}

function Get-ToolJsonProperty {
    param(
        [Parameter(Mandatory = $true)]$Response,
        [Parameter(Mandatory = $true)][string]$Property
    )

    if ($Response.PSObject.Properties.Name -contains 'error') {
        throw "MCP tool call failed: $($Response.error.message)"
    }
    if ($Response.result.PSObject.Properties.Name -contains 'isError' -and $Response.result.isError) {
        throw 'MCP tool returned an error result.'
    }

    foreach ($structuredName in @('structuredContent', 'structured_content')) {
        if ($Response.result.PSObject.Properties.Name -contains $structuredName) {
            $structured = $Response.result.$structuredName
            if ($structured -and $structured.PSObject.Properties.Name -contains $Property) {
                return [string]$structured.$Property
            }
        }
    }

    foreach ($item in @($Response.result.content)) {
        if ($item -and $item.PSObject.Properties.Name -contains 'text') {
            try {
                $decoded = $item.text | ConvertFrom-Json
                if ($decoded.PSObject.Properties.Name -contains $Property) {
                    return [string]$decoded.$Property
                }
            }
            catch {
                # Ignore non-JSON text content and continue searching.
            }
        }
    }

    throw "MCP tool result did not contain property '$Property'."
}

$BridgePath = (Resolve-Path -LiteralPath $BridgePath).Path
$GitPath = (Resolve-Path -LiteralPath $GitPath).Path

New-Item -ItemType Directory -Force -Path $FixtureRoot | Out-Null
$FixtureRoot = (Resolve-Path -LiteralPath $FixtureRoot).Path
$fixture = Join-Path $FixtureRoot ("optic-git-integrate-smoke-" + [Guid]::NewGuid().ToString('N'))
$repo = Join-Path $fixture 'repo'
$integrationRoot = Join-Path $fixture 'integration'
$targetRef = 'refs/optic/integration/smoke'
$process = $null

try {
    New-Item -ItemType Directory -Force -Path $repo | Out-Null
    New-Item -ItemType Directory -Force -Path $integrationRoot | Out-Null

    Invoke-Git -Git $GitPath -Repository $repo -Arguments @('init', '--quiet')
    Invoke-Git -Git $GitPath -Repository $repo -Arguments @('config', 'user.email', 'optic-smoke@example.invalid')
    Invoke-Git -Git $GitPath -Repository $repo -Arguments @('config', 'user.name', 'Optic Smoke')

    Set-Content -LiteralPath (Join-Path $repo 'tracked.txt') -Value 'alpha' -Encoding ascii
    Invoke-Git -Git $GitPath -Repository $repo -Arguments @('add', 'tracked.txt')
    Invoke-Git -Git $GitPath -Repository $repo -Arguments @('commit', '--quiet', '-m', 'initial')
    $initial = Invoke-Git -Git $GitPath -Repository $repo -Arguments @('rev-parse', '--verify', 'HEAD') -Capture
    Invoke-Git -Git $GitPath -Repository $repo -Arguments @('update-ref', $targetRef, $initial)

    Set-Content -LiteralPath (Join-Path $repo 'tracked.txt') -Value 'beta' -Encoding ascii
    Invoke-Git -Git $GitPath -Repository $repo -Arguments @('add', 'tracked.txt')
    Invoke-Git -Git $GitPath -Repository $repo -Arguments @('commit', '--quiet', '-m', 'source')
    $source = Invoke-Git -Git $GitPath -Repository $repo -Arguments @('rev-parse', '--verify', 'HEAD') -Capture

    $bridgeArgs = @(
        "--git-integration-executable=$GitPath",
        "--git-integration-root=$integrationRoot",
        "--git-integration-ref=$targetRef",
        '--allow-git-integrate',
        $repo
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
        jsonrpc = '2.0'
        id = 1
        method = 'initialize'
        params = [ordered]@{
            protocolVersion = '2025-06-18'
            capabilities = @{}
            clientInfo = [ordered]@{ name = 'optic-git-integration-smoke'; version = '1' }
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
    foreach ($integrationTool in @('git_integration_status', 'git_integrate')) {
        if ($integrationTool -notin $toolNames) {
            throw "$integrationTool was not exposed with explicit integration authority."
        }
    }
    foreach ($readTool in @('git_status', 'git_diff', 'git_log')) {
        if ($readTool -in $toolNames) {
            throw "$readTool was exposed even though GitRead authority was not configured."
        }
    }

    $statusBeforeCall = [ordered]@{
        jsonrpc = '2.0'
        id = 3
        method = 'tools/call'
        params = [ordered]@{ name = 'git_integration_status'; arguments = @{} }
    } | ConvertTo-Json -Depth 8 -Compress
    $process.StandardInput.WriteLine($statusBeforeCall)
    $process.StandardInput.Flush()
    $statusBeforeResponse = Read-McpResponse -Process $process -Id 3 -Timeout $TimeoutMs
    $statusBefore = Get-ToolJsonProperty -Response $statusBeforeResponse -Property 'target_head'
    if ($statusBefore -ne $initial) {
        throw "Integration status returned unexpected initial target. expected=$initial observed=$statusBefore"
    }

    $integrate = [ordered]@{
        jsonrpc = '2.0'
        id = 4
        method = 'tools/call'
        params = [ordered]@{
            name = 'git_integrate'
            arguments = [ordered]@{
                source_head = $source
                expected_target_head = $statusBefore
            }
        }
    } | ConvertTo-Json -Depth 8 -Compress
    $process.StandardInput.WriteLine($integrate)
    $process.StandardInput.Flush()

    $integrationResponse = Read-McpResponse -Process $process -Id 4 -Timeout $TimeoutMs
    if ($integrationResponse.PSObject.Properties.Name -contains 'error') {
        throw "git_integrate failed: $($integrationResponse.error.message)"
    }
    if ($integrationResponse.result.PSObject.Properties.Name -contains 'isError' -and $integrationResponse.result.isError) {
        throw 'git_integrate returned an MCP tool error result.'
    }

    $statusAfterCall = [ordered]@{
        jsonrpc = '2.0'
        id = 5
        method = 'tools/call'
        params = [ordered]@{ name = 'git_integration_status'; arguments = @{} }
    } | ConvertTo-Json -Depth 8 -Compress
    $process.StandardInput.WriteLine($statusAfterCall)
    $process.StandardInput.Flush()
    $statusAfterResponse = Read-McpResponse -Process $process -Id 5 -Timeout $TimeoutMs
    $statusAfter = Get-ToolJsonProperty -Response $statusAfterResponse -Property 'target_head'
    if ($statusAfter -ne $source) {
        throw "Integration status did not observe the new target. expected=$source observed=$statusAfter"
    }

    $observed = Invoke-Git -Git $GitPath -Repository $repo -Arguments @('rev-parse', '--verify', $targetRef) -Capture
    if ($observed -ne $source) {
        throw "Integration ref mismatch. expected=$source observed=$observed"
    }

    # Reusing the now-stale observed precondition must fail closed and must not move the ref.
    $staleIntegrate = [ordered]@{
        jsonrpc = '2.0'
        id = 6
        method = 'tools/call'
        params = [ordered]@{
            name = 'git_integrate'
            arguments = [ordered]@{
                source_head = $source
                expected_target_head = $statusBefore
            }
        }
    } | ConvertTo-Json -Depth 8 -Compress
    $process.StandardInput.WriteLine($staleIntegrate)
    $process.StandardInput.Flush()

    $staleResponse = Read-McpResponse -Process $process -Id 6 -Timeout $TimeoutMs
    $staleRejected = $false
    if ($staleResponse.PSObject.Properties.Name -contains 'error') {
        $staleRejected = $true
    } elseif (
        $staleResponse.PSObject.Properties.Name -contains 'result' -and
        $staleResponse.result.PSObject.Properties.Name -contains 'isError' -and
        $staleResponse.result.isError
    ) {
        $staleRejected = $true
    }
    if (-not $staleRejected) {
        throw 'A stale expected_target_head was not rejected.'
    }

    $afterStale = Invoke-Git -Git $GitPath -Repository $repo -Arguments @('rev-parse', '--verify', $targetRef) -Capture
    if ($afterStale -ne $source) {
        throw "Stale integration changed the target ref. expected=$source observed=$afterStale"
    }

    [pscustomobject]@{
        Ok = $true
        ProtocolVersion = [string]$initResponse.result.protocolVersion
        ToolCount = $toolNames.Count
        SourceHead = $source
        PreviousTargetHead = $initial
        ObservedTargetHead = $observed
        IntegrationStatusToolPresent = $true
        IntegrationToolPresent = $true
        GitReadToolsAbsent = $true
        StatusBeforeHead = $statusBefore
        StatusAfterHead = $statusAfter
        StaleTargetRejected = $true
    }
}
finally {
    if ($process -and -not $process.HasExited) {
        try { $process.Kill() } catch { }
        try { $process.WaitForExit(2000) | Out-Null } catch { }
    }
    if ($process) { $process.Dispose() }
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
