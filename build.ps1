# StatsWidget Build Script
# powershell -ExecutionPolicy Bypass -File build.ps1
# Output: release/StatsWidget.exe

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectRoot

Write-Host "=== StatsWidget Build Script ===" -ForegroundColor Cyan

Write-Host "`n[1/5] Checking prerequisites..." -ForegroundColor Yellow

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Node.js not found. Install from https://nodejs.org" -ForegroundColor Red
    exit 1
}
Write-Host "  Node.js : $(node --version)"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Rust/Cargo not found. Install from https://rustup.rs" -ForegroundColor Red
    exit 1
}
Write-Host "  Rust    : $(cargo --version)"

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: npm not found." -ForegroundColor Red
    exit 1
}
Write-Host "  npm     : $(npm --version)"

Write-Host "`n[2/5] Installing npm dependencies..." -ForegroundColor Yellow
npm install

Write-Host "`n[3/5] Building frontend (Vite + TypeScript)..." -ForegroundColor Yellow
npm run build

Write-Host "`n[4/5] Building Tauri app..." -ForegroundColor Yellow
npx tauri build 2>&1 | ForEach-Object { Write-Host "  $_" }

Write-Host "`n[5/5] Collecting outputs..." -ForegroundColor Yellow

$releaseDir = Join-Path $projectRoot "release"
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir | Out-Null
}

$builtExe = Join-Path $projectRoot "src-tauri\target\release\StatsWidget.exe"
if (Test-Path $builtExe) {
    Copy-Item $builtExe (Join-Path $releaseDir "StatsWidget.exe") -Force
    Write-Host "  [OK] Portable EXE : release/StatsWidget.exe" -ForegroundColor Green
} else {
    $fallbackExe = Join-Path $projectRoot "src-tauri\target\release\tauri-app.exe"
    if (Test-Path $fallbackExe) {
        Copy-Item $fallbackExe (Join-Path $releaseDir "StatsWidget.exe") -Force
        Write-Host "  [OK] Portable EXE : release/StatsWidget.exe" -ForegroundColor Green
    } else {
        Write-Host "  [!!] Standalone exe not found." -ForegroundColor Red
        Write-Host "       Check src-tauri/target/release/ for the .exe" -ForegroundColor Red
    }
}

Write-Host "`n=== Build complete ===" -ForegroundColor Cyan
Write-Host "  Portable EXE : release/StatsWidget.exe"
Write-Host "  (No install required. Requires WebView2 - preinstalled on Win 10 1803+/Win 11)"
Write-Host ""
Write-Host "Press any key to open the release folder..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
Invoke-Item $releaseDir
