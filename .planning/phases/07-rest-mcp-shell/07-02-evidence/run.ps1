param([string]$Name, [string]$FilePath = 'cargo', [string[]]$ArgumentList, [string]$TargetTest)
$ErrorActionPreference = 'Stop'
$env:CARGO_TERM_COLOR = 'never'
$env:CARGO_BUILD_JOBS = '2'
$started = [DateTime]::UtcNow
$log = Join-Path $PSScriptRoot "$Name.log"
[IO.File]::WriteAllText($log, '')
& $FilePath @ArgumentList 2>&1 | Tee-Object -FilePath $log
$code = $LASTEXITCODE
$record = [ordered]@{ command = "$FilePath $($ArgumentList -join ' ')"; exitCode = $code; runStartedAt = $started.ToString('o'); runFinishedAt = [DateTime]::UtcNow.ToString('o'); cwd = (Get-Location).Path; logPath = $log; cargoBuildJobs = 2 }
if ($TargetTest) {
    $report = Get-Item (Join-Path $PSScriptRoot 'junit.xml')
    if ($report.LastWriteTimeUtc -lt $started) { throw 'stale JUnit report' }
    $xml = [IO.File]::ReadAllText($report.FullName)
    $parsed = [xml]$xml
    $targets = @($parsed.SelectNodes('//testcase') | Where-Object { $_.name -eq $TargetTest })
    if ($targets.Count -ne 1) { throw "Expected one target, got $($targets.Count)" }
    $copy = Join-Path $PSScriptRoot "$Name.junit.xml"
    Copy-Item -LiteralPath $report.FullName -Destination $copy
    $record.output = $xml
    $record.targetTest = $TargetTest
    $record.targetFile = 'crates/ontoforge-action/tests/frozen_definition.rs'
    $record.expected = 'snapshot replay returns original receipt after external Action removal'
    $record.actual = 'execute_with_snapshot returned ScopeDenied after external Action removal'
    $record.reportPath = $copy
    $record.reportModifiedAt = $report.LastWriteTimeUtc.ToString('o')
    $record.semanticAssessment = 'Real persisted command replay failed at admission snapshot assertion after another store removed Action; unchanged native JUnit contains assertion failure.'
}
$record | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $PSScriptRoot "$Name.json") -Encoding utf8
Write-Output "Evidence $Name exit=$code"
exit $code
