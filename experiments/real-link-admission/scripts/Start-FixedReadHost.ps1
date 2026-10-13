param([ValidateRange(1024,65535)][int]$Port = 8008, [switch]$AllowRandomTunnelHost)
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Experiments = Split-Path -Parent $Root
$Repository = Split-Path -Parent $Experiments
$Python = Join-Path $Experiments 'francestudent-probe\.venv\Scripts\python.exe'
$Binary = Join-Path $Repository 'packaging\fixed-read-test\optic-bridge.exe'
if (-not (Test-Path -LiteralPath $Python)) { throw 'Run Setup-FixedReadHost.ps1 first.' }
if (-not (Test-Path -LiteralPath $Binary)) { throw 'Use the tested CI package containing its matching binary.' }
$Manifest = Get-Content -Raw -LiteralPath (Join-Path $Repository 'FIXTURE_MANIFEST.json') | ConvertFrom-Json
if ($Manifest.binary_sha256 -notmatch '^[0-9a-fA-F]{64}$' -or $Manifest.source_commit -notmatch '^[0-9a-f]{40}$') { throw 'Invalid test package manifest.' }
if ((Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash -ne $Manifest.binary_sha256) { throw 'Binary differs from the matching test package.' }
$RunnerArguments = @((Join-Path $Root 'host_runner.py'), '--binary', $Binary, '--port', "$Port")
if ($AllowRandomTunnelHost) { $RunnerArguments += '--allow-random-tunnel-host' }
& $Python @RunnerArguments
if ($LASTEXITCODE -ne 0) { throw 'Generated fixture runner failed; no real project is supported.' }
