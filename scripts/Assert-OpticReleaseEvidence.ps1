# Functions only: no network, build or publication occurs by dot-sourcing this file.
Set-StrictMode -Version Latest

function Assert-OpticReleaseEvidence {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Repository,
        [Parameter(Mandatory)][string]$SourceSha,
        [Parameter(Mandatory)][string]$Tag,
        [Parameter(Mandatory)][string]$Version,
        [Parameter(Mandatory)]$Comparison,
        [Parameter(Mandatory)]$Run,
        [Parameter(Mandatory)][AllowEmptyCollection()][array]$Jobs
    )
    if ($Repository -cne 'ELITEAYOTO/Optic-AI-Bridge-') { throw 'Unexpected release repository.' }
    if ($SourceSha -cnotmatch '^[0-9a-f]{40}$') { throw 'Invalid release source SHA.' }
    if ($Version -cnotmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$' -or $Tag -cne "v$Version") {
        throw 'Release tag/version mismatch or invalid version.'
    }
    # GET compare/<source>...main: ahead means main descends from source.
    if ($Comparison.status -cnotin @('ahead', 'identical')) { throw 'Release source is not an ancestor of main.' }
    if ($Comparison.merge_base_commit.sha -cne $SourceSha) { throw 'Release source ancestry is not proven.' }
    if ($Run.head_sha -cne $SourceSha -or $Run.head_branch -cne 'main' -or
        $Run.event -cne 'push' -or $Run.path -cne '.github/workflows/ci.yml' -or
        $Run.status -cne 'completed' -or $Run.conclusion -cne 'success') {
        throw 'Exact-source main CI is not completed successfully.'
    }
    if ($Jobs.Count -eq 0) { throw 'Release CI has no job evidence.' }
    foreach ($name in @('Rust / windows-latest', 'Rust / ubuntu-latest', 'Dependency policy')) {
        $matchingJobs = @($Jobs | Where-Object { $_.name -ceq $name })
        if ($matchingJobs.Count -ne 1) { throw 'Required release CI job is missing or ambiguous.' }
    }
    foreach ($job in $Jobs) {
        if ($job.status -cne 'completed' -or $job.conclusion -cne 'success') {
            throw 'Release CI contains an incomplete or unsuccessful job.'
        }
    }
    [pscustomobject]@{
        source_sha = $SourceSha
        ci_run_id = $Run.id
        ci_attempt = $Run.run_attempt
        verified_job_count = $Jobs.Count
    }
}

function Get-OpticReleaseEvidence {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$SourceSha,
        [Parameter(Mandatory)][string]$Tag,
        [Parameter(Mandatory)][string]$Version
    )
    $repository = 'ELITEAYOTO/Optic-AI-Bridge-'
    if ($env:GITHUB_REPOSITORY -cne $repository -or $SourceSha -cnotmatch '^[0-9a-f]{40}$') {
        throw 'Unexpected release repository or source SHA.'
    }
    if ([string]::IsNullOrWhiteSpace($env:GH_TOKEN)) { throw 'Release evidence token unavailable.' }
    $headers = @{ Authorization = "Bearer $env:GH_TOKEN"; Accept = 'application/vnd.github+json'; 'X-GitHub-Api-Version' = '2022-11-28' }
    $base = "https://api.github.com/repos/$repository"
    $comparison = Invoke-RestMethod -Uri "$base/compare/$SourceSha...main" -Headers $headers -TimeoutSec 30
    $runs = Invoke-RestMethod -Uri "$base/actions/runs?head_sha=$SourceSha&event=push&per_page=100" -Headers $headers -TimeoutSec 30
    # Latest matching run only; never select an older green run over a newer failure.
    $run = @($runs.workflow_runs | Where-Object {
        $_.head_sha -ceq $SourceSha -and $_.head_branch -ceq 'main' -and
        $_.event -ceq 'push' -and $_.path -ceq '.github/workflows/ci.yml'
    } | Sort-Object -Property id -Descending | Select-Object -First 1)
    if ($run.Count -ne 1) { throw 'No exact-source main CI run found in bounded evidence page.' }
    $run = $run[0]
    $jobs = Invoke-RestMethod -Uri "$base/actions/runs/$($run.id)/jobs?per_page=100" -Headers $headers -TimeoutSec 30
    if ($jobs.total_count -gt 100 -or $jobs.total_count -ne @($jobs.jobs).Count) {
        throw 'Release CI job evidence is incomplete.'
    }
    Assert-OpticReleaseEvidence -Repository $repository -SourceSha $SourceSha -Tag $Tag -Version $Version -Comparison $comparison -Run $run -Jobs @($jobs.jobs)
}
