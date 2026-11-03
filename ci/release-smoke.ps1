[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ArtifactDir,
    [Parameter(Mandatory)][ValidateSet('x86_64-unknown-linux-gnu', 'aarch64-apple-darwin', 'x86_64-pc-windows-msvc')][string]$ExpectedTarget,
    [string]$ExpectedCommit,
    [string]$ExpectedVersion
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$ArtifactDir = [IO.Path]::GetFullPath($ArtifactDir)
& (Join-Path $PSScriptRoot 'release-package.ps1') -Target $ExpectedTarget -OutDir $ArtifactDir -VerifyOnly -ExpectedCommit $ExpectedCommit -ExpectedVersion $ExpectedVersion
$manifest = Get-Content (Get-ChildItem -LiteralPath $ArtifactDir -Filter '*.manifest.json').FullName -Raw | ConvertFrom-Json
$native = if ($IsWindows) { 'x86_64-pc-windows-msvc' } elseif ($IsMacOS) { 'aarch64-apple-darwin' } else { 'x86_64-unknown-linux-gnu' }
$architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$expectedArchitecture = if ($ExpectedTarget -eq 'aarch64-apple-darwin') { 'Arm64' } else { 'X64' }
if ($native -cne $ExpectedTarget -or $architecture -cne $expectedArchitecture) { throw 'Smoke must run on the native target OS and architecture' }
$root = Join-Path ([IO.Path]::GetTempPath()) "ontoforge-install-$([guid]::NewGuid().ToString('N'))"
$helper = Join-Path $PSScriptRoot 'windows-server.py'
$server = $null
$bridge = $null
$serverState = $null
$serverStderr = $null
$serverStdout = $null
$base = $null
$port = 0

function Assert-That([bool]$Condition, [string]$Message) {
    if (!$Condition) { throw $Message }
}
function Start-Child([string]$File, [string[]]$Arguments, [hashtable]$Environment = @{}) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $File
    $info.WorkingDirectory = $root
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    foreach ($key in $Environment.Keys) { $info.Environment[$key] = $Environment[$key] }
    $child = [Diagnostics.Process]::new()
    $child.StartInfo = $info
    $null = $child.Start()
    return $child
}
function Run-Child([string]$File, [string[]]$Arguments, [int]$Timeout = 30000) {
    $child = Start-Child $File $Arguments
    try {
        $stdout = $child.StandardOutput.ReadToEndAsync()
        $stderr = $child.StandardError.ReadToEndAsync()
        $child.StandardInput.Close()
        if (!$child.WaitForExit($Timeout)) { throw 'Child command timed out' }
        if ($child.ExitCode -ne 0) { throw "Child command failed with exit $($child.ExitCode)" }
        return $stdout.GetAwaiter().GetResult().Trim()
    } finally {
        if (!$child.HasExited) { $child.Kill($true); $child.WaitForExit() }
        $child.Dispose()
    }
}
function Port-IsOpen {
    $socket = [Net.Sockets.TcpClient]::new()
    try { $socket.Connect('127.0.0.1', $script:port); return $true } catch { return $false } finally { $socket.Dispose() }
}
function Start-Server([bool]$Reset = $false) {
    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $script:port = $listener.LocalEndpoint.Port
    $listener.Stop()
    $script:base = "http://127.0.0.1:$script:port"
    $arguments = @('serve', '--dev', '--data-dir', (Join-Path $root 'data'), '--bind', "127.0.0.1:$script:port")
    if ($Reset) { $arguments += '--reset' }
    if ($IsWindows) {
        $id = [guid]::NewGuid().ToString('N')
        $script:serverState = Join-Path $root "server-$id.json"
        $script:server = Start-Child 'python' (@('-B', $helper, 'run', '--state', $script:serverState, '--log', (Join-Path $root "server-$id.log"), '--', $binary) + $arguments)
    } else { $script:server = Start-Child $binary $arguments }
    $script:serverStdout = $script:server.StandardOutput.ReadToEndAsync()
    $script:serverStderr = $script:server.StandardError.ReadToEndAsync()
    $script:server.StandardInput.Close()
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($script:server.HasExited) { throw "Server exited during startup: $($script:server.ExitCode)" }
        try {
            $response = Invoke-WebRequest -Uri "$script:base/health" -TimeoutSec 2 -SkipHttpErrorCheck
            if ($response.StatusCode -eq 200) { return }
        } catch { }
        Start-Sleep -Milliseconds 100
    }
    throw 'Server readiness timed out'
}
function Stop-Server {
    if ($null -eq $script:server) { return }
    if ($IsWindows) {
        $null = Run-Child 'python' @('-B', $helper, 'stop', '--state', $script:serverState, '--timeout', '30') 35000
    } else {
        $null = Run-Child '/bin/kill' @('-INT', "$($script:server.Id)")
    }
    Assert-That ($script:server.WaitForExit(35000)) 'Server shutdown timed out'
    Assert-That ($script:server.ExitCode -eq 0) "Server owner exited with $($script:server.ExitCode)"
    if ($IsWindows) {
        $state = Get-Content -LiteralPath $script:serverState -Raw | ConvertFrom-Json
        Assert-That ($state.phase -ceq 'exited' -and $state.exit_code -eq 0) 'Missing durable successful server shutdown record'
    }
    Assert-That (!(Port-IsOpen)) 'Server port remains open after shutdown'
    $script:server.Dispose()
    $script:server = $null
    Write-Host 'SMOKE graceful shutdown: stop/owner exit=0, durable state (Windows), port closed'
}
function Get-Token {
    return Run-Child $binary @('dev-token', '--server-config', (Join-Path $root 'data/server.toml'), '--sub', 'alice', '--groups', 'dispatchers')
}
function Request-Json([string]$Method, [string]$Path, [string]$Token, $Body = $null, [int]$ExpectedStatus = 200) {
    $arguments = @{ Uri = "$script:base$Path"; Method = $Method; Headers = @{ Authorization = "Bearer $Token" }; TimeoutSec = 10; SkipHttpErrorCheck = $true }
    if ($null -ne $Body) { $arguments.Body = ConvertTo-Json $Body -Depth 30 -Compress; $arguments.ContentType = 'application/json' }
    try { $response = Invoke-WebRequest @arguments } catch { throw "HTTP transport failed: $Method $Path" }
    Assert-That ($response.StatusCode -eq $ExpectedStatus) "Unexpected HTTP status: $Method $Path ($($response.StatusCode))"
    return $response.Content | ConvertFrom-Json
}
function Assert-Flight($Response, [string]$Status, [int]$Delay) {
    $rows = @($Response.rows | Where-Object { $_.pk -ceq 'F-1001' })
    Assert-That ($rows.Count -eq 1) 'Query did not return exactly one F-1001'
    Assert-That ($rows[0].properties.flightNumber -ceq 'LH100' -and $rows[0].properties.status -ceq $Status -and $rows[0].properties.delayMinutes -eq $Delay) 'Unexpected Flight values'
}
function Read-Frame([string]$ExpectedId) {
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while ([DateTime]::UtcNow -lt $deadline) {
        $read = $script:bridge.StandardOutput.ReadLineAsync()
        $remaining = [Math]::Max(1, [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds)
        if (!$read.Wait($remaining)) { throw 'MCP response timed out' }
        $line = $read.GetAwaiter().GetResult()
        if ($null -eq $line) { throw 'MCP closed stdout before response' }
        $frame = $line | ConvertFrom-Json -AsHashtable
        if ($frame.ContainsKey('id') -and $frame.id -ceq $ExpectedId) {
            Assert-That (!$frame.ContainsKey('error')) 'MCP returned JSON-RPC error'
            return $frame
        }
    }
    throw 'MCP matching response not received'
}
function Send-Frame($Frame) {
    $script:bridge.StandardInput.WriteLine((ConvertTo-Json $Frame -Depth 30 -Compress))
    $script:bridge.StandardInput.Flush()
}

New-Item -ItemType Directory -Path $root | Out-Null
try {
    $archive = Join-Path $ArtifactDir $manifest.archive
    if ($IsWindows) { Expand-Archive -LiteralPath $archive -DestinationPath $root } else { tar -xzf $archive -C $root }
    $binary = Join-Path $root $manifest.binary
    Assert-That (!(Test-Path (Join-Path $root 'Cargo.toml')) -and !(Test-Path (Join-Path $root '.git'))) 'Installation cwd contains checkout files'
    Assert-That ((Run-Child $binary @('--version')) -ceq "ontoforge $($manifest.version)") 'Installed version mismatch'
    Assert-That ((Run-Child $binary @('--help')) -match 'serve') 'Installed help missing serve'
    Write-Host "SMOKE installed version/help target=$ExpectedTarget commit=$($manifest.commit) isolated_cwd=$root"
    $query = @{ set = @{ kind = 'type'; object_type = 'Flight' }; select = @('flightNumber', 'status', 'delayMinutes'); page = @{ first = 100 } }
    Start-Server
    $token = Get-Token
    Assert-Flight (Request-Json POST '/api/query' $token $query) 'scheduled' 0
    Write-Host 'SMOKE query: F-1001 LH100 scheduled/0'
    $commandId = "release-smoke-$([guid]::NewGuid().ToString('N'))"
    $receipt = Request-Json POST '/api/actions/delayFlight' $token @{ commandId = $commandId; items = @(@{ params = @{ flight = 'F-1001'; delayMinutes = 15 } }) }
    Assert-That ($receipt.commandId -ceq $commandId -and $receipt.items.Count -eq 1 -and $receipt.items[0].edits.Count -gt 0 -and $receipt.items[0].edits[0].typeName -ceq 'Flight') 'Invalid Action receipt'
    $receiptJson = ConvertTo-Json $receipt -Depth 30 -Compress
    Assert-Flight (Request-Json POST '/api/query' $token $query) 'DELAYED' 15
    $history = Request-Json GET '/api/objects/Flight/F-1001/history?limit=10' $token
    Assert-That ($history.entries.Count -eq 1 -and $history.entries[0].action -ceq 'delayFlight' -and $history.entries[0].eventId -ceq $receipt.eventId -and $history.entries[0].committedAt -ceq $receipt.committedAt) 'History does not match receipt'
    Assert-That ((ConvertTo-Json (Request-Json GET "/api/commands/$commandId" $token) -Depth 30 -Compress) -ceq $receiptJson) 'Command receipt mismatch'
    $historyJson = ConvertTo-Json $history -Depth 30 -Compress
    Write-Host 'SMOKE Action/query/history/command receipt: DELAYED/15, matching event and timestamp'
    Stop-Server
    Start-Server
    Assert-Flight (Request-Json POST '/api/query' $token $query) 'DELAYED' 15
    Assert-That ((ConvertTo-Json (Request-Json GET "/api/commands/$commandId" $token) -Depth 30 -Compress) -ceq $receiptJson) 'Restart lost receipt'
    Assert-That ((ConvertTo-Json (Request-Json GET '/api/objects/Flight/F-1001/history?limit=10' $token) -Depth 30 -Compress) -ceq $historyJson) 'Restart changed history'
    Write-Host 'SMOKE restart: old token valid, query/history/receipt retained'
    Stop-Server
    Start-Server $true
    $null = Request-Json POST '/api/query' $token $query 401
    $token = Get-Token
    Assert-Flight (Request-Json POST '/api/query' $token $query) 'scheduled' 0
    Assert-That ((Request-Json GET '/api/objects/Flight/F-1001/history?limit=10' $token).entries.Count -eq 0) 'Reset retained history'
    $null = Request-Json GET "/api/commands/$commandId" $token $null 404
    Write-Host 'SMOKE reset: old token rejected, fresh scheduled/0, empty history, old command 404'
    $bridge = Start-Child $binary @('mcp', '--server-url', "$base/mcp") @{ ONTOFORGE_TOKEN = $token }
    $bridgeErrors = $bridge.StandardError.ReadToEndAsync()
    Send-Frame @{ jsonrpc = '2.0'; id = 'init'; method = 'initialize'; params = @{ protocolVersion = '2025-11-25'; capabilities = @{}; clientInfo = @{ name = 'release-smoke'; version = '1' } } }
    $init = Read-Frame 'init'
    Assert-That ($init.result.protocolVersion -ceq '2025-11-25') 'MCP protocol mismatch'
    Send-Frame @{ jsonrpc = '2.0'; method = 'notifications/initialized' }
    Send-Frame @{ jsonrpc = '2.0'; id = 'query'; method = 'tools/call'; params = @{ name = 'query'; arguments = @{ request = $query } } }
    $frame = Read-Frame 'query'
    Assert-That (!$frame.result.ContainsKey('isError') -or !$frame.result.isError) 'MCP query tool failed'
    Assert-Flight ([pscustomobject]$frame.result.structuredContent) 'scheduled' 0
    $bridge.StandardInput.Close()
    Assert-That ($bridge.WaitForExit(15000)) 'MCP bridge did not terminate after stdin EOF'
    Assert-That ($bridge.ExitCode -eq 0) 'MCP bridge exit was not zero'
    Assert-That (!$bridgeErrors.GetAwaiter().GetResult().Contains($token)) 'MCP stderr leaked credentials'
    $bridge.Dispose()
    $bridge = $null
    Write-Host 'SMOKE stdio: initialize/query structuredContent verified, stdin EOF exit=0'
    Stop-Server
    Write-Host "SMOKE PASSED target=$ExpectedTarget version=$($manifest.version) commit=$($manifest.commit)"
} finally {
    if ($null -ne $bridge) {
        if (!$bridge.HasExited) { $bridge.Kill($true); $bridge.WaitForExit() }
        $bridge.Dispose()
    }
    try {
        if ($null -ne $server) {
            try { Stop-Server } finally {
                if ($null -ne $server) {
                    if (!$server.HasExited) { $server.Kill($true); $server.WaitForExit() }
                    $server.Dispose()
                }
            }
        }
    } finally {
        if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
    }
}
