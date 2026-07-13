#Requires -Version 5.1
$ErrorActionPreference = "Stop"

# Windows PowerShell 5.1 may not enable TLS 1.2 by default.
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Root = Split-Path -Parent $PSScriptRoot
$Target = Join-Path $Root "src-tauri\runtime\maa"
$Temp = Join-Path ([IO.Path]::GetTempPath()) ("maa-framework-runtime-{0}.zip" -f [Guid]::NewGuid().ToString("N"))
$Api = "https://api.github.com/repos/MaaXYZ/MaaFramework/releases/latest"
$Headers = @{
    "User-Agent" = "Epic7-Summer-Helper"
    "Accept" = "application/vnd.github+json"
}

Write-Host "Checking the latest MaaFramework Windows x64 release..." -ForegroundColor Cyan
$Release = Invoke-RestMethod -Headers $Headers -Uri $Api
$Asset = $Release.assets |
    Where-Object { $_.name -match '^MAA-win-x86_64-.*\.zip$' } |
    Select-Object -First 1

if (-not $Asset) {
    $Names = ($Release.assets | ForEach-Object { $_.name }) -join ", "
    throw "No MAA-win-x86_64-*.zip asset was found. Available assets: $Names"
}

try {
    Write-Host "Downloading $($Asset.name)..." -ForegroundColor Cyan
    Invoke-WebRequest -UseBasicParsing -Headers $Headers -Uri $Asset.browser_download_url -OutFile $Temp

    if (Test-Path $Target) {
        Remove-Item $Target -Recurse -Force
    }
    New-Item -ItemType Directory -Path $Target -Force | Out-Null

    Write-Host "Extracting the runtime..." -ForegroundColor Cyan
    Expand-Archive -Path $Temp -DestinationPath $Target -Force

    $Dll = Get-ChildItem $Target -Recurse -Filter "MaaFramework.dll" -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if (-not $Dll) {
        throw "Extraction finished, but MaaFramework.dll was not found under: $Target"
    }

    Write-Host "MaaFramework runtime is ready: $($Dll.DirectoryName)" -ForegroundColor Green
}
finally {
    if (Test-Path $Temp) {
        Remove-Item $Temp -Force -ErrorAction SilentlyContinue
    }
}