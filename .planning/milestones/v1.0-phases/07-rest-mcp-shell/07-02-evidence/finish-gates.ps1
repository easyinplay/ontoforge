$ErrorActionPreference = 'Stop'
$steps = @(
    @{ Name = 'fmt'; Args = @('fmt', '--all', '--', '--check'); Exit = 0 },
    @{ Name = 'history-same-store'; Args = @('test', '-p', 'ontoforge-storage', '--test', 'frozen_history', '--locked', '--', '--nocapture'); Exit = 0 },
    @{ Name = 'workspace-clippy-terminal'; Args = @('clippy', '--workspace', '--all-targets', '--all-features', '--locked', '--', '-D', 'warnings'); Exit = 0 },
    @{ Name = 'workspace-clippy-default'; Args = @('clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings'); Exit = 0 },
    @{ Name = 'embedded-pg-clippy'; Args = @('clippy', '-p', 'ontoforge-storage', '--features', 'embedded-pg', '--all-targets', '--locked', '--', '-D', 'warnings'); Exit = 0 },
    @{ Name = 'tls-native'; Args = @('tree', '--workspace', '--locked', '--features', 'ontoforge-storage/embedded-pg', '-e', 'normal,build,dev', '-i', 'native-tls'); Exit = 101 },
    @{ Name = 'tls-openssl'; Args = @('tree', '--workspace', '--locked', '--features', 'ontoforge-storage/embedded-pg', '-e', 'normal,build,dev', '-i', 'openssl-sys'); Exit = 101 },
    @{ Name = 'workspace-build'; Args = @('build', '--workspace', '--locked'); Exit = 0 }
)
foreach ($step in $steps) {
    & (Join-Path $PSScriptRoot 'run.ps1') -Name $step.Name -ArgumentList $step.Args
    if ($LASTEXITCODE -ne $step.Exit) { throw "Gate $($step.Name) expected $($step.Exit), got $LASTEXITCODE" }
    if ($step.Name.StartsWith('tls-')) {
        $output = [IO.File]::ReadAllText((Join-Path $PSScriptRoot "$($step.Name).log"))
        if (-not $output.Contains('did not match any packages')) { throw 'TLS graph check failed without proving package absence' }
    }
}
