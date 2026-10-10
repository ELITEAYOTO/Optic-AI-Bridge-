param([int]$Port = 8000)
$ErrorActionPreference = 'Stop'
if (-not (Get-Command cloudflared -ErrorAction SilentlyContinue)) {
    throw 'cloudflared was not found. Install it with: winget install --id Cloudflare.cloudflared'
}
Write-Host 'Start the host probe first with Start-HostProbe.ps1 -AllowRandomTunnelHost.'
Write-Host 'Use the printed https://...trycloudflare.com URL with /mcp appended.'
& cloudflared tunnel --url "http://127.0.0.1:$Port"
