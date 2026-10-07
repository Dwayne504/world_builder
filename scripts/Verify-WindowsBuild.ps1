[CmdletBinding()]
param(
    [string]$TargetDirectory = '',
    [switch]$CheckOnly
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'This verification script requires Windows.' }
$repository = Split-Path -Parent $PSScriptRoot
$application = Join-Path $repository 'app'

function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program failed with exit code $LASTEXITCODE. Verification stopped."
    }
}

function Assert-CleanSource {
    # Tauri may rewrite Cargo.toml with equivalent line endings on Windows.
    # Compare Git-normalized content, including the index, instead of mtime status.
    & git diff --quiet HEAD --
    if ($LASTEXITCODE -eq 1) {
        throw 'Build verification requires unchanged source. Preserve local work and use a separate worktree; do not discard it.'
    }
    if ($LASTEXITCODE -ne 0) { throw 'Cannot compare the source with the checked-out commit.' }
    $untracked = & git ls-files --others --exclude-standard
    if ($LASTEXITCODE -ne 0) { throw 'Cannot verify untracked files.' }
    if ($untracked) { throw 'Review untracked files before verifying this build.' }
}

Push-Location $repository
$previousTarget = $env:CARGO_TARGET_DIR
try {
    foreach ($program in @('git', 'node', 'npm.cmd', 'cargo', 'rustc')) {
        if (-not (Get-Command $program -ErrorAction SilentlyContinue)) {
            throw "Required tool is unavailable: $program"
        }
    }
    $revision = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot identify the checked-out commit.' }
    Assert-CleanSource
    $package = Get-Content -LiteralPath (Join-Path $application 'package.json') -Raw | ConvertFrom-Json
    $tauri = Get-Content -LiteralPath (Join-Path $application 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
    $cargoText = Get-Content -LiteralPath (Join-Path $application 'src-tauri/Cargo.toml') -Raw
    $packageSection = [regex]::Match($cargoText, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)').Groups[1].Value
    $cargoVersion = [regex]::Match($packageSection, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
    if (-not $cargoVersion -or $cargoVersion -ne $package.version -or $cargoVersion -ne $tauri.version) {
        throw 'App, Cargo and Tauri versions must agree before producing a verified build.'
    }
    $hostInfo = & rustc -vV
    if ($LASTEXITCODE -ne 0) { throw 'Cannot identify the Rust toolchain.' }
    $hostTriple = ($hostInfo | Where-Object { $_ -like 'host: *' }) -replace '^host: ', ''
    if ($hostTriple -notlike '*-pc-windows-msvc') {
        throw 'Use a Windows MSVC Rust toolchain with the Visual Studio C++ build tools and Windows SDK.'
    }
    if ($CheckOnly) {
        Write-Output "Prerequisites and version consistency passed for $revision ($cargoVersion). Compilation and desktop behavior are not yet verified."
        return
    }
    if (-not $TargetDirectory) {
        $TargetDirectory = Join-Path $application 'src-tauri/target'
    }
    $env:CARGO_TARGET_DIR = [System.IO.Path]::GetFullPath($TargetDirectory)
    Set-Location $application
    Invoke-Checked 'npm.cmd' @('ci')
    foreach ($task in @('typecheck', 'lint', 'format:check')) {
        Invoke-Checked 'npm.cmd' @('run', $task)
    }
    Invoke-Checked 'npm.cmd' @('test', '--', '--run', '--maxWorkers=4')
    Invoke-Checked 'cargo' @('fmt', '--manifest-path', 'src-tauri/Cargo.toml', '--check')
    Invoke-Checked 'cargo' @('test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml')
    Invoke-Checked 'cargo' @('clippy', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--all-targets', '--', '-D', 'warnings')
    Invoke-Checked 'cargo' @('check', '--locked', '--manifest-path', 'src-tauri/Cargo.toml')
    # Tauri runs the configured frontend production build before compiling the native executable.
    Invoke-Checked 'npm.cmd' @('run', 'tauri', '--', 'build', '--no-bundle', '--ci', '--', '--locked')
    $executable = Join-Path $env:CARGO_TARGET_DIR 'release/worldcrafter.exe'
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw 'The expected native executable was not produced.'
    }
    Set-Location $repository
    Invoke-Checked 'git' @('diff', '--check')
    Assert-CleanSource
    $report = [ordered]@{
        commit = $revision
        version = $cargoVersion
        target = $hostTriple
        node = (& node --version)
        rust = ($hostInfo -join "`n")
        executable = 'worldcrafter.exe'
        sha256 = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
        builtAtUtc = [DateTime]::UtcNow.ToString('o')
        checks = 'frontend, Rust, formatting, lint, production build, Git diff'
        limitations = @('Unsigned executable; no installer or updater', 'Native UI, DPI and upgrade acceptance require separate verification', 'Reproducible procedure; byte-for-byte reproducibility not asserted')
    }
    $reportPath = Join-Path $env:CARGO_TARGET_DIR 'release/windows-verification.json'
    $report | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $reportPath -Encoding UTF8
    Write-Output "Verified build: $executable"
    Write-Output "Commit and checksum report: $reportPath"
} finally {
    $env:CARGO_TARGET_DIR = $previousTarget
    Pop-Location
}
