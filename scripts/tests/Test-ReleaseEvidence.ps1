# Offline fixtures only; production function definitions have no top-level effects.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '..\Assert-OpticReleaseEvidence.ps1')
$script:cases = 0
function New-Evidence {
    @{
        Repository = 'ELITEAYOTO/Optic-AI-Bridge-'
        SourceSha = ('a' * 40)
        Tag = 'v0.1.4'
        Version = '0.1.4'
        Comparison = @{ status = 'identical'; merge_base_commit = @{ sha = ('a' * 40) } }
        Run = @{ head_sha = ('a' * 40); head_branch = 'main'; event = 'push'; path = '.github/workflows/ci.yml'; status = 'completed'; conclusion = 'success'; id = 123; run_attempt = 1 }
        Jobs = @('Rust / windows-latest', 'Rust / ubuntu-latest', 'Dependency policy') | ForEach-Object { @{ name = $_; status = 'completed'; conclusion = 'success' } }
    }
}
function Deny([string]$Label, [scriptblock]$Change) {
    $evidence = New-Evidence
    & $Change $evidence
    $denied = $false
    try { Assert-OpticReleaseEvidence @evidence | Out-Null } catch { $denied = $true }
    if (-not $denied) { throw "Gate accepted invalid fixture: $Label" }
    $script:cases++
}
$valid = New-Evidence
$result = Assert-OpticReleaseEvidence @valid
if ($result.source_sha -cne ('a' * 40) -or $result.verified_job_count -ne 3) { throw 'Valid fixture failed.' }
$script:cases++
$valid.Comparison.status = 'ahead'
Assert-OpticReleaseEvidence @valid | Out-Null
$script:cases++
Deny 'foreign repo' { param($e) $e.Repository = 'other/repo' }
Deny 'invalid SHA' { param($e) $e.SourceSha = 'main' }
Deny 'tag mismatch' { param($e) $e.Tag = 'v0.1.5' }
Deny 'unsafe version' { param($e) $e.Version = '0.1.4; throw'; $e.Tag = 'v0.1.4; throw' }
Deny 'main behind release source' { param($e) $e.Comparison.status = 'behind' }
Deny 'diverged source' { param($e) $e.Comparison.status = 'diverged' }
Deny 'unproven merge base' { param($e) $e.Comparison.merge_base_commit.sha = ('b' * 40) }
Deny 'different source CI' { param($e) $e.Run.head_sha = ('b' * 40) }
Deny 'branch CI substitution' { param($e) $e.Run.head_branch = 'bootstrap/research' }
Deny 'PR CI substitution' { param($e) $e.Run.event = 'pull_request' }
Deny 'different workflow' { param($e) $e.Run.path = '.github/workflows/other.yml' }
Deny 'pending CI' { param($e) $e.Run.status = 'in_progress'; $e.Run.conclusion = $null }
Deny 'failed CI' { param($e) $e.Run.conclusion = 'failure' }
Deny 'missing Windows job' { param($e) $e.Jobs = @($e.Jobs | Where-Object { $_.name -ne 'Rust / windows-latest' }) }
Deny 'failed Windows job' { param($e) $e.Jobs[0].conclusion = 'failure' }
Deny 'skipped Windows job' { param($e) $e.Jobs[0].conclusion = 'skipped' }
Deny 'running job' { param($e) $e.Jobs[0].status = 'in_progress' }
Deny 'duplicate Windows job' { param($e) $e.Jobs += $e.Jobs[0] }
Deny 'empty jobs' { param($e) $e.Jobs = @() }
Deny 'unknown failed job' { param($e) $e.Jobs += @{ name = 'future gate'; status = 'completed'; conclusion = 'failure' } }

# Mock the API boundary: these tests must never contact GitHub.
$savedRepo = $env:GITHUB_REPOSITORY
$savedToken = $env:GH_TOKEN
try {
    $env:GITHUB_REPOSITORY = 'ELITEAYOTO/Optic-AI-Bridge-'
    $env:GH_TOKEN = 'synthetic-test-token'
    $script:fixture = New-Evidence
    $script:latestFails = $false
    $script:apiFails = $false
    $script:jobsIncomplete = $false
    $script:apiCalls = 0
    function Invoke-RestMethod {
        param($Uri, $Headers, $TimeoutSec)
        $script:apiCalls++
        if ($script:apiFails) { throw 'Synthetic API failure.' }
        if ($TimeoutSec -ne 30) { throw 'API request not bounded.' }
        if (-not $Uri.StartsWith('https://api.github.com/repos/ELITEAYOTO/Optic-AI-Bridge-/')) { throw 'Unexpected API host/repository.' }
        if ($Uri.Contains('/compare/')) { return $script:fixture.Comparison }
        if ($Uri.Contains('/actions/runs?')) {
            $old = @{} + $script:fixture.Run
            $new = @{} + $script:fixture.Run
            $new.id = 456
            if ($script:latestFails) { $new.conclusion = 'failure' }
            return @{ workflow_runs = @($old, $new) }
        }
        if ($Uri.Contains('/jobs?')) {
            if (-not $Uri.Contains('/456/')) { throw 'Latest run was not selected.' }
            return @{ total_count = $(if ($script:jobsIncomplete) { 101 } else { 3 }); jobs = $script:fixture.Jobs }
        }
        throw 'Unexpected API resource.'
    }
    $observed = Get-OpticReleaseEvidence -SourceSha ('a' * 40) -Tag 'v0.1.4' -Version '0.1.4'
    if ($observed.ci_run_id -ne 456 -or $script:apiCalls -ne 3) { throw 'API provenance fixture failed.' }
    $script:cases++
    foreach ($failure in @('latestFails', 'apiFails', 'jobsIncomplete')) {
        Set-Variable -Name $failure -Scope Script -Value $true
        $denied = $false
        try { Get-OpticReleaseEvidence -SourceSha ('a' * 40) -Tag 'v0.1.4' -Version '0.1.4' | Out-Null } catch { $denied = $true }
        if (-not $denied) { throw "API fixture accepted: $failure" }
        Set-Variable -Name $failure -Scope Script -Value $false
        $script:cases++
    }
}
finally {
    $env:GITHUB_REPOSITORY = $savedRepo
    $env:GH_TOKEN = $savedToken
    Remove-Item Function:Invoke-RestMethod -ErrorAction SilentlyContinue
}
Write-Output "OPTIC_RELEASE_EVIDENCE_TESTS cases=$script:cases result=passed offline=true"
