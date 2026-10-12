param([string]$Url = 'http://127.0.0.1:8000/mcp')
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Python = Join-Path $Root '.venv\Scripts\python.exe'
if (-not (Test-Path $Python)) { throw 'Run Setup-HostProbe.ps1 first.' }
& $Python (Join-Path $Root 'test_client.py') $Url
