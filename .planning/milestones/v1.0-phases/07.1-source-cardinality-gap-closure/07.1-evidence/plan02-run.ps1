param([string]$Name, [string[]]$CargoArgs)
$ErrorActionPreference = 'Stop'
$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_TERM_COLOR = 'never'
$started = [DateTime]::UtcNow
$head = git rev-parse HEAD
$log = Join-Path $PSScriptRoot "$Name.log"
& cargo @CargoArgs 2>&1 | Tee-Object -FilePath $log
$code = $LASTEXITCODE
[ordered]@{
    command = "cargo $($CargoArgs -join ' ')"
    exitCode = $code
    started = $started.ToString('o')
    finished = [DateTime]::UtcNow.ToString('o')
    head = $head
    cwd = (Get-Location).Path
    cargoBuildJobs = 2
    logPath = $log
} | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot "$Name.json") -Encoding utf8
exit $code
