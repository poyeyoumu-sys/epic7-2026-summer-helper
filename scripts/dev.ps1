$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not (Get-ChildItem "src-tauri\runtime\maa" -Recurse -Filter "MaaFramework.dll" -ErrorAction SilentlyContinue)) {
    & "$PSScriptRoot\download-maa-runtime.ps1"
}
if (-not (Test-Path "node_modules")) { npm install }
npm run tauri dev
