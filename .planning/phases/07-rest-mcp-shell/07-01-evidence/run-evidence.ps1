param(
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][string]$FilePath,
    [Parameter(Mandatory)][string[]]$ArgumentList,
    [string]$TargetTest,
    [string]$TargetFile,
    [string]$Expected,
    [string]$Actual
)

$ErrorActionPreference = 'Stop'
$evidenceRoot = $PSScriptRoot
$env:CARGO_TERM_COLOR = 'never'
$started = [DateTime]::UtcNow
$logPath = Join-Path $evidenceRoot "$Name.log"
& $FilePath @ArgumentList 2>&1 | Tee-Object -FilePath $logPath
$commandExit = $LASTEXITCODE
$finished = [DateTime]::UtcNow
$metadata = [ordered]@{
    command = "$FilePath $($ArgumentList -join ' ')"
    exitCode = $commandExit
    runStartedAt = $started.ToString('o')
    runFinishedAt = $finished.ToString('o')
    logPath = $logPath
    cwd = (Get-Location).Path
}
if ($TargetTest) {
    $report = Get-Item (Join-Path $evidenceRoot 'junit.xml')
    if ($report.LastWriteTimeUtc -lt $started) { throw 'JUnit report is older than the run' }
    $xml = [System.IO.File]::ReadAllText($report.FullName)
    $parsed = [xml]$xml
    $targets = @($parsed.SelectNodes('//testcase') | Where-Object { $_.name -eq $TargetTest })
    if ($targets.Count -ne 1) { throw "Expected exactly one JUnit target, got $($targets.Count)" }
    $savedReport = Join-Path $evidenceRoot "$Name.junit.xml"
    Copy-Item -LiteralPath $report.FullName -Destination $savedReport
    $metadata.output = $xml
    $metadata.targetTest = $TargetTest
    $metadata.targetFile = $TargetFile
    $metadata.expected = $Expected
    $metadata.actual = $Actual
    $metadata.reportPath = $savedReport
    $metadata.reportModifiedAt = $report.LastWriteTimeUtc.ToString('o')
    $metadata.semanticAssessment = 'Target executed and failed on the planned behavior assertion; inspect the unchanged JUnit failure output.'
}
$metadata | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $evidenceRoot "$Name.json") -Encoding utf8
Write-Output "Evidence: $Name exit=$commandExit start=$($started.ToString('o')) end=$($finished.ToString('o'))"
exit $commandExit
