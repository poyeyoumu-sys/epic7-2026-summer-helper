$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$Required = @(
    "package.json",
    "src\App.tsx",
    "src-tauri\Cargo.toml",
    "src-tauri\resources\config\recognition_config.json",
    "src-tauri\resources\tools\adbutils\binaries\adb.exe",
    "src-tauri\src\controller\maa.rs",
    "src-tauri\src\game\runner.rs"
)

foreach ($Path in $Required) {
    if (-not (Test-Path $Path)) {
        throw "缺少必要文件：$Path"
    }
}

if (-not (Test-Path "node_modules")) {
    npm ci
}

npm run build

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    cargo check --manifest-path src-tauri\Cargo.toml
} else {
    Write-Warning "未安装 Rust，已跳过 cargo check。"
}

Write-Host "项目检查完成。" -ForegroundColor Green
