$ErrorActionPreference = 'Stop'
$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_TERM_COLOR = 'never'
$root = (Resolve-Path '.').Path
$evidence = $PSScriptRoot
$expectedHead = '6be5c0861c8e80c9f1ba11311fb29ddce3e50aa6'
$head = git rev-parse HEAD
if ($head -ne $expectedHead) { throw 'Unexpected source HEAD' }
if (@(Get-Process cargo,rustc -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'Another Cargo or rustc process is active'
}

function Save-BuildSnapshot([string]$Name) {
    $files = @(
        Get-ChildItem -LiteralPath 'target/debug/deps' -Filter '*.exe' -File
        Get-ChildItem -LiteralPath 'target/debug/.fingerprint' -Directory -Filter 'ontoforge-*' |
            Get-ChildItem -File
    )
    [ordered]@{
        observedUtc = [DateTime]::UtcNow.ToString('o')
        head = $head
        scope = 'workspace ontoforge-* fingerprint directories and deps exe metadata; excludes third-party fingerprint history'
        files = @($files | ForEach-Object {
            [ordered]@{
                path = [IO.Path]::GetRelativePath($root, $_.FullName)
                size = $_.Length
                mtimeUtc = $_.LastWriteTimeUtc.ToString('o')
            }
        })
    } | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $evidence "$Name.json")
}

Save-BuildSnapshot 'plan03-diagnostic-before'
$started = [DateTime]::UtcNow
$stdout = Join-Path $evidence 'plan03-diagnostic-build.stdout.jsonl'
$stderr = Join-Path $evidence 'plan03-diagnostic-build.stderr.log'
$telemetry = Join-Path $evidence 'plan03-diagnostic-processes.jsonl'
$arguments = @(
    'C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs',
    'run-with-timeout', '600', '--',
    'cargo', 'test', '--workspace', '--locked', '--no-run',
    '--message-format=json', '-vv'
)
$process = Start-Process -FilePath (Get-Command node).Source -ArgumentList $arguments `
    -WorkingDirectory $root -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$knownIds = [Collections.Generic.HashSet[int]]::new()
[void]$knownIds.Add($process.Id)
$nextSample = [DateTime]::MinValue
while (-not $process.HasExited) {
    if ([DateTime]::UtcNow -ge $nextSample) {
        $all = @(Get-CimInstance Win32_Process)
        do {
            $added = $false
            foreach ($candidate in $all) {
                if ($knownIds.Contains([int]$candidate.ParentProcessId)) {
                    if ($knownIds.Add([int]$candidate.ProcessId)) { $added = $true }
                }
            }
        } while ($added)
        $owned = @($all | Where-Object { $knownIds.Contains([int]$_.ProcessId) })
        $sample = [ordered]@{
            observedUtc = [DateTime]::UtcNow.ToString('o')
            wrapperPid = $process.Id
            processes = @($owned | ForEach-Object {
                $target = $null
                if ($_.CommandLine -match '--crate-name\s+(\S+)') { $target = $Matches[1] }
                if (-not $target -and $_.CommandLine -match '/OUT:(?:"([^"]+)"|(\S+))') {
                    $target = if ($Matches[1]) { $Matches[1] } else { $Matches[2] }
                }
                [ordered]@{
                    pid = $_.ProcessId
                    parentPid = $_.ParentProcessId
                    name = $_.Name
                    target = $target
                    cpuSeconds = ([double]$_.KernelModeTime + [double]$_.UserModeTime) / 10000000
                    workingSetBytes = $_.WorkingSetSize
                    commandLine = if ($_.Name -in @('cargo.exe', 'rustc.exe', 'link.exe', 'rust-lld.exe', 'lld-link.exe')) { $_.CommandLine } else { $null }
                }
            })
            latestExe = @(Get-ChildItem -LiteralPath 'target/debug/deps' -Filter '*.exe' -File |
                Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 3 Name,Length,LastWriteTimeUtc)
        }
        $sample | ConvertTo-Json -Depth 7 -Compress | Add-Content -LiteralPath $telemetry
        Write-Output ("sample {0}: {1}" -f $sample.observedUtc, (($sample.processes | ForEach-Object { '{0}:{1}' -f $_.name,$_.target }) -join ', '))
        $nextSample = [DateTime]::UtcNow.AddSeconds(30)
    }
    Start-Sleep -Seconds 5
    $process.Refresh()
}
$process.WaitForExit()
$code = $process.ExitCode
$remaining = @(Get-CimInstance Win32_Process | Where-Object { $knownIds.Contains([int]$_.ProcessId) })
Save-BuildSnapshot 'plan03-diagnostic-after'
$events = @(Get-Content -LiteralPath $stdout | ForEach-Object {
    try { $_ | ConvertFrom-Json -ErrorAction Stop } catch { }
})
$artifacts = @($events | Where-Object reason -eq 'compiler-artifact')
$finishedEvents = @($events | Where-Object reason -eq 'build-finished')
[ordered]@{
    command = 'node gsd-tools.cjs run-with-timeout 600 -- cargo test --workspace --locked --no-run --message-format=json -vv'
    head = $head
    started = $started.ToString('o')
    finished = [DateTime]::UtcNow.ToString('o')
    exitCode = $code
    cargoBuildJobs = 2
    timeoutSeconds = 600
    wrapperPid = $process.Id
    ownedPids = @($knownIds)
    remainingOwnedProcesses = @($remaining | Select-Object Name,ProcessId,ParentProcessId)
    artifactCount = $artifacts.Count
    freshArtifactCount = @($artifacts | Where-Object fresh -eq $true).Count
    nonfreshArtifactCount = @($artifacts | Where-Object fresh -eq $false).Count
    artifacts = @($artifacts | Select-Object package_id,target,profile,filenames,executable,fresh)
    buildFinished = $finishedEvents
} | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $evidence 'plan03-diagnostic-build.json')
Write-Output "diagnostic exit=$code artifacts=$($artifacts.Count) remaining_owned=$($remaining.Count)"
exit $code
