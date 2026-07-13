$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

function Stop-WithMessage {
    param([string]$Message)
    Write-Host ""
    Write-Host $Message -ForegroundColor Red
    exit 1
}

function Test-CommandExists {
    param([string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Get-CommandPath {
    param([string[]]$Names)

    foreach ($Name in $Names) {
        $Command = Get-Command $Name -ErrorAction SilentlyContinue
        if ($null -ne $Command) {
            if ($Command.Source) {
                return $Command.Source
            }
            if ($Command.Path) {
                return $Command.Path
            }
            return $Name
        }
    }

    return $null
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Epic7 2026 Summer Helper Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Set-Location $ProjectRoot
Write-Host "Project root: $ProjectRoot"

if (-not (Test-Path (Join-Path $ProjectRoot "package.json"))) {
    Stop-WithMessage "package.json was not found in the project root."
}

if (-not (Test-Path (Join-Path $ProjectRoot "src-tauri\Cargo.toml"))) {
    Stop-WithMessage "src-tauri\Cargo.toml was not found."
}

$NodeExe = Get-CommandPath @("node.exe", "node")
$NpmCmd = Get-CommandPath @("npm.cmd")

if (-not $NodeExe) {
    Stop-WithMessage "Node.js was not found. Install Node.js LTS and reopen the terminal."
}

if (-not $NpmCmd) {
    Stop-WithMessage "npm.cmd was not found. Reinstall Node.js LTS and reopen the terminal."
}

if (-not (Test-CommandExists "cargo")) {
    $CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    $CargoExe = Join-Path $CargoBin "cargo.exe"

    if (Test-Path $CargoExe) {
        $env:PATH = "$CargoBin;$env:PATH"
        Write-Host "Cargo was added to PATH for this build session."
    }
    else {
        Stop-WithMessage "Cargo was not found. Install Rust with rustup and reopen the terminal."
    }
}

if (-not (Test-CommandExists "rustc")) {
    Stop-WithMessage "rustc was not found. Install the Rust MSVC toolchain with rustup."
}

Write-Host ""
Write-Host "Environment versions:" -ForegroundColor Yellow
& $NodeExe --version
& $NpmCmd --version
rustc --version
cargo --version

if (-not (Test-Path (Join-Path $ProjectRoot "node_modules"))) {
    Write-Host ""
    Write-Host "node_modules was not found. Running npm install..." -ForegroundColor Yellow
    & $NpmCmd install
    if ($LASTEXITCODE -ne 0) {
        Stop-WithMessage "npm install failed."
    }
}

$MaaRuntime = Join-Path $ProjectRoot "src-tauri\runtime\maa"
if (-not (Test-Path $MaaRuntime)) {
    Write-Host ""
    Write-Host "Warning: MaaFramework runtime directory was not found:" -ForegroundColor Yellow
    Write-Host $MaaRuntime -ForegroundColor Yellow

    $DownloadScript = Join-Path $ScriptDir "download-maa-runtime.ps1"
    if (Test-Path $DownloadScript) {
        Write-Host "Downloading MaaFramework runtime..." -ForegroundColor Yellow
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $DownloadScript
        if ($LASTEXITCODE -ne 0) {
            Stop-WithMessage "MaaFramework runtime download failed."
        }
    }
    else {
        Stop-WithMessage "MaaFramework runtime is missing and the download script was not found."
    }
}

Write-Host ""
Write-Host "Starting Tauri release build..." -ForegroundColor Green
Write-Host "npm executable: $NpmCmd" -ForegroundColor DarkGray

& $NpmCmd run tauri -- build
$BuildExitCode = $LASTEXITCODE

if ($BuildExitCode -ne 0) {
    Stop-WithMessage "Tauri build failed. Review the error messages above."
}

$BundleDir = Join-Path $ProjectRoot "src-tauri\target\release\bundle"

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Build completed successfully." -ForegroundColor Green
Write-Host "Bundle directory:" -ForegroundColor Green
Write-Host $BundleDir -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green