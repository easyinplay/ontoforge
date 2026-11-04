[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('x86_64-unknown-linux-gnu', 'aarch64-apple-darwin', 'x86_64-pc-windows-msvc')][string]$Target,
    [Parameter(Mandatory)][string]$OutDir,
    [switch]$VerifyOnly,
    [string]$ExpectedCommit,
    [string]$ExpectedVersion
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$repo = Split-Path $PSScriptRoot -Parent
if (!$ExpectedCommit) { $ExpectedCommit = (git -C $repo rev-parse HEAD).Trim() }
if (!$ExpectedVersion) {
    $toml = Get-Content (Join-Path $repo 'Cargo.toml') -Raw
    $match = [regex]::Match($toml, '(?ms)^\[workspace\.package\]\s*\r?\n(?:(?!^\[).)*?^version\s*=\s*"([^"]+)"')
    if (!$match.Success) { throw 'Missing workspace package version' }
    $ExpectedVersion = $match.Groups[1].Value
}
if ($ExpectedCommit -cnotmatch '^[0-9a-f]{40}$' -or $ExpectedVersion -cnotmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') { throw 'Invalid expected commit or version' }
$OutDir = [IO.Path]::GetFullPath($OutDir)
$binaryName = if ($Target -eq 'x86_64-pc-windows-msvc') { 'ontoforge.exe' } else { 'ontoforge' }
$stem = "ontoforge-$ExpectedVersion-$Target"
$archiveName = if ($binaryName.EndsWith('.exe')) { "$stem.zip" } else { "$stem.tar.gz" }
$archive = Join-Path $OutDir $archiveName
$manifestPath = Join-Path $OutDir "$stem.manifest.json"
$checksumPath = Join-Path $OutDir "$archiveName.sha256"
$scratch = Join-Path ([IO.Path]::GetTempPath()) "ontoforge-package-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    if (!$VerifyOnly) {
        $actualCommit = (git -C $repo rev-parse HEAD).Trim()
        if ($actualCommit -cne $ExpectedCommit) { throw 'Expected commit differs from checked-out source' }
        $native = if ($IsWindows) { 'x86_64-pc-windows-msvc' } elseif ($IsMacOS) { 'aarch64-apple-darwin' } elseif ($IsLinux) { 'x86_64-unknown-linux-gnu' } else { throw 'Unsupported native OS' }
        $architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
        $expectedArchitecture = if ($Target -eq 'aarch64-apple-darwin') { 'Arm64' } else { 'X64' }
        $rustHost = ((rustc -vV) | Where-Object { $_ -like 'host: *' }) -replace '^host: ', ''
        if ($native -cne $Target -or $architecture -cne $expectedArchitecture -or $rustHost -cne $Target) { throw "Native target mismatch: OS=$native architecture=$architecture rust=$rustHost requested=$Target" }
        if ((Test-Path $OutDir) -and @(Get-ChildItem -Force $OutDir).Count -ne 0) { throw 'Output directory must be empty; existing artifacts are never overwritten' }
        New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
        Push-Location $repo
        try {
            $metadata = cargo metadata --no-deps --format-version 1 --locked | ConvertFrom-Json
            cargo build --release --locked -p ontoforge --target $Target
        } finally { Pop-Location }
        $binary = Join-Path $metadata.target_directory "$Target/release/$binaryName"
        if (!(Test-Path $binary) -or (Get-Item $binary).Length -eq 0) { throw 'Release binary missing or empty' }
        $versionOutput = (& $binary --version).Trim()
        if ($versionOutput -cne "ontoforge $ExpectedVersion") { throw 'Binary version does not match workspace version' }
        $stage = Join-Path $scratch 'stage'
        New-Item -ItemType Directory -Path $stage | Out-Null
        Copy-Item -LiteralPath $binary -Destination (Join-Path $stage $binaryName)
        $manifest = [ordered]@{
            schema_version = 1; version = $ExpectedVersion; commit = $actualCommit; target = $Target
            binary = $binaryName; binary_sha256 = (Get-FileHash $binary -Algorithm SHA256).Hash.ToLowerInvariant()
            archive = $archiveName; os = [Runtime.InteropServices.RuntimeInformation]::OSDescription
            architecture = $architecture; rustc = (rustc --version)
        }
        if ($IsLinux) { $manifest.glibc = ((& ldd --version) | Select-Object -First 1) }
        $manifest | ConvertTo-Json | Set-Content -LiteralPath $manifestPath -Encoding utf8NoBOM
        Copy-Item -LiteralPath $manifestPath -Destination (Join-Path $stage 'manifest.json')
        if ($IsWindows) {
            Compress-Archive -LiteralPath (Join-Path $stage $binaryName), (Join-Path $stage 'manifest.json') -DestinationPath $archive
        } else {
            chmod 755 (Join-Path $stage $binaryName)
            tar -czf $archive -C $stage $binaryName manifest.json
        }
        "$((Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant())  $archiveName" | Set-Content -LiteralPath $checksumPath -Encoding ascii
    }
    $expectedFiles = @($archiveName, "$stem.manifest.json", "$archiveName.sha256") | Sort-Object
    $actualFiles = @(Get-ChildItem -LiteralPath $OutDir -Force | ForEach-Object { $_.Name }) | Sort-Object
    if (@(Compare-Object $expectedFiles $actualFiles).Count -ne 0) { throw 'Artifact directory must contain exactly archive, manifest and checksum' }
    foreach ($path in @($archive, $manifestPath, $checksumPath)) {
        if (!(Test-Path -LiteralPath $path -PathType Leaf) -or (Get-Item -LiteralPath $path).Length -eq 0) { throw 'Artifact file missing or empty' }
    }
    $checksum = (Get-Content -LiteralPath $checksumPath -Raw).Trim()
    $actualHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($checksum -cne "$actualHash  $archiveName") { throw 'Archive SHA256 mismatch' }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.schema_version -ne 1 -or $manifest.version -cne $ExpectedVersion -or $manifest.commit -cne $ExpectedCommit -or $manifest.target -cne $Target -or $manifest.binary -cne $binaryName -or $manifest.archive -cne $archiveName -or $manifest.binary_sha256 -cnotmatch '^[0-9a-f]{64}$') { throw 'Manifest identity mismatch' }
    $expectedArchitecture = if ($Target -eq 'aarch64-apple-darwin') { 'Arm64' } else { 'X64' }
    if ($manifest.architecture -cne $expectedArchitecture) { throw 'Manifest architecture mismatch' }
    if ([string]::IsNullOrWhiteSpace($manifest.os) -or [string]::IsNullOrWhiteSpace($manifest.rustc)) { throw 'Manifest build environment is incomplete' }
    if ($Target -eq 'x86_64-unknown-linux-gnu' -and [string]::IsNullOrWhiteSpace($manifest.glibc)) { throw 'Manifest glibc baseline is missing' }
    $extracted = Join-Path $scratch 'extracted'
    New-Item -ItemType Directory -Path $extracted | Out-Null
    if ($binaryName.EndsWith('.exe')) {
        $zip = [IO.Compression.ZipFile]::OpenRead($archive)
        try { $entries = @($zip.Entries | ForEach-Object { $_.FullName }) } finally { $zip.Dispose() }
        if ($entries.Count -ne 2 -or @(Compare-Object @($binaryName, 'manifest.json') $entries).Count -ne 0) { throw 'Unexpected archive entries' }
        Expand-Archive -LiteralPath $archive -DestinationPath $extracted
    } else {
        $entries = @(tar -tzf $archive)
        $listing = @(tar -tvzf $archive)
        if ($entries.Count -ne 2 -or @(Compare-Object @($binaryName, 'manifest.json') $entries).Count -ne 0 -or @($listing | Where-Object { !($_.StartsWith('-')) }).Count -ne 0) { throw 'Unexpected archive entries or non-regular files' }
        tar -xzf $archive -C $extracted
    }
    if ((Get-FileHash (Join-Path $extracted 'manifest.json')).Hash -cne (Get-FileHash $manifestPath).Hash -or (Get-FileHash (Join-Path $extracted $binaryName)).Hash.ToLowerInvariant() -cne $manifest.binary_sha256) { throw 'Extracted binary or manifest mismatch' }
    Write-Host "PACKAGE VERIFIED target=$Target version=$ExpectedVersion commit=$ExpectedCommit archive=$archiveName sha256=$actualHash"
} finally {
    if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
}
