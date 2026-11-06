$ErrorActionPreference = 'Stop'
$root = 'D:/DataSpellProjects/ontoforge'
$evidence = Join-Path $root '.planning/milestones/v1.0-audit/evidence/2026-11-05'
Set-Location $root
$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_TARGET_DIR = 'C:/Users/easyi/AppData/Local/Temp/ontoforge-reverify-060019f-20261105'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TERM_COLOR = 'never'
$env:NO_COLOR = '1'
$env:PYTHONDONTWRITEBYTECODE = '1'
$env:INSTA_UPDATE = 'no'
New-Item -ItemType Directory -Force $env:CARGO_TARGET_DIR | Out-Null
$results = [System.Collections.Generic.List[object]]::new()
function Invoke-Recorded {
    param([string]$Name, [string]$Executable, [string[]]$Arguments)
    $log = Join-Path $evidence "reverify-$Name.log"
    if (Test-Path -LiteralPath $log) { throw "Refusing to overwrite $log" }
    $started = [DateTime]::UtcNow.ToString('o')
    "started_utc=$started`nhead=$(git rev-parse HEAD)`ncommand=$Executable $($Arguments -join ' ')`nCARGO_BUILD_JOBS=$env:CARGO_BUILD_JOBS`nCARGO_TARGET_DIR=$env:CARGO_TARGET_DIR" | Tee-Object -FilePath $log
    & $Executable @Arguments 2>&1 | ForEach-Object { "$_" } | Tee-Object -FilePath $log -Append
    $code = $LASTEXITCODE
    "exit_code=$code`nended_utc=$([DateTime]::UtcNow.ToString('o'))" | Tee-Object -FilePath $log -Append
    $results.Add([ordered]@{name=$Name;command="$Executable $($Arguments -join ' ')";exit_code=$code;log=$log;started_utc=$started;ended_utc=[DateTime]::UtcNow.ToString('o')})
    $results | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $evidence 'reverify-local-results.json') -Encoding utf8
}
if (Get-Process cargo,rustc -ErrorAction SilentlyContinue) { throw 'Cargo/rustc already running; no concurrent Cargo allowed' }
Invoke-Recorded 'workspace-discovery-c' 'cargo' @('test','--workspace','--locked','--','--list')
Invoke-Recorded 'workspace' 'cargo' @('test','--workspace','--locked','--','--test-threads=1')
Invoke-Recorded 'sqlite-atomic-discovery' 'cargo' @('test','-p','ontoforge-storage','--features','test-support','--test','conformance_sqlite','--locked','action_atomic_','--','--list')
Invoke-Recorded 'sqlite-atomic' 'cargo' @('test','-p','ontoforge-storage','--features','test-support','--test','conformance_sqlite','--locked','action_atomic_','--','--nocapture','--test-threads=1')
Invoke-Recorded 'tls-hermetic' 'python' @('-B','-m','unittest','discover','-s','ci','-p','test_tls_deps.py','-v')
Invoke-Recorded 'tls-real' 'C:/Program Files/Git/bin/bash.exe' @('ci/check-tls-deps.sh')
