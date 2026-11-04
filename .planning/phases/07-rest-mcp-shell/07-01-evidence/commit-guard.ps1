$ErrorActionPreference = 'Stop'
$PinnedRoot = 'D:/DataSpellProjects/ontoforge'
$ActualRoot = git rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) { throw 'FATAL: actual root capture failed' }
$PinnedTop = git -C $PinnedRoot rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) { throw 'FATAL: pinned root capture failed' }
if ($ActualRoot -ne $PinnedTop) { throw 'FATAL: executor root does not match orchestrator root pin' }
$ExpectedBranch = 'gsd/phase-07-rest-mcp-shell'
$ActualBranch = git branch --show-current
if ($ActualBranch -ne $ExpectedBranch) { throw 'FATAL: wrong branch' }
$HeadRef = git symbolic-ref --quiet HEAD
if ($LASTEXITCODE -ne 0 -or !$HeadRef) { throw 'FATAL: refusing to commit detached HEAD' }
$Protected = node C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs query git.base-branch --is-protected $ActualBranch --raw
if ($LASTEXITCODE -ne 0 -or $Protected -ne 'false') { throw 'FATAL: refusing protected or unknown branch' }
$Sequential = $true
if (!$Sequential) { throw 'FATAL: this executor requires sequential dispatch' }
$ledger = Join-Path (git rev-parse --git-dir) 'gsd-plan-head-before-07-01'
if (!(Test-Path -LiteralPath $ledger)) { git rev-parse HEAD | Set-Content -LiteralPath $ledger }
Write-Output "Commit guard passed: $ActualRoot $ActualBranch SEQUENTIAL=$Sequential"
