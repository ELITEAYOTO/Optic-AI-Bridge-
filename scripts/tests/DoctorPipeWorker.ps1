param(
    [Parameter(Mandatory=$true)][string]$Doctor,
    [Parameter(Mandatory=$true)][string]$Python,
    [Parameter(Mandatory=$true)][string]$Fixture,
    [Parameter(Mandatory=$true)][string]$Root,
    [Parameter(Mandatory=$true)][string]$Mode
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# Import functions only. Never run doctor deployment/profile/workspace setup.
$tokens=$null; $errors=$null
$ast=[Management.Automation.Language.Parser]::ParseFile($Doctor,[ref]$tokens,[ref]$errors)
if($errors.Count){throw 'Doctor syntax invalid'}
foreach($name in @('Read-OpticBoundedStderr','Throw-McpTimeout','Read-McpResponse')) {
    $function=$ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name},$false)
    if($null -ne $function){ . ([scriptblock]::Create($function.Extent.Text)) }
}
if($Mode -in @('eof-init-failure','timeout-init-failure')) {
    # Deterministic failure only; no host policy/security change.
    function Add-Type { throw 'injected diagnostic initialization failure' }
}
# Warm compilation before timings and before any fixture can run.
if(Get-Command Read-OpticBoundedStderr -ErrorAction SilentlyContinue) {
    try { Read-OpticBoundedStderr -Process ([pscustomobject]@{StandardError=$null}) | Out-Null } catch {}
}
'ready' | Set-Content -LiteralPath (Join-Path $Root 'worker.ready') -Encoding ascii
$until=[Diagnostics.Stopwatch]::StartNew()
while(-not [IO.File]::Exists((Join-Path $Root 'go'))) {
    if($until.ElapsedMilliseconds -gt 8000){throw 'Outer ownership gate missing'}
    Start-Sleep -Milliseconds 5
}
$psi=[Diagnostics.ProcessStartInfo]::new()
$psi.FileName=$Python
$ready=Join-Path $Root 'ready'
$psi.ArgumentList.Add('-I');$psi.ArgumentList.Add($Fixture)
$fixtureMode = switch ($Mode) {
    'eof-init-failure' { 'eof-live' }
    'timeout-init-failure' { 'timeout-descendant' }
    default { $Mode }
}
$psi.ArgumentList.Add($fixtureMode);$psi.ArgumentList.Add($ready)
$psi.UseShellExecute=$false
$psi.RedirectStandardInput=$true
$psi.RedirectStandardOutput=$true
$psi.RedirectStandardError=$true
$p=[Diagnostics.Process]::new()
$p.StartInfo=$psi
try {
    if(-not $p.Start()){throw 'Fixture start failed'}
    if($Mode -ne 'normal'){
        $until.Restart()
        while(-not [IO.File]::Exists($ready)){
            if($until.ElapsedMilliseconds -gt 2500){throw 'Fixture did not become ready'}
            Start-Sleep -Milliseconds 5
        }
    }
    $clock=[Diagnostics.Stopwatch]::StartNew()
    $message=$null; $protocolError=$null
    if($Mode -eq 'capture-live'){
        # No tasks or handles should accumulate when a live writer remains silent.
        $beforeHandles=[Diagnostics.Process]::GetCurrentProcess().HandleCount
        foreach($attempt in 1..4){$text=Read-OpticBoundedStderr -Process $p; if($text.Length){throw 'Unexpected bytes'}}
        $afterHandles=[Diagnostics.Process]::GetCurrentProcess().HandleCount
        if($afterHandles - $beforeHandles -gt 2){throw 'Native capture grew handle count'}
    } else {
        try {$message=Read-McpResponse -Process $p -Id 1 -Timeout 1000}
        catch {$protocolError=$_.Exception.Message}
    }
    $elapsed=$clock.ElapsedMilliseconds
    if($Mode -eq 'normal'){
        if($null -eq $message -or -not $message.result.ok){throw 'Normal response lost'}
    } elseif($Mode -ne 'capture-live'){
        if($null -eq $protocolError){throw 'Protocol failure was silently accepted'}
        if($protocolError.Length -gt 4600){throw 'Error diagnostic exceeded ceiling'}
        if($Mode -eq 'eof-live' -and $protocolError -notmatch 'OPTIC_STARTUP phase=entry elapsed_ms=0'){throw 'Actual stderr evidence was discarded'}
        if($Mode -eq 'oversized' -and $protocolError -notmatch '\.\.\.\[truncated\]'){throw 'Oversized stderr was not captured and capped'}
        if($Mode -eq 'partial-utf8' -and $protocolError -notmatch [char]0xFFFD){throw 'Partial UTF-8 was not bounded-decoded'}
        if($Mode -in @('timeout-descendant','timeout-init-failure')){
            if($protocolError -notmatch 'Timed out after 1000ms'){throw 'Primary timeout lost'}
        } elseif($protocolError -notmatch 'closed before MCP response'){throw 'Primary EOF lost'}
    }
    if($elapsed -gt 3500){throw 'Bounded worker exceeded conservative guard'}
    [ordered]@{mode=$Mode;elapsed_ms=$elapsed;normal=($Mode -eq 'normal');primary_error_preserved=($null -ne $protocolError);diagnostic_chars=if($protocolError){$protocolError.Length}else{0};capture_handle_delta=if($Mode -eq 'capture-live'){$afterHandles-$beforeHandles}else{$null}} |
        ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $Root 'result.json') -Encoding utf8
} finally {
    if(-not $p.HasExited){try{$p.Kill()}catch{};try{$p.WaitForExit(1000)|Out-Null}catch{}}
    $p.Dispose()
}
